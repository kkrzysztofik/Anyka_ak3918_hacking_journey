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

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "platform/platform.h"
#include "services/common/onvif_imaging_types.h"
#include "services/common/video_config_types.h"
#include "utils/error/error_handling.h"
#include "utils/stream/stream_config_utils.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Global configuration state */
static volatile bool g_config_loaded = false;           // NOLINT
static config_manager_t g_config_global_config_manager; // NOLINT

/* ---------------------------- Public Interface ------------------------- */

int config_lifecycle_allocate_memory(struct application_config* cfg) {
  platform_log_info("Allocating configuration memory...\n");

  /* Allocate memory for imaging and auto_daynight structures */
  cfg->imaging = malloc(sizeof(struct imaging_settings));
  if (!cfg->imaging) {
    platform_log_error("Failed to allocate memory for imaging settings\n");
    return -1;
  }
  memset(cfg->imaging, 0, sizeof(struct imaging_settings));

  cfg->auto_daynight = malloc(sizeof(struct auto_daynight_config));
  if (!cfg->auto_daynight) {
    platform_log_error("Failed to allocate memory for auto day/night config\n");
    free(cfg->imaging);
    return -1;
  }
  memset(cfg->auto_daynight, 0, sizeof(struct auto_daynight_config));

  /* Allocate memory for network and device structures */
  cfg->network = malloc(sizeof(struct network_settings));
  if (!cfg->network) {
    platform_log_error("Failed to allocate memory for network settings\n");
    free(cfg->imaging);
    free(cfg->auto_daynight);
    return -1;
  }
  memset(cfg->network, 0, sizeof(struct network_settings));

  cfg->device = malloc(sizeof(struct device_info));
  if (!cfg->device) {
    platform_log_error("Failed to allocate memory for device info\n");
    free(cfg->imaging);
    free(cfg->auto_daynight);
    free(cfg->network);
    return -1;
  }
  memset(cfg->device, 0, sizeof(struct device_info));

  cfg->logging = malloc(sizeof(struct logging_settings));
  if (!cfg->logging) {
    platform_log_error("Failed to allocate memory for logging settings\n");
    free(cfg->imaging);
    free(cfg->auto_daynight);
    free(cfg->network);
    free(cfg->device);
    return -1;
  }
  memset(cfg->logging, 0, sizeof(struct logging_settings));

  /* Allocate memory for stream configurations */
  cfg->main_stream = malloc(sizeof(video_config_t));
  if (!cfg->main_stream) {
    platform_log_error("Failed to allocate memory for main stream config\n");
    free(cfg->imaging);
    free(cfg->auto_daynight);
    free(cfg->network);
    free(cfg->device);
    free(cfg->logging);
    return -1;
  }
  memset(cfg->main_stream, 0, sizeof(video_config_t));

  cfg->sub_stream = malloc(sizeof(video_config_t));
  if (!cfg->sub_stream) {
    platform_log_error("Failed to allocate memory for sub stream config\n");
    free(cfg->imaging);
    free(cfg->auto_daynight);
    free(cfg->network);
    free(cfg->device);
    free(cfg->logging);
    free(cfg->main_stream);
    return -1;
  }
  memset(cfg->sub_stream, 0, sizeof(video_config_t));

  platform_log_info("Configuration memory allocated successfully\n");
  return 0;
}

int config_lifecycle_load_configuration(struct application_config* cfg) {
  platform_log_info("Loading configuration...\n");

  /* Initialize config_manager structure to prevent garbage data */
  memset(&g_config_global_config_manager, 0, sizeof(config_manager_t));

  if (config_init(&g_config_global_config_manager, cfg) != ONVIF_SUCCESS) {
    platform_log_error("error: failed to initialize config\n");
    return -1;
  }

  if (config_load(&g_config_global_config_manager, ONVIF_CONFIG_FILE) != ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to read config at %s\n", ONVIF_CONFIG_FILE);
    platform_log_warning("warning: using default configuration (embedded)\n");
  }

  // Initialize stream configurations from anyka_cfg.ini parameters
  int main_fps = 15; // Default to 15fps for sensor compatibility
  int main_kbps = 2048;
  int sub_fps = 15; // Default to 15fps for sensor compatibility
  int sub_kbps = 800;

  // Try to get values from config file, but use sensor frame rate if available
  int config_main_fps = 25;
  int config_sub_fps = 25;
  config_get_value(&g_config_global_config_manager, CONFIG_SECTION_MAIN_STREAM, "main_fps",
                   &config_main_fps, CONFIG_TYPE_INT);
  config_get_value(&g_config_global_config_manager, CONFIG_SECTION_MAIN_STREAM, "main_kbps",
                   &main_kbps, CONFIG_TYPE_INT);
  config_get_value(&g_config_global_config_manager, CONFIG_SECTION_SUB_STREAM, "sub_fps",
                   &config_sub_fps, CONFIG_TYPE_INT);
  config_get_value(&g_config_global_config_manager, CONFIG_SECTION_SUB_STREAM, "sub_kbps",
                   &sub_kbps, CONFIG_TYPE_INT);

  // Use config values, but ensure they're compatible with sensor capabilities
  main_fps = config_main_fps;
  sub_fps = config_sub_fps;

  // Initialize main stream configuration
  if (stream_config_init_from_anyka(cfg->main_stream, true, (unsigned int)main_kbps, main_fps) !=
      ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to initialize main stream config, using defaults\n");
    stream_config_init_defaults(cfg->main_stream, true);
  }

  // Initialize sub stream configuration
  if (stream_config_init_from_anyka(cfg->sub_stream, false, (unsigned int)sub_kbps, sub_fps) !=
      ONVIF_SUCCESS) {
    platform_log_warning("warning: failed to initialize sub stream config, using defaults\n");
    stream_config_init_defaults(cfg->sub_stream, false);
  }

  /* Print loaded configuration for debugging and verification */
  char config_summary[1024];
  if (config_get_summary(&g_config_global_config_manager, config_summary, sizeof(config_summary)) ==
      ONVIF_SUCCESS) {
    platform_log_notice("Loaded configuration:\n%s\n", config_summary);
  } else {
    platform_log_warning("warning: failed to generate configuration summary\n");
  }

  g_config_loaded = true;
  platform_log_info("Configuration loaded successfully\n");
  return 0;
}

void config_lifecycle_free_memory(struct application_config* cfg) {
  platform_log_info("Freeing configuration memory...\n");

  if (cfg->imaging) {
    free(cfg->imaging);
    cfg->imaging = NULL;
  }
  if (cfg->auto_daynight) {
    free(cfg->auto_daynight);
    cfg->auto_daynight = NULL;
  }
  if (cfg->network) {
    free(cfg->network);
    cfg->network = NULL;
  }
  if (cfg->device) {
    free(cfg->device);
    cfg->device = NULL;
  }
  if (cfg->logging) {
    free(cfg->logging);
    cfg->logging = NULL;
  }
  if (cfg->main_stream) {
    free(cfg->main_stream);
    cfg->main_stream = NULL;
  }
  if (cfg->sub_stream) {
    free(cfg->sub_stream);
    cfg->sub_stream = NULL;
  }

  g_config_loaded = false;
  platform_log_info("Configuration memory freed\n");
}

bool config_lifecycle_loaded(void) {
  return g_config_loaded;
}

int config_lifecycle_get_summary(char* summary, size_t size) {
  if (!g_config_loaded) {
    return -1;
  }
  return config_get_summary(&g_config_global_config_manager, summary, size);
}
