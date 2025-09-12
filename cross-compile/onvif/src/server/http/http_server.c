/**
 * @file http_server.c
 * @brief Modular HTTP server with ONVIF support.
 * 
 * This server uses modular components for maintainability:
 *  - http_parser: HTTP request parsing
 *  - connection_manager: Connection lifecycle
 *  - thread_pool: Concurrent processing
 *  - buffer_pool: Memory management
 *  - epoll_server: Async I/O
 */

#include "http_server.h"
#include "http_parser.h"
#include "http_onvif_adapter.h"
#include "server/common/connection_manager.h"
#include "server/common/thread_pool.h"
#include "server/common/buffer_pool.h"
#include "server/common/epoll_server.h"
#include "platform/platform.h"
#include "services/device/onvif_device.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/network_utils.h"
#include "utils/constants_clean.h"
#include "utils/xml_utils.h"
#include "utils/logging_utils.h"
#include "common/onvif_types.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <pthread.h>
#include <errno.h>
#include <fcntl.h>
#include <strings.h>

/* Fallback implementations for missing functions */
#ifndef strnlen
static size_t strnlen(const char *s, size_t maxlen) {
    size_t len = 0;
    while (len < maxlen && s[len] != '\0') {
        len++;
    }
    return len;
}
#endif

#ifndef strcasestr
static char *strcasestr(const char *haystack, const char *needle) {
    size_t needle_len = strlen(needle);
    size_t haystack_len = strlen(haystack);
    
    if (needle_len > haystack_len) {
        return NULL;
    }
    
    for (size_t i = 0; i <= haystack_len - needle_len; i++) {
        if (strncasecmp(haystack + i, needle, needle_len) == 0) {
            return (char *)(haystack + i);
        }
    }
    return NULL;
}
#endif

/* Global instances for shared components */
buffer_pool_t g_buffer_pool;
thread_pool_t g_thread_pool;


/* Server state */
typedef struct {
    int running;
    int socket;
    pthread_t epoll_thread;
    thread_pool_t thread_pool;
    buffer_pool_t buffer_pool;
    uint64_t connection_count;
    uint64_t request_count;
} server_state_t;

static server_state_t g_server = {0, -1, 0, {0}, {0}, 0, 0};

/* Forward declarations */
static onvif_service_type_t get_service_type(const char *path);
static onvif_action_type_t get_action_type(const char *body);
void process_connection(void *conn);
static int handle_onvif_request(const http_request_t *request, char *response, size_t response_size);

/**
 * @brief Get ONVIF service type from path
 */
static onvif_service_type_t get_service_type(const char *path) {
    if (strstr(path, "/device")) return ONVIF_SERVICE_DEVICE;
    if (strstr(path, "/media")) return ONVIF_SERVICE_MEDIA;
    if (strstr(path, "/ptz")) return ONVIF_SERVICE_PTZ;
    if (strstr(path, "/imaging")) return ONVIF_SERVICE_IMAGING;
    return ONVIF_SERVICE_DEVICE; // Default
}

/**
 * @brief Get ONVIF action type from SOAP body
 */
static onvif_action_type_t get_action_type(const char *body) {
    if (!body) return ONVIF_ACTION_GET_CAPABILITIES;
    
    if (strstr(body, "GetCapabilities")) return ONVIF_ACTION_GET_CAPABILITIES;
    if (strstr(body, "GetDeviceInformation")) return ONVIF_ACTION_GET_DEVICE_INFORMATION;
    if (strstr(body, "GetSystemDateAndTime")) return ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME;
    if (strstr(body, "SetSystemDateAndTime")) return ONVIF_ACTION_SET_SYSTEM_DATE_AND_TIME;
    if (strstr(body, "GetSystemLogging")) return ONVIF_ACTION_GET_SYSTEM_LOGGING;
    if (strstr(body, "GetScopes")) return ONVIF_ACTION_GET_SCOPES;
    if (strstr(body, "SetScopes")) return ONVIF_ACTION_SET_SCOPES;
    if (strstr(body, "AddScopes")) return ONVIF_ACTION_ADD_SCOPES;
    if (strstr(body, "RemoveScopes")) return ONVIF_ACTION_REMOVE_SCOPES;
    if (strstr(body, "GetDiscoveryMode")) return ONVIF_ACTION_GET_DISCOVERY_MODE;
    if (strstr(body, "SetDiscoveryMode")) return ONVIF_ACTION_SET_DISCOVERY_MODE;
    if (strstr(body, "GetRemoteDiscoveryMode")) return ONVIF_ACTION_GET_REMOTE_DISCOVERY_MODE;
    if (strstr(body, "SetRemoteDiscoveryMode")) return ONVIF_ACTION_SET_REMOTE_DISCOVERY_MODE;
    if (strstr(body, "GetDPAddresses")) return ONVIF_ACTION_GET_DPADDRESSES;
    if (strstr(body, "SetDPAddresses")) return ONVIF_ACTION_SET_DPADDRESSES;
    if (strstr(body, "GetNetworkInterfaces")) return ONVIF_ACTION_GET_NETWORK_INTERFACES;
    if (strstr(body, "SetNetworkInterfaces")) return ONVIF_ACTION_SET_NETWORK_INTERFACES;
    if (strstr(body, "GetNetworkProtocols")) return ONVIF_ACTION_GET_NETWORK_PROTOCOLS;
    if (strstr(body, "SetNetworkProtocols")) return ONVIF_ACTION_SET_NETWORK_PROTOCOLS;
    if (strstr(body, "GetNetworkDefaultGateway")) return ONVIF_ACTION_GET_NETWORK_DEFAULT_GATEWAY;
    if (strstr(body, "SetNetworkDefaultGateway")) return ONVIF_ACTION_SET_NETWORK_DEFAULT_GATEWAY;
    if (strstr(body, "GetZeroConfiguration")) return ONVIF_ACTION_GET_ZERO_CONFIGURATION;
    if (strstr(body, "SetZeroConfiguration")) return ONVIF_ACTION_SET_ZERO_CONFIGURATION;
    if (strstr(body, "GetIPAddressFilter")) return ONVIF_ACTION_GET_IP_ADDRESS_FILTER;
    if (strstr(body, "SetIPAddressFilter")) return ONVIF_ACTION_SET_IP_ADDRESS_FILTER;
    if (strstr(body, "AddIPAddressFilter")) return ONVIF_ACTION_ADD_IP_ADDRESS_FILTER;
    if (strstr(body, "RemoveIPAddressFilter")) return ONVIF_ACTION_REMOVE_IP_ADDRESS_FILTER;
    if (strstr(body, "GetAccessPolicy")) return ONVIF_ACTION_GET_ACCESS_POLICY;
    if (strstr(body, "SetAccessPolicy")) return ONVIF_ACTION_SET_ACCESS_POLICY;
    if (strstr(body, "CreateCertificate")) return ONVIF_ACTION_CREATE_CERTIFICATE;
    if (strstr(body, "GetCertificates")) return ONVIF_ACTION_GET_CERTIFICATES;
    if (strstr(body, "GetCertificateInformation")) return ONVIF_ACTION_GET_CERTIFICATE_INFORMATION;
    if (strstr(body, "SetCertificate")) return ONVIF_ACTION_SET_CERTIFICATE;
    if (strstr(body, "DeleteCertificate")) return ONVIF_ACTION_DELETE_CERTIFICATE;
    if (strstr(body, "GetPkcs10Request")) return ONVIF_ACTION_GET_PKCS_10_REQUEST;
    if (strstr(body, "LoadCertificateWithPrivateKey")) return ONVIF_ACTION_LOAD_CERTIFICATE_WITH_PRIVATE_KEY;
    if (strstr(body, "GetClientCertificateMode")) return ONVIF_ACTION_GET_CLIENT_CERTIFICATE_MODE;
    if (strstr(body, "SetClientCertificateMode")) return ONVIF_ACTION_SET_CLIENT_CERTIFICATE_MODE;
    if (strstr(body, "GetRelayOutputs")) return ONVIF_ACTION_GET_RELAY_OUTPUTS;
    if (strstr(body, "SetRelayOutputSettings")) return ONVIF_ACTION_SET_RELAY_OUTPUT_SETTINGS;
    if (strstr(body, "SetRelayOutputState")) return ONVIF_ACTION_SET_RELAY_OUTPUT_STATE;
    if (strstr(body, "GetServiceCapabilities")) return ONVIF_ACTION_GET_SERVICE_CAPABILITIES;
    if (strstr(body, "GetVideoSources")) return ONVIF_ACTION_GET_VIDEO_SOURCES;
    if (strstr(body, "GetVideoOutputs")) return ONVIF_ACTION_GET_VIDEO_OUTPUTS;
    if (strstr(body, "GetAudioSources")) return ONVIF_ACTION_GET_AUDIO_SOURCES;
    if (strstr(body, "GetAudioOutputs")) return ONVIF_ACTION_GET_AUDIO_OUTPUTS;
    if (strstr(body, "GetAudioSourceConfigurations")) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATIONS;
    if (strstr(body, "GetAudioOutputConfigurations")) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATIONS;
    if (strstr(body, "GetVideoSourceConfigurations")) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATIONS;
    if (strstr(body, "GetVideoOutputConfigurations")) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATIONS;
    if (strstr(body, "GetMetadataConfigurations")) return ONVIF_ACTION_GET_METADATA_CONFIGURATIONS;
    if (strstr(body, "GetCompositeConfigurations")) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATIONS;
    if (strstr(body, "GetAudioDecoderConfigurations")) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATIONS;
    if (strstr(body, "GetVideoAnalyticsConfigurations")) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATIONS;
    if (strstr(body, "GetPTZConfigurations")) return ONVIF_ACTION_GET_PTZ_CONFIGURATIONS;
    if (strstr(body, "GetVideoSourceConfiguration")) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION;
    if (strstr(body, "GetVideoOutputConfiguration")) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION;
    if (strstr(body, "GetAudioSourceConfiguration")) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION;
    if (strstr(body, "GetAudioOutputConfiguration")) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION;
    if (strstr(body, "GetMetadataConfiguration")) return ONVIF_ACTION_GET_METADATA_CONFIGURATION;
    if (strstr(body, "GetCompositeConfiguration")) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION;
    if (strstr(body, "GetAudioDecoderConfiguration")) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION;
    if (strstr(body, "GetVideoAnalyticsConfiguration")) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION;
    if (strstr(body, "GetPTZConfiguration")) return ONVIF_ACTION_GET_PTZ_CONFIGURATION;
    if (strstr(body, "GetVideoSourceConfigurationOptions")) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetVideoOutputConfigurationOptions")) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetAudioSourceConfigurationOptions")) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetAudioOutputConfigurationOptions")) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetMetadataConfigurationOptions")) return ONVIF_ACTION_GET_METADATA_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetCompositeConfigurationOptions")) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetAudioDecoderConfigurationOptions")) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetVideoAnalyticsConfigurationOptions")) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetPTZConfigurationOptions")) return ONVIF_ACTION_GET_PTZ_CONFIGURATION_OPTIONS;
    if (strstr(body, "GetGuaranteedVideoItemBounds")) return ONVIF_ACTION_GET_GUARANTEED_VIDEO_ITEM_BOUNDS;
    if (strstr(body, "GetStreamUri")) return ONVIF_ACTION_GET_STREAM_URI;
    if (strstr(body, "GetSnapshotUri")) return ONVIF_ACTION_GET_SNAPSHOT_URI;
    if (strstr(body, "GetProfiles")) return ONVIF_ACTION_GET_PROFILES;
    if (strstr(body, "AddProfile")) return ONVIF_ACTION_ADD_PROFILE;
    if (strstr(body, "RemoveProfile")) return ONVIF_ACTION_REMOVE_PROFILE;
    if (strstr(body, "GetVideoSourceMode")) return ONVIF_ACTION_GET_VIDEO_SOURCE_MODE;
    if (strstr(body, "SetVideoSourceMode")) return ONVIF_ACTION_SET_VIDEO_SOURCE_MODE;
    if (strstr(body, "GetOSD")) return ONVIF_ACTION_GET_OSD;
    if (strstr(body, "GetOSDOptions")) return ONVIF_ACTION_GET_OSD_OPTIONS;
    if (strstr(body, "SetOSD")) return ONVIF_ACTION_SET_OSD;
    if (strstr(body, "CreateOSD")) return ONVIF_ACTION_CREATE_OSD;
    if (strstr(body, "DeleteOSD")) return ONVIF_ACTION_DELETE_OSD;
    if (strstr(body, "GetMoveOptions")) return ONVIF_ACTION_GET_MOVE_OPTIONS;
    if (strstr(body, "GetStatus")) return ONVIF_ACTION_GET_STATUS;
    if (strstr(body, "GetConfiguration")) return ONVIF_ACTION_GET_CONFIGURATION;
    if (strstr(body, "GetConfigurations")) return ONVIF_ACTION_GET_CONFIGURATIONS;
    if (strstr(body, "GetCompatibleConfigurations")) return ONVIF_ACTION_GET_COMPATIBLE_CONFIGURATIONS;
    if (strstr(body, "SetConfiguration")) return ONVIF_ACTION_SET_CONFIGURATION;
    if (strstr(body, "GetConfigurationOptions")) return ONVIF_ACTION_GET_CONFIGURATION_OPTIONS;
    if (strstr(body, "Stop")) return ONVIF_ACTION_STOP;
    if (strstr(body, "AbsoluteMove")) return ONVIF_ACTION_ABSOLUTE_MOVE;
    if (strstr(body, "RelativeMove")) return ONVIF_ACTION_RELATIVE_MOVE;
    if (strstr(body, "ContinuousMove")) return ONVIF_ACTION_CONTINUOUS_MOVE;
    if (strstr(body, "GetPresets")) return ONVIF_ACTION_GET_PRESETS;
    if (strstr(body, "SetPreset")) return ONVIF_ACTION_SET_PRESET;
    if (strstr(body, "RemovePreset")) return ONVIF_ACTION_REMOVE_PRESET;
    if (strstr(body, "GotoPreset")) return ONVIF_ACTION_GOTO_PRESET;
    if (strstr(body, "GetImagingSettings")) return ONVIF_ACTION_GET_IMAGING_SETTINGS;
    if (strstr(body, "SetImagingSettings")) return ONVIF_ACTION_SET_IMAGING_SETTINGS;
    if (strstr(body, "GetOptions")) return ONVIF_ACTION_GET_OPTIONS;
    
    return ONVIF_ACTION_GET_CAPABILITIES; // Default
}

/**
 * @brief Process a single connection
 */
void process_connection(void *conn_ptr) {
    connection_t *conn = (connection_t *)conn_ptr;
    if (!conn) return;
    
    conn->last_activity = platform_get_time_ms();
    
    // Read data from socket
    ssize_t n = recv(conn->fd, conn->buffer + conn->buffer_used, 
                    conn->buffer_size - conn->buffer_used - 1, 0);
    if (n <= 0) {
        if (n < 0) {
            platform_log_error("Recv failed on fd %d: %s\n", conn->fd, strerror(errno));
        }
        connection_remove_from_list(conn);
        buffer_pool_return(&g_server.buffer_pool, conn->buffer);
        connection_destroy(conn);
        return;
    }
    
    conn->buffer_used += n;
    conn->buffer[conn->buffer_used] = '\0';
    
    // Parse HTTP request
    http_request_t request;
    int need_more_data = 0;
    int parse_result = parse_http_request_state_machine(conn->buffer, conn->buffer_used, 
                                                      &request, &need_more_data);
    
    if (parse_result != 0) {
        platform_log_error("HTTP parse error on fd %d\n", conn->fd);
        connection_remove_from_list(conn);
        buffer_pool_return(&g_server.buffer_pool, conn->buffer);
        connection_destroy(conn);
        return;
    }
    
    if (need_more_data) {
        // Need more data, continue reading
        return;
    }
    
    // Process complete request
    char response_buffer[32768];
    int handled = handle_onvif_request(&request, response_buffer, sizeof(response_buffer));
    
    // Send response
    http_response_t response;
    if (handled) {
        response = create_http_200_response(response_buffer, strlen(response_buffer), NULL);
    } else {
        response = create_http_404_response();
    }
    
    send_http_response(conn->fd, &response);
    
    // Check for keep-alive
    const char *connection = strcasestr(conn->buffer, "Connection:");
    if (connection && strcasestr(connection, "keep-alive")) {
        conn->keepalive_count++;
        if (conn->keepalive_count < 100) {
            // Reset for next request
            conn->buffer_used = 0;
            conn->content_length = 0;
            conn->header_length = 0;
            conn->state = CONN_STATE_READING_HEADERS;
        } else {
            // Close after max requests
            connection_remove_from_list(conn);
            buffer_pool_return(&g_server.buffer_pool, conn->buffer);
            connection_destroy(conn);
        }
    } else {
        // Close connection
        connection_remove_from_list(conn);
        buffer_pool_return(&g_server.buffer_pool, conn->buffer);
        connection_destroy(conn);
    }
    
    g_server.request_count++;
}

/**
 * @brief Handle ONVIF request
 */
static int handle_onvif_request(const http_request_t *request, char *response, size_t response_size) {
    if (!request || !response) return 0;
    
    // Only handle POST requests
    if (strcmp(request->method, "POST") != 0) return 0;
    
    onvif_service_type_t service = get_service_type(request->path);
    onvif_action_type_t action = get_action_type(request->body);
    
    // Convert HTTP request to ONVIF request
    onvif_request_t onvif_req;
    onvif_response_t onvif_resp;
    
    if (http_to_onvif_request(request, &onvif_req) != 0) {
        platform_log_error("Failed to convert HTTP request to ONVIF request\n");
        return 0;
    }
    
    onvif_req.action = action;
    
    // Initialize response
    memset(&onvif_resp, 0, sizeof(onvif_response_t));
    
    // Route to appropriate service handler
    int result = 0;
    switch (service) {
        case ONVIF_SERVICE_DEVICE:
            result = onvif_device_handle_request(action, &onvif_req, &onvif_resp);
            break;
        case ONVIF_SERVICE_MEDIA:
            result = onvif_media_handle_request(action, &onvif_req, &onvif_resp);
            break;
        case ONVIF_SERVICE_PTZ:
            result = onvif_ptz_handle_request(action, &onvif_req, &onvif_resp);
            break;
        case ONVIF_SERVICE_IMAGING:
            result = onvif_imaging_handle_request(action, &onvif_req, &onvif_resp);
            break;
        default:
            platform_log_error("Unknown service type: %d\n", service);
            onvif_request_cleanup(&onvif_req);
            return 0;
    }
    
    // Convert ONVIF response back to HTTP response
    if (result == 0 && onvif_resp.body) {
        size_t copy_size = (onvif_resp.body_length < response_size - 1) ? 
                          onvif_resp.body_length : response_size - 1;
        memcpy(response, onvif_resp.body, copy_size);
        response[copy_size] = '\0';
    }
    
    // Cleanup
    onvif_request_cleanup(&onvif_req);
    onvif_response_cleanup(&onvif_resp);
    
    return result;
}

/**
 * @brief Start HTTP server
 */
int http_server_start(int port) {
    if (g_server.running) {
        platform_log_warning("HTTP server already running\n");
        return -1;
    }
    
    platform_log_info("Starting modular HTTP server on port %d...\n", port);
    
    // Initialize modules
    if (connection_manager_init() != 0) {
        platform_log_error("Failed to initialize connection manager\n");
        return -1;
    }
    
    if (buffer_pool_init(&g_server.buffer_pool) != 0) {
        platform_log_error("Failed to initialize buffer pool\n");
        connection_manager_cleanup();
        return -1;
    }
    
    if (thread_pool_init(&g_server.thread_pool) != 0) {
        platform_log_error("Failed to initialize thread pool\n");
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Create server socket
    g_server.socket = socket(AF_INET, SOCK_STREAM, 0);
    if (g_server.socket < 0) {
        platform_log_error("Failed to create socket: %s\n", strerror(errno));
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Set socket options
    int opt = 1;
    if (setsockopt(g_server.socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
        platform_log_error("Failed to set socket options: %s\n", strerror(errno));
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Set socket to non-blocking
    int flags = fcntl(g_server.socket, F_GETFL, 0);
    if (fcntl(g_server.socket, F_SETFL, flags | O_NONBLOCK) < 0) {
        platform_log_error("Failed to set socket to non-blocking: %s\n", strerror(errno));
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Bind and listen
    struct sockaddr_in server_addr;
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(port);
    
    if (bind(g_server.socket, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        platform_log_error("Failed to bind socket: %s\n", strerror(errno));
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    if (listen(g_server.socket, 128) < 0) {
        platform_log_error("Failed to listen on socket: %s\n", strerror(errno));
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Initialize epoll server
    if (epoll_server_init(g_server.socket) != 0) {
        platform_log_error("Failed to initialize epoll server\n");
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    // Start epoll event loop
    if (pthread_create(&g_server.epoll_thread, NULL, epoll_server_loop, NULL) != 0) {
        platform_log_error("Failed to create epoll thread: %s\n", strerror(errno));
        epoll_server_cleanup();
        close(g_server.socket);
        thread_pool_cleanup(&g_server.thread_pool);
        buffer_pool_cleanup(&g_server.buffer_pool);
        connection_manager_cleanup();
        return -1;
    }
    
    g_server.running = 1;
    g_server.connection_count = 0;
    g_server.request_count = 0;
    
    platform_log_info("HTTP server started successfully on port %d\n", port);
    platform_log_info("Features: Modular Architecture, Thread Pool, Buffer Pool, Epoll I/O\n");
    
    return 0;
}

/**
 * @brief Stop HTTP server
 */
int http_server_stop(void) {
    if (!g_server.running) {
        platform_log_info("HTTP server not running\n");
        return 0;
    }
    
    platform_log_info("Stopping HTTP server...\n");
    g_server.running = 0;
    
    // Stop epoll server
    epoll_server_cleanup();
    
    // Close server socket
    if (g_server.socket >= 0) {
        close(g_server.socket);
        g_server.socket = -1;
    }
    
    // Wait for epoll thread
    if (g_server.epoll_thread) {
        pthread_join(g_server.epoll_thread, NULL);
        g_server.epoll_thread = 0;
    }
    
    // Cleanup modules
    thread_pool_cleanup(&g_server.thread_pool);
    buffer_pool_cleanup(&g_server.buffer_pool);
    connection_manager_cleanup();
    
    platform_log_info("HTTP server stopped\n");
    platform_log_info("Final stats: %llu connections, %llu requests processed\n", 
                      (unsigned long long)g_server.connection_count,
                      (unsigned long long)g_server.request_count);
    
    return 0;
}
