/**
 * @file onvif_snapshot.c
 * @brief ONVIF Snapshot service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_snapshot.h"

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"
#include "utils/network/network_utils.h"

/* Snapshot service state */
static bool g_snapshot_initialized = false;                          // NOLINT
static platform_snapshot_handle_t g_snapshot_handle = NULL;          // NOLINT
static platform_vi_handle_t g_vi_handle = NULL;                      // NOLINT
static pthread_mutex_t g_snapshot_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT

/* Service handler instance */
static onvif_service_handler_instance_t g_snapshot_handler; // NOLINT
static int g_handler_initialized = 0;                       // NOLINT

/* Default snapshot resolution fallbacks */
#define DEFAULT_SNAPSHOT_WIDTH  640
#define DEFAULT_SNAPSHOT_HEIGHT 480

int onvif_snapshot_init(void) {
  if (g_snapshot_initialized) {
    return ONVIF_SUCCESS;
  }

  platform_log_info("Initializing ONVIF Snapshot service\n");

  // Get configuration from unified config system
  const struct application_config* config = config_runtime_snapshot();
  int snapshot_width = DEFAULT_SNAPSHOT_WIDTH;   // fallback default
  int snapshot_height = DEFAULT_SNAPSHOT_HEIGHT; // fallback default

  if (config && config->snapshot) {
    snapshot_width = config->snapshot->width;
    snapshot_height = config->snapshot->height;
    platform_log_info("Using snapshot resolution from config: %dx%d\n",
                     snapshot_width, snapshot_height);
  } else {
    platform_log_warning("No snapshot config available, using defaults: %dx%d\n",
                        snapshot_width, snapshot_height);
  }

  // Open video input for snapshots
  platform_result_t result = platform_vi_open(&g_vi_handle);
  if (result != PLATFORM_SUCCESS) {
    platform_log_error("Failed to open video input for snapshots\n");
    return ONVIF_ERROR;
  }

  // Initialize snapshot capture with configured resolution
  result = platform_snapshot_init(&g_snapshot_handle, g_vi_handle, snapshot_width,
                                  snapshot_height);
  if (result != PLATFORM_SUCCESS) {
    platform_log_error("Failed to initialize snapshot capture\n");
    platform_vi_close(g_vi_handle);
    g_vi_handle = NULL;
    return ONVIF_ERROR;
  }

  g_snapshot_initialized = true;
  platform_log_info("ONVIF Snapshot service initialized successfully with %dx%d\n",
                   snapshot_width, snapshot_height);
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
// NOTE: GetSnapshotUri has been removed - it belongs in the imaging service
// Snapshot service only handles actual JPEG snapshot retrieval from video frames via HTTP GET

/* Action definitions */
static const service_action_def_t snapshot_actions[] = {
  // No SOAP actions - snapshot service only serves JPEG files via HTTP GET, not SOAP requests
};

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
