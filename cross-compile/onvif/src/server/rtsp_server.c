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

#include "rtsp_server.h"
#include "ak_common.h"
#include "ak_thread.h"
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/* RTSP response codes */
#define RTSP_OK                 200
#define RTSP_BAD_REQUEST        400
#define RTSP_UNAUTHORIZED       401
#define RTSP_NOT_FOUND          404
#define RTSP_METHOD_NOT_ALLOWED 405
#define RTSP_NOT_ACCEPTABLE     406
#define RTSP_SESSION_NOT_FOUND  454
#define RTSP_INTERNAL_ERROR     500

/* RTP payload types */
#define RTP_PT_H264             96
#define RTP_PT_H265             97
#define RTP_PT_PCMU             0      /* G.711 μ-law */
#define RTP_PT_PCMA             8      /* G.711 A-law */
#define RTP_PT_AAC              97     /* AAC */

/* Global session counter for generating session IDs */
static uint32_t g_session_counter = 1;

/* Forward declarations */
static void* rtsp_accept_thread(void *arg);
static void* rtsp_session_thread(void *arg);
static void* rtsp_encoder_thread(void *arg);
static void* rtsp_audio_thread(void *arg);
static int rtsp_handle_request(rtsp_session_t *session, const char *request);
static int rtsp_send_response(rtsp_session_t *session, int code, const char *headers, const char *body);
static int rtsp_parse_method(const char *line);
static int rtsp_setup_encoder(rtsp_server_t *server);
static void rtsp_cleanup_encoder(rtsp_server_t *server);
static int rtsp_setup_audio_encoder(rtsp_server_t *server);
static void rtsp_cleanup_audio_encoder(rtsp_server_t *server);
static int rtsp_init_rtp_session(rtsp_session_t *session);
static void rtsp_cleanup_rtp_session(rtsp_session_t *session);
static int rtsp_init_audio_rtp_session(rtsp_session_t *session);
static void rtsp_cleanup_audio_rtp_session(rtsp_session_t *session);
static int rtsp_send_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, bool marker);
static int rtsp_send_audio_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, bool marker, int payload_type);

/**
 * Create RTSP server
 */
rtsp_server_t* rtsp_server_create(rtsp_stream_config_t *config)
{
    if (!config) {
        ak_print_error("Invalid config parameter\n");
        return NULL;
    }
    
    rtsp_server_t *server = calloc(1, sizeof(rtsp_server_t));
    if (!server) {
        ak_print_error("Failed to allocate server memory\n");
        return NULL;
    }
    
    // Copy configuration
    memcpy(&server->config, config, sizeof(rtsp_stream_config_t));
    
    // Initialize mutex
    if (pthread_mutex_init(&server->sessions_mutex, NULL) != 0) {
        ak_print_error("Failed to initialize sessions mutex\n");
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
    server->audio_stream_handle = NULL;
    server->audio_encoder_initialized = false;
    server->audio_frames_sent = 0;
    
    ak_print_notice("RTSP server created for stream: %s on port %d (Audio: %s)\n", 
                    config->stream_path, config->port, 
                    config->audio_enabled ? "enabled" : "disabled");
    
    return server;
}

/**
 * Start RTSP server
 */
int rtsp_server_start(rtsp_server_t *server)
{
    if (!server) {
        ak_print_error("Invalid server parameter\n");
        return -1;
    }
    
    if (server->running) {
        ak_print_warning("RTSP server already running\n");
        return 0;
    }
    
    // Create listening socket
    server->listen_sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (server->listen_sockfd < 0) {
        ak_print_error("Failed to create listening socket: %s\n", strerror(errno));
        return -1;
    }
    
    // Set socket options
    int opt = 1;
    if (setsockopt(server->listen_sockfd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
        ak_print_warning("Failed to set SO_REUSEADDR: %s\n", strerror(errno));
    }
    
    // Bind socket
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons(server->config.port);
    
    if (bind(server->listen_sockfd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        ak_print_error("Failed to bind socket to port %d: %s\n", 
                       server->config.port, strerror(errno));
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Start listening
    if (listen(server->listen_sockfd, RTSP_MAX_CLIENTS) < 0) {
        ak_print_error("Failed to listen on socket: %s\n", strerror(errno));
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Setup video encoder
    if (rtsp_setup_encoder(server) < 0) {
        ak_print_error("Failed to setup video encoder\n");
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Setup audio encoder if enabled
    if (server->config.audio_enabled) {
        if (rtsp_setup_audio_encoder(server) < 0) {
            ak_print_error("Failed to setup audio encoder\n");
            rtsp_cleanup_encoder(server);
            close(server->listen_sockfd);
            server->listen_sockfd = -1;
            return -1;
        }
    }
    
    server->running = true;
    
    // Start accept thread
    if (pthread_create(&server->accept_thread, NULL, rtsp_accept_thread, server) != 0) {
        ak_print_error("Failed to create accept thread: %s\n", strerror(errno));
        server->running = false;
        rtsp_cleanup_encoder(server);
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Start encoder thread
    if (pthread_create(&server->encoder_thread, NULL, rtsp_encoder_thread, server) != 0) {
        ak_print_error("Failed to create encoder thread: %s\n", strerror(errno));
        server->running = false;
        pthread_cancel(server->accept_thread);
        rtsp_cleanup_encoder(server);
        if (server->config.audio_enabled) {
            rtsp_cleanup_audio_encoder(server);
        }
        close(server->listen_sockfd);
        server->listen_sockfd = -1;
        return -1;
    }
    
    // Start audio thread if enabled
    if (server->config.audio_enabled) {
        if (pthread_create(&server->audio_thread, NULL, rtsp_audio_thread, server) != 0) {
            ak_print_error("Failed to create audio thread: %s\n", strerror(errno));
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
    
    ak_print_notice("RTSP server started on port %d, stream: %s\n", 
                    server->config.port, server->config.stream_path);
    
    return 0;
}

/**
 * Stop RTSP server
 */
int rtsp_server_stop(rtsp_server_t *server)
{
    if (!server || !server->running) {
        return 0;
    }
    
    ak_print_notice("Stopping RTSP server...\n");
    
    server->running = false;
    
    // Close listening socket to break accept loop
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
    
    // Cleanup all sessions
    pthread_mutex_lock(&server->sessions_mutex);
    rtsp_session_t *session = server->sessions;
    while (session) {
        rtsp_session_t *next = session->next;
        if (session->active) {
            session->active = false;
            pthread_join(session->thread, NULL);
        }
        rtsp_cleanup_rtp_session(session);
        if (session->audio_enabled) {
            rtsp_cleanup_audio_rtp_session(session);
        }
        if (session->recv_buffer) free(session->recv_buffer);
        if (session->send_buffer) free(session->send_buffer);
        free(session);
        session = next;
    }
    server->sessions = NULL;
    pthread_mutex_unlock(&server->sessions_mutex);
    
    // Cleanup encoder
    rtsp_cleanup_encoder(server);
    
    // Cleanup audio encoder
    if (server->config.audio_enabled) {
        rtsp_cleanup_audio_encoder(server);
    }
    
    ak_print_notice("RTSP server stopped\n");
    
    return 0;
}

/**
 * Destroy RTSP server
 */
int rtsp_server_destroy(rtsp_server_t *server)
{
    if (!server) {
        return -1;
    }
    
    rtsp_server_stop(server);
    pthread_mutex_destroy(&server->sessions_mutex);
    free(server);
    return 0;
}

/**
 * Get server statistics
 */
int rtsp_server_get_stats(rtsp_server_t *server, uint64_t *bytes_sent, 
                         uint32_t *frames_sent, uint32_t *sessions_count)
{
    if (!server) {
        return -1;
    }
    
    pthread_mutex_lock(&server->sessions_mutex);
    if (bytes_sent) *bytes_sent = server->bytes_sent;
    if (frames_sent) *frames_sent = server->frames_sent;
    if (sessions_count) *sessions_count = server->sessions_count;
    pthread_mutex_unlock(&server->sessions_mutex);
    
    return 0;
}

/**
 * Get stream URL
 */
int rtsp_server_get_stream_url(rtsp_server_t *server, char *url, size_t url_size)
{
    if (!server || !url || url_size == 0) {
        return -1;
    }
    
    snprintf(url, url_size, "rtsp://192.168.1.100:%d%s", 
             server->config.port, server->config.stream_path);
    
    return 0;
}

/**
 * Accept thread - handles new client connections
 */
static void* rtsp_accept_thread(void *arg)
{
    rtsp_server_t *server = (rtsp_server_t*)arg;
    
    ak_print_notice("RTSP accept thread started\n");
    
    while (server->running) {
        struct sockaddr_in client_addr;
        socklen_t addr_len = sizeof(client_addr);
        
        int client_sockfd = accept(server->listen_sockfd, 
                                  (struct sockaddr*)&client_addr, &addr_len);
        
        if (client_sockfd < 0) {
            if (server->running) {
                ak_print_error("Failed to accept client connection: %s\n", strerror(errno));
            }
            continue;
        }
        
        ak_print_notice("New RTSP client connected from %s:%d\n", 
                        inet_ntoa(client_addr.sin_addr), ntohs(client_addr.sin_port));
        
        // Create new session
        rtsp_session_t *session = calloc(1, sizeof(rtsp_session_t));
        if (!session) {
            ak_print_error("Failed to allocate session memory\n");
            close(client_sockfd);
            continue;
        }
        
        session->sockfd = client_sockfd;
        memcpy(&session->addr, &client_addr, sizeof(client_addr));
        session->state = RTSP_STATE_INIT;
        session->active = true;
        session->audio_enabled = server->config.audio_enabled; // Initialize audio support based on server config
        snprintf(session->session_id, sizeof(session->session_id), "%u", g_session_counter++);
        
        // Allocate buffers
        session->recv_buffer = malloc(RTSP_BUFFER_SIZE);
        session->send_buffer = malloc(RTSP_BUFFER_SIZE);
        if (!session->recv_buffer || !session->send_buffer) {
            ak_print_error("Failed to allocate session buffers\n");
            if (session->recv_buffer) free(session->recv_buffer);
            if (session->send_buffer) free(session->send_buffer);
            free(session);
            close(client_sockfd);
            continue;
        }
        
        // Add to sessions list
        pthread_mutex_lock(&server->sessions_mutex);
        session->next = server->sessions;
        server->sessions = session;
        server->sessions_count++;
        pthread_mutex_unlock(&server->sessions_mutex);
        
        // Start session thread
        if (pthread_create(&session->thread, NULL, rtsp_session_thread, session) != 0) {
            ak_print_error("Failed to create session thread: %s\n", strerror(errno));
            
            // Remove from list and cleanup
            pthread_mutex_lock(&server->sessions_mutex);
            if (server->sessions == session) {
                server->sessions = session->next;
            } else {
                rtsp_session_t *prev = server->sessions;
                while (prev && prev->next != session) {
                    prev = prev->next;
                }
                if (prev) {
                    prev->next = session->next;
                }
            }
            server->sessions_count--;
            pthread_mutex_unlock(&server->sessions_mutex);
            
            free(session->recv_buffer);
            free(session->send_buffer);
            free(session);
            close(client_sockfd);
        }
    }
    
    ak_print_notice("RTSP accept thread finished\n");
    return NULL;
}
/**
 * Session thread - handles RTSP protocol for one client
 */
static void* rtsp_session_thread(void *arg)
{
    rtsp_session_t *session = (rtsp_session_t*)arg;
    
    ak_print_notice("RTSP session thread started for client %s:%d\n", 
                    inet_ntoa(session->addr.sin_addr), ntohs(session->addr.sin_port));
    
    session->recv_pos = 0;
    
    while (session->active) {
        // Receive data
        ssize_t received = recv(session->sockfd, 
                               session->recv_buffer + session->recv_pos,
                               RTSP_BUFFER_SIZE - session->recv_pos - 1, 0);
        
        if (received <= 0) {
            if (received < 0 && errno != ECONNRESET) {
                ak_print_error("Failed to receive data: %s\n", strerror(errno));
            }
            break;
        }
        
        session->recv_pos += received;
        session->recv_buffer[session->recv_pos] = '\0';
        
        // Look for complete RTSP request (ends with \r\n\r\n)
        char *end_marker = strstr(session->recv_buffer, "\r\n\r\n");
        if (end_marker) {
            *end_marker = '\0';
            
            // Handle the request
            if (rtsp_handle_request(session, session->recv_buffer) < 0) {
                ak_print_error("Failed to handle RTSP request\n");
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
        
        // Prevent buffer overflow
        if (session->recv_pos >= RTSP_BUFFER_SIZE - 1) {
            ak_print_warning("RTSP receive buffer overflow, resetting\n");
            session->recv_pos = 0;
        }
    }
    
    // Cleanup
    if (session->sockfd >= 0) {
        close(session->sockfd);
        session->sockfd = -1;
    }
    
    rtsp_cleanup_rtp_session(session);
    
    ak_print_notice("RTSP session thread finished for client %s:%d\n", 
                    inet_ntoa(session->addr.sin_addr), ntohs(session->addr.sin_port));
    
    return NULL;
}

/**
 * Handle RTSP request
 */
static int rtsp_handle_request(rtsp_session_t *session, const char *request)
{
    char method_line[512];
    char *line_end = strchr(request, '\r');
    if (!line_end) {
        line_end = strchr(request, '\n');
    }
    
    if (!line_end) {
        ak_print_error("Invalid RTSP request format\n");
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
    
    switch (method) {
        case RTSP_METHOD_OPTIONS:
            return rtsp_send_response(session, RTSP_OK, 
                "Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE\r\n", NULL);
            
        case RTSP_METHOD_DESCRIBE: {
            // Extract URI
            char uri[RTSP_MAX_URI_LEN];
            if (sscanf(method_line, "DESCRIBE %s", uri) != 1) {
                return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
            }
            
            strncpy(session->uri, uri, sizeof(session->uri) - 1);
            session->uri[sizeof(session->uri) - 1] = '\0';
            
            // Generate SDP
            char sdp[2048];
            if (session->audio_enabled) {
                // SDP with both video and audio
                snprintf(sdp, sizeof(sdp),
                    "v=0\r\n"
                    "o=- %u %u IN IP4 0.0.0.0\r\n"
                    "s=RTSP Session\r\n"
                    "c=IN IP4 0.0.0.0\r\n"
                    "t=0 0\r\n"
                    "m=video 0 RTP/AVP %d\r\n"
                    "a=rtpmap:%d H264/90000\r\n"
                    "a=fmtp:%d profile-level-id=42001e\r\n"
                    "a=control:track0\r\n"
                    "m=audio 0 RTP/AVP %d\r\n"
                    "a=rtpmap:%d PCMA/8000\r\n"
                    "a=control:track1\r\n",
                    (uint32_t)time(NULL), (uint32_t)time(NULL),
                    RTP_PT_H264, RTP_PT_H264, RTP_PT_H264,
                    RTP_PT_PCMA, RTP_PT_PCMA);
            } else {
                // SDP with video only
                snprintf(sdp, sizeof(sdp),
                    "v=0\r\n"
                    "o=- %u %u IN IP4 0.0.0.0\r\n"
                    "s=RTSP Session\r\n"
                    "c=IN IP4 0.0.0.0\r\n"
                    "t=0 0\r\n"
                    "m=video 0 RTP/AVP %d\r\n"
                    "a=rtpmap:%d H264/90000\r\n"
                    "a=fmtp:%d profile-level-id=42001e\r\n"
                    "a=control:track0\r\n",
                    (uint32_t)time(NULL), (uint32_t)time(NULL),
                    RTP_PT_H264, RTP_PT_H264, RTP_PT_H264);
            }
                
            char headers[512];
            snprintf(headers, sizeof(headers),
                "Content-Type: application/sdp\r\n"
                "Content-Length: %zu\r\n", strlen(sdp));
                
            return rtsp_send_response(session, RTSP_OK, headers, sdp);
        }
        
        case RTSP_METHOD_SETUP: {
            // Parse Transport header
            const char *transport_line = strstr(request, "Transport:");
            if (!transport_line) {
                return rtsp_send_response(session, RTSP_BAD_REQUEST, NULL, NULL);
            }
            
            // Check if this is for audio track (track1) or video track (track0)
            bool is_audio_track = strstr(request, "track1") != NULL;
            
            if (is_audio_track) {
                // Setup audio RTP session
                if (rtsp_init_audio_rtp_session(session) < 0) {
                    return rtsp_send_response(session, RTSP_INTERNAL_ERROR, NULL, NULL);
                }
                
                char headers[512];
                snprintf(headers, sizeof(headers),
                    "Transport: RTP/AVP;unicast;client_port=%d-%d;server_port=%d-%d\r\n"
                    "Session: %s\r\n",
                    session->audio_rtp_session.rtp_port, session->audio_rtp_session.rtcp_port,
                    session->audio_rtp_session.rtp_port, session->audio_rtp_session.rtcp_port,
                    session->session_id);
                    
                return rtsp_send_response(session, RTSP_OK, headers, NULL);
            } else {
                // Setup video RTP session
                if (rtsp_init_rtp_session(session) < 0) {
                    return rtsp_send_response(session, RTSP_INTERNAL_ERROR, NULL, NULL);
                }
                
                session->state = RTSP_STATE_READY;
                
                char headers[512];
                snprintf(headers, sizeof(headers),
                    "Transport: RTP/AVP;unicast;client_port=%d-%d;server_port=%d-%d\r\n"
                    "Session: %s\r\n",
                    session->rtp_session.rtp_port, session->rtp_session.rtcp_port,
                    session->rtp_session.rtp_port, session->rtp_session.rtcp_port,
                    session->session_id);
                    
                return rtsp_send_response(session, RTSP_OK, headers, NULL);
            }
        }
        
        case RTSP_METHOD_PLAY: {
            if (session->state != RTSP_STATE_READY) {
                return rtsp_send_response(session, RTSP_METHOD_NOT_ALLOWED, NULL, NULL);
            }
            
            session->state = RTSP_STATE_PLAYING;
            
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_PAUSE: {
            if (session->state != RTSP_STATE_PLAYING) {
                return rtsp_send_response(session, RTSP_METHOD_NOT_ALLOWED, NULL, NULL);
            }
            
            session->state = RTSP_STATE_READY;
            
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            
            return rtsp_send_response(session, RTSP_OK, headers, NULL);
        }
        
        case RTSP_METHOD_TEARDOWN: {
            char headers[256];
            snprintf(headers, sizeof(headers), "Session: %s\r\n", session->session_id);
            
            rtsp_send_response(session, RTSP_OK, headers, NULL);
            session->active = false;
            return 0;
        }
        
        default:
            return rtsp_send_response(session, RTSP_METHOD_NOT_ALLOWED, NULL, NULL);
    }
}

/**
 * Send RTSP response
 */
static int rtsp_send_response(rtsp_session_t *session, int code, const char *headers, const char *body)
{
    const char *reason;
    switch (code) {
        case RTSP_OK: reason = "OK"; break;
        case RTSP_BAD_REQUEST: reason = "Bad Request"; break;
        case RTSP_UNAUTHORIZED: reason = "Unauthorized"; break;
        case RTSP_NOT_FOUND: reason = "Not Found"; break;
        case RTSP_METHOD_NOT_ALLOWED: reason = "Method Not Allowed"; break;
        case RTSP_INTERNAL_ERROR: reason = "Internal Server Error"; break;
        default: reason = "Unknown"; break;
    }
    
    char response[4096];
    int len = snprintf(response, sizeof(response),
        "RTSP/1.0 %d %s\r\n"
        "CSeq: %u\r\n"
        "Server: Anyka-ONVIF-RTSP/1.0\r\n",
        code, reason, session->cseq);
    
    if (headers) {
        len += snprintf(response + len, sizeof(response) - len, "%s", headers);
    }
    
    len += snprintf(response + len, sizeof(response) - len, "\r\n");
    
    if (body) {
        len += snprintf(response + len, sizeof(response) - len, "%s", body);
    }
    
    ssize_t sent = send(session->sockfd, response, len, 0);
    if (sent < 0) {
        ak_print_error("Failed to send RTSP response: %s\n", strerror(errno));
        return -1;
    }
    
    return 0;
}

/**
 * Parse RTSP method
 */
static int rtsp_parse_method(const char *line)
{
    if (strncmp(line, "OPTIONS", 7) == 0) return RTSP_METHOD_OPTIONS;
    if (strncmp(line, "DESCRIBE", 8) == 0) return RTSP_METHOD_DESCRIBE;
    if (strncmp(line, "SETUP", 5) == 0) return RTSP_METHOD_SETUP;
    if (strncmp(line, "PLAY", 4) == 0) return RTSP_METHOD_PLAY;
    if (strncmp(line, "PAUSE", 5) == 0) return RTSP_METHOD_PAUSE;
    if (strncmp(line, "TEARDOWN", 8) == 0) return RTSP_METHOD_TEARDOWN;
    if (strncmp(line, "GET_PARAMETER", 13) == 0) return RTSP_METHOD_GET_PARAMETER;
    if (strncmp(line, "SET_PARAMETER", 13) == 0) return RTSP_METHOD_SET_PARAMETER;
    return RTSP_METHOD_UNKNOWN;
}

/**
 * Setup video encoder
 */
static int rtsp_setup_encoder(rtsp_server_t *server)
{
    if (!server->vi_handle) {
        ak_print_error("Video input handle not set\n");
        return -1;
    }
    
    // Setup encoder parameters
    struct encode_param enc_param;
    memset(&enc_param, 0, sizeof(enc_param));
    
    enc_param.width = server->config.video_config.width;
    enc_param.height = server->config.video_config.height;
    enc_param.fps = server->config.video_config.fps;
    enc_param.bps = server->config.video_config.bitrate * 1000; // Convert to bps
    enc_param.goplen = server->config.video_config.gop_size;
    enc_param.minqp = 28;
    enc_param.maxqp = 42;
    enc_param.profile = PROFILE_MAIN;
    enc_param.use_chn = ENCODE_MAIN_CHN;
    enc_param.enc_grp = ENCODE_MAINCHN_NET;
    enc_param.enc_out_type = server->config.video_config.codec_type;
    enc_param.br_mode = server->config.video_config.br_mode;
    
    // Open encoder
    server->venc_handle = ak_venc_open(&enc_param);
    if (!server->venc_handle) {
        ak_print_error("Failed to open video encoder\n");
        return -1;
    }
    
    // Request stream
    server->stream_handle = ak_venc_request_stream(server->vi_handle, server->venc_handle);
    if (!server->stream_handle) {
        ak_print_error("Failed to request video stream\n");
        ak_venc_close(server->venc_handle);
        server->venc_handle = NULL;
        return -1;
    }
    
    server->encoder_initialized = true;
    
    ak_print_notice("Video encoder initialized: %dx%d @ %d fps, %d kbps\n",
                    enc_param.width, enc_param.height, enc_param.fps, 
                    server->config.video_config.bitrate);
    
    return 0;
}

/**
 * Cleanup video encoder
 */
static void rtsp_cleanup_encoder(rtsp_server_t *server)
{
    if (server->stream_handle) {
        ak_venc_cancel_stream(server->stream_handle);
        server->stream_handle = NULL;
    }
    
    if (server->venc_handle) {
        ak_venc_close(server->venc_handle);
        server->venc_handle = NULL;
    }
    
    server->encoder_initialized = false;
}

/**
 * Initialize RTP session
 */
static int rtsp_init_rtp_session(rtsp_session_t *session)
{
    // Generate random SSRC
    session->rtp_session.ssrc = rand();
    session->rtp_session.seq_num = rand() & 0xFFFF;
    session->rtp_session.timestamp = rand();
    
    // Use fixed ports for simplicity (in real implementation, allocate dynamically)
    session->rtp_session.rtp_port = 8000;
    session->rtp_session.rtcp_port = 8001;
    session->rtp_session.transport = RTP_TRANSPORT_UDP;
    
    // Create RTP socket
    session->rtp_session.rtp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (session->rtp_session.rtp_sockfd < 0) {
        ak_print_error("Failed to create RTP socket: %s\n", strerror(errno));
        return -1;
    }
    
    // Create RTCP socket
    session->rtp_session.rtcp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (session->rtp_session.rtcp_sockfd < 0) {
        ak_print_error("Failed to create RTCP socket: %s\n", strerror(errno));
        close(session->rtp_session.rtp_sockfd);
        session->rtp_session.rtp_sockfd = -1;
        return -1;
    }
    
    // Set client address for UDP transmission
    memcpy(&session->rtp_session.client_addr, &session->addr, sizeof(session->addr));
    session->rtp_session.client_addr.sin_port = htons(session->rtp_session.rtp_port);
    
    return 0;
}

/**
 * Cleanup RTP session
 */
static void rtsp_cleanup_rtp_session(rtsp_session_t *session)
{
    if (session->rtp_session.rtp_sockfd >= 0) {
        close(session->rtp_session.rtp_sockfd);
        session->rtp_session.rtp_sockfd = -1;
    }
    
    if (session->rtp_session.rtcp_sockfd >= 0) {
        close(session->rtp_session.rtcp_sockfd);
        session->rtp_session.rtcp_sockfd = -1;
    }
}

/**
 * Initialize audio RTP session
 */
static int rtsp_init_audio_rtp_session(rtsp_session_t *session)
{
    // Generate random SSRC for audio
    session->audio_rtp_session.ssrc = rand() + 1000; // Different from video SSRC
    session->audio_rtp_session.sequence = rand() & 0xFFFF;
    session->audio_rtp_session.timestamp = rand();
    
    // Use different ports for audio
    session->audio_rtp_session.rtp_port = 8002;
    session->audio_rtp_session.rtcp_port = 8003;
    session->audio_rtp_session.transport = RTP_TRANSPORT_UDP;
    
    // Create audio RTP socket
    session->audio_rtp_session.rtp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (session->audio_rtp_session.rtp_sockfd < 0) {
        ak_print_error("Failed to create audio RTP socket: %s\n", strerror(errno));
        return -1;
    }
    
    // Create audio RTCP socket
    session->audio_rtp_session.rtcp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (session->audio_rtp_session.rtcp_sockfd < 0) {
        ak_print_error("Failed to create audio RTCP socket: %s\n", strerror(errno));
        close(session->audio_rtp_session.rtp_sockfd);
        session->audio_rtp_session.rtp_sockfd = -1;
        return -1;
    }
    
    // Set client address for UDP transmission
    memcpy(&session->audio_rtp_session.client_rtp_addr, &session->addr, sizeof(session->addr));
    session->audio_rtp_session.client_rtp_addr.sin_port = htons(session->audio_rtp_session.rtp_port);
    
    session->audio_enabled = true;
    
    return 0;
}

/**
 * Cleanup audio RTP session
 */
static void rtsp_cleanup_audio_rtp_session(rtsp_session_t *session)
{
    if (session->audio_rtp_session.rtp_sockfd >= 0) {
        close(session->audio_rtp_session.rtp_sockfd);
        session->audio_rtp_session.rtp_sockfd = -1;
    }
    
    if (session->audio_rtp_session.rtcp_sockfd >= 0) {
        close(session->audio_rtp_session.rtcp_sockfd);
        session->audio_rtp_session.rtcp_sockfd = -1;
    }
    
    session->audio_enabled = false;
}

/**
 * Send RTP packet
 */
static int rtsp_send_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, bool marker)
{
    if (session->rtp_session.rtp_sockfd < 0 || !data || len == 0) {
        return -1;
    }
    
    uint8_t rtp_packet[RTSP_RTP_BUFFER_SIZE];
    uint8_t *rtp_header = rtp_packet;
    uint8_t *rtp_payload = rtp_packet + 12; // RTP header is 12 bytes
    
    // Build RTP header
    rtp_header[0] = 0x80; // Version 2, no padding, no extension, no CSRC
    rtp_header[1] = RTP_PT_H264 | (marker ? 0x80 : 0x00); // Payload type and marker
    rtp_header[2] = (session->rtp_session.seq_num >> 8) & 0xFF;
    rtp_header[3] = session->rtp_session.seq_num & 0xFF;
    rtp_header[4] = (session->rtp_session.timestamp >> 24) & 0xFF;
    rtp_header[5] = (session->rtp_session.timestamp >> 16) & 0xFF;
    rtp_header[6] = (session->rtp_session.timestamp >> 8) & 0xFF;
    rtp_header[7] = session->rtp_session.timestamp & 0xFF;
    rtp_header[8] = (session->rtp_session.ssrc >> 24) & 0xFF;
    rtp_header[9] = (session->rtp_session.ssrc >> 16) & 0xFF;
    rtp_header[10] = (session->rtp_session.ssrc >> 8) & 0xFF;
    rtp_header[11] = session->rtp_session.ssrc & 0xFF;
    
    // Copy payload
    size_t payload_len = (len <= RTSP_RTP_BUFFER_SIZE - 12) ? len : RTSP_RTP_BUFFER_SIZE - 12;
    memcpy(rtp_payload, data, payload_len);
    
    // Send packet
    ssize_t sent = sendto(session->rtp_session.rtp_sockfd, rtp_packet, 12 + payload_len, 0,
                         (struct sockaddr*)&session->rtp_session.client_addr,
                         sizeof(session->rtp_session.client_addr));
    
    if (sent < 0) {
        ak_print_error("Failed to send RTP packet: %s\n", strerror(errno));
        return -1;
    }
    
    // Update sequence number
    session->rtp_session.seq_num++;
    
    return payload_len;
}

/**
 * Send audio RTP packet
 */
static int rtsp_send_audio_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, bool marker, int payload_type)
{
    if (session->audio_rtp_session.rtp_sockfd < 0 || !data || len == 0) {
        return -1;
    }
    
    uint8_t rtp_packet[RTSP_RTP_BUFFER_SIZE];
    uint8_t *rtp_header = rtp_packet;
    uint8_t *rtp_payload = rtp_packet + 12; // RTP header is 12 bytes
    
    // Build RTP header
    rtp_header[0] = 0x80; // Version 2, no padding, no extension, no CSRC
    rtp_header[1] = payload_type | (marker ? 0x80 : 0x00); // Payload type and marker
    rtp_header[2] = (session->audio_rtp_session.sequence >> 8) & 0xFF;
    rtp_header[3] = session->audio_rtp_session.sequence & 0xFF;
    rtp_header[4] = (session->audio_rtp_session.timestamp >> 24) & 0xFF;
    rtp_header[5] = (session->audio_rtp_session.timestamp >> 16) & 0xFF;
    rtp_header[6] = (session->audio_rtp_session.timestamp >> 8) & 0xFF;
    rtp_header[7] = session->audio_rtp_session.timestamp & 0xFF;
    rtp_header[8] = (session->audio_rtp_session.ssrc >> 24) & 0xFF;
    rtp_header[9] = (session->audio_rtp_session.ssrc >> 16) & 0xFF;
    rtp_header[10] = (session->audio_rtp_session.ssrc >> 8) & 0xFF;
    rtp_header[11] = session->audio_rtp_session.ssrc & 0xFF;
    
    // Copy payload
    size_t payload_len = (len <= RTSP_RTP_BUFFER_SIZE - 12) ? len : RTSP_RTP_BUFFER_SIZE - 12;
    memcpy(rtp_payload, data, payload_len);
    
    // Send packet
    ssize_t sent = sendto(session->audio_rtp_session.rtp_sockfd, rtp_packet, 12 + payload_len, 0,
                         (struct sockaddr*)&session->audio_rtp_session.client_rtp_addr,
                         sizeof(session->audio_rtp_session.client_rtp_addr));
    
    if (sent < 0) {
        ak_print_error("Failed to send audio RTP packet: %s\n", strerror(errno));
        return -1;
    }
    
    // Update sequence number
    session->audio_rtp_session.sequence++;
    
    return payload_len;
}

/**
 * Encoder thread - captures frames and sends to playing clients
 */
static void* rtsp_encoder_thread(void *arg)
{
    rtsp_server_t *server = (rtsp_server_t*)arg;
    struct video_stream stream;
    
    ak_print_notice("RTSP encoder thread started\n");
    
    while (server->running && server->encoder_initialized) {
        // Get encoded frame
        int ret = ak_venc_get_stream(server->stream_handle, &stream);
        if (ret != 0) {
            ak_sleep_ms(10);
            continue;
        }
        
        // Send to all playing clients
        pthread_mutex_lock(&server->sessions_mutex);
        rtsp_session_t *session = server->sessions;
        while (session) {
            if (session->active && session->state == RTSP_STATE_PLAYING) {
                // For H.264, we need to packetize the NAL units
                uint8_t *data = stream.data;
                size_t remaining = stream.len;
                
                while (remaining > 0) {
                    // Find next NAL unit
                    size_t nal_len = remaining;
                    bool marker = true; // Mark last packet of frame
                    
                    if (nal_len > RTSP_RTP_BUFFER_SIZE - 12) {
                        nal_len = RTSP_RTP_BUFFER_SIZE - 12;
                        marker = false;
                    }
                    
                    int sent = rtsp_send_rtp_packet(session, data, nal_len, marker);
                    if (sent > 0) {
                        data += sent;
                        remaining -= sent;
                        server->bytes_sent += sent;
                    } else {
                        break;
                    }
                }
            }
            session = session->next;
        }
        
        server->frames_sent++;
        
        // Update timestamp for next frame (90kHz clock for video)
        rtsp_session_t *session_iter = server->sessions;
        while (session_iter) {
            if (session_iter->active && session_iter->state == RTSP_STATE_PLAYING) {
                session_iter->rtp_session.timestamp += (90000 / server->config.video_config.fps);
            }
            session_iter = session_iter->next;
        }
        
        pthread_mutex_unlock(&server->sessions_mutex);
        
        // Release frame
        ak_venc_release_stream(server->stream_handle, &stream);
    }
    
    ak_print_notice("RTSP encoder thread finished\n");
    return NULL;
}

/**
 * Setup audio encoder
 */
static int rtsp_setup_audio_encoder(rtsp_server_t *server)
{
    // Setup audio input
    struct pcm_param ai_param = {0};
    ai_param.sample_bits = server->config.audio_config.bits_per_sample;
    ai_param.channel_num = server->config.audio_config.channels;
    ai_param.sample_rate = server->config.audio_config.sample_rate;
    
    server->ai_handle = ak_ai_open(&ai_param);
    if (!server->ai_handle) {
        ak_print_error("Failed to open audio input\n");
        return -1;
    }
    
    int ret = ak_ai_start_capture(server->ai_handle);
    if (ret != 0) {
        ak_print_error("Failed to start audio capture: %d\n", ret);
        ak_ai_close(server->ai_handle);
        server->ai_handle = NULL;
        return -1;
    }
    
    // Setup audio encoder
    struct audio_param aenc_param = {0};
    aenc_param.type = server->config.audio_config.codec_type;
    aenc_param.sample_bits = server->config.audio_config.bits_per_sample;
    aenc_param.channel_num = server->config.audio_config.channels;
    aenc_param.sample_rate = server->config.audio_config.sample_rate;
    
    server->aenc_handle = ak_aenc_open(&aenc_param);
    if (!server->aenc_handle) {
        ak_print_error("Failed to open audio encoder\n");
        ak_ai_stop_capture(server->ai_handle);
        ak_ai_close(server->ai_handle);
        server->ai_handle = NULL;
        return -1;
    }
    
    server->audio_encoder_initialized = true;
    
    ak_print_notice("Audio encoder initialized (rate: %d, channels: %d, codec: %d)\n",
                    server->config.audio_config.sample_rate,
                    server->config.audio_config.channels,
                    server->config.audio_config.codec_type);
    
    return 0;
}

/**
 * Cleanup audio encoder
 */
static void rtsp_cleanup_audio_encoder(rtsp_server_t *server)
{
    if (server->audio_encoder_initialized) {
        server->audio_encoder_initialized = false;
        
        if (server->aenc_handle) {
            ak_aenc_close(server->aenc_handle);
            server->aenc_handle = NULL;
        }
        
        if (server->ai_handle) {
            ak_ai_stop_capture(server->ai_handle);
            ak_ai_close(server->ai_handle);
            server->ai_handle = NULL;
        }
        
        ak_print_notice("Audio encoder cleanup completed\n");
    }
}

/**
 * Audio thread - captures audio frames and sends to playing clients
 */
static void* rtsp_audio_thread(void *arg)
{
    rtsp_server_t *server = (rtsp_server_t*)arg;
    struct frame audio_frame;
    
    ak_print_notice("RTSP audio thread started\n");
    
    while (server->running && server->audio_encoder_initialized) {
        // Get audio frame from AI
        int ret = ak_ai_get_frame(server->ai_handle, &audio_frame, 100);
        if (ret != 0) {
            ak_sleep_ms(10);
            continue;
        }
        
        // Encode audio frame
        struct audio_stream audio_stream;
        
        ret = ak_aenc_send_frame(server->aenc_handle, &audio_frame, &audio_stream);
        if (ret >= 0 && audio_stream.data && audio_stream.len > 0) {
            // Send encoded audio to all playing clients
            pthread_mutex_lock(&server->sessions_mutex);
            rtsp_session_t *session = server->sessions;
            while (session) {
                if (session->active && session->state == RTSP_STATE_PLAYING && session->audio_enabled) {
                    
                    int payload_type = RTP_PT_PCMA; // Default to G.711 A-law
                    switch (server->config.audio_config.codec_type) {
                        case AK_AUDIO_TYPE_PCM_ALAW:
                            payload_type = RTP_PT_PCMA;
                            break;
                        case AK_AUDIO_TYPE_PCM_ULAW:
                            payload_type = RTP_PT_PCMU;
                            break;
                        case AK_AUDIO_TYPE_AAC:
                            payload_type = RTP_PT_AAC;
                            break;
                        default:
                            payload_type = RTP_PT_PCMA;
                            break;
                    }
                    
                    int sent = rtsp_send_audio_rtp_packet(session, audio_stream.data, 
                                                        audio_stream.len, true, payload_type);
                    if (sent > 0) {
                        server->bytes_sent += sent;
                    }
                }
                session = session->next;
            }
            
            server->audio_frames_sent++;
            
            // Update audio timestamp (8kHz clock for G.711)
            uint32_t timestamp_increment = server->config.audio_config.sample_rate / 50; // 20ms frames
            rtsp_session_t *session_iter = server->sessions;
            while (session_iter) {
                if (session_iter->active && session_iter->state == RTSP_STATE_PLAYING && session_iter->audio_enabled) {
                    session_iter->audio_rtp_session.timestamp += timestamp_increment;
                }
                session_iter = session_iter->next;
            }
            
            pthread_mutex_unlock(&server->sessions_mutex);
        }
        
        // Release audio input frame
        ak_ai_release_frame(server->ai_handle, &audio_frame);
    }
    
    ak_print_notice("RTSP audio thread finished\n");
    return NULL;
}
