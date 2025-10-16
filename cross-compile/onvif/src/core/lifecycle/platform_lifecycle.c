/**
 * @file platform_lifecycle.c
 * @brief Platform initialization and cleanup management implementation
 *
 * This module implements platform initialization, memory management,
 * and overall system cleanup operations.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#include "core/lifecycle/platform_lifecycle.h"

#include <stdbool.h>

#include "core/lifecycle/network_lifecycle.h"
#include "core/lifecycle/video_lifecycle.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

/* Global platform state - static variables with internal linkage only */
static volatile bool g_platform_initialized = false; // NOLINT
static volatile int g_platform_status = 0;           // NOLINT

/* ---------------------------- Public Interface ------------------------- */

int platform_lifecycle_init(void) {
  platform_log_info("Initializing platform...\n");

  // Initialize memory manager first
  if (memory_manager_init() != ONVIF_SUCCESS) {
    platform_log_error("Failed to initialize memory manager\n");
    return ONVIF_ERROR_INITIALIZATION;
  }

  // Initialize platform components
  if (platform_init() != PLATFORM_SUCCESS) {
    platform_log_error("Failed to initialize platform\n");
    memory_manager_cleanup();
    return ONVIF_ERROR_HARDWARE;
  }

  g_platform_initialized = true;
  g_platform_status = PLATFORM_SUCCESS;
  platform_log_info("Platform initialized successfully\n");
  return ONVIF_SUCCESS;
}

void platform_lifecycle_cleanup(void) {
  static bool cleanup_done = false;

  if (cleanup_done) {
    platform_log_debug("Platform cleanup already performed, skipping\n");
    return;
  }

  platform_log_info("Performing full system cleanup...\n");

  // Cleanup video system first
  video_lifecycle_cleanup();

  // Cleanup network services
  network_lifecycle_cleanup();

  // Cleanup platform components
  platform_log_info("Cleaning up platform components...\n");
  platform_ptz_cleanup(); // Cleanup PTZ after services
  platform_cleanup();     // Cleanup platform last

  // Cleanup memory manager last
  platform_log_info("Cleaning up memory manager...\n");
  memory_manager_cleanup();

  g_platform_initialized = false;
  g_platform_status = 0;
  cleanup_done = true;
  platform_log_info("System cleanup completed\n");
}

bool platform_lifecycle_initialized(void) {
  return g_platform_initialized;
}

int platform_lifecycle_get_status(void) {
  return g_platform_status;
}
