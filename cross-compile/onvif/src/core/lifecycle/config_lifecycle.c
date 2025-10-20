/**
 * @file config_lifecycle.c
 * @brief Configuration management lifecycle implementation
 *
 * This module implements configuration loading, memory allocation for
 * configuration structures, and configuration cleanup operations.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#include "core/lifecycle/config_lifecycle.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "platform/platform.h"
#include "services/common/onvif_imaging_types.h"
#include "utils/error/error_handling.h"
#include "utils/stream/stream_config_utils.h"

/* ============================================================================
 * Constants - Video Stream Defaults
 * ============================================================================ */

/* Default video stream parameters */
#define CONFIG_DEFAULT_FPS       15   /* Default FPS for sensor compatibility */
#define CONFIG_MAIN_KBPS_DEFAULT 2048 /* Main stream default bitrate (kbps) */
#define CONFIG_SUB_KBPS_DEFAULT  800  /* Sub stream default bitrate (kbps) */

/* ============================================================================
 * Global State
 * ============================================================================ */

static volatile bool g_config_loaded = false; // NOLINT

/* ============================================================================
 * PUBLIC API - Memory Management
 * ============================================================================ */

int config_lifecycle_allocate_memory(struct application_config* cfg) {
  platform_log_info("Initializing configuration structures...\n");

  /* Initialize all struct fields to zero */
  memset(cfg, 0, sizeof(struct application_config));

  platform_log_info("Configuration structures initialized successfully\n");
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PUBLIC API - Configuration Loading
 * ============================================================================ */

int config_lifecycle_load_configuration(struct application_config* cfg) {
  platform_log_info("Loading configuration...\n");

  /* T019: Initialize runtime configuration manager with loaded config */
  if (config_runtime_init(cfg) != ONVIF_SUCCESS) {
    platform_log_error("error: failed to initialize runtime configuration system\n");
    return ONVIF_ERROR_INITIALIZATION;
  }

  /* Load configuration from INI file using new storage system */
  if (config_storage_load(ONVIF_CONFIG_FILE, NULL) != ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to read config at %s\n", ONVIF_CONFIG_FILE);
    platform_log_warning("warning: using default configuration (embedded)\n");
  }

  // Initialize stream configurations from anyka_cfg.ini parameters
  int main_fps = CONFIG_DEFAULT_FPS; // Default FPS for sensor compatibility
  int main_kbps = CONFIG_MAIN_KBPS_DEFAULT;
  int sub_fps = CONFIG_DEFAULT_FPS; // Default FPS for sensor compatibility
  int sub_kbps = CONFIG_SUB_KBPS_DEFAULT;

  // Try to get values from config file using new runtime API
  config_runtime_get_int(CONFIG_SECTION_MAIN_STREAM, "main_fps", &main_fps);
  config_runtime_get_int(CONFIG_SECTION_MAIN_STREAM, "main_kbps", &main_kbps);
  config_runtime_get_int(CONFIG_SECTION_SUB_STREAM, "sub_fps", &sub_fps);
  config_runtime_get_int(CONFIG_SECTION_SUB_STREAM, "sub_kbps", &sub_kbps);

  // Initialize main stream configuration
  if (stream_config_init_from_anyka(&cfg->main_stream, true, (unsigned int)main_kbps, main_fps) != ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to initialize main stream config, using defaults\n");
    stream_config_init_defaults(&cfg->main_stream, true);
  }

  // Initialize sub stream configuration
  if (stream_config_init_from_anyka(&cfg->sub_stream, false, (unsigned int)sub_kbps, sub_fps) != ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to initialize sub stream config, using defaults\n");
    stream_config_init_defaults(&cfg->sub_stream, false);
  }

  /* Print loaded configuration for debugging and verification */
  const struct application_config* snapshot = config_runtime_snapshot();
  if (snapshot) {
    platform_log_notice("Loaded configuration:\n");
    platform_log_notice("ONVIF: enabled=%d, port=%d, auth_enabled=%d\n", snapshot->onvif.enabled, snapshot->onvif.http_port,
                        snapshot->onvif.auth_enabled);
    platform_log_notice("Imaging: brightness=%d, contrast=%d, saturation=%d\n", snapshot->imaging.brightness, snapshot->imaging.contrast,
                        snapshot->imaging.saturation);
  } else {
    platform_log_warning("warning: failed to get configuration snapshot\n");
  }

  g_config_loaded = true;
  platform_log_info("Configuration loaded successfully\n");
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PUBLIC API - Cleanup and Utility
 * ============================================================================ */

void config_lifecycle_free_memory(struct application_config* cfg) {
  platform_log_info("Cleaning up configuration...\n");

  /* T019: Shutdown runtime configuration system */
  config_runtime_cleanup();

  /* No memory to free since all fields are now direct struct members */
  (void)cfg; /* Suppress unused parameter warning */

  g_config_loaded = false;
  platform_log_info("Configuration cleaned up\n");
}

bool config_lifecycle_loaded(void) {
  return g_config_loaded;
}

int config_lifecycle_get_summary(char* summary, size_t size) {
  if (!g_config_loaded || !summary || size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  const struct application_config* snapshot = config_runtime_snapshot();
  if (!snapshot) {
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  int result = snprintf(summary, size,
                        "ONVIF: enabled=%d, port=%d, auth_enabled=%d\n"
                        "Imaging: brightness=%d, contrast=%d, saturation=%d, sharpness=%d, hue=%d\n"
                        "Auto Day/Night: enabled=%d, mode=%d, thresholds=%d/%d, lock_time=%ds",
                        snapshot->onvif.enabled, snapshot->onvif.http_port, snapshot->onvif.auth_enabled, snapshot->imaging.brightness,
                        snapshot->imaging.contrast, snapshot->imaging.saturation, snapshot->imaging.sharpness, snapshot->imaging.hue,
                        snapshot->auto_daynight.enable_auto_switching, snapshot->auto_daynight.mode, snapshot->auto_daynight.day_to_night_threshold,
                        snapshot->auto_daynight.night_to_day_threshold, snapshot->auto_daynight.lock_time_seconds);

  if (result < 0 || (size_t)result >= size) {
    return ONVIF_ERROR_BUFFER_TOO_SMALL;
  }

  return ONVIF_SUCCESS;
}
