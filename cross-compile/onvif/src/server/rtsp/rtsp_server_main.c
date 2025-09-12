/**
 * @file rtsp_server_main.c
 * @brief RTSP Server Main Implementation
 * 
 * This file contains the core RTSP server functionality including server creation,
 * request handling, and the main server loop.
 */

#include "rtsp_server.h"
#include "platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <time.h>
#include <signal.h>
#include <pthread.h>
#include <sys/select.h>

/* Forward declarations */
static void* rtsp_accept_thread(void *arg);
static void* rtsp_session_thread(void *arg);
static void* rtsp_encoder_thread(void *arg);
static void* rtsp_audio_thread(void *arg);
static void* rtsp_timeout_thread(void *arg);
static int rtsp_handle_request(rtsp_session_t *session, const char *request);
static int rtsp_send_response(rtsp_session_t *session, int code, const char *headers, const char *body);
static int rtsp_parse_method(const char *line);
static void h264_extract_sps_pps(rtsp_server_t *server, const uint8_t *buf, size_t len);
static void base64_encode(const uint8_t *input, size_t input_len, char *output, size_t output_len);
static void get_local_ip_address(char *ip_str, size_t ip_str_size);

/* Global session counter for unique session IDs */
static uint32_t g_session_counter = 1;
static pthread_mutex_t g_session_counter_mutex = PTHREAD_MUTEX_INITIALIZER;

/* ==================== Server Management Functions ==================== */

/**
 * Create RTSP server
 */
rtsp_server_t* rtsp_server_create(rtsp_stream_config_t *config) {
    if (!config) {
        platform_log_error("Invalid config parameter\n");
        return NULL;
    }
    
    rtsp_server_t *server = calloc(1, sizeof(rtsp_server_t));
    if (!server) {
        platform_log_error("Failed to allocate server memory\n");
        return NULL;
    }
    
    // Copy configuration
    memcpy(&server->config, config, sizeof(rtsp_stream_config_t));
    
    // Initialize mutex
    if (pthread_mutex_init(&server->sessions_mutex, NULL) != 0) {
        platform_log_error("Failed to initialize sessions mutex\n");
        free(server);
        return NULL;
    }
    
    server->listen_sockfd = -1;
    server->running = false;
    server->sessions = NULL;
    server->vi_handle = config->vi_handle;
    
    // Initialize audio fields
    server->ai_handle = NULL;
    server->aenc_handle = NULL;
    server->audio_encoder_initialized = false;
    server->audio_frames_sent = 0;
    
    // Initialize H.264 parameter sets
    server->h264_sps_b64[0] = '\0';
    server->h264_pps_b64[0] = '\0';
    
    // Initialize authentication
    rtsp_auth_init(&server->auth_config);
    
    // Initialize SDP session
    sdp_init_session(&server->sdp_session, "RTSP Server", NULL);
    
    platform_log_notice("RTSP server created for stream: %s on port %d (Audio: %s)\n", 
                    config->stream_path, config->port, 
                    config->audio_enabled ? "enabled" : "disabled");
    
    return server;
}

/**
 * Start RTSP server
 */
int rtsp_server_start(rtsp_server_t *server) {
    if (!server) {
        platform_log_error("Invalid server parameter\n");
        return -1;
    }
    
    if (server->running) {
        platform_log_warning("Server is already running\n");
        return 0;
    }
    
    // Create socket
    server->listen_sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (server->listen_sockfd < 0) {
        platform_log_error("Failed to create socket: %s\n", strerror(errno));
        return -1;
    }
    
    // Set socket options
    int opt = 1;
    if (setsockopt(server->listen_sockfd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
        platform_log_error("Failed to set socket options: %s\n", strerror(errno));
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Bind socket
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons(server->config.port);
    
    if (bind(server->listen_sockfd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        platform_log_error("Failed to bind socket: %s\n", strerror(errno));
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Listen for connections
    if (listen(server->listen_sockfd, 10) < 0) {
        platform_log_error("Failed to listen on socket: %s\n", strerror(errno));
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Setup encoder
    if (rtsp_setup_encoder(server) < 0) {
        platform_log_error("Failed to setup encoder\n");
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Setup audio encoder if enabled
    if (server->config.audio_enabled) {
        if (rtsp_setup_audio_encoder(server) < 0) {
            platform_log_error("Failed to setup audio encoder\n");
            rtsp_cleanup_encoder(server);
            close(server->listen_sockfd);
            server->listen_sockfd = -1;
            return -1;
        }
    }
    
    // Start threads
    server->running = true;
    
    if (pthread_create(&server->accept_thread, NULL, rtsp_accept_thread, server) != 0) {
        platform_log_error("Failed to create accept thread: %s\n", strerror(errno));
        server->running = false;
        rtsp_cleanup_encoder(server);
        rtsp_cleanup_audio_encoder(server);
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    if (pthread_create(&server->encoder_thread, NULL, rtsp_encoder_thread, server) != 0) {
        platform_log_error("Failed to create encoder thread: %s\n", strerror(errno));
        server->running = false;
        pthread_cancel(server->accept_thread);
        rtsp_cleanup_encoder(server);
        rtsp_cleanup_audio_encoder(server);
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Start audio thread if enabled
    if (server->config.audio_enabled) {
        if (pthread_create(&server->audio_thread, NULL, rtsp_audio_thread, server) != 0) {
            platform_log_error("Failed to create audio thread: %s\n", strerror(errno));
            server->running = false;
            pthread_cancel(server->accept_thread);
            pthread_cancel(server->encoder_thread);
            rtsp_cleanup_encoder(server);
            rtsp_cleanup_audio_encoder(server);
            close(server->listen_sockfd);
            server->listen_sockfd = -1;
            return -1;
        }
    }
    
    // Start timeout thread
    if (pthread_create(&server->timeout_thread, NULL, rtsp_timeout_thread, server) != 0) {
        platform_log_error("Failed to create timeout thread: %s\n", strerror(errno));
        server->running = false;
        pthread_cancel(server->accept_thread);
        pthread_cancel(server->encoder_thread);
        if (server->config.audio_enabled) {
            pthread_cancel(server->audio_thread);
        }
        rtsp_cleanup_encoder(server);
        rtsp_cleanup_audio_encoder(server);
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    platform_log_notice("RTSP server started on port %d\n", server->config.port);
    return 0;
}

/**
 * Stop RTSP server
 */
int rtsp_server_stop(rtsp_server_t *server) {
    if (!server) {
        return -1;
    }
    
    if (!server->running) {
        return 0;
    }
    
    server->running = false;
    
    // Close listening socket
    if (server->listen_sockfd >= 0) {
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
    }
    
    // Wait for threads to finish
    pthread_join(server->accept_thread, NULL);
    pthread_join(server->encoder_thread, NULL);
    if (server->config.audio_enabled) {
        pthread_join(server->audio_thread, NULL);
    }
    pthread_join(server->timeout_thread, NULL);
    
    // Cleanup all sessions
    rtsp_session_cleanup_all(server);
    
    // Cleanup encoder
    rtsp_cleanup_encoder(server);
    rtsp_cleanup_audio_encoder(server);
    
    platform_log_notice("RTSP server stopped\n");
    return 0;
}

/**
 * Destroy RTSP server
 */
int rtsp_server_destroy(rtsp_server_t *server) {
    if (!server) {
        return -1;
    }
    
    rtsp_server_stop(server);
    
    // Cleanup authentication
    rtsp_auth_cleanup(&server->auth_config);
    
    // Cleanup SDP session
    sdp_cleanup_session(&server->sdp_session);
    
    pthread_mutex_destroy(&server->sessions_mutex);
    free(server);
    return 0;
}

/**
 * Get server statistics
 */
int rtsp_server_get_stats(rtsp_server_t *server, uint64_t *bytes_sent, 
                         uint32_t *frames_sent, uint32_t *sessions_count) {
    if (!server) {
        return -1;
    }
    
    if (bytes_sent) *bytes_sent = server->bytes_sent;
    if (frames_sent) *frames_sent = server->frames_sent;
    if (sessions_count) *sessions_count = rtsp_session_get_count(server);
    
    return 0;
}

/**
 * Get stream URL
 */
int rtsp_server_get_stream_url(rtsp_server_t *server, char *url, size_t url_size) {
    if (!server || !url || url_size < 32) {
        return -1;
    }
    
    char ip_str[64];
    get_local_ip_address(ip_str, sizeof(ip_str));
    
    snprintf(url, url_size, "rtsp://%s:%d%s", ip_str, server->config.port, server->config.stream_path);
    return 0;
}

/* ==================== Thread Functions ==================== */

/**
 * Accept thread - handles new connections
 */
static void* rtsp_accept_thread(void *arg) {
    rtsp_server_t *server = (rtsp_server_t*)arg;
    
    platform_log_notice("RTSP accept thread started\n");
    
    while (server->running) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);
        
        int client_sockfd = accept(server->listen_sockfd, (struct sockaddr*)&client_addr, &client_len);
        if (client_sockfd < 0) {
            if (server->running) {
                platform_log_error("Failed to accept connection: %s\n", strerror(errno));
            }
            continue;
        }
        
        // Create new session
        rtsp_session_t *session = calloc(1, sizeof(rtsp_session_t));
        if (!session) {
            platform_log_error("Failed to allocate session memory\n");
            close(client_sockfd);
            continue;
        }
        
        session->sockfd = client_sockfd;
        memcpy(&session->addr, &client_addr, sizeof(client_addr));
        session->state = RTSP_STATE_INIT;
        session->active = true;
        session->audio_enabled = server->config.audio_enabled;
        
        // Initialize session timeout
        session->timeout_seconds = RTSP_SESSION_TIMEOUT_SEC;
        session->last_activity = time(NULL);
        session->created_time = time(NULL);
        
        // Initialize authentication
        session->authenticated = false;
        session->auth_username[0] = '\0';
        session->auth_nonce[0] = '\0';
        
        // Initialize headers
        session->headers = NULL;
        
        // Thread-safe session ID generation
        pthread_mutex_lock(&g_session_counter_mutex);
        uint32_t session_id = g_session_counter++;
        pthread_mutex_unlock(&g_session_counter_mutex);
        snprintf(session->session_id, sizeof(session->session_id), "%u", session_id);
        
        // Allocate buffers
        session->recv_buffer = malloc(RTSP_BUFFER_SIZE);
        session->send_buffer = malloc(RTSP_BUFFER_SIZE);
        if (!session->recv_buffer || !session->send_buffer) {
            platform_log_error("Failed to allocate session buffers\n");
            free(session->recv_buffer);
            free(session->send_buffer);
            free(session);
            close(client_sockfd);
            continue;
        }
        
        session->recv_pos = 0;
        session->server = server;
        
        // Add to server sessions
        rtsp_session_add(server, session);
        
        // Create session thread
        if (pthread_create(&session->thread, NULL, rtsp_session_thread, session) != 0) {
            platform_log_error("Failed to create session thread: %s\n", strerror(errno));
            rtsp_session_remove(server, session);
            rtsp_cleanup_session(session);
            free(session);
            close(client_sockfd);
            continue;
        }
        
        char client_ip[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &client_addr.sin_addr, client_ip, INET_ADDRSTRLEN);
        platform_log_notice("New RTSP connection from %s:%d (Session: %s)\n", 
                           client_ip, ntohs(client_addr.sin_port), session->session_id);
    }
    
    platform_log_notice("RTSP accept thread finished\n");
    return NULL;
}

/**
 * Session thread - handles individual client sessions
 */
static void* rtsp_session_thread(void *arg) {
    rtsp_session_t *session = (rtsp_session_t*)arg;
    
    platform_log_notice("RTSP session thread started for session %s\n", session->session_id);
    
    while (session->active && session->sockfd >= 0) {
        // Check for timeout
        if (rtsp_session_has_timed_out(session)) {
            platform_log_notice("Session %s timed out\n", session->session_id);
            break;
        }
        
        // Receive data
        ssize_t bytes = recv(session->sockfd, session->recv_buffer + session->recv_pos, 
                           RTSP_BUFFER_SIZE - session->recv_pos - 1, 0);
        
        if (bytes <= 0) {
            if (bytes == 0) {
                platform_log_notice("Client disconnected (session %s)\n", session->session_id);
            } else {
                platform_log_error("Failed to receive data: %s\n", strerror(errno));
            }
            break;
        }
        
        session->recv_pos += bytes;
        session->recv_buffer[session->recv_pos] = '\0';
        
        // Look for complete RTSP request (ends with \r\n\r\n)
        char *end_marker = strstr(session->recv_buffer, "\r\n\r\n");
        if (end_marker) {
            *end_marker = '\0';
            
            // Update session activity
            rtsp_update_session_activity(session);
            
            // Validate request format
            if (rtsp_validate_request(session->recv_buffer, end_marker - session->recv_buffer) < 0) {
                platform_log_error("Invalid RTSP request format\n");
                rtsp_send_error_response(session, RTSP_BAD_REQUEST, "Invalid request format");
                break;
            }
            
            // Parse headers
            if (rtsp_parse_headers_enhanced(session->recv_buffer, &session->headers) < 0) {
                platform_log_error("Failed to parse RTSP headers\n");
                rtsp_send_error_response(session, RTSP_BAD_REQUEST, "Failed to parse headers");
                break;
            }
            
            // Handle the request
            if (rtsp_handle_request(session, session->recv_buffer) < 0) {
                platform_log_error("Failed to handle RTSP request\n");
                break;
            }
            
            // Move remaining data to beginning of buffer
            size_t processed = end_marker + 4 - session->recv_buffer;
            size_t remaining = session->recv_pos - processed;
            if (remaining > 0) {
                memmove(session->recv_buffer, end_marker + 4, remaining);
            }
            session->recv_pos = remaining;
        }
        
        // Check if buffer is full
        if (session->recv_pos >= RTSP_BUFFER_SIZE - 1) {
            platform_log_error("RTSP buffer overflow\n");
            rtsp_send_error_response(session, RTSP_BAD_REQUEST, "Request too large");
            break;
        }
    }
    
    // Cleanup session
    rtsp_session_remove(session->server, session);
    rtsp_cleanup_session(session);
    free(session);
    
    platform_log_notice("RTSP session thread finished\n");
    return NULL;
}

/**
 * Encoder thread - handles video encoding and RTP transmission
 */
static void* rtsp_encoder_thread(void *arg) {
    rtsp_server_t *server = (rtsp_server_t*)arg;
    
    platform_log_notice("RTSP encoder thread started\n");
    
    while (server->running) {
        // Get video stream using platform abstraction
        platform_venc_stream_t stream;
        if (platform_venc_get_stream(server->venc_handle, &stream, 1000) == PLATFORM_SUCCESS) {
            // Extract SPS/PPS from keyframes
            if (!server->h264_sps_b64[0] || !server->h264_pps_b64[0]) {
                h264_extract_sps_pps(server, stream.data, stream.len);
            }
            
            // Send to all active sessions
            pthread_mutex_lock(&server->sessions_mutex);
            rtsp_session_t *session = server->sessions;
            while (session) {
                if (session->active && session->state == RTSP_STATE_PLAYING) {
                    rtsp_send_rtp_packet(session, stream.data, stream.len, stream.timestamp);
                }
                session = session->next;
            }
            pthread_mutex_unlock(&server->sessions_mutex);
            
            // Update statistics
            server->bytes_sent += stream.len;
            server->frames_sent++;
            
            // Release stream using platform abstraction
            platform_venc_release_stream(server->venc_handle, &stream);
        }
    }
    
    platform_log_notice("RTSP encoder thread finished\n");
    return NULL;
}

/**
 * Audio thread - handles audio encoding and RTP transmission
 */
static void* rtsp_audio_thread(void *arg) {
    rtsp_server_t *server = (rtsp_server_t*)arg;
    
    platform_log_notice("RTSP audio thread started\n");
    
    while (server->running) {
        // Get audio stream using platform abstraction
        platform_aenc_stream_t stream;
        if (platform_aenc_get_stream(server->aenc_handle, &stream, 1000) == PLATFORM_SUCCESS) {
            // Send to all active sessions
            pthread_mutex_lock(&server->sessions_mutex);
            rtsp_session_t *session = server->sessions;
            while (session) {
                if (session->active && session->state == RTSP_STATE_PLAYING && session->audio_enabled) {
                    rtsp_send_audio_rtp_packet(session, stream.data, stream.len, stream.timestamp);
                }
                session = session->next;
            }
            pthread_mutex_unlock(&server->sessions_mutex);
            
            // Update statistics
            server->audio_frames_sent++;
            
            // Release stream using platform abstraction
            platform_aenc_release_stream(server->aenc_handle, &stream);
        }
    }
    
    platform_log_notice("RTSP audio thread finished\n");
    return NULL;
}

/**
 * Timeout thread - cleans up timed out sessions
 */
static void* rtsp_timeout_thread(void *arg) {
    rtsp_server_t *server = (rtsp_server_t*)arg;
    
    platform_log_notice("RTSP timeout thread started\n");
    
    while (server->running) {
        // Cleanup timeout sessions every 10 seconds
        rtsp_session_cleanup_timeout_sessions(server);
        platform_sleep_ms(10000);
    }
    
    platform_log_notice("RTSP timeout thread finished\n");
    return NULL;
}

/* ==================== Request Handling Functions ==================== */

/**
 * Handle RTSP request
 */
static int rtsp_handle_request(rtsp_session_t *session, const char *request) {
    char method_line[512];
    char *line_end = strchr(request, '\r');
    if (!line_end) {
        line_end = strchr(request, '\n');
    }
    
    if (!line_end) {
        platform_log_error("Invalid RTSP request format\n");
        return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
    }
    
    size_t line_len = line_end - request;
    if (line_len >= sizeof(method_line)) {
        line_len = sizeof(method_line) - 1;
    }
    
    strncpy(method_line, request, line_len);
    method_line[line_len] = '\0';
    
    // Parse CSeq
    const char *cseq_line = strstr(request, "CSeq:");
    if (cseq_line) {
        session->cseq = atoi(cseq_line + 5);
    }
    
    // Parse method
    int method = rtsp_parse_method(method_line);
    
    // Check authentication for methods that require it
    if (method != RTSP_METHOD_OPTIONS && rtsp_auth_require_auth(session)) {
        const char *auth_header = strstr(request, "Authorization:");
        if (!auth_header) {
            return rtsp_handle_auth_required(session);
        }
        
        // Validate authentication
        if (session->server->auth_config.auth_type == RTSP_AUTH_BASIC) {
            if (rtsp_auth_validate_basic(session, auth_header) < 0) {
                return rtsp_handle_auth_required(session);
            }
        } else if (session->server->auth_config.auth_type == RTSP_AUTH_DIGEST) {
            // Extract URI from method line
            char *uri_start = strchr(method_line, ' ');
            if (uri_start) {
                uri_start++;
                char *uri_end = strchr(uri_start, ' ');
                if (uri_end) {
                    *uri_end = '\0';
                    if (rtsp_auth_validate_digest(session, auth_header, method_line, uri_start) < 0) {
                        return rtsp_handle_auth_required(session);
                    }
                }
            }
        }
    }
    
    switch (method) {
        case RTSP_METHOD_OPTIONS:
            return rtsp_send_response(session, RTSP_OK, 
                "Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE, GET_PARAMETER, SET_PARAMETER\r\n", NULL);
            
        case RTSP_METHOD_DESCRIBE: {
            // Extract URI
            char uri[RTSP_MAX_URI_LEN];
            if (sscanf(method_line, "DESCRIBE %s", uri) != 1) {
                return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
            }
            
            strncpy(session->uri, uri, sizeof(session->uri) - 1);
            session->uri[sizeof(session->uri) - 1] = '\0';
            
            // Generate enhanced SDP using SDP functions
            char sdp[4096];
            rtsp_server_t *srv = session->server;
            
            // Initialize SDP session for this request
            struct sdp_session sdp_session;
            char ip_str[64];
            get_local_ip_address(ip_str, sizeof(ip_str));
            char origin[128];
            snprintf(origin, sizeof(origin), "- %u %u IN IP4 %s", 
                    (uint32_t)time(NULL), (uint32_t)time(NULL), ip_str);
            
            sdp_init_session(&sdp_session, "RTSP Session", origin);
            
            // Set connection information
            snprintf(sdp_session.connection, sizeof(sdp_session.connection), "IN IP4 %s", ip_str);
            
            // Add video media
            sdp_add_media(&sdp_session, SDP_MEDIA_VIDEO, 0, "RTP/AVP", RTP_PT_H264, "H264", 90000, 0);
            sdp_set_media_control(&sdp_session, SDP_MEDIA_VIDEO, "track0");
            
            // Set H.264 format parameters
            const char *sps = (srv && srv->h264_sps_b64[0]) ? srv->h264_sps_b64 : NULL;
            const char *pps = (srv && srv->h264_pps_b64[0]) ? srv->h264_pps_b64 : NULL;
            char fmtp[512];
            if (sps && pps) {
                snprintf(fmtp, sizeof(fmtp), "packetization-mode=1;profile-level-id=42001e;sprop-parameter-sets=%s,%s", 
                        sps, pps);
            } else {
                snprintf(fmtp, sizeof(fmtp), "packetization-mode=1;profile-level-id=42001e");
            }
            sdp_set_media_fmtp(&sdp_session, SDP_MEDIA_VIDEO, fmtp);
            
            // Add audio media if enabled
            if (session->audio_enabled) {
                sdp_add_media(&sdp_session, SDP_MEDIA_AUDIO, 0, "RTP/AVP", RTP_PT_PCMA, "PCMA", 8000, 1);
                sdp_set_media_control(&sdp_session, SDP_MEDIA_AUDIO, "track1");
            }
            
            // Generate SDP content
            int sdp_len = sdp_generate(&sdp_session, sdp, sizeof(sdp));
            if (sdp_len < 0) {
                sdp_cleanup_session(&sdp_session);
                return rtsp_send_response(session, RTSP_INTERNAL_ERROR, NULL, NULL);
            }
            
            // Cleanup SDP session
            sdp_cleanup_session(&sdp_session);
                
            char headers[512];
            snprintf(headers, sizeof(headers),
                "Content-Type: application/sdp\r\n"
                "Content-Length: %zu\r\n", strlen(sdp));
            
            return rtsp_send_response(session, RTSP_OK, headers, sdp);
        }
        
        case RTSP_METHOD_SETUP: {
            // Extract URI and transport
            char uri[RTSP_MAX_URI_LEN];
            char transport[256];
            if (sscanf(method_line, "SETUP %s", uri) != 1) {
                return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
            }
            
            // Find Transport header
            const char *transport_line = strstr(request, "Transport:");
            if (!transport_line) {
                return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
            }
            
            // Parse transport parameters
            if (strstr(transport_line, "RTP/AVP/TCP")) {
                // TCP transport
                if (rtsp_init_rtp_session(session) < 0) {
                    return rtsp_send_response(session, RTSP_INTERNAL_ERROR, NULL, NULL);
                }
                session->rtp_session.transport = RTP_TRANSPORT_TCP;
                session->rtp_session.tcp_channel_rtp = 0;
                session->rtp_session.tcp_channel_rtcp = 1;
                
                char headers[512];
                snprintf(headers, sizeof(headers),
                    "Transport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n"
                    "Session: %s\r\n", session->session_id);
                return rtsp_send_response(session, RTSP_OK, headers, NULL);
                
            } else if (strstr(transport_line, "RTP/AVP")) {
                // UDP transport
                if (rtsp_init_rtp_session(session) < 0) {
                    return rtsp_send_response(session, RTSP_INTERNAL_ERROR, NULL, NULL);
                }
                session->rtp_session.transport = RTP_TRANSPORT_UDP;
                
                char headers[512];
                char ip_str[64];
                get_local_ip_address(ip_str, sizeof(ip_str));
                snprintf(headers, sizeof(headers),
                    "Transport: RTP/AVP;unicast;client_port=%d-%d;server_port=%d-%d;ssrc=%u\r\n"
                    "Session: %s\r\n",
                    session->rtp_session.rtp_port, session->rtp_session.rtcp_port,
                    session->rtp_session.rtp_port, session->rtp_session.rtcp_port,
                    session->rtp_session.ssrc, session->session_id);
                return rtsp_send_response(session, RTSP_OK, headers, NULL);
            } else {
                return rtsp_send_response(session, RTSP_UNSUPPORTED_TRANSPORT, NULL, NULL);
            }
        }
        
        case RTSP_METHOD_PLAY: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            session->state = RTSP_STATE_PLAYING;
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_PAUSE: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            session->state = RTSP_STATE_READY;
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_TEARDOWN: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            session->active = false;
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_GET_PARAMETER: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_SET_PARAMETER: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_ANNOUNCE: {
            return rtsp_send_error_response(session, RTSP_NOT_IMPLEMENTED, "ANNOUNCE method not implemented");
        }
        
        case RTSP_METHOD_RECORD: {
            return rtsp_send_error_response(session, RTSP_NOT_IMPLEMENTED, "RECORD method not implemented");
        }
        
        case RTSP_METHOD_REDIRECT: {
            return rtsp_send_error_response(session, RTSP_NOT_IMPLEMENTED, "REDIRECT method not implemented");
        }
        
        default:
            return rtsp_send_response(session, RTSP_METHOD_NOT_ALLOWED, NULL, NULL);
    }
}

/**
 * Send RTSP response
 */
static int rtsp_send_response(rtsp_session_t *session, int code, const char *headers, const char *body) {
    if (!session || session->sockfd < 0) return -1;
    
    const char *status_text = "OK";
    switch (code) {
        case RTSP_OK: status_text = "OK"; break;
        case RTSP_BAD_REQUEST: status_text = "Bad Request"; break;
        case RTSP_UNAUTHORIZED: status_text = "Unauthorized"; break;
        case RTSP_NOT_FOUND: status_text = "Not Found"; break;
        case RTSP_METHOD_NOT_ALLOWED: status_text = "Method Not Allowed"; break;
        case RTSP_NOT_ACCEPTABLE: status_text = "Not Acceptable"; break;
        case RTSP_SESSION_NOT_FOUND: status_text = "Session Not Found"; break;
        case RTSP_INTERNAL_ERROR: status_text = "Internal Server Error"; break;
        case RTSP_NOT_IMPLEMENTED: status_text = "Not Implemented"; break;
        case RTSP_UNSUPPORTED_TRANSPORT: status_text = "Unsupported Transport"; break;
        default: status_text = "Unknown"; break;
    }
    
    int len = snprintf(session->send_buffer, RTSP_BUFFER_SIZE,
        "RTSP/1.0 %d %s\r\n"
        "CSeq: %d\r\n"
        "Server: RTSP Server/1.0\r\n"
        "%s%s%s",
        code, status_text, session->cseq,
        headers ? headers : "",
        body ? "\r\n" : "",
        body ? body : "");
    
    if (len >= RTSP_BUFFER_SIZE) {
        platform_log_error("Response too large\n");
        return -1;
    }
    
    ssize_t sent = send(session->sockfd, session->send_buffer, len, 0);
    if (sent != len) {
        platform_log_error("Failed to send response: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

/**
 * Send RTSP error response
 */
int rtsp_send_error_response(rtsp_session_t *session, enum rtsp_error_code code, const char *message) {
    char headers[512];
    if (message) {
        snprintf(headers, sizeof(headers), "Warning: %s\r\n", message);
    } else {
        headers[0] = '\0';
    }
    
    return rtsp_send_response(session, code, headers, NULL);
}

/**
 * Parse RTSP method
 */
static int rtsp_parse_method(const char *line) {
    if (strncmp(line, "OPTIONS", 7) == 0) return RTSP_METHOD_OPTIONS;
    if (strncmp(line, "DESCRIBE", 8) == 0) return RTSP_METHOD_DESCRIBE;
    if (strncmp(line, "SETUP", 5) == 0) return RTSP_METHOD_SETUP;
    if (strncmp(line, "PLAY", 4) == 0) return RTSP_METHOD_PLAY;
    if (strncmp(line, "PAUSE", 5) == 0) return RTSP_METHOD_PAUSE;
    if (strncmp(line, "TEARDOWN", 8) == 0) return RTSP_METHOD_TEARDOWN;
    if (strncmp(line, "GET_PARAMETER", 13) == 0) return RTSP_METHOD_GET_PARAMETER;
    if (strncmp(line, "SET_PARAMETER", 13) == 0) return RTSP_METHOD_SET_PARAMETER;
    if (strncmp(line, "ANNOUNCE", 8) == 0) return RTSP_METHOD_ANNOUNCE;
    if (strncmp(line, "RECORD", 6) == 0) return RTSP_METHOD_RECORD;
    if (strncmp(line, "REDIRECT", 8) == 0) return RTSP_METHOD_REDIRECT;
    return RTSP_METHOD_UNKNOWN;
}

/* ==================== Utility Functions ==================== */

/**
 * Get local IP address
 */
static void get_local_ip_address(char *ip_str, size_t ip_str_size) {
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        strcpy(ip_str, "127.0.0.1");
        return;
    }
    
    struct sockaddr_in addr;
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = inet_addr("8.8.8.8");
    addr.sin_port = htons(80);
    
    if (connect(sock, (struct sockaddr*)&addr, sizeof(addr)) == 0) {
        socklen_t len = sizeof(addr);
        getsockname(sock, (struct sockaddr*)&addr, &len);
        inet_ntop(AF_INET, &addr.sin_addr, ip_str, ip_str_size);
    } else {
        strcpy(ip_str, "127.0.0.1");
    }
    
    close(sock);
}

/**
 * Extract H.264 SPS/PPS from stream
 */
static void h264_extract_sps_pps(rtsp_server_t *server, const uint8_t *buf, size_t len) {
    for (size_t i = 0; i < len - 4; i++) {
        if (buf[i] == 0x00 && buf[i+1] == 0x00 && buf[i+2] == 0x00 && buf[i+3] == 0x01) {
            uint8_t nal_type = buf[i+4] & 0x1F;
            size_t nal_start = i + 4;
            size_t j = i + 4;
            while (j < len - 4) {
                if (buf[j] == 0x00 && buf[j+1] == 0x00 && buf[j+2] == 0x00 && buf[j+3] == 0x01) {
                    break;
                }
                j++;
            }
            size_t nal_end = j;
            if (nal_type == 7 && server->h264_sps_b64[0] == '\0') {
                base64_encode(buf + nal_start, nal_end - nal_start, server->h264_sps_b64, sizeof(server->h264_sps_b64));
            } else if (nal_type == 8 && server->h264_pps_b64[0] == '\0') {
                base64_encode(buf + nal_start, nal_end - nal_start, server->h264_pps_b64, sizeof(server->h264_pps_b64));
            }
            if (server->h264_sps_b64[0] && server->h264_pps_b64[0]) return;
            i = j;
        }
    }
}

/**
 * Base64 encode
 */
static void base64_encode(const uint8_t *input, size_t input_len, char *output, size_t output_len) {
    const char base64_chars[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    size_t i = 0, j = 0;
    
    while (i < input_len && j < output_len - 1) {
        uint32_t a = i < input_len ? input[i++] : 0;
        uint32_t b = i < input_len ? input[i++] : 0;
        uint32_t c = i < input_len ? input[i++] : 0;
        
        uint32_t triple = (a << 16) | (b << 8) | c;
        
        if (j < output_len - 1) output[j++] = base64_chars[(triple >> 18) & 0x3F];
        if (j < output_len - 1) output[j++] = base64_chars[(triple >> 12) & 0x3F];
        if (j < output_len - 1) output[j++] = base64_chars[(triple >> 6) & 0x3F];
        if (j < output_len - 1) output[j++] = base64_chars[triple & 0x3F];
    }
    
    // Add padding
    while (j < output_len - 1 && (input_len * 4) % 3 != 0) {
        output[j++] = '=';
    }
    
    output[j] = '\0';
}

/* ==================== Missing Function Implementations ==================== */

/**
 * Cleanup session
 */
void rtsp_cleanup_session(rtsp_session_t *session) {
    if (!session) return;
    
    // Close socket
    if (session->sockfd >= 0) {
        close(session->sockfd);
        session->sockfd = -1;
    }
    
    // Cleanup RTP sessions
    rtsp_cleanup_rtp_session(session);
    rtsp_cleanup_audio_rtp_session(session);
    
    // Cleanup headers
    struct rtsp_header *header = session->headers;
    while (header) {
        struct rtsp_header *next = header->next;
        free(header);
        header = next;
    }
    session->headers = NULL;
    
    // Free buffers
    free(session->recv_buffer);
    free(session->send_buffer);
    session->recv_buffer = NULL;
    session->send_buffer = NULL;
}

/**
 * Validate RTSP request
 */
int rtsp_validate_request(const char *request, size_t request_len) {
    if (!request || request_len < 4) return -1;
    
    // Check for basic RTSP request format
    if (strncmp(request, "RTSP/", 5) == 0) {
        // Response format
        return 0;
    }
    
    // Check for method format (METHOD URI RTSP/1.0)
    const char *space1 = strchr(request, ' ');
    if (!space1) return -1;
    
    const char *space2 = strchr(space1 + 1, ' ');
    if (!space2) return -1;
    
    if (strncmp(space2 + 1, "RTSP/1.0", 8) != 0) return -1;
    
    return 0;
}

/**
 * Parse headers enhanced
 */
int rtsp_parse_headers_enhanced(const char *request, struct rtsp_header **headers) {
    if (!request || !headers) return -1;
    
    *headers = NULL;
    struct rtsp_header *last = NULL;
    
    const char *line_start = strstr(request, "\r\n");
    if (!line_start) return -1;
    line_start += 2; // Skip \r\n
    
    while (line_start && *line_start != '\0') {
        const char *line_end = strstr(line_start, "\r\n");
        if (!line_end) break;
        
        if (line_end == line_start) break; // Empty line, end of headers
        
        // Find colon separator
        const char *colon = strchr(line_start, ':');
        if (!colon || colon >= line_end) {
            line_start = line_end + 2;
            continue;
        }
        
        // Create new header
        struct rtsp_header *header = malloc(sizeof(struct rtsp_header));
        if (!header) return -1;
        
        // Copy name
        size_t name_len = colon - line_start;
        if (name_len >= sizeof(header->name)) name_len = sizeof(header->name) - 1;
        strncpy(header->name, line_start, name_len);
        header->name[name_len] = '\0';
        
        // Copy value (skip colon and whitespace)
        const char *value_start = colon + 1;
        while (*value_start == ' ' || *value_start == '\t') value_start++;
        
        size_t value_len = line_end - value_start;
        if (value_len >= sizeof(header->value)) value_len = sizeof(header->value) - 1;
        strncpy(header->value, value_start, value_len);
        header->value[value_len] = '\0';
        
        header->next = NULL;
        
        // Add to list
        if (last) {
            last->next = header;
        } else {
            *headers = header;
        }
        last = header;
        
        line_start = line_end + 2;
    }
    
    return 0;
}
