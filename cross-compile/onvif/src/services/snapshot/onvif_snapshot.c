/**
 * @file onvif_snapshot.c
 * @brief ONVIF Snapshot service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_snapshot.h"

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"
#include "utils/network/network_utils.h"

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Snapshot service state */
static bool g_snapshot_initialized = false;                          // NOLINT
static platform_snapshot_handle_t g_snapshot_handle = NULL;          // NOLINT
static platform_vi_handle_t g_vi_handle = NULL;                      // NOLINT
static pthread_mutex_t g_snapshot_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT

/* Service handler instance */
static onvif_service_handler_instance_t g_snapshot_handler; // NOLINT
static int g_handler_initialized = 0;                       // NOLINT

/* Default snapshot resolution */
#define DEFAULT_SNAPSHOT_WIDTH  640
#define DEFAULT_SNAPSHOT_HEIGHT 480

int onvif_snapshot_init(void) {
  if (g_snapshot_initialized) {
    return ONVIF_SUCCESS;
  }

  platform_log_info("Initializing ONVIF Snapshot service\n");

  // Open video input for snapshots
  platform_result_t result = platform_vi_open(&g_vi_handle);
  if (result != PLATFORM_SUCCESS) {
    platform_log_error("Failed to open video input for snapshots\n");
    return ONVIF_ERROR;
  }

  // Initialize snapshot capture
  result = platform_snapshot_init(&g_snapshot_handle, g_vi_handle, DEFAULT_SNAPSHOT_WIDTH,
                                  DEFAULT_SNAPSHOT_HEIGHT);
  if (result != PLATFORM_SUCCESS) {
    platform_log_error("Failed to initialize snapshot capture\n");
    platform_vi_close(g_vi_handle);
    g_vi_handle = NULL;
    return ONVIF_ERROR;
  }

  g_snapshot_initialized = true;
  platform_log_info("ONVIF Snapshot service initialized successfully\n");
  return ONVIF_SUCCESS;
}

void onvif_snapshot_cleanup(void) {
  if (!g_snapshot_initialized) {
    return;
  }

  platform_log_info("Cleaning up ONVIF Snapshot service\n");

  pthread_mutex_lock(&g_snapshot_mutex);

  if (g_snapshot_handle) {
    platform_snapshot_cleanup(g_snapshot_handle);
    g_snapshot_handle = NULL;
  }

  if (g_vi_handle) {
    platform_vi_close(g_vi_handle);
    g_vi_handle = NULL;
  }

  g_snapshot_initialized = false;

  pthread_mutex_unlock(&g_snapshot_mutex);
}

int onvif_snapshot_capture(const snapshot_dimensions_t* dimensions, uint8_t** data, size_t* size) {
  if (!g_snapshot_initialized || !dimensions || !data || !size) {
    return ONVIF_ERROR_NULL;
  }

  // Validate dimensions
  if (dimensions->width <= 0 || dimensions->height <= 0) {
    platform_log_error("Invalid snapshot dimensions: %dx%d\n", dimensions->width,
                       dimensions->height);
    return ONVIF_ERROR_INVALID;
  }

  // Check if dimensions match the configured snapshot resolution
  if (dimensions->width != DEFAULT_SNAPSHOT_WIDTH ||
      dimensions->height != DEFAULT_SNAPSHOT_HEIGHT) {
    platform_log_warning("Requested snapshot dimensions %dx%d differ from configured %dx%d\n",
                         dimensions->width, dimensions->height, DEFAULT_SNAPSHOT_WIDTH,
                         DEFAULT_SNAPSHOT_HEIGHT);
    // Note: For now, we capture at the configured resolution
    // TODO: Implement dynamic resolution reconfiguration if needed
  }

  pthread_mutex_lock(&g_snapshot_mutex);

  platform_snapshot_t snapshot;
  platform_result_t result = platform_snapshot_capture(g_snapshot_handle, &snapshot, 5000);

  if (result != PLATFORM_SUCCESS) {
    platform_log_error("Failed to capture snapshot\n");
    pthread_mutex_unlock(&g_snapshot_mutex);
    return ONVIF_ERROR;
  }

  // Allocate memory for the snapshot data using safe memory management
  *data = ONVIF_MALLOC(snapshot.len);
  if (!*data) {
    platform_log_error("Failed to allocate memory for snapshot data\n");
    platform_snapshot_release(g_snapshot_handle, &snapshot);
    pthread_mutex_unlock(&g_snapshot_mutex);
    return ONVIF_ERROR_MEMORY;
  }

  // Copy snapshot data safely
  if (!memory_safe_memcpy(*data, snapshot.len, snapshot.data, snapshot.len)) {
    platform_log_error("Failed to copy snapshot data safely\n");
    MEMORY_SAFE_FREE(*data);
    platform_snapshot_release(g_snapshot_handle, &snapshot);
    pthread_mutex_unlock(&g_snapshot_mutex);
    return ONVIF_ERROR;
  }
  *size = snapshot.len;

  // Release the platform snapshot
  platform_snapshot_release(g_snapshot_handle, &snapshot);

  pthread_mutex_unlock(&g_snapshot_mutex);

  platform_log_info("Snapshot captured successfully: %zu bytes\n", *size);
  return ONVIF_SUCCESS;
}

void onvif_snapshot_release(uint8_t* data) {
  MEMORY_SAFE_FREE(data);
}

int onvif_snapshot_get_uri(const char* profile_token, struct stream_uri* uri) {
  if (!profile_token || !uri) {
    return ONVIF_ERROR_NULL;
  }

  // Generate snapshot URI
  build_device_url("http", ONVIF_SNAPSHOT_PORT_DEFAULT, SNAPSHOT_PATH, uri->uri, sizeof(uri->uri));
  uri->invalid_after_connect = 0;
  uri->invalid_after_reboot = 0;
  uri->timeout = 60;

  return ONVIF_SUCCESS;
}

/* Service handler action implementations */
static int handle_get_snapshot_uri(const service_handler_config_t* config,
                                   const http_request_t* request, http_response_t* response,
                                   onvif_gsoap_context_t* gsoap_ctx) {
  (void)request; // Suppress unused parameter warning
  // Basic parameter validation
  if (!config || !response || !gsoap_ctx) {
    return ONVIF_ERROR_NULL;
  }

  // Parse profile token from request using gsoap parser
  char profile_token[32];
  int parse_result =
    onvif_gsoap_parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token));
  if (parse_result != ONVIF_SUCCESS) {
    return onvif_gsoap_generate_fault_response(gsoap_ctx, SOAP_FAULT_SERVER,
                                               "Invalid profile token");
  }

  // Get snapshot URI using existing function
  struct stream_uri uri;
  int result = onvif_snapshot_get_uri(profile_token, &uri);

  if (result != ONVIF_SUCCESS) {
    return onvif_gsoap_generate_fault_response(gsoap_ctx, SOAP_FAULT_SERVER,
                                               "Internal server error");
  }

  // Generate simple SOAP response
  char response_buffer[1024];
  int response_len =
    snprintf(response_buffer, sizeof(response_buffer),
             "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
             "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" "
             "xmlns:timg=\"http://www.onvif.org/ver20/imaging/wsdl\" "
             "xmlns:tt=\"http://www.onvif.org/ver10/schema\">\n"
             "  <soap:Body>\n"
             "    <timg:GetSnapshotUriResponse>\n"
             "      <timg:MediaUri>\n"
             "        <tt:Uri>%s</tt:Uri>\n"
             "        <tt:InvalidAfterConnect>%s</tt:InvalidAfterConnect>\n"
             "        <tt:InvalidAfterReboot>%s</tt:InvalidAfterReboot>\n"
             "        <tt:Timeout>PT%dS</tt:Timeout>\n"
             "      </timg:MediaUri>\n"
             "    </timg:GetSnapshotUriResponse>\n"
             "  </soap:Body>\n"
             "</soap:Envelope>\n",
             uri.uri, uri.invalid_after_connect ? "true" : "false",
             uri.invalid_after_reboot ? "true" : "false", uri.timeout);

  if (response_len >= (int)sizeof(response_buffer)) {
    return onvif_gsoap_generate_fault_response(gsoap_ctx, SOAP_FAULT_SERVER, "Response too large");
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return onvif_gsoap_generate_fault_response(gsoap_ctx, SOAP_FAULT_SERVER,
                                                 "Memory allocation failed");
    }
  }

  // Copy response to output
  strncpy(response->body, response_buffer, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

/* Action definitions */
static const service_action_def_t snapshot_actions[] = {
  {"GetSnapshotUri", handle_get_snapshot_uri, 1}};

/* Service handler functions */
int onvif_snapshot_service_init(config_manager_t* config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {
    .service_type = ONVIF_SERVICE_IMAGING, // Snapshot is part of Imaging service
    .service_name = "Snapshot",
    .config = config,
    .enable_validation = 1,
    .enable_logging = 1};

  int result = onvif_service_handler_init(&g_snapshot_handler, &handler_config, snapshot_actions,
                                          sizeof(snapshot_actions) / sizeof(snapshot_actions[0]));

  if (result == ONVIF_SUCCESS) {
    g_handler_initialized = 1;
  }

  return result;
}

void onvif_snapshot_service_cleanup(void) {
  if (g_handler_initialized) {
    onvif_service_handler_cleanup(&g_snapshot_handler);
    g_handler_initialized = 0;

    // Check for memory leaks
    memory_manager_check_leaks();
  }
}

int onvif_snapshot_handle_request(const char* action_name, const http_request_t* request,
                                  http_response_t* response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }
  return onvif_service_handler_handle_request(&g_snapshot_handler, action_name, request, response);
}
