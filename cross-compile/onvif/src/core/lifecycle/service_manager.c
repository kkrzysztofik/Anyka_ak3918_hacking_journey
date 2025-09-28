/**
 * @file service_manager.c
 * @brief ONVIF service manager implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "service_manager.h"

#include <stdbool.h>
#include <string.h>

#include "core/config/config.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "services/common/service_dispatcher.h"
#include "services/device/onvif_device.h"
#include "services/imaging/onvif_imaging.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"

static volatile bool g_service_services_initialized = false; // NOLINT

int onvif_services_init(platform_vi_handle_t vi_handle) {
  if (g_service_services_initialized) {
    return 0; // Already initialized
  }

  platform_log_info("Initializing ONVIF services...\n");

  // Create a temporary config manager for service initialization
  config_manager_t config;
  struct application_config app_config;
  memset(&config, 0, sizeof(config_manager_t));
  memset(&app_config, 0, sizeof(struct application_config));

  // Initialize config manager
  if (config_init(&config, &app_config) != 0) {
    platform_log_error("Failed to initialize config manager\n");
    return -1;
  }

  // Initialize service dispatcher (required before any service registration)
  if (onvif_service_dispatcher_init() != 0) {
    platform_log_error("Failed to initialize service dispatcher\n");
    config_cleanup(&config);
    return -1;
  }
  platform_log_info("Service dispatcher initialized\n");

  // Initialize Device service (required)
  if (onvif_device_init(&config) != 0) {
    platform_log_error("Failed to initialize Device service\n");
    config_cleanup(&config);
    return -1;
  }
  platform_log_info("Device service initialized\n");

  // Initialize Media service (required)
  if (onvif_media_init(&config) != 0) {
    platform_log_error("Failed to initialize Media service\n");
    onvif_device_cleanup();
    config_cleanup(&config);
    return -1;
  }
  platform_log_info("Media service initialized\n");

  // Initialize PTZ service (optional)
  if (ptz_adapter_init() != 0) {
    platform_log_warning("Failed to initialize PTZ service\n");
    // PTZ is optional, continue with other services
  } else {
    platform_log_info("PTZ service initialized\n");
  }

  // Initialize Imaging service (optional)
  if (onvif_imaging_init(vi_handle) != 0) {
    platform_log_warning("Failed to initialize Imaging service\n");
    // Imaging is optional, continue with other services
  } else {
    platform_log_info("Imaging service initialized\n");
  }

  // Clean up temporary config
  config_cleanup(&config);

  g_service_services_initialized = true;
  platform_log_info("ONVIF services initialization completed\n");

  return 0;
}

void onvif_services_cleanup(void) {
  if (!g_service_services_initialized) {
    return; // Already cleaned up
  }

  platform_log_info("Cleaning up ONVIF services...\n");

  // Cleanup services in reverse order
  onvif_imaging_cleanup();
  ptz_adapter_shutdown();
  onvif_media_cleanup();
  onvif_device_cleanup();

  // Cleanup service dispatcher last
  onvif_service_dispatcher_cleanup();

  g_service_services_initialized = false;
  platform_log_info("ONVIF services cleanup completed\n");
}

bool onvif_services_initialized(void) {
  return g_service_services_initialized;
}
