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

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_imaging_types.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/validation/common_validation.h"

static struct auto_daynight_config g_imaging_auto_config;           // NOLINT
static struct imaging_settings g_imaging_settings;                  // NOLINT
static platform_vi_handle_t g_imaging_vi_handle = NULL;             // NOLINT
static pthread_mutex_t g_imaging_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT
static int g_imaging_initialized = 0;                               // NOLINT

/* Forward declarations for helper functions */
static int convert_onvif_to_vpss_brightness(int onvif_value);
static int convert_onvif_to_vpss_contrast(int onvif_value);
static int convert_onvif_to_vpss_saturation(int onvif_value);
static int convert_onvif_to_vpss_sharpness(int onvif_value);
static int convert_onvif_to_vpss_hue(int onvif_value);
static int convert_vpss_to_onvif_brightness(int vpss_value);
static int convert_vpss_to_onvif_contrast(int vpss_value);
static int convert_vpss_to_onvif_saturation(int vpss_value);
static int convert_vpss_to_onvif_sharpness(int vpss_value);
static int convert_vpss_to_onvif_hue(int vpss_value);
static void init_default_imaging_settings(struct imaging_settings* settings);
static void init_default_auto_config(struct auto_daynight_config* config);
static int apply_imaging_settings_to_vpss(const struct imaging_settings* settings);
static int validate_imaging_settings_local(const struct imaging_settings* settings);

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

/* Buffer Size Constants */
#define IMAGING_RESPONSE_BUFFER_SIZE 4096
#define IMAGING_SETTINGS_BUFFER_SIZE 1024
#define IMAGING_OPTIONS_BUFFER_SIZE  2048
#define IMAGING_TOKEN_BUFFER_SIZE    32
#define IMAGING_VALUE_BUFFER_SIZE    16

/* XML Tag Constants */
#define IMAGING_XML_BRIGHTNESS_TAG  "<tt:Brightness>"
#define IMAGING_XML_CONTRAST_TAG    "<tt:Contrast>"
#define IMAGING_XML_SATURATION_TAG  "<tt:ColorSaturation>"
#define IMAGING_XML_SHARPNESS_TAG   "<tt:Sharpness>"
#define IMAGING_XML_HUE_TAG         "<tt:Hue>"
#define IMAGING_XML_VIDEO_TOKEN_TAG "<VideoSourceToken>"

/* Legacy constants for backward compatibility */
#define DEFAULT_DAY_TO_NIGHT_LUM IMAGING_DAY_TO_NIGHT_LUM_DEFAULT
#define DEFAULT_NIGHT_TO_DAY_LUM IMAGING_NIGHT_TO_DAY_LUM_DEFAULT
#define DEFAULT_LOCK_TIME        IMAGING_LOCK_TIME_DEFAULT
#define DEFAULT_IRLED_LEVEL      IMAGING_IRLED_LEVEL_DEFAULT

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

  *settings = g_imaging_settings;
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

  // Apply settings to VPSS using helper function
  if (apply_imaging_settings_to_vpss(settings) != 0) {
    platform_log_error("Failed to apply imaging settings to VPSS\n");
    ret = -1;
  } else {
    // Update local settings only if VPSS application succeeded
    g_imaging_settings = *settings;
  }

  // Update day/night configuration
  g_imaging_settings.daynight = settings->daynight;

  // Save settings to configuration (imaging section)
  if (ret == 0) {
    // Note: Configuration saving is now handled by the centralized config
    // system
    platform_log_notice("Imaging settings updated successfully\n");
  }

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

/* Helper Functions */

static void init_default_imaging_settings(struct imaging_settings* settings) {
  if (!settings) {
    return;
  }

  memset(settings, 0, sizeof(struct imaging_settings));
  settings->brightness = IMAGING_DEFAULT_BRIGHTNESS;
  settings->contrast = IMAGING_DEFAULT_CONTRAST;
  settings->saturation = IMAGING_DEFAULT_SATURATION;
  settings->sharpness = IMAGING_DEFAULT_SHARPNESS;
  settings->hue = IMAGING_DEFAULT_HUE;
  settings->daynight.mode = DAY_NIGHT_AUTO;
  settings->daynight.day_to_night_threshold = IMAGING_DAY_TO_NIGHT_LUM_DEFAULT;
  settings->daynight.night_to_day_threshold = IMAGING_NIGHT_TO_DAY_LUM_DEFAULT;
  settings->daynight.lock_time_seconds = IMAGING_LOCK_TIME_DEFAULT;
  settings->daynight.ir_led_mode = IR_LED_AUTO;
  settings->daynight.ir_led_level = IMAGING_IRLED_LEVEL_DEFAULT;
  settings->daynight.enable_auto_switching = 1;
}

static void init_default_auto_config(struct auto_daynight_config* config) {
  if (!config) {
    return;
  }

  memset(config, 0, sizeof(struct auto_daynight_config));
  config->mode = DAY_NIGHT_AUTO;
  config->day_to_night_threshold = IMAGING_DAY_TO_NIGHT_LUM_DEFAULT;
  config->night_to_day_threshold = IMAGING_NIGHT_TO_DAY_LUM_DEFAULT;
  config->lock_time_seconds = IMAGING_LOCK_TIME_DEFAULT;
  config->ir_led_mode = IR_LED_AUTO;
  config->ir_led_level = IMAGING_IRLED_LEVEL_DEFAULT;
  config->enable_auto_switching = 1;
}

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

static int convert_vpss_to_onvif_brightness(int vpss_value) {
  return vpss_value * IMAGING_VPSS_BRIGHTNESS_DIVISOR;
}

static int convert_vpss_to_onvif_contrast(int vpss_value) {
  return vpss_value * IMAGING_VPSS_CONTRAST_DIVISOR;
}

static int convert_vpss_to_onvif_saturation(int vpss_value) {
  return vpss_value * IMAGING_VPSS_SATURATION_DIVISOR;
}

static int convert_vpss_to_onvif_sharpness(int vpss_value) {
  return vpss_value * IMAGING_VPSS_SHARPNESS_DIVISOR;
}

static int convert_vpss_to_onvif_hue(int vpss_value) {
  return (vpss_value * IMAGING_VPSS_HUE_DIVISOR) / IMAGING_VPSS_HUE_MULTIPLIER;
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

/* Legacy service handler removed - using modern service handler implementation
 * below */

/* Refactored imaging service implementation */

// Service handler instance
static onvif_service_handler_instance_t g_imaging_handler; // NOLINT
static int g_handler_initialized = 0;                      // NOLINT

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

  // Generate imaging settings response using callback infrastructure
  // Prepare callback data
  imaging_settings_callback_data_t callback_data = {.settings = &settings};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, imaging_settings_response_callback, &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, -ENOENT, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, -ENOMEM, "allocate_response", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  service_log_debug(&log_ctx, "Response XML: %s", soap_response);
  service_log_debug(&log_ctx, "Response structure: body=%p, body_length=%zu, status_code=%d",
                    response->body, response->body_length, response->status_code);
  service_log_operation_success(&log_ctx, "Imaging settings retrieved");

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

  // Parse video source token from request using gSOAP
  char video_source_token[IMAGING_TOKEN_BUFFER_SIZE] = "VideoSource0";
  result = onvif_gsoap_parse_value(gsoap_ctx, "//tt:VideoSourceToken", video_source_token,
                                   sizeof(video_source_token));
  if (result != 0) {
    platform_log_warning("Failed to parse video source token, using default\n");
  }

  // Parse imaging settings from request using gSOAP
  struct imaging_settings settings;
  memset(&settings, 0, sizeof(settings));

  // Parse brightness
  char brightness_str[IMAGING_VALUE_BUFFER_SIZE];
  if (onvif_gsoap_parse_value(gsoap_ctx, "//tt:Brightness", brightness_str,
                              sizeof(brightness_str)) == 0) {
    settings.brightness = atoi(brightness_str);
  }

  // Parse contrast
  char contrast_str[IMAGING_VALUE_BUFFER_SIZE];
  if (onvif_gsoap_parse_value(gsoap_ctx, "//tt:Contrast", contrast_str, sizeof(contrast_str)) ==
      0) {
    settings.contrast = atoi(contrast_str);
  }

  // Parse saturation
  char saturation_str[IMAGING_VALUE_BUFFER_SIZE];
  if (onvif_gsoap_parse_value(gsoap_ctx, "//tt:ColorSaturation", saturation_str,
                              sizeof(saturation_str)) == 0) {
    settings.saturation = atoi(saturation_str);
  }

  // Parse sharpness
  char sharpness_str[IMAGING_VALUE_BUFFER_SIZE];
  if (onvif_gsoap_parse_value(gsoap_ctx, "//tt:Sharpness", sharpness_str, sizeof(sharpness_str)) ==
      0) {
    settings.sharpness = atoi(sharpness_str);
  }

  // Parse hue
  char hue_str[IMAGING_VALUE_BUFFER_SIZE];
  if (onvif_gsoap_parse_value(gsoap_ctx, "//tt:Hue", hue_str, sizeof(hue_str)) == 0) {
    settings.hue = atoi(hue_str);
  }

  // Validate settings before applying
  if (validate_imaging_settings_local(&settings) != 0) {
    return error_handle_validation(&error_ctx, -1, "imaging_settings", response);
  }

  // Apply settings
  result = onvif_imaging_set_settings(&settings);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "set_imaging_settings", response);
  }

  // Generate set imaging settings response using callback infrastructure
  // Prepare callback data
  set_imaging_settings_callback_data_t callback_data = {.message =
                                                          "Imaging settings updated successfully"};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, set_imaging_settings_response_callback, &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, -ENOENT, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, -ENOMEM, "allocate_response", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  service_log_debug(&log_ctx, "Response XML: %s", soap_response);
  service_log_debug(&log_ctx, "Response structure: body=%p, body_length=%zu, status_code=%d",
                    response->body, response->body_length, response->status_code);
  service_log_operation_success(&log_ctx, "Imaging settings updated");

  return ONVIF_SUCCESS;
}

// Action definitions
static const service_action_def_t imaging_actions[] = {
  {"GetImagingSettings", handle_get_imaging_settings, 1},
  {"SetImagingSettings", handle_set_imaging_settings, 1}};

int onvif_imaging_service_init(config_manager_t* config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {.service_type = ONVIF_SERVICE_IMAGING,
                                             .service_name = "Imaging",
                                             .config = config,
                                             .enable_validation = 1,
                                             .enable_logging = 1};

  int result = onvif_service_handler_init(&g_imaging_handler, &handler_config, imaging_actions,
                                          sizeof(imaging_actions) / sizeof(imaging_actions[0]));

  if (result == ONVIF_SUCCESS) {
    // Register imaging-specific error handlers
    // Error handler registration not implemented yet
    // Error handler registration not implemented yet

    g_handler_initialized = 1;
  }

  return result;
}

void onvif_imaging_service_cleanup(void) {
  if (g_handler_initialized) {
    error_context_t error_ctx;
    error_context_init(&error_ctx, "Imaging", "Cleanup", "service_cleanup");

    onvif_service_handler_cleanup(&g_imaging_handler);

    // Unregister error handlers
    // Error handler unregistration not implemented yet

    g_handler_initialized = 0;

    // Check for memory leaks
    memory_manager_check_leaks();
  }
}

int onvif_imaging_handle_request(const char* action_name, const http_request_t* request,
                                 http_response_t* response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }
  return onvif_service_handler_handle_request(&g_imaging_handler, action_name, request, response);
}
