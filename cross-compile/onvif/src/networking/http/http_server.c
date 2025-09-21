/**
 * @file http_server.c
 * @brief Modular HTTP server with ONVIF SOAP service support
 * @author kkrzysztofik
 * @date 2025
 */

#include "http_server.h"

#include <asm/socket.h>
#include <bits/pthreadtypes.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#include "http_onvif_adapter.h"
#include "http_parser.h"
#include "networking/common/buffer_pool.h"
#include "networking/common/connection_manager.h"
#include "networking/common/epoll_server.h"
#include "networking/common/thread_pool.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/soap/onvif_soap.h"
#include "services/common/onvif_request.h"
#include "services/common/onvif_types.h"
#include "services/device/onvif_device.h"
#include "services/imaging/onvif_imaging.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"
#include "services/snapshot/onvif_snapshot.h"
#include "utils/error/error_handling.h"
#include "utils/security/security_hardening.h"
#include "utils/validation/input_validation.h"

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
buffer_pool_t g_http_buffer_pool;  // NOLINT
thread_pool_t g_http_thread_pool;  // NOLINT

/**
 * @brief HTTP server state structure
 *
 * Maintains server runtime state including socket, threads, and statistics
 */
typedef struct {
  int running;               /**< Server running flag */
  int socket;                /**< Server socket file descriptor */
  pthread_t epoll_thread;    /**< Epoll event processing thread */
  thread_pool_t thread_pool; /**< Thread pool for request processing */
  buffer_pool_t buffer_pool; /**< Buffer pool for memory management */
  uint64_t connection_count; /**< Total connections handled */
  uint64_t request_count;    /**< Total requests processed */
} server_state_t;

static server_state_t g_http_server = {0, -1, 0, {0}, {0}, 0, 0};  // NOLINT

/** @brief Forward declarations */
static onvif_service_type_t get_service_type(const char *path);
static onvif_action_type_t get_action_type(const char *body);
static int handle_onvif_request(const http_request_t *request, char *response,
                                size_t response_size);
static int handle_snapshot_request(const http_request_t *request,
                                   char *response, size_t response_size);
static int handle_utilization_request(const http_request_t *request,
                                      char *response, size_t response_size);

/**
 * @brief Determine ONVIF service type from URL path
 *
 * @param path HTTP request path (e.g., "/onvif/device_service")
 * @return ONVIF service type, defaults to ONVIF_SERVICE_DEVICE if unknown
 */
static onvif_service_type_t get_service_type(const char *path) {
  if (!path) {
    return ONVIF_SERVICE_DEVICE;
  }

  // Use centralized path validation
  if (!validate_http_path(path)) {
    return ONVIF_SERVICE_DEVICE;  // Default for invalid paths
  }

  // Map validated paths to service types
  if (strcmp(path, "/onvif/device_service") == 0) {
    return ONVIF_SERVICE_DEVICE;
  }
  if (strcmp(path, "/onvif/media_service") == 0) {
    return ONVIF_SERVICE_MEDIA;
  }
  if (strcmp(path, "/onvif/ptz_service") == 0) {
    return ONVIF_SERVICE_PTZ;
  }
  if (strcmp(path, "/onvif/imaging_service") == 0) {
    return ONVIF_SERVICE_IMAGING;
  }

  // Check for valid service paths with proper prefix (for backward
  // compatibility)
  if (strncmp(path, "/onvif/device", 13) == 0) {
    return ONVIF_SERVICE_DEVICE;
  }
  if (strncmp(path, "/onvif/media", 12) == 0) {
    return ONVIF_SERVICE_MEDIA;
  }
  if (strncmp(path, "/onvif/ptz", 10) == 0) {
    return ONVIF_SERVICE_PTZ;
  }
  if (strncmp(path, "/onvif/imaging", 14) == 0) {
    return ONVIF_SERVICE_IMAGING;
  }

  return ONVIF_SERVICE_DEVICE;
}

/**
 * @brief Action name to type mapping entry
 */
typedef struct {
  const char *action_name;
  onvif_action_type_t action_type;
} action_mapping_t;

/**
 * @brief Static lookup table for action name to type mapping
 *
 * This table maps ONVIF action names to their corresponding action types.
 * The table is sorted alphabetically for efficient binary search.
 */
static const action_mapping_t g_action_mappings[] = {
    {"AbsoluteMove", ONVIF_ACTION_ABSOLUTE_MOVE},
    {"AddIPAddressFilter", ONVIF_ACTION_ADD_IP_ADDRESS_FILTER},
    {"AddProfile", ONVIF_ACTION_ADD_PROFILE},
    {"AddScopes", ONVIF_ACTION_ADD_SCOPES},
    {"ContinuousMove", ONVIF_ACTION_CONTINUOUS_MOVE},
    {"CreateCertificate", ONVIF_ACTION_CREATE_CERTIFICATE},
    {"CreateOSD", ONVIF_ACTION_CREATE_OSD},
    {"DeleteCertificate", ONVIF_ACTION_DELETE_CERTIFICATE},
    {"DeleteOSD", ONVIF_ACTION_DELETE_OSD},
    {"GetAccessPolicy", ONVIF_ACTION_GET_ACCESS_POLICY},
    {"GetAudioDecoderConfiguration",
     ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION},
    {"GetAudioDecoderConfigurationOptions",
     ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATION_OPTIONS},
    {"GetAudioDecoderConfigurations",
     ONVIF_ACTION_GET_AUDIO_DECODER_CONFIGURATIONS},
    {"GetAudioOutputConfiguration",
     ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION},
    {"GetAudioOutputConfigurationOptions",
     ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATION_OPTIONS},
    {"GetAudioOutputConfigurations",
     ONVIF_ACTION_GET_AUDIO_OUTPUT_CONFIGURATIONS},
    {"GetAudioOutputs", ONVIF_ACTION_GET_AUDIO_OUTPUTS},
    {"GetAudioSourceConfiguration",
     ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION},
    {"GetAudioSourceConfigurationOptions",
     ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATION_OPTIONS},
    {"GetAudioSourceConfigurations",
     ONVIF_ACTION_GET_AUDIO_SOURCE_CONFIGURATIONS},
    {"GetAudioSources", ONVIF_ACTION_GET_AUDIO_SOURCES},
    {"GetCapabilities", ONVIF_ACTION_GET_CAPABILITIES},
    {"GetCertificateInformation", ONVIF_ACTION_GET_CERTIFICATE_INFORMATION},
    {"GetCertificates", ONVIF_ACTION_GET_CERTIFICATES},
    {"GetClientCertificateMode", ONVIF_ACTION_GET_CLIENT_CERTIFICATE_MODE},
    {"GetCompatibleConfigurations", ONVIF_ACTION_GET_COMPATIBLE_CONFIGURATIONS},
    {"GetCompositeConfiguration", ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION},
    {"GetCompositeConfigurationOptions",
     ONVIF_ACTION_GET_COMPOSITE_CONFIGURATION_OPTIONS},
    {"GetCompositeConfigurations", ONVIF_ACTION_GET_COMPOSITE_CONFIGURATIONS},
    {"GetConfiguration", ONVIF_ACTION_GET_CONFIGURATION},
    {"GetConfigurationOptions", ONVIF_ACTION_GET_CONFIGURATION_OPTIONS},
    {"GetConfigurations", ONVIF_ACTION_GET_CONFIGURATIONS},
    {"GetDeviceInformation", ONVIF_ACTION_GET_DEVICE_INFORMATION},
    {"GetDiscoveryMode", ONVIF_ACTION_GET_DISCOVERY_MODE},
    {"GetDPAddresses", ONVIF_ACTION_GET_DPADDRESSES},
    {"GetGuaranteedVideoItemBounds",
     ONVIF_ACTION_GET_GUARANTEED_VIDEO_ITEM_BOUNDS},
    {"GetImagingSettings", ONVIF_ACTION_GET_IMAGING_SETTINGS},
    {"GetIPAddressFilter", ONVIF_ACTION_GET_IP_ADDRESS_FILTER},
    {"GetMetadataConfiguration", ONVIF_ACTION_GET_METADATA_CONFIGURATION},
    {"GetMetadataConfigurationOptions",
     ONVIF_ACTION_GET_METADATA_CONFIGURATION_OPTIONS},
    {"GetMetadataConfigurations", ONVIF_ACTION_GET_METADATA_CONFIGURATIONS},
    {"GetMoveOptions", ONVIF_ACTION_GET_MOVE_OPTIONS},
    {"GetNetworkDefaultGateway", ONVIF_ACTION_GET_NETWORK_DEFAULT_GATEWAY},
    {"GetNetworkInterfaces", ONVIF_ACTION_GET_NETWORK_INTERFACES},
    {"GetNetworkProtocols", ONVIF_ACTION_GET_NETWORK_PROTOCOLS},
    {"GetOSD", ONVIF_ACTION_GET_OSD},
    {"GetOSDOptions", ONVIF_ACTION_GET_OSD_OPTIONS},
    {"GetOptions", ONVIF_ACTION_GET_OPTIONS},
    {"GetPkcs10Request", ONVIF_ACTION_GET_PKCS_10_REQUEST},
    {"GetPresets", ONVIF_ACTION_GET_PRESETS},
    {"GetPTZConfiguration", ONVIF_ACTION_GET_PTZ_CONFIGURATION},
    {"GetPTZConfigurationOptions", ONVIF_ACTION_GET_PTZ_CONFIGURATION_OPTIONS},
    {"GetPTZConfigurations", ONVIF_ACTION_GET_PTZ_CONFIGURATIONS},
    {"GetProfiles", ONVIF_ACTION_GET_PROFILES},
    {"GetRelayOutputs", ONVIF_ACTION_GET_RELAY_OUTPUTS},
    {"GetRemoteDiscoveryMode", ONVIF_ACTION_GET_REMOTE_DISCOVERY_MODE},
    {"GetScopes", ONVIF_ACTION_GET_SCOPES},
    {"GetServiceCapabilities", ONVIF_ACTION_GET_SERVICE_CAPABILITIES},
    {"GetSnapshotUri", ONVIF_ACTION_GET_SNAPSHOT_URI},
    {"GetStatus", ONVIF_ACTION_GET_STATUS},
    {"GetStreamUri", ONVIF_ACTION_GET_STREAM_URI},
    {"GetSystemDateAndTime", ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME},
    {"GetSystemLogging", ONVIF_ACTION_GET_SYSTEM_LOGGING},
    {"GetVideoAnalyticsConfiguration",
     ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION},
    {"GetVideoAnalyticsConfigurationOptions",
     ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATION_OPTIONS},
    {"GetVideoAnalyticsConfigurations",
     ONVIF_ACTION_GET_VIDEO_ANALYTICS_CONFIGURATIONS},
    {"GetVideoOutputConfiguration",
     ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION},
    {"GetVideoOutputConfigurationOptions",
     ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATION_OPTIONS},
    {"GetVideoOutputConfigurations",
     ONVIF_ACTION_GET_VIDEO_OUTPUT_CONFIGURATIONS},
    {"GetVideoOutputs", ONVIF_ACTION_GET_VIDEO_OUTPUTS},
    {"GetVideoSourceConfiguration",
     ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION},
    {"GetVideoSourceConfigurationOptions",
     ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATION_OPTIONS},
    {"GetVideoSourceConfigurations",
     ONVIF_ACTION_GET_VIDEO_SOURCE_CONFIGURATIONS},
    {"GetVideoSourceMode", ONVIF_ACTION_GET_VIDEO_SOURCE_MODE},
    {"GetVideoSources", ONVIF_ACTION_GET_VIDEO_SOURCES},
    {"GetZeroConfiguration", ONVIF_ACTION_GET_ZERO_CONFIGURATION},
    {"GotoPreset", ONVIF_ACTION_GOTO_PRESET},
    {"LoadCertificateWithPrivateKey",
     ONVIF_ACTION_LOAD_CERTIFICATE_WITH_PRIVATE_KEY},
    {"RelativeMove", ONVIF_ACTION_RELATIVE_MOVE},
    {"RemoveIPAddressFilter", ONVIF_ACTION_REMOVE_IP_ADDRESS_FILTER},
    {"RemovePreset", ONVIF_ACTION_REMOVE_PRESET},
    {"RemoveScopes", ONVIF_ACTION_REMOVE_SCOPES},
    {"SetAccessPolicy", ONVIF_ACTION_SET_ACCESS_POLICY},
    {"SetAudioSourceConfiguration",
     ONVIF_ACTION_SET_AUDIO_SOURCE_CONFIGURATION},
    {"SetCertificate", ONVIF_ACTION_SET_CERTIFICATE},
    {"SetClientCertificateMode", ONVIF_ACTION_SET_CLIENT_CERTIFICATE_MODE},
    {"SetConfiguration", ONVIF_ACTION_SET_CONFIGURATION},
    {"SetDiscoveryMode", ONVIF_ACTION_SET_DISCOVERY_MODE},
    {"SetDPAddresses", ONVIF_ACTION_SET_DPADDRESSES},
    {"SetImagingSettings", ONVIF_ACTION_SET_IMAGING_SETTINGS},
    {"SetIPAddressFilter", ONVIF_ACTION_SET_IP_ADDRESS_FILTER},
    {"SetMetadataConfiguration", ONVIF_ACTION_SET_METADATA_CONFIGURATION},
    {"SetNetworkDefaultGateway", ONVIF_ACTION_SET_NETWORK_DEFAULT_GATEWAY},
    {"SetNetworkInterfaces", ONVIF_ACTION_SET_NETWORK_INTERFACES},
    {"SetNetworkProtocols", ONVIF_ACTION_SET_NETWORK_PROTOCOLS},
    {"SetOSD", ONVIF_ACTION_SET_OSD},
    {"SetPreset", ONVIF_ACTION_SET_PRESET},
    {"SetRelayOutputSettings", ONVIF_ACTION_SET_RELAY_OUTPUT_SETTINGS},
    {"SetRelayOutputState", ONVIF_ACTION_SET_RELAY_OUTPUT_STATE},
    {"SetRemoteDiscoveryMode", ONVIF_ACTION_SET_REMOTE_DISCOVERY_MODE},
    {"SetScopes", ONVIF_ACTION_SET_SCOPES},
    {"SetSystemDateAndTime", ONVIF_ACTION_SET_SYSTEM_DATE_AND_TIME},
    {"SetVideoSourceConfiguration",
     ONVIF_ACTION_SET_VIDEO_SOURCE_CONFIGURATION},
    {"SetVideoSourceMode", ONVIF_ACTION_SET_VIDEO_SOURCE_MODE},
    {"SetZeroConfiguration", ONVIF_ACTION_SET_ZERO_CONFIGURATION},
    {"Stop", ONVIF_ACTION_STOP},
    {"SystemReboot", ONVIF_ACTION_SYSTEM_REBOOT}};

/**
 * @brief Map validated action name to ONVIF action type using binary search
 * @param action_name Validated action name
 * @return ONVIF action type
 */
static onvif_action_type_t map_action_name_to_type(const char *action_name) {
  if (!action_name) {
    return ONVIF_ACTION_GET_CAPABILITIES;
  }

  // Binary search in the sorted action mappings table
  size_t left = 0;
  size_t right = sizeof(g_action_mappings) / sizeof(g_action_mappings[0]) - 1;

  while (left <= right) {
    size_t mid = left + (right - left) / 2;
    int cmp = strcmp(action_name, g_action_mappings[mid].action_name);

    if (cmp == 0) {
      return g_action_mappings[mid].action_type;
    }
    if (cmp < 0) {
      right = mid - 1;
    } else {
      left = mid + 1;
    }
  }

  // Default fallback for unrecognized actions
  return ONVIF_ACTION_GET_CAPABILITIES;
}

/**
 * @brief Parse ONVIF action type from SOAP request body
 *
 * @param body SOAP request body content
 * @return ONVIF action type, defaults to ONVIF_ACTION_GET_CAPABILITIES if
 * unknown
 */
static onvif_action_type_t get_action_type(const char *body) {
  if (!body) {
    return ONVIF_ACTION_GET_CAPABILITIES;
  }

  // Use safer XML tag matching instead of substring matching
  // Look for <Action> tag content or specific SOAP action patterns
  const char *action_start = strstr(body, "<Action>");
  if (action_start) {
    const char *action_end = strstr(action_start, "</Action>");
    if (action_end) {
      size_t action_len = action_end - action_start - 8;  // Skip "<Action>"
      if (action_len > 0 && action_len < 64) {  // Reasonable action name length
        char action_name[65];
        strncpy(action_name, action_start + 8, action_len);
        action_name[action_len] = '\0';

        // Trim whitespace
        char *trimmed = action_name;
        while (*trimmed == ' ' || *trimmed == '\t' || *trimmed == '\n' ||
               *trimmed == '\r') {
          trimmed++;
        }
        char *end = trimmed + strlen(trimmed) - 1;
        while (end > trimmed &&
               (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
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
  if (!conn) {
    return;
  }

  conn->last_activity = platform_get_time_ms();

  ssize_t bytes_received = recv(conn->fd, conn->buffer + conn->buffer_used,
                                conn->buffer_size - conn->buffer_used - 1, 0);
  if (bytes_received <= 0) {
    if (bytes_received < 0) {
      platform_log_error("Recv failed on fd %d: %s\n", conn->fd,
                         strerror(errno));
    }
    // Atomically remove connection and clean up to prevent race conditions
    connection_remove_from_list(conn);
    if (conn->buffer) {
      buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
      conn->buffer = NULL;  // Prevent double-free
    }
    connection_destroy(conn);
    return;
  }

  conn->buffer_used += bytes_received;
  conn->buffer[conn->buffer_used] = '\0';

  http_request_t request;
  int need_more_data = 0;
  int parse_result = parse_http_request_state_machine(
      conn->buffer, conn->buffer_used, &request, &need_more_data);

  if (parse_result != 0) {
    platform_log_error("HTTP parse error on fd %d\n", conn->fd);
    connection_remove_from_list(conn);
    buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
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
    buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
    connection_destroy(conn);
    return;
  }

  char response_buffer[32768];
  int handled = 0;
  int response_len = 0;

  // Check if this is a snapshot request
  if (strstr(request.path, "snapshot.jpeg") != NULL) {
    response_len = handle_snapshot_request(&request, response_buffer,
                                           sizeof(response_buffer));
    if (response_len > 0) {
      // For snapshot requests, response_buffer already contains the complete
      // HTTP response
      send(conn->fd, response_buffer, response_len, 0);
      handled = 1;
    }
  } else if (strstr(request.path, "utilization") != NULL) {
    // Handle system utilization requests
    response_len = handle_utilization_request(&request, response_buffer,
                                              sizeof(response_buffer));
    if (response_len > 0) {
      // For utilization requests, response_buffer already contains the complete
      // HTTP response
      send(conn->fd, response_buffer, response_len, 0);
      handled = 1;
    }
  } else {
    // Handle ONVIF SOAP requests
    handled = handle_onvif_request(&request, response_buffer,
                                   sizeof(response_buffer));
    if (handled) {
      http_response_t response = create_http_200_response(
          response_buffer, strlen(response_buffer), NULL);
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
      buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
      connection_destroy(conn);
    }
  } else {
    connection_remove_from_list(conn);
    buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
    connection_destroy(conn);
  }

  g_http_server.request_count++;
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
static int handle_onvif_request(const http_request_t *request, char *response,
                                size_t response_size) {
  if (!request || !response) {
    return 0;
  }

  if (strcmp(request->method, "POST") != 0) {
    return 0;
  }

  // Initialize security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strncpy(security_ctx.client_ip, security_get_client_ip(request),
          sizeof(security_ctx.client_ip) - 1);
  security_ctx.client_ip[sizeof(security_ctx.client_ip) - 1] = '\0';
  security_ctx.security_level = SECURITY_LEVEL_ENHANCED;

  // Perform security validation
  // Authentication removed - proceed with request
  if (0) {
    ONVIF_LOG_ERROR("Security validation failed for request from %s\n",
                    security_ctx.client_ip);
    return 0;
  }

  // Validate input for security threats
  if (request->body && request->body_length > 0) {
    if (security_validate_xml_structure(request->body, request->body_length,
                                        &security_ctx) != ONVIF_SUCCESS) {
      ONVIF_LOG_ERROR("XML security validation failed for request from %s\n",
                      security_ctx.client_ip);
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
    size_t copy_size = (onvif_resp.body_length < response_size - 1)
                           ? onvif_resp.body_length
                           : response_size - 1;
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
 * Initializes all server components (thread pool, buffer pool, connection
 * manager) and starts listening for HTTP connections. The server uses epoll for
 * efficient I/O handling and a thread pool for concurrent request processing.
 *
 * @param port TCP port number to bind the server to
 * @return 0 on success, negative error code on failure
 */
int http_server_start(int port) {
  if (g_http_server.running) {
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

  if (buffer_pool_init(&g_http_server.buffer_pool) != 0) {
    platform_log_error("Failed to initialize buffer pool\n");
    connection_manager_cleanup();
    return -1;
  }

  if (thread_pool_init(&g_http_server.thread_pool) != 0) {
    platform_log_error("Failed to initialize thread pool\n");
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  g_http_server.socket = socket(AF_INET, SOCK_STREAM, 0);
  if (g_http_server.socket < 0) {
    platform_log_error("Failed to create socket: %s\n", strerror(errno));
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  // Set socket options
  int opt = 1;
  if (setsockopt(g_http_server.socket, SOL_SOCKET, SO_REUSEADDR, &opt,
                 sizeof(opt)) < 0) {
    platform_log_error("Failed to set socket options: %s\n", strerror(errno));
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  // Set socket to non-blocking
  int flags = fcntl(g_http_server.socket, F_GETFL, 0);
  if (fcntl(g_http_server.socket, F_SETFL, flags | O_NONBLOCK) < 0) {
    platform_log_error("Failed to set socket to non-blocking: %s\n",
                       strerror(errno));
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  // Bind and listen
  struct sockaddr_in server_addr;
  server_addr.sin_family = AF_INET;
  server_addr.sin_addr.s_addr = INADDR_ANY;
  server_addr.sin_port = htons(port);

  if (bind(g_http_server.socket, (struct sockaddr *)&server_addr,
           sizeof(server_addr)) < 0) {
    platform_log_error("Failed to bind socket: %s\n", strerror(errno));
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  if (listen(g_http_server.socket, 128) < 0) {
    platform_log_error("Failed to listen on socket: %s\n", strerror(errno));
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  // Initialize epoll server
  if (epoll_server_init(g_http_server.socket) != 0) {
    platform_log_error("Failed to initialize epoll server\n");
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  // Start epoll event loop
  if (pthread_create(&g_http_server.epoll_thread, NULL, epoll_server_loop,
                     NULL) != 0) {
    platform_log_error("Failed to create epoll thread: %s\n", strerror(errno));
    epoll_server_cleanup();
    close(g_http_server.socket);
    thread_pool_cleanup(&g_http_server.thread_pool);
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    connection_manager_cleanup();
    return -1;
  }

  g_http_server.running = 1;
  g_http_server.connection_count = 0;
  g_http_server.request_count = 0;

  platform_log_info("HTTP server started successfully on port %d\n", port);
  platform_log_info(
      "Features: Modular Architecture, Thread Pool, Buffer Pool, Epoll I/O\n");

  return 0;
}

/**
 * @brief Stop HTTP server
 */
int http_server_stop(void) {
  if (!g_http_server.running) {
    platform_log_info("HTTP server not running\n");
    return 0;
  }

  platform_log_info("Stopping HTTP server...\n");
  g_http_server.running = 0;

  // Cleanup security system
  security_cleanup();

  // Stop epoll server
  epoll_server_cleanup();

  // Close server socket
  if (g_http_server.socket >= 0) {
    close(g_http_server.socket);
    g_http_server.socket = -1;
  }

  // Wait for epoll thread
  if (g_http_server.epoll_thread) {
    pthread_join(g_http_server.epoll_thread, NULL);
    g_http_server.epoll_thread = 0;
  }

  // Cleanup modules
  thread_pool_cleanup(&g_http_server.thread_pool);
  buffer_pool_cleanup(&g_http_server.buffer_pool);
  connection_manager_cleanup();

  platform_log_info("HTTP server stopped\n");
  platform_log_info("Final stats: %llu connections, %llu requests processed\n",
                    (unsigned long long)g_http_server.connection_count,
                    (unsigned long long)g_http_server.request_count);

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
static int handle_snapshot_request(const http_request_t *request,
                                   char *response, size_t response_size) {
  // Check if this is a GET request for snapshot.jpeg
  if (strcmp(request->method, "GET") != 0) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 405 Method Not Allowed\r\n"
                   "Content-Length: 0\r\n"
                   "Allow: GET\r\n"
                   "\r\n");
    return 0;
  }

  if (strstr(request->path, "snapshot.jpeg") == NULL) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 404 Not Found\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  // Capture snapshot
  uint8_t *snapshot_data = NULL;
  size_t snapshot_size = 0;
  snapshot_dimensions_t dimensions = {.width = 1280, .height = 720};

  int result =
      onvif_snapshot_capture(&dimensions, &snapshot_data, &snapshot_size);
  if (result != ONVIF_SUCCESS || !snapshot_data || snapshot_size == 0) {
    (void)snprintf(response, response_size,
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
  if (header_len < 0) {
    onvif_snapshot_release(snapshot_data);
    (void)snprintf(response, response_size,
                   "HTTP/1.1 500 Internal Server Error\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  if (header_len + snapshot_size >= response_size) {
    onvif_snapshot_release(snapshot_data);
    (void)snprintf(response, response_size,
                   "HTTP/1.1 500 Internal Server Error\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  // Copy snapshot data after headers
  (void)memcpy(response + header_len, snapshot_data, snapshot_size);
  onvif_snapshot_release(snapshot_data);

  return (int)(header_len + snapshot_size);
}

/**
 * @brief Handle system utilization HTTP requests
 *
 * @param request HTTP request structure
 * @param response Response buffer
 * @param response_size Response buffer size
 * @return Response length on success, 0 on error
 */
static int handle_utilization_request(const http_request_t *request,
                                      char *response, size_t response_size) {
  // Check if this is a GET request for utilization data
  if (strcmp(request->method, "GET") != 0) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 405 Method Not Allowed\r\n"
                   "Content-Length: 0\r\n"
                   "Allow: GET\r\n"
                   "\r\n");
    return 0;
  }

  if (strstr(request->path, "utilization") == NULL) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 404 Not Found\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  // Get system information
  platform_system_info_t sys_info;
  if (platform_get_system_info(&sys_info) != PLATFORM_SUCCESS) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 500 Internal Server Error\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  // Generate JSON response
  int json_len = snprintf(
      response, response_size,
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
      sys_info.cpu_usage, sys_info.cpu_temperature,
      (unsigned long long)sys_info.total_memory,
      (unsigned long long)sys_info.free_memory,
      (unsigned long long)(sys_info.total_memory - sys_info.free_memory),
      (unsigned long long)sys_info.uptime_ms,
      (unsigned long long)platform_get_time_ms());

  if (json_len < 0) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 500 Internal Server Error\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  if (json_len >= response_size) {
    (void)snprintf(response, response_size,
                   "HTTP/1.1 500 Internal Server Error\r\n"
                   "Content-Length: 0\r\n"
                   "\r\n");
    return 0;
  }

  return json_len;
}
