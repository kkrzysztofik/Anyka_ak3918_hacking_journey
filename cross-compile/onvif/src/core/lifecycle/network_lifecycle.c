/**
 * @file network_lifecycle.c
 * @brief Network services lifecycle management implementation
 *
 * This module implements the complete network services lifecycle management,
 * including HTTP server, WS-Discovery, and snapshot service initialization
 * and cleanup.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#include "core/lifecycle/network_lifecycle.h"

#include <stdbool.h>

#include "core/config/config.h"
#include "core/lifecycle/service_manager.h"
#include "core/lifecycle/video_lifecycle.h"
#include "networking/discovery/ws_discovery.h"
#include "networking/http/http_server.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"

/* Global network services state - static variable with internal linkage only */
static volatile bool g_network_services_initialized = false; // NOLINT

/* WS-Discovery protocol constants */
#define WS_DISCOVERY_PORT 3702 /* Standard WS-Discovery multicast port */

/* ---------------------------- Public Interface ------------------------- */

int network_lifecycle_init(const struct application_config* cfg) {
  platform_log_info("Initializing network services...\n");

  // Initialize ONVIF services
  if (onvif_services_init(video_lifecycle_get_vi_handle()) != 0) {
    platform_log_warning("warning: failed to initialize ONVIF services\n");
  }

  // Initialize snapshot service (temporarily disabled)
  // if (onvif_snapshot_init() != 0) {
  //   platform_log_warning("warning: failed to initialize snapshot service\n");
  // } else {
  //   platform_log_notice("Snapshot service initialized\n");
  // }
  platform_log_notice("Snapshot service temporarily disabled\n");

  // Start HTTP server (fatal if fails)
  if (http_server_start(cfg->onvif.http_port, cfg) != 0) {
    platform_log_error("failed to start HTTP server on port %d\n", cfg->onvif.http_port);
    return ONVIF_ERROR_NETWORK;
  }

  // Start WS-Discovery (non-fatal if fails)
  if (ws_discovery_start(cfg->onvif.http_port) != 0) {
    platform_log_warning("warning: WS-Discovery failed to start\n");
  } else {
    platform_log_notice("WS-Discovery responder active (multicast %s:%d)\n", "239.255.255.250", WS_DISCOVERY_PORT);
  }

  g_network_services_initialized = true;
  platform_log_info("Network services initialized successfully\n");
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PUBLIC API - Cleanup
 * ============================================================================ */

void network_lifecycle_cleanup(void) {
  static bool cleanup_done = false;

  if (cleanup_done) {
    platform_log_debug("Network cleanup already performed, skipping\n");
    return;
  }

  platform_log_info("Cleaning up network services...\n");

  // Stop network services
  ws_discovery_stop();
  http_server_stop();

  // Cleanup ONVIF services
  platform_log_info("Cleaning up ONVIF services...\n");
  // onvif_snapshot_cleanup(); // Temporarily disabled
  onvif_services_cleanup();

  g_network_services_initialized = false;
  cleanup_done = true;
  platform_log_info("Network services cleanup completed\n");
}

bool network_lifecycle_initialized(void) {
  return g_network_services_initialized;
}
