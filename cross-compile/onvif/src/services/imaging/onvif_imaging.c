/**
 * @file onvif_imaging.c
 * @brief ONVIF Imaging service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_imaging.h"

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "core/config/config.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_imaging_types.h"
#include "services/common/onvif_types.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/memory/smart_response_builder.h"
#include "utils/validation/common_validation.h"

static struct auto_daynight_config g_imaging_auto_config;           // NOLINT
static struct imaging_settings g_imaging_settings;                  // NOLINT
static platform_vi_handle_t g_imaging_vi_handle = NULL;             // NOLINT
static pthread_mutex_t g_imaging_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT
static int g_imaging_initialized = 0;                               // NOLINT

// Imaging service configuration and state
static struct {
  service_handler_config_t config;
  onvif_gsoap_context_t* gsoap_ctx;
} g_imaging_handler = {0};            // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
static int g_handler_initialized = 0; // NOLINT

// Buffer pool for imaging service responses
static buffer_pool_t g_imaging_response_buffer_pool; // NOLINT
// Parameter cache optimization for frequently accessed settings
#define IMAGING_PARAM_CACHE_SIZE       8
#define IMAGING_PARAM_NAME_MAX_LENGTH  32
#define IMAGING_PARAM_NAME_SAFE_LENGTH 31

typedef struct {
  char param_name[IMAGING_PARAM_NAME_MAX_LENGTH];
  int cached_value;
  time_t cache_timestamp;
  int is_valid;
} imaging_param_cache_entry_t;

static imaging_param_cache_entry_t g_imaging_param_cache[IMAGING_PARAM_CACHE_SIZE]; // NOLINT
static pthread_mutex_t g_imaging_param_cache_mutex = PTHREAD_MUTEX_INITIALIZER;     // NOLINT
static int g_imaging_param_cache_initialized = 0;                                   // NOLINT

/**
 * @brief Initialize imaging parameter cache
 */
static void init_imaging_param_cache(void) {
  pthread_mutex_lock(&g_imaging_param_cache_mutex);
  if (!g_imaging_param_cache_initialized) {
    memset(g_imaging_param_cache, 0, sizeof(g_imaging_param_cache));
    g_imaging_param_cache_initialized = 1;
  }
  pthread_mutex_unlock(&g_imaging_param_cache_mutex);
}

/**
 * @brief Get cached parameter value
 * @param param_name Parameter name to lookup
 * @param cached_value Output for cached value
 * @return ONVIF_SUCCESS if found in cache, ONVIF_ERROR otherwise
 */
static int get_cached_imaging_param(const char* param_name, int* cached_value) {
  if (!param_name || !cached_value) {
    return ONVIF_ERROR;
  }

  pthread_mutex_lock(&g_imaging_param_cache_mutex);
  for (int i = 0; i < IMAGING_PARAM_CACHE_SIZE; i++) {
    if (g_imaging_param_cache[i].is_valid &&
        strcmp(g_imaging_param_cache[i].param_name, param_name) == 0) {
      *cached_value = g_imaging_param_cache[i].cached_value;
      pthread_mutex_unlock(&g_imaging_param_cache_mutex);
      return ONVIF_SUCCESS;
    }
  }
  pthread_mutex_unlock(&g_imaging_param_cache_mutex);
  return ONVIF_ERROR;
}

/**
 * @brief Cache parameter value
 * @param param_name Parameter name
 * @param value Value to cache
 */
static void cache_imaging_param(const char* param_name, int value) {
  if (!param_name) {
    return;
  }

  pthread_mutex_lock(&g_imaging_param_cache_mutex);

  // Find existing entry or use oldest slot
  int target_slot = -1;
  time_t oldest_time = time(NULL);

  for (int i = 0; i < IMAGING_PARAM_CACHE_SIZE; i++) {
    if (!g_imaging_param_cache[i].is_valid ||
        strcmp(g_imaging_param_cache[i].param_name, param_name) == 0) {
      target_slot = i;
      break;
    }
    if (g_imaging_param_cache[i].cache_timestamp < oldest_time) {
      oldest_time = g_imaging_param_cache[i].cache_timestamp;
      target_slot = i;
    }
  }

  if (target_slot >= 0) {
    strncpy(g_imaging_param_cache[target_slot].param_name, param_name,
            sizeof(g_imaging_param_cache[target_slot].param_name) - 1);
    g_imaging_param_cache[target_slot].param_name[IMAGING_PARAM_NAME_SAFE_LENGTH] = '\0';
    g_imaging_param_cache[target_slot].cached_value = value;
    g_imaging_param_cache[target_slot].cache_timestamp = time(NULL);
    g_imaging_param_cache[target_slot].is_valid = 1;
  }

  pthread_mutex_unlock(&g_imaging_param_cache_mutex);
}

/**
 * @brief Bulk update imaging parameters with validation caching
 * @param settings Imaging settings to apply
 * @return ONVIF_SUCCESS on success, error code on failure
 */
// Function will be moved to after helper functions

/**
 * @brief Optimized batch parameter update
 * @param settings Settings to apply
 * @return ONVIF_SUCCESS on success, error code on failure
 */
// Function will be moved to after helper functions

/* ============================================================================
 * Constants and Definitions
 * ============================================================================ */

/* Imaging Constants */
#define IMAGING_DEFAULT_BRIGHTNESS 0
#define IMAGING_DEFAULT_CONTRAST   0
#define IMAGING_DEFAULT_SATURATION 0
#define IMAGING_DEFAULT_SHARPNESS  0
#define IMAGING_DEFAULT_HUE        0

/* Day/Night Constants */
#define IMAGING_DAY_TO_NIGHT_LUM_DEFAULT 30
#define IMAGING_NIGHT_TO_DAY_LUM_DEFAULT 70
#define IMAGING_LOCK_TIME_DEFAULT        10
#define IMAGING_IRLED_LEVEL_DEFAULT      1

/* VPSS Conversion Constants */
#define IMAGING_VPSS_BRIGHTNESS_DIVISOR 2
#define IMAGING_VPSS_CONTRAST_DIVISOR   2
#define IMAGING_VPSS_SATURATION_DIVISOR 2
#define IMAGING_VPSS_SHARPNESS_DIVISOR  2
#define IMAGING_VPSS_HUE_MULTIPLIER     50
#define IMAGING_VPSS_HUE_DIVISOR        180

/* Token parsing constants */
#define IMAGING_TOKEN_BUFFER_SIZE 32

/* Parameter optimization constants */
#define IMAGING_PARAM_CACHE_SIZE 8

/* HTTP Status Constants */
#define IMAGING_HTTP_STATUS_OK 200

/* Legacy constants for backward compatibility */
#define DEFAULT_DAY_TO_NIGHT_LUM IMAGING_DAY_TO_NIGHT_LUM_DEFAULT
#define DEFAULT_NIGHT_TO_DAY_LUM IMAGING_NIGHT_TO_DAY_LUM_DEFAULT
#define DEFAULT_LOCK_TIME        IMAGING_LOCK_TIME_DEFAULT
#define DEFAULT_IRLED_LEVEL      IMAGING_IRLED_LEVEL_DEFAULT

/* Forward declarations for action handlers */
static int handle_get_imaging_settings(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx);
static int handle_set_imaging_settings(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx);

/* ============================================================================
 * Helper/Utility Functions
 * ============================================================================ */

static int convert_onvif_to_vpss_brightness(int onvif_value) {
  return onvif_value / IMAGING_VPSS_BRIGHTNESS_DIVISOR;
}

static int convert_onvif_to_vpss_contrast(int onvif_value) {
  return onvif_value / IMAGING_VPSS_CONTRAST_DIVISOR;
}

static int convert_onvif_to_vpss_saturation(int onvif_value) {
  return onvif_value / IMAGING_VPSS_SATURATION_DIVISOR;
}

static int convert_onvif_to_vpss_sharpness(int onvif_value) {
  return onvif_value / IMAGING_VPSS_SHARPNESS_DIVISOR;
}

static int convert_onvif_to_vpss_hue(int onvif_value) {
  return (onvif_value * IMAGING_VPSS_HUE_MULTIPLIER) / IMAGING_VPSS_HUE_DIVISOR;
}

static void init_default_imaging_settings(struct imaging_settings* settings) {
  if (!settings) {
    return;
  }

  settings->brightness = IMAGING_DEFAULT_BRIGHTNESS;
  settings->contrast = IMAGING_DEFAULT_CONTRAST;
  settings->saturation = IMAGING_DEFAULT_SATURATION;
  settings->sharpness = IMAGING_DEFAULT_SHARPNESS;
  settings->hue = IMAGING_DEFAULT_HUE;
}

static void init_default_auto_config(struct auto_daynight_config* config) {
  if (!config) {
    return;
  }

  config->day_to_night_threshold = IMAGING_DAY_TO_NIGHT_LUM_DEFAULT;
  config->night_to_day_threshold = IMAGING_NIGHT_TO_DAY_LUM_DEFAULT;
  config->lock_time_seconds = IMAGING_LOCK_TIME_DEFAULT;
  config->ir_led_level = IMAGING_IRLED_LEVEL_DEFAULT;
  config->ir_led_mode = IR_LED_AUTO;
  config->enable_auto_switching = 1;
}

static int apply_imaging_settings_to_vpss(const struct imaging_settings* settings) {
  if (!settings || !g_imaging_vi_handle) {
    return ONVIF_ERROR_INVALID;
  }

  int ret = 0;

  // Set brightness
  int brightness_vpss = convert_onvif_to_vpss_brightness(settings->brightness);
  if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_BRIGHTNESS,
                               brightness_vpss) != 0) {
    platform_log_error("Failed to set brightness\n");
    ret = -1;
  }

  // Set contrast
  int contrast_vpss = convert_onvif_to_vpss_contrast(settings->contrast);
  if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_CONTRAST, contrast_vpss) !=
      0) {
    platform_log_error("Failed to set contrast\n");
    ret = -1;
  }

  // Set saturation
  int saturation_vpss = convert_onvif_to_vpss_saturation(settings->saturation);
  if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_SATURATION,
                               saturation_vpss) != 0) {
    platform_log_error("Failed to set saturation\n");
    ret = -1;
  }

  // Set sharpness
  int sharpness_vpss = convert_onvif_to_vpss_sharpness(settings->sharpness);
  if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_SHARPNESS,
                               sharpness_vpss) != 0) {
    platform_log_error("Failed to set sharpness\n");
    ret = -1;
  }

  // Set hue
  int hue_vpss = convert_onvif_to_vpss_hue(settings->hue);
  if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_HUE, hue_vpss) != 0) {
    platform_log_error("Failed to set hue\n");
    ret = -1;
  }

  return ret;
}

static int validate_imaging_settings_local(const struct imaging_settings* settings) {
  validation_result_t result = validate_imaging_settings(settings);

  if (!validation_is_valid(&result)) {
    service_log_context_t log_ctx;
    service_log_init_context(&log_ctx, "Imaging", "validate_imaging_settings", SERVICE_LOG_ERROR);
    service_log_validation_error(&log_ctx, validation_get_field_name(&result), "Invalid value");
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Bulk update imaging parameters with validation caching
 * @param settings Imaging settings to apply
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int bulk_update_imaging_params(const struct imaging_settings* settings) {
  if (!settings) {
    return ONVIF_ERROR;
  }

  // Validate all parameters before applying any changes
  static struct imaging_settings last_validated_settings = {0};
  static int validation_cache_valid = 0;

  // Check if settings match last validated settings to avoid re-validation
  if (validation_cache_valid &&
      memcmp(settings, &last_validated_settings, sizeof(struct imaging_settings)) == 0) {
    // Skip validation as these exact settings were recently validated
    return ONVIF_SUCCESS;
  }

  // Validate new settings
  int validation_result = validate_imaging_settings_local(settings);
  if (validation_result != ONVIF_SUCCESS) {
    return validation_result;
  }

  // Cache validated settings
  last_validated_settings = *settings;
  validation_cache_valid = 1;

  return ONVIF_SUCCESS;
}

/**
 * @brief Optimized batch parameter update
 * @param settings Settings to apply
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int optimized_batch_param_update(const struct imaging_settings* settings) {
  if (!settings) {
    return ONVIF_ERROR;
  }

  // Check if any parameters actually changed to avoid unnecessary VPSS calls
  int brightness_changed = (g_imaging_settings.brightness != settings->brightness);
  int contrast_changed = (g_imaging_settings.contrast != settings->contrast);
  int saturation_changed = (g_imaging_settings.saturation != settings->saturation);
  int sharpness_changed = (g_imaging_settings.sharpness != settings->sharpness);
  int hue_changed = (g_imaging_settings.hue != settings->hue);

  if (!brightness_changed && !contrast_changed && !saturation_changed && !sharpness_changed &&
      !hue_changed) {
    // No changes needed
    return ONVIF_SUCCESS;
  }

  // Apply only changed parameters to minimize VPSS calls
  int ret = 0;

  if (brightness_changed) {
    int brightness_vpss = convert_onvif_to_vpss_brightness(settings->brightness);
    if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_BRIGHTNESS,
                                 brightness_vpss) != 0) {
      platform_log_error("Failed to set brightness\n");
      ret = -1;
    }
  }

  if (contrast_changed) {
    int contrast_vpss = convert_onvif_to_vpss_contrast(settings->contrast);
    if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_CONTRAST,
                                 contrast_vpss) != 0) {
      platform_log_error("Failed to set contrast\n");
      ret = -1;
    }
  }

  if (saturation_changed) {
    int saturation_vpss = convert_onvif_to_vpss_saturation(settings->saturation);
    if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_SATURATION,
                                 saturation_vpss) != 0) {
      platform_log_error("Failed to set saturation\n");
      ret = -1;
    }
  }

  if (sharpness_changed) {
    int sharpness_vpss = convert_onvif_to_vpss_sharpness(settings->sharpness);
    if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_SHARPNESS,
                                 sharpness_vpss) != 0) {
      platform_log_error("Failed to set sharpness\n");
      ret = -1;
    }
  }

  if (hue_changed) {
    int hue_vpss = convert_onvif_to_vpss_hue(settings->hue);
    if (platform_vpss_effect_set(g_imaging_vi_handle, PLATFORM_VPSS_EFFECT_HUE, hue_vpss) != 0) {
      platform_log_error("Failed to set hue\n");
      ret = -1;
    }
  }

  return ret;
}

/* ============================================================================
 * Service Registration and Dispatch Structures
 * ============================================================================ */

/**
 * @brief Imaging service initialization handler
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int imaging_service_init_handler(void) {
  // gSOAP context is already initialized in onvif_imaging_service_init
  return ONVIF_SUCCESS;
}

/**
 * @brief Imaging service cleanup handler
 */
static void imaging_service_cleanup_handler(void) {
  // gSOAP context cleanup is handled in onvif_imaging_service_cleanup
}

/**
 * @brief Imaging service capabilities handler
 * @param capability_name Capability name to check
 * @return 1 if capability is supported, 0 otherwise
 */
static int imaging_service_capabilities_handler(const char* capability_name) {
  if (!capability_name) {
    return 0;
  }

  // Check against known imaging service capabilities
  if (strcmp(capability_name, "GetImagingSettings") == 0 ||
      strcmp(capability_name, "SetImagingSettings") == 0) {
    return 1;
  }

  return 0;
}

/**
 * @brief Imaging service registration structure using standardized interface
 */
static const onvif_service_registration_t g_imaging_service_registration = {
  .service_name = "imaging",
  .namespace_uri = "http://www.onvif.org/ver20/imaging/wsdl",
  .operation_handler = onvif_imaging_handle_operation,
  .init_handler = imaging_service_init_handler,
  .cleanup_handler = imaging_service_cleanup_handler,
  .capabilities_handler = imaging_service_capabilities_handler,
  .reserved = {NULL, NULL, NULL, NULL}};

/* ============================================================================
 * Operation Dispatch Table
 * ============================================================================ */

/**
 * @brief Imaging operation handler function pointer type
 */
typedef int (*imaging_operation_handler_t)(const service_handler_config_t* config,
                                           const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Imaging operation dispatch entry
 */
typedef struct {
  const char* operation_name;
  imaging_operation_handler_t handler;
} imaging_operation_entry_t;

/**
 * @brief Imaging service operation dispatch table
 */
static const imaging_operation_entry_t g_imaging_operations[] = {
  {"GetImagingSettings", handle_get_imaging_settings},
  {"SetImagingSettings", handle_set_imaging_settings}};

#define IMAGING_OPERATIONS_COUNT (sizeof(g_imaging_operations) / sizeof(g_imaging_operations[0]))

/**
 * @brief Handle ONVIF Imaging service requests by operation name (Standardized Interface)
 * @param operation_name ONVIF operation name (e.g., "GetImagingSettings")
 * @param request HTTP request structure
 * @param response HTTP response structure
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function implements the standardized onvif_service_operation_handler_t interface
 */
int onvif_imaging_handle_operation(const char* operation_name, const http_request_t* request,
                                   http_response_t* response) {
  if (!g_handler_initialized || !operation_name || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Dispatch using lookup table for O(n) performance with small constant factor
  for (size_t i = 0; i < IMAGING_OPERATIONS_COUNT; i++) {
    if (strcmp(operation_name, g_imaging_operations[i].operation_name) == 0) {
      return g_imaging_operations[i].handler(&g_imaging_handler.config, request, response,
                                             g_imaging_handler.gsoap_ctx);
    }
  }

  // Operation not found
  return ONVIF_ERROR_NOT_FOUND;
}

/* ============================================================================
 * Public API Functions
 * ============================================================================ */

int onvif_imaging_init(void* vi_handle) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    return ONVIF_SUCCESS;
  }

  g_imaging_vi_handle = vi_handle;

  // Initialize default imaging settings using helper function
  init_default_imaging_settings(&g_imaging_settings);

  // Note: Imaging settings are now managed through the centralized config
  // system The centralized config will be passed to the service initialization
  platform_log_notice("Imaging service initialized with default settings\n");

  // Initialize auto day/night configuration using helper function
  init_default_auto_config(&g_imaging_auto_config);

  // Load auto configuration
  // Note: Auto configuration is now managed through the centralized config
  // system

  // Initialize IR LED driver via HAL
  if (platform_irled_init(g_imaging_auto_config.ir_led_level) != 0) {
    platform_log_error("Failed to initialize IR LED driver\n");
  } else {
    platform_log_notice("IR LED driver initialized with level %d\n",
                        g_imaging_auto_config.ir_led_level);

    // Set initial IR LED mode
    if (g_imaging_auto_config.ir_led_mode == IR_LED_ON) {
      platform_irled_set_mode(1);
    } else if (g_imaging_auto_config.ir_led_mode == IR_LED_OFF) {
      platform_irled_set_mode(0);
    }
  }

  // Apply current imaging settings to VPSS using helper function
  if (g_imaging_vi_handle) {
    if (apply_imaging_settings_to_vpss(&g_imaging_settings) == 0) {
      platform_log_notice("Applied imaging settings to VPSS\n");
    } else {
      platform_log_error("Failed to apply imaging settings to VPSS\n");
    }
  }

  // Initialize auto day/night mode if enabled
  if (g_imaging_auto_config.enable_auto_switching) {
    platform_log_notice("Auto day/night mode enabled (thresholds: %d/%d)\n",
                        g_imaging_auto_config.day_to_night_threshold,
                        g_imaging_auto_config.night_to_day_threshold);
  }

  g_imaging_initialized = 1;
  pthread_mutex_unlock(&g_imaging_mutex);
  log_service_init_success("Imaging");
  return ONVIF_SUCCESS;
}

void onvif_imaging_cleanup(void) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (g_imaging_initialized) {
    g_imaging_initialized = 0;
    log_service_cleanup("Imaging");
  }
  pthread_mutex_unlock(&g_imaging_mutex);
}

int onvif_imaging_get_settings(struct imaging_settings* settings) {
  if (!settings) {
    log_invalid_parameters("onvif_imaging_get_settings");
    return ONVIF_ERROR;
  }

  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  // Initialize parameter cache if needed
  if (!g_imaging_param_cache_initialized) {
    init_imaging_param_cache();
  }

  // Try to get frequently accessed parameters from cache first
  int cached_brightness = 0;
  int cached_contrast = 0;
  int cached_saturation = 0;
  int brightness_cached =
    (get_cached_imaging_param("brightness", &cached_brightness) == ONVIF_SUCCESS);
  int contrast_cached = (get_cached_imaging_param("contrast", &cached_contrast) == ONVIF_SUCCESS);
  int saturation_cached =
    (get_cached_imaging_param("saturation", &cached_saturation) == ONVIF_SUCCESS);

  // Use cached values if available, otherwise use current settings
  *settings = g_imaging_settings;

  if (brightness_cached) {
    settings->brightness = cached_brightness;
  } else {
    cache_imaging_param("brightness", settings->brightness);
  }

  if (contrast_cached) {
    settings->contrast = cached_contrast;
  } else {
    cache_imaging_param("contrast", settings->contrast);
  }

  if (saturation_cached) {
    settings->saturation = cached_saturation;
  } else {
    cache_imaging_param("saturation", settings->saturation);
  }

  pthread_mutex_unlock(&g_imaging_mutex);
  return ONVIF_SUCCESS;
}

int onvif_imaging_set_settings(const struct imaging_settings* settings) {
  if (!settings) {
    platform_log_error("Invalid parameters\n");
    return ONVIF_ERROR;
  }

  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  int ret = 0;

  // Use bulk validation with caching
  ret = bulk_update_imaging_params(settings);
  if (ret != ONVIF_SUCCESS) {
    pthread_mutex_unlock(&g_imaging_mutex);
    return ret;
  }

  // Use optimized batch parameter update instead of apply_imaging_settings_to_vpss
  if (optimized_batch_param_update(settings) != 0) {
    platform_log_error("Failed to apply imaging settings to VPSS\n");
    ret = -1;
  } else {
    // Update local settings only if VPSS application succeeded
    g_imaging_settings = *settings;

    // Update parameter cache with new values
    cache_imaging_param("brightness", settings->brightness);
    cache_imaging_param("contrast", settings->contrast);
    cache_imaging_param("saturation", settings->saturation);
    cache_imaging_param("sharpness", settings->sharpness);
    cache_imaging_param("hue", settings->hue);
  }

  // Update day/night configuration
  g_imaging_settings.daynight = settings->daynight;

  pthread_mutex_unlock(&g_imaging_mutex);
  return ret;
}

int onvif_imaging_set_day_night_mode(enum day_night_mode mode) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  int ret = 0;
  platform_daynight_mode_t vi_mode = PLATFORM_DAYNIGHT_DAY;

  switch (mode) {
  case DAY_NIGHT_DAY:
    vi_mode = PLATFORM_DAYNIGHT_DAY;
    break;
  case DAY_NIGHT_NIGHT:
    vi_mode = PLATFORM_DAYNIGHT_NIGHT;
    break;
  case DAY_NIGHT_AUTO:
  default:
    // For auto mode, we'll use day mode by default and let the auto switching
    // handle it
    vi_mode = PLATFORM_DAYNIGHT_DAY;
    break;
  }

  if (platform_vi_switch_day_night(g_imaging_vi_handle, vi_mode) != 0) {
    platform_log_error("Failed to switch day/night mode\n");
    ret = -1;
  } else {
    g_imaging_settings.daynight.mode = mode;
    platform_log_notice("Day/night mode set to %d\n", mode);
  }

  pthread_mutex_unlock(&g_imaging_mutex);
  return ret;
}

int onvif_imaging_get_day_night_mode(void) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  enum day_night_mode mode = g_imaging_settings.daynight.mode;
  pthread_mutex_unlock(&g_imaging_mutex);
  return mode;
}

int onvif_imaging_set_irled_mode(enum ir_led_mode mode) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  int ret = 0;
  switch (mode) {
  case IR_LED_OFF:
    platform_irled_set_mode(0);
    break;
  case IR_LED_ON:
    platform_irled_set_mode(1);
    break;
  case IR_LED_AUTO:
  default:
    // For auto mode, we'll enable it by default and let the auto day/night
    // control it
    platform_irled_set_mode(2);
    break;
  }
  g_imaging_settings.daynight.ir_led_mode = mode;
  platform_log_notice("IR LED mode set to %d\n", mode);

  pthread_mutex_unlock(&g_imaging_mutex);
  return ret;
}

int onvif_imaging_get_irled_status(void) {
  int status = platform_irled_get_status();
  if (status < 0) {
    platform_log_error("Failed to get IR LED status\n");
    return ONVIF_SUCCESS; // Return off if we can't determine status
  }
  return status;
}

int onvif_imaging_set_flip_mirror(int flip, int mirror) {
  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  int ret = platform_vi_set_flip_mirror(g_imaging_vi_handle, flip, mirror);
  if (ret != 0) {
    platform_log_error("Failed to set flip/mirror: flip=%d, mirror=%d\n", flip, mirror);
  } else {
    platform_log_notice("Flip/mirror set: flip=%d, mirror=%d\n", flip, mirror);
  }

  pthread_mutex_unlock(&g_imaging_mutex);
  return ret;
}

int onvif_imaging_set_auto_config(const struct auto_daynight_config* config) {
  if (!config) {
    platform_log_error("Invalid parameters\n");
    return ONVIF_ERROR;
  }

  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  g_imaging_auto_config = *config;
  g_imaging_settings.daynight = *config;

  // Save auto configuration section
  // Note: Configuration saving is now handled by the centralized config system
  platform_log_notice("Auto day/night configuration updated\n");

  pthread_mutex_unlock(&g_imaging_mutex);
  return ONVIF_SUCCESS;
}

int onvif_imaging_get_auto_config(struct auto_daynight_config* config) {
  if (!config) {
    platform_log_error("Invalid parameters\n");
    return ONVIF_ERROR;
  }

  pthread_mutex_lock(&g_imaging_mutex);
  if (!g_imaging_initialized) {
    pthread_mutex_unlock(&g_imaging_mutex);
    log_service_not_initialized("Imaging");
    return ONVIF_ERROR;
  }

  *config = g_imaging_auto_config;
  pthread_mutex_unlock(&g_imaging_mutex);
  return ONVIF_SUCCESS;
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

int onvif_imaging_service_init(config_manager_t* config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  // Initialize imaging handler configuration
  g_imaging_handler.config.service_type = ONVIF_SERVICE_IMAGING;
  g_imaging_handler.config.service_name = "Imaging";
  g_imaging_handler.config.config = config;
  g_imaging_handler.config.enable_validation = 1;
  g_imaging_handler.config.enable_logging = 1;

  // Initialize gSOAP context for imaging service
  g_imaging_handler.gsoap_ctx = ONVIF_MALLOC(sizeof(onvif_gsoap_context_t));
  if (!g_imaging_handler.gsoap_ctx) {
    return ONVIF_ERROR;
  }

  if (onvif_gsoap_init(g_imaging_handler.gsoap_ctx) != ONVIF_SUCCESS) {
    ONVIF_FREE(g_imaging_handler.gsoap_ctx);
    g_imaging_handler.gsoap_ctx = NULL;
    return ONVIF_ERROR;
  }

  if (buffer_pool_init(&g_imaging_response_buffer_pool) != 0) {
    onvif_gsoap_cleanup(g_imaging_handler.gsoap_ctx);
    ONVIF_FREE(g_imaging_handler.gsoap_ctx);
    g_imaging_handler.gsoap_ctx = NULL;
    return ONVIF_ERROR;
  }

  g_handler_initialized = 1;

  // Register with standardized service dispatcher
  int result = onvif_service_dispatcher_register_service(&g_imaging_service_registration);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("Failed to register imaging service with dispatcher: %d\n", result);
    onvif_imaging_service_cleanup();
    return result;
  }

  platform_log_info("Imaging service initialized and registered with dispatcher\n");

  return ONVIF_SUCCESS;
}

void onvif_imaging_service_cleanup(void) {
  if (!g_handler_initialized) {
    return;
  }

  // Unregister from standardized service dispatcher
  int unregister_result = onvif_service_dispatcher_unregister_service("imaging");
  if (unregister_result != ONVIF_SUCCESS) {
    platform_log_error("Failed to unregister imaging service from dispatcher: %d\n",
                       unregister_result);
    // Don't fail cleanup for this, but log the error
  }

  // Cleanup gSOAP context
  if (g_imaging_handler.gsoap_ctx) {
    onvif_gsoap_cleanup(g_imaging_handler.gsoap_ctx);
    ONVIF_FREE(g_imaging_handler.gsoap_ctx);
    g_imaging_handler.gsoap_ctx = NULL;
  }

  buffer_pool_cleanup(&g_imaging_response_buffer_pool);

  g_handler_initialized = 0;

  // Check for memory leaks
  memory_manager_check_leaks();
}

/* ============================================================================
 * Action Handlers
 * ============================================================================ */

// Action handlers
static int handle_get_imaging_settings(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Imaging", "GetImagingSettings", "settings_retrieval");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Imaging", "GetImagingSettings", SERVICE_LOG_INFO);

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "gsoap_ctx", "missing", response);
  }

  // Initialize gSOAP request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "init_request_parsing", response);
  }

  // Parse video source token from request using gSOAP
  char video_source_token[IMAGING_TOKEN_BUFFER_SIZE] = "VideoSource0";
  result = onvif_gsoap_parse_value(gsoap_ctx, "//tt:VideoSourceToken", video_source_token,
                                   sizeof(video_source_token));
  if (result != 0) {
    platform_log_warning("Failed to parse video source token, using default\n");
  }

  // Get current imaging settings
  struct imaging_settings settings;
  result = onvif_imaging_get_settings(&settings);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "get_imaging_settings", response);
  }

  // Generate SOAP response with gSOAP and smart response builder
  imaging_settings_callback_data_t callback_data = {.settings = &settings};
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, imaging_settings_response_callback, &callback_data);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "generate_imaging_settings_response", response);
  }

  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "get_response_data", response);
  }

  size_t estimated_size = smart_response_estimate_size(soap_response);
  if (smart_response_build(response, soap_response, estimated_size,
                           &g_imaging_response_buffer_pool) != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "smart_response_build", response);
  }

  // Set response headers
  response->status_code = IMAGING_HTTP_STATUS_OK;
  response->content_type = "application/soap+xml; charset=utf-8";

  service_log_info(&log_ctx, "Successfully retrieved imaging settings for token: %s\n",
                   video_source_token);
  return ONVIF_SUCCESS;
}

static int handle_set_imaging_settings(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Imaging", "SetImagingSettings", "settings_update");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Imaging", "SetImagingSettings", SERVICE_LOG_INFO);

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "gsoap_ctx", "missing", response);
  }

  // Initialize gSOAP request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "init_request_parsing", response);
  }

  // Parse imaging settings from request using gSOAP
  struct imaging_settings settings;
  result = onvif_gsoap_parse_imaging_settings(gsoap_ctx, &settings);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "parse_imaging_settings", response);
  }

  // Validate imaging settings
  result = validate_imaging_settings_local(&settings);
  if (result != ONVIF_SUCCESS) {
    return error_handle_parameter(&error_ctx, "settings", "invalid", response);
  }

  // Apply imaging settings
  result = onvif_imaging_set_settings(&settings);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "set_imaging_settings", response);
  }

  // Generate SOAP response with gSOAP and smart response builder
  set_imaging_settings_callback_data_t callback_data = {.message = NULL};
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, set_imaging_settings_response_callback, &callback_data);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "generate_set_imaging_response", response);
  }

  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "get_response_data", response);
  }

  size_t estimated_size = smart_response_estimate_size(soap_response);
  if (smart_response_build(response, soap_response, estimated_size,
                           &g_imaging_response_buffer_pool) != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "smart_response_build", response);
  }

  // Set response headers
  response->status_code = IMAGING_HTTP_STATUS_OK;
  response->content_type = "application/soap+xml; charset=utf-8";

  service_log_info(&log_ctx, "Successfully updated imaging settings\n");
  return ONVIF_SUCCESS;
}
