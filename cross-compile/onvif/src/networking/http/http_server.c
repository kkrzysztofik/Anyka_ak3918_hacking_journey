/**
 * @file http_server.c
 * @brief Modular HTTP server with ONVIF SOAP service support
 * @author kkrzysztofik
 * @date 2025
 */

#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <unistd.h>

#include "http_onvif_adapter.h"
#include "http_parser.h"
#include "http_server.h"
#include "networking/common/buffer_pool.h"
#include "networking/common/connection_manager.h"
#include "networking/common/epoll_server.h"
#include "networking/common/thread_pool.h"
#include "platform/platform.h"
#include "networking/http/http_parser.h"
#include "protocol/xml/unified_xml.h"
#include "services/common/onvif_types.h"
#include "services/device/onvif_device.h"
#include "services/imaging/onvif_imaging.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"
#include "services/snapshot/onvif_snapshot.h"
#include "common/onvif_constants.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/network/network_utils.h"
#include "utils/validation/input_validation.h"
#include "utils/security/security_hardening.h"


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

/** @brief Global instances for shared components */
buffer_pool_t g_buffer_pool;
thread_pool_t g_thread_pool;

/** 
 * @brief HTTP server state structure
 * 
 * Maintains server runtime state including socket, threads, and statistics
 */
typedef struct {
    int running;              /**< Server running flag */
    int socket;               /**< Server socket file descriptor */
    pthread_t epoll_thread;   /**< Epoll event processing thread */
    thread_pool_t thread_pool;    /**< Thread pool for request processing */
    buffer_pool_t buffer_pool;    /**< Buffer pool for memory management */
    uint64_t connection_count;    /**< Total connections handled */
    uint64_t request_count;       /**< Total requests processed */
} server_state_t;

static server_state_t g_server = {0, -1, 0, {0}, {0}, 0, 0};

/** @brief Forward declarations */
static onvif_service_type_t get_service_type(const char *path);
static onvif_action_type_t get_action_type(const char *body);
void process_connection(void *conn);
static int handle_onvif_request(const http_request_t *request, char *response, size_t response_size);
static int handle_snapshot_request(const http_request_t *request, char *response, size_t response_size);
static int handle_utilization_request(const http_request_t *request, char *response, size_t response_size);

/**
 * @brief Determine ONVIF service type from URL path
 * 
 * @param path HTTP request path (e.g., "/onvif/device_service")
 * @return ONVIF service type, defaults to ONVIF_SERVICE_DEVICE if unknown
 */
static onvif_service_type_t get_service_type(const char *path) {
    if (!path) return ONVIF_SERVICE_DEVICE;
    
    // Use centralized path validation
    if (!validate_http_path(path)) {
        return ONVIF_SERVICE_DEVICE; // Default for invalid paths
    }
    
    // Map validated paths to service types
    if (strcmp(path, "/onvif/device_service") == 0) return ONVIF_SERVICE_DEVICE;
    if (strcmp(path, "/onvif/media_service") == 0) return ONVIF_SERVICE_MEDIA;
    if (strcmp(path, "/onvif/ptz_service") == 0) return ONVIF_SERVICE_PTZ;
    if (strcmp(path, "/onvif/imaging_service") == 0) return ONVIF_SERVICE_IMAGING;
    
    // Check for valid service paths with proper prefix (for backward compatibility)
    if (strncmp(path, "/onvif/device", 13) == 0) return ONVIF_SERVICE_DEVICE;
    if (strncmp(path, "/onvif/media", 12) == 0) return ONVIF_SERVICE_MEDIA;
    if (strncmp(path, "/onvif/ptz", 10) == 0) return ONVIF_SERVICE_PTZ;
    if (strncmp(path, "/onvif/imaging", 14) == 0) return ONVIF_SERVICE_IMAGING;
    
    return ONVIF_SERVICE_DEVICE;
}

/**
 * @brief Map validated action name to ONVIF action type
 * @param action_name Validated action name
 * @return ONVIF action type
 */
static onvif_action_type_t map_action_name_to_type(const char *action_name) {
    // Map the most common actions first for performance
    if (strcmp(action_name, "GetCapabilities") == 0) return ONVIF_ACTION_GET_CAPABILITIES;
    if (strcmp(action_name, "GetDeviceInformation") == 0) return ONVIF_ACTION_GET_DEVICE_INFORMATION;
    if (strcmp(action_name, "GetSystemDateAndTime") == 0) return ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME;
    if (strcmp(action_name, "SetSystemDateAndTime") == 0) return ONVIF_ACTION_SET_SYSTEM_DATE_AND_TIME;
    if (strcmp(action_name, "GetSystemLogging") == 0) return ONVIF_ACTION_GET_SYSTEM_LOGGING;
    if (strcmp(action_name, "GetScopes") == 0) return ONVIF_ACTION_GET_SCOPES;
    if (strcmp(action_name, "SetScopes") == 0) return ONVIF_ACTION_SET_SCOPES;
    if (strcmp(action_name, "AddScopes") == 0) return ONVIF_ACTION_ADD_SCOPES;
    if (strcmp(action_name, "RemoveScopes") == 0) return ONVIF_ACTION_REMOVE_SCOPES;
    if (strcmp(action_name, "GetDiscoveryMode") == 0) return ONVIF_ACTION_GET_DISCOVERY_MODE;
    if (strcmp(action_name, "SetDiscoveryMode") == 0) return ONVIF_ACTION_SET_DISCOVERY_MODE;
    if (strcmp(action_name, "GetRemoteDiscoveryMode") == 0) return ONVIF_ACTION_GET_REMOTE_DISCOVERY_MODE;
    if (strcmp(action_name, "SetRemoteDiscoveryMode") == 0) return ONVIF_ACTION_SET_REMOTE_DISCOVERY_MODE;
    if (strcmp(action_name, "GetDPAddresses") == 0) return ONVIF_ACTION_GET_DPADDRESSES;
    if (strcmp(action_name, "SetDPAddresses") == 0) return ONVIF_ACTION_SET_DPADDRESSES;
    if (strcmp(action_name, "GetNetworkInterfaces") == 0) return ONVIF_ACTION_GET_NETWORK_INTERFACES;
    if (strcmp(action_name, "SetNetworkInterfaces") == 0) return ONVIF_ACTION_SET_NETWORK_INTERFACES;
    if (strcmp(action_name, "GetNetworkProtocols") == 0) return ONVIF_ACTION_GET_NETWORK_PROTOCOLS;
    if (strcmp(action_name, "SetNetworkProtocols") == 0) return ONVIF_ACTION_SET_NETWORK_PROTOCOLS;
    if (strcmp(action_name, "GetNetworkDefaultGateway") == 0) return ONVIF_ACTION_GET_NETWORK_DEFAULT_GATEWAY;
    if (strcmp(action_name, "SetNetworkDefaultGateway") == 0) return ONVIF_ACTION_SET_NETWORK_DEFAULT_GATEWAY;
    if (strcmp(action_name, "GetZeroConfiguration") == 0) return ONVIF_ACTION_GET_ZERO_CONFIGURATION;
    if (strcmp(action_name, "SetZeroConfiguration") == 0) return ONVIF_ACTION_SET_ZERO_CONFIGURATION;
    if (strcmp(action_name, "GetIPAddressFilter") == 0) return ONVIF_ACTION_GET_IP_ADDRESS_FILTER;
    if (strcmp(action_name, "SetIPAddressFilter") == 0) return ONVIF_ACTION_SET_IP_ADDRESS_FILTER;
    if (strcmp(action_name, "AddIPAddressFilter") == 0) return ONVIF_ACTION_ADD_IP_ADDRESS_FILTER;
    if (strcmp(action_name, "RemoveIPAddressFilter") == 0) return ONVIF_ACTION_REMOVE_IP_ADDRESS_FILTER;
    if (strcmp(action_name, "GetAccessPolicy") == 0) return ONVIF_ACTION_GET_ACCESS_POLICY;
    if (strcmp(action_name, "SetAccessPolicy") == 0) return ONVIF_ACTION_SET_ACCESS_POLICY;
    if (strcmp(action_name, "CreateCertificate") == 0) return ONVIF_ACTION_CREATE_CERTIFICATE;
    if (strcmp(action_name, "GetCertificates") == 0) return ONVIF_ACTION_GET_CERTIFICATES;
    if (strcmp(action_name, "GetCertificateInformation") == 0) return ONVIF_ACTION_GET_CERTIFICATE_INFORMATION;
    if (strcmp(action_name, "SetCertificate") == 0) return ONVIF_ACTION_SET_CERTIFICATE;
    if (strcmp(action_name, "DeleteCertificate") == 0) return ONVIF_ACTION_DELETE_CERTIFICATE;
    if (strcmp(action_name, "GetPkcs10Request") == 0) return ONVIF_ACTION_GET_PKCS_10_REQUEST;
    if (strcmp(action_name, "LoadCertificateWithPrivateKey") == 0) return ONVIF_ACTION_LOAD_CERTIFICATE_WITH_PRIVATE_KEY;
    if (strcmp(action_name, "GetClientCertificateMode") == 0) return ONVIF_ACTION_GET_CLIENT_CERTIFICATE_MODE;
    if (strcmp(action_name, "SetClientCertificateMode") == 0) return ONVIF_ACTION_SET_CLIENT_CERTIFICATE_MODE;
    if (strcmp(action_name, "GetRelayOutputs") == 0) return ONVIF_ACTION_GET_RELAY_OUTPUTS;
    if (strcmp(action_name, "SetRelayOutputSettings") == 0) return ONVIF_ACTION_SET_RELAY_OUTPUT_SETTINGS;
    if (strcmp(action_name, "SetRelayOutputState") == 0) return ONVIF_ACTION_SET_RELAY_OUTPUT_STATE;
    if (strcmp(action_name, "GetServiceCapabilities") == 0) return ONVIF_ACTION_GET_SERVICE_CAPABILITIES;
    if (strcmp(action_name, "SystemReboot") == 0) return ONVIF_ACTION_SYSTEM_REBOOT;
    if (strcmp(action_name, "GetVideoSources") == 0) return ONVIF_ACTION_GET_VIDEO_SOURCES;
    if (strcmp(action_name, "GetVideoOutputs") == 0) return ONVIF_ACTION_GET_VIDEO_OUTPUTS;
    if (strcmp(action_name, "GetAudioSources") == 0) return ONVIF_ACTION_GET_AUDIO_SOURCES;
    if (strcmp(action_name, "GetAudioOutputs") == 0) return ONVIF_ACTION_GET_AUDIO_OUTPUTS;
    if (strcmp(action_name, "GetAudioSourceConfigurations") == 0) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATIONS;
    if (strcmp(action_name, "GetAudioOutputConfigurations") == 0) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATIONS;
    if (strcmp(action_name, "GetVideoSourceConfigurations") == 0) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATIONS;
    if (strcmp(action_name, "GetVideoOutputConfigurations") == 0) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATIONS;
    if (strcmp(action_name, "GetMetadataConfigurations") == 0) return ONVIF_ACTION_GET_METADATA_CONFIGURATIONS;
    if (strcmp(action_name, "GetCompositeConfigurations") == 0) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATIONS;
    if (strcmp(action_name, "GetAudioDecoderConfigurations") == 0) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATIONS;
    if (strcmp(action_name, "GetVideoAnalyticsConfigurations") == 0) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATIONS;
    if (strcmp(action_name, "GetPTZConfigurations") == 0) return ONVIF_ACTION_GET_PTZ_CONFIGURATIONS;
    if (strcmp(action_name, "GetVideoSourceConfiguration") == 0) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION;
    if (strcmp(action_name, "GetVideoOutputConfiguration") == 0) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION;
    if (strcmp(action_name, "GetAudioSourceConfiguration") == 0) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION;
    if (strcmp(action_name, "GetAudioOutputConfiguration") == 0) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION;
    if (strcmp(action_name, "GetMetadataConfiguration") == 0) return ONVIF_ACTION_GET_METADATA_CONFIGURATION;
    if (strcmp(action_name, "GetCompositeConfiguration") == 0) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION;
    if (strcmp(action_name, "GetAudioDecoderConfiguration") == 0) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION;
    if (strcmp(action_name, "GetVideoAnalyticsConfiguration") == 0) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION;
    if (strcmp(action_name, "GetPTZConfiguration") == 0) return ONVIF_ACTION_GET_PTZ_CONFIGURATION;
    if (strcmp(action_name, "GetVideoSourceConfigurationOptions") == 0) return ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetVideoOutputConfigurationOptions") == 0) return ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetAudioSourceConfigurationOptions") == 0) return ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetAudioOutputConfigurationOptions") == 0) return ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetMetadataConfigurationOptions") == 0) return ONVIF_ACTION_GET_METADATA_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetCompositeConfigurationOptions") == 0) return ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetAudioDecoderConfigurationOptions") == 0) return ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetVideoAnalyticsConfigurationOptions") == 0) return ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetPTZConfigurationOptions") == 0) return ONVIF_ACTION_GET_PTZ_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "GetGuaranteedVideoItemBounds") == 0) return ONVIF_ACTION_GET_GUARANTEED_VIDEO_ITEM_BOUNDS;
    if (strcmp(action_name, "GetStreamUri") == 0) return ONVIF_ACTION_GET_STREAM_URI;
    if (strcmp(action_name, "GetSnapshotUri") == 0) return ONVIF_ACTION_GET_SNAPSHOT_URI;
    if (strcmp(action_name, "GetProfiles") == 0) return ONVIF_ACTION_GET_PROFILES;
    if (strcmp(action_name, "AddProfile") == 0) return ONVIF_ACTION_ADD_PROFILE;
    if (strcmp(action_name, "RemoveProfile") == 0) return ONVIF_ACTION_REMOVE_PROFILE;
    if (strcmp(action_name, "GetVideoSourceMode") == 0) return ONVIF_ACTION_GET_VIDEO_SOURCE_MODE;
    if (strcmp(action_name, "SetVideoSourceMode") == 0) return ONVIF_ACTION_SET_VIDEO_SOURCE_MODE;
    if (strcmp(action_name, "GetOSD") == 0) return ONVIF_ACTION_GET_OSD;
    if (strcmp(action_name, "GetOSDOptions") == 0) return ONVIF_ACTION_GET_OSD_OPTIONS;
    if (strcmp(action_name, "SetOSD") == 0) return ONVIF_ACTION_SET_OSD;
    if (strcmp(action_name, "CreateOSD") == 0) return ONVIF_ACTION_CREATE_OSD;
    if (strcmp(action_name, "DeleteOSD") == 0) return ONVIF_ACTION_DELETE_OSD;
    if (strcmp(action_name, "GetMoveOptions") == 0) return ONVIF_ACTION_GET_MOVE_OPTIONS;
    if (strcmp(action_name, "GetStatus") == 0) return ONVIF_ACTION_GET_STATUS;
    if (strcmp(action_name, "GetConfiguration") == 0) return ONVIF_ACTION_GET_CONFIGURATION;
    if (strcmp(action_name, "GetConfigurations") == 0) return ONVIF_ACTION_GET_CONFIGURATIONS;
    if (strcmp(action_name, "GetCompatibleConfigurations") == 0) return ONVIF_ACTION_GET_COMPATIBLE_CONFIGURATIONS;
    if (strcmp(action_name, "SetConfiguration") == 0) return ONVIF_ACTION_SET_CONFIGURATION;
    if (strcmp(action_name, "GetConfigurationOptions") == 0) return ONVIF_ACTION_GET_CONFIGURATION_OPTIONS;
    if (strcmp(action_name, "Stop") == 0) return ONVIF_ACTION_STOP;
    if (strcmp(action_name, "AbsoluteMove") == 0) return ONVIF_ACTION_ABSOLUTE_MOVE;
    if (strcmp(action_name, "RelativeMove") == 0) return ONVIF_ACTION_RELATIVE_MOVE;
    if (strcmp(action_name, "ContinuousMove") == 0) return ONVIF_ACTION_CONTINUOUS_MOVE;
    if (strcmp(action_name, "GetPresets") == 0) return ONVIF_ACTION_GET_PRESETS;
    if (strcmp(action_name, "SetPreset") == 0) return ONVIF_ACTION_SET_PRESET;
    if (strcmp(action_name, "RemovePreset") == 0) return ONVIF_ACTION_REMOVE_PRESET;
    if (strcmp(action_name, "GotoPreset") == 0) return ONVIF_ACTION_GOTO_PRESET;
    if (strcmp(action_name, "GetImagingSettings") == 0) return ONVIF_ACTION_GET_IMAGING_SETTINGS;
    if (strcmp(action_name, "SetImagingSettings") == 0) return ONVIF_ACTION_SET_IMAGING_SETTINGS;
    if (strcmp(action_name, "GetOptions") == 0) return ONVIF_ACTION_GET_OPTIONS;
    
    // Default fallback
    return ONVIF_ACTION_GET_CAPABILITIES;
}

/**
 * @brief Parse ONVIF action type from SOAP request body
 * 
 * @param body SOAP request body content
 * @return ONVIF action type, defaults to ONVIF_ACTION_GET_CAPABILITIES if unknown
 */
static onvif_action_type_t get_action_type(const char *body) {
    if (!body) return ONVIF_ACTION_GET_CAPABILITIES;
    
    // Use safer XML tag matching instead of substring matching
    // Look for <Action> tag content or specific SOAP action patterns
    const char *action_start = strstr(body, "<Action>");
    if (action_start) {
        const char *action_end = strstr(action_start, "</Action>");
        if (action_end) {
            size_t action_len = action_end - action_start - 8; // Skip "<Action>"
            if (action_len > 0 && action_len < 64) { // Reasonable action name length
                char action_name[65];
                strncpy(action_name, action_start + 8, action_len);
                action_name[action_len] = '\0';
                
                // Trim whitespace
                char *trimmed = action_name;
                while (*trimmed == ' ' || *trimmed == '\t' || *trimmed == '\n' || *trimmed == '\r') {
                    trimmed++;
                }
                char *end = trimmed + strlen(trimmed) - 1;
                while (end > trimmed && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
                    *end = '\0';
                    end--;
                }
                
                // Use centralized validation
                if (validate_soap_action(trimmed)) {
                    // Map validated action to action type
                    return map_action_name_to_type(trimmed);
                }
            }
        }
    }
    
    // Default fallback for unrecognized actions
    return ONVIF_ACTION_GET_CAPABILITIES;
}

/**
 * @brief Process a single HTTP connection and handle ONVIF requests
 * 
 * This function handles the complete lifecycle of an HTTP connection:
 * 1. Reads data from socket
 * 2. Parses HTTP request
 * 3. Routes to appropriate ONVIF service handler
 * 4. Sends response back to client
 * 5. Manages connection keep-alive or cleanup
 * 
 * @param conn_ptr Pointer to connection_t structure
 */
void process_connection(void *conn_ptr) {
    connection_t *conn = (connection_t *)conn_ptr;
    if (!conn) return;
    
    conn->last_activity = platform_get_time_ms();
    
    ssize_t n = recv(conn->fd, conn->buffer + conn->buffer_used, 
                    conn->buffer_size - conn->buffer_used - 1, 0);
    if (n <= 0) {
        if (n < 0) {
            platform_log_error("Recv failed on fd %d: %s\n", conn->fd, strerror(errno));
        }
        // Atomically remove connection and clean up to prevent race conditions
        connection_remove_from_list(conn);
        if (conn->buffer) {
            buffer_pool_return(&g_server.buffer_pool, conn->buffer);
            conn->buffer = NULL; // Prevent double-free
        }
        connection_destroy(conn);
        return;
    }
    
    conn->buffer_used += n;
    conn->buffer[conn->buffer_used] = '\0';
    
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
        return;
    }
    
    // Validate HTTP request for security
    if (!validate_http_request(&request)) {
        platform_log_error("Invalid HTTP request on fd %d\n", conn->fd);
        http_response_t response = create_http_400_response();
        send_http_response(conn->fd, &response);
        connection_remove_from_list(conn);
        buffer_pool_return(&g_server.buffer_pool, conn->buffer);
        connection_destroy(conn);
        return;
    }
    
    char response_buffer[32768];
    int handled = 0;
    int response_len = 0;
    
    // Check if this is a snapshot request
    if (strstr(request.path, "snapshot.jpeg") != NULL) {
        response_len = handle_snapshot_request(&request, response_buffer, sizeof(response_buffer));
        if (response_len > 0) {
            // For snapshot requests, response_buffer already contains the complete HTTP response
            send(conn->fd, response_buffer, response_len, 0);
            handled = 1;
        }
    } else if (strstr(request.path, "utilization") != NULL) {
        // Handle system utilization requests
        response_len = handle_utilization_request(&request, response_buffer, sizeof(response_buffer));
        if (response_len > 0) {
            // For utilization requests, response_buffer already contains the complete HTTP response
            send(conn->fd, response_buffer, response_len, 0);
            handled = 1;
        }
    } else {
        // Handle ONVIF SOAP requests
        handled = handle_onvif_request(&request, response_buffer, sizeof(response_buffer));
        if (handled) {
            http_response_t response = create_http_200_response(response_buffer, strlen(response_buffer), NULL);
            send_http_response(conn->fd, &response);
        }
    }
    
    if (!handled) {
        http_response_t response = create_http_404_response();
        send_http_response(conn->fd, &response);
    }
    
    const char *connection = strcasestr(conn->buffer, "Connection:");
    if (connection && strcasestr(connection, "keep-alive")) {
        conn->keepalive_count++;
        if (conn->keepalive_count < 100) {
            conn->buffer_used = 0;
            conn->content_length = 0;
            conn->header_length = 0;
            conn->state = CONN_STATE_READING_HEADERS;
        } else {
            connection_remove_from_list(conn);
            buffer_pool_return(&g_server.buffer_pool, conn->buffer);
            connection_destroy(conn);
        }
    } else {
        connection_remove_from_list(conn);
        buffer_pool_return(&g_server.buffer_pool, conn->buffer);
        connection_destroy(conn);
    }
    
    g_server.request_count++;
}

/**
 * @brief Handle ONVIF SOAP request and generate response
 * 
 * Routes the request to the appropriate ONVIF service handler based on
 * URL path and SOAP action, then generates the SOAP response.
 * 
 * @param request HTTP request containing SOAP body
 * @param response Buffer to write SOAP response
 * @param response_size Size of response buffer
 * @return 1 if request was handled successfully, 0 otherwise
 */
static int handle_onvif_request(const http_request_t *request, char *response, size_t response_size) {
    if (!request || !response) return 0;
    
    if (strcmp(request->method, "POST") != 0) return 0;
    
    // Initialize security context
    security_context_t security_ctx;
    memset(&security_ctx, 0, sizeof(security_ctx));
    strncpy(security_ctx.client_ip, security_get_client_ip(request), sizeof(security_ctx.client_ip) - 1);
    security_ctx.client_ip[sizeof(security_ctx.client_ip) - 1] = '\0';
    security_ctx.security_level = SECURITY_LEVEL_ENHANCED;
    
    // Perform security validation
    // Authentication removed - proceed with request
    if (0) {
        ONVIF_LOG_ERROR("Security validation failed for request from %s\n", security_ctx.client_ip);
        return 0;
    }
    
    // Validate input for security threats
    if (request->body && request->body_length > 0) {
        if (security_validate_xml_structure(request->body, request->body_length, &security_ctx) != ONVIF_SUCCESS) {
            ONVIF_LOG_ERROR("XML security validation failed for request from %s\n", security_ctx.client_ip);
            return 0;
        }
    }
    
    onvif_service_type_t service = get_service_type(request->path);
    onvif_action_type_t action = get_action_type(request->body);
    
    onvif_request_t onvif_req;
    onvif_response_t onvif_resp;
    
    if (http_to_onvif_request(request, &onvif_req) != 0) {
        platform_log_error("Failed to convert HTTP request to ONVIF request\n");
        return 0;
    }
    
    onvif_req.action = action;
    memset(&onvif_resp, 0, sizeof(onvif_response_t));
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
    
    if (result == 0 && onvif_resp.body) {
        size_t copy_size = (onvif_resp.body_length < response_size - 1) ? 
                          onvif_resp.body_length : response_size - 1;
        memcpy(response, onvif_resp.body, copy_size);
        response[copy_size] = '\0';
    }
    
    onvif_request_cleanup(&onvif_req);
    onvif_response_cleanup(&onvif_resp);
    
    return result;
}

/**
 * @brief Start the HTTP server on specified port
 * 
 * Initializes all server components (thread pool, buffer pool, connection manager)
 * and starts listening for HTTP connections. The server uses epoll for efficient
 * I/O handling and a thread pool for concurrent request processing.
 * 
 * @param port TCP port number to bind the server to
 * @return 0 on success, negative error code on failure
 */
int http_server_start(int port) {
    if (g_server.running) {
        platform_log_warning("HTTP server already running\n");
        return -1;
    }
    
    platform_log_info("Starting modular HTTP server on port %d...\n", port);
    
    // Initialize security system
    if (security_init(SECURITY_LEVEL_ENHANCED) != ONVIF_SUCCESS) {
        platform_log_error("Failed to initialize security system\n");
        return -1;
    }
    
    if (connection_manager_init() != 0) {
        platform_log_error("Failed to initialize connection manager\n");
        security_cleanup();
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
    
    // Cleanup security system
    security_cleanup();
    
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

/**
 * @brief Handle snapshot HTTP requests
 * 
 * @param request HTTP request structure
 * @param response Response buffer
 * @param response_size Response buffer size
 * @return 0 on success, -1 on error
 */
static int handle_snapshot_request(const http_request_t *request, char *response, size_t response_size) {
    // Check if this is a GET request for snapshot.jpeg
    if (strcmp(request->method, "GET") != 0) {
        snprintf(response, response_size, 
                "HTTP/1.1 405 Method Not Allowed\r\n"
                "Content-Length: 0\r\n"
                "Allow: GET\r\n"
                "\r\n");
        return 0;
    }
    
    if (strstr(request->path, "snapshot.jpeg") == NULL) {
        snprintf(response, response_size,
                "HTTP/1.1 404 Not Found\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    // Capture snapshot
    uint8_t *snapshot_data = NULL;
    size_t snapshot_size = 0;
    
    int result = onvif_snapshot_capture(1280, 720, &snapshot_data, &snapshot_size);
    if (result != ONVIF_SUCCESS || !snapshot_data || snapshot_size == 0) {
        snprintf(response, response_size,
                "HTTP/1.1 500 Internal Server Error\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    // Send HTTP response with JPEG data
    int header_len = snprintf(response, response_size,
            "HTTP/1.1 200 OK\r\n"
            "Content-Type: image/jpeg\r\n"
            "Content-Length: %zu\r\n"
            "Connection: close\r\n"
            "\r\n",
            snapshot_size);
    
    if (header_len < 0 || header_len + snapshot_size >= response_size) {
        onvif_snapshot_release(snapshot_data);
        snprintf(response, response_size,
                "HTTP/1.1 500 Internal Server Error\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    // Copy snapshot data after headers
    memcpy(response + header_len, snapshot_data, snapshot_size);
    onvif_snapshot_release(snapshot_data);
    
    return header_len + snapshot_size;
}

/**
 * @brief Handle system utilization HTTP requests
 * 
 * @param request HTTP request structure
 * @param response Response buffer
 * @param response_size Response buffer size
 * @return Response length on success, 0 on error
 */
static int handle_utilization_request(const http_request_t *request, char *response, size_t response_size) {
    // Check if this is a GET request for utilization data
    if (strcmp(request->method, "GET") != 0) {
        snprintf(response, response_size, 
                "HTTP/1.1 405 Method Not Allowed\r\n"
                "Content-Length: 0\r\n"
                "Allow: GET\r\n"
                "\r\n");
        return 0;
    }
    
    if (strstr(request->path, "utilization") == NULL) {
        snprintf(response, response_size,
                "HTTP/1.1 404 Not Found\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    // Get system information
    platform_system_info_t sys_info;
    if (platform_get_system_info(&sys_info) != PLATFORM_SUCCESS) {
        snprintf(response, response_size,
                "HTTP/1.1 500 Internal Server Error\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    // Generate JSON response
    int json_len = snprintf(response, response_size,
            "HTTP/1.1 200 OK\r\n"
            "Content-Type: application/json\r\n"
            "Access-Control-Allow-Origin: *\r\n"
            "Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n"
            "Access-Control-Allow-Headers: Content-Type\r\n"
            "Connection: close\r\n"
            "\r\n"
            "{\n"
            "  \"cpu_usage\": %.2f,\n"
            "  \"cpu_temperature\": %.2f,\n"
            "  \"memory_total\": %llu,\n"
            "  \"memory_free\": %llu,\n"
            "  \"memory_used\": %llu,\n"
            "  \"uptime_ms\": %llu,\n"
            "  \"timestamp\": %llu\n"
            "}",
            sys_info.cpu_usage,
            sys_info.cpu_temperature,
            (unsigned long long)sys_info.total_memory,
            (unsigned long long)sys_info.free_memory,
            (unsigned long long)(sys_info.total_memory - sys_info.free_memory),
            (unsigned long long)sys_info.uptime_ms,
            (unsigned long long)platform_get_time_ms()
    );
    
    if (json_len < 0 || json_len >= response_size) {
        snprintf(response, response_size,
                "HTTP/1.1 500 Internal Server Error\r\n"
                "Content-Length: 0\r\n"
                "\r\n");
        return 0;
    }
    
    return json_len;
}
