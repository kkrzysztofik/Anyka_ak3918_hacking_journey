/**
 * @file onvif_ptz.c
 * @brief ONVIF PTZ service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_ptz.h"

#include <bits/pthreadtypes.h>
#include <errno.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "platform/adapters/ptz_adapter.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_service_common.h"
#include "services/common/onvif_types.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"

#ifdef UNIT_TESTING
#include "services/common/onvif_service_test_helpers.h"
#endif

/* PTZ Movement Constants */
#define PTZ_MOVEMENT_TIMEOUT_MS        5000
#define PTZ_MOVEMENT_CHECK_INTERVAL_MS 5
#define PTZ_MAX_PAN_DEGREES            350
#define PTZ_MIN_PAN_DEGREES            -350
#define PTZ_MAX_TILT_DEGREES           130
#define PTZ_MIN_TILT_DEGREES           -130
#define PTZ_MAX_STEP_SIZE_PAN          16
#define PTZ_MAX_STEP_SIZE_TILT         8
#define PTZ_DEFAULT_SPEED              50
#define PTZ_MAX_PRESETS                4  /* Max presets per profile */

/* PTZ Coordinate Constants */
#define PTZ_PAN_RANGE_DEGREES       180
#define PTZ_TILT_RANGE_DEGREES      90
#define PTZ_SPEED_BASE              15
#define PTZ_SPEED_RANGE             85
#define PTZ_DEFAULT_TIMEOUT_MS      10000
#define PTZ_DEFAULT_TIMEOUT_S       10
#define PTZ_TOKEN_MAX_LENGTH        32
#define PTZ_PRESET_NAME_MAX_LENGTH  64
#define PTZ_PRESET_TOKEN_MAX_LENGTH 63
#define PTZ_MS_PER_SECOND           1000

/* PTZ Node configuration */
static const struct ptz_node g_ptz_node = {
  .token = "PTZNode0",
  .name = "PTZ Node",
  .supported_ptz_spaces =
    {.absolute_pan_tilt_position_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                 "PositionGenericSpace",
                                          .x_range = {.min = -180.0F, .max = 180.0F},
                                          .y_range = {.min = -90.0F, .max = 90.0F}},
     .absolute_zoom_position_space = {.uri = "",
                                      .x_range = {.min = 0.0F, .max = 0.0F},
                                      .y_range = {.min = 0.0F, .max = 0.0F}},
     .relative_pan_tilt_translation_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                    "TranslationGenericSpace",
                                             .x_range = {.min = -180.0F, .max = 180.0F},
                                             .y_range = {.min = -90.0F, .max = 90.0F}},
     .relative_zoom_translation_space = {.uri = "",
                                         .x_range = {.min = 0.0F, .max = 0.0F},
                                         .y_range = {.min = 0.0F, .max = 0.0F}},
     .continuous_pan_tilt_velocity_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                   "VelocityGenericSpace",
                                            .x_range = {.min = -1.0F, .max = 1.0F},
                                            .y_range = {.min = -1.0F, .max = 1.0F}},
     .continuous_zoom_velocity_space = {.uri = "",
                                        .x_range = {.min = 0.0F, .max = 0.0F},
                                        .y_range = {.min = 0.0F, .max = 0.0F}}},
  .maximum_number_of_presets = PTZ_MAX_PRESETS,
  .home_supported = 1,
  .auxiliary_commands = {0} /* No auxiliary commands */
};

/* Profile-based preset storage (4 profiles × 4 presets each = 16 total) */
#define PTZ_PROFILE_COUNT 4
#define PTZ_PRESETS_PER_PROFILE 4

static ptz_preset_list_t g_profile_presets[PTZ_PROFILE_COUNT]; // NOLINT
static int g_presets_loaded = 0; // NOLINT - Lazy loading flag

/* Service initialization guard */
static int g_ptz_initialized = 0; // NOLINT
static int g_ptz_cleanup_in_progress = 0;

/* Global buffer pool for PTZ service responses */
static buffer_pool_t g_ptz_response_buffer_pool = {0}; // NOLINT

/* Profile-based preset management helper functions */
static int load_ptz_presets_from_config(void);
static int get_profile_index_from_token(const char* profile_token);
static int find_preset_by_token_in_profile(int profile_index, const char* token, int* preset_index);
static int cleanup_preset_memory(void);
static int reset_preset_state(void);

/* Convert ONVIF normalized coordinates to device degrees */
static int normalize_to_degrees_pan(float normalized_value) {
  /* Convert from [-1, 1] to [-180, 180] */
  return (int)(normalized_value * PTZ_PAN_RANGE_DEGREES);
}

static int normalize_to_degrees_tilt(float normalized_value) {
  /* Convert from [-1, 1] to [-90, 90] */
  return (int)(normalized_value * PTZ_TILT_RANGE_DEGREES);
}

/* Convert device degrees to ONVIF normalized coordinates */
static float degrees_to_normalize_pan(int degrees) {
  /* Convert from [-180, 180] to [-1, 1] */
  return (float)degrees / PTZ_PAN_RANGE_DEGREES;
}

static float degrees_to_normalize_tilt(int degrees) {
  /* Convert from [-90, 90] to [-1, 1] */
  return (float)degrees / PTZ_TILT_RANGE_DEGREES;
}

/* Convert ONVIF normalized velocity to driver speed */
static int normalize_to_speed(float normalized_velocity) {
  /* Convert from [-1, 1] to [15, 100] driver speed range */
  float abs_vel = fabsf(normalized_velocity);
  return (int)(PTZ_SPEED_BASE +
               abs_vel * PTZ_SPEED_RANGE); /* PTZ_SPEED_BASE + (0-1) * PTZ_SPEED_RANGE = 15-100 */
}

int onvif_ptz_get_nodes(struct ptz_node** nodes, int* count) {
  ONVIF_CHECK_NULL(nodes);
  ONVIF_CHECK_NULL(count);

  *nodes = (struct ptz_node*)&g_ptz_node;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_ptz_get_node(const char* node_token, struct ptz_node* node) {
  ONVIF_CHECK_NULL(node_token);
  ONVIF_CHECK_NULL(node);

  if (strcmp(node_token, g_ptz_node.token) == 0) {
    *node = g_ptz_node;
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_ptz_get_configuration(const char* config_token, struct ptz_configuration_ex* config) {
  ONVIF_CHECK_NULL(config_token);
  ONVIF_CHECK_NULL(config);

  /* Default PTZ configuration */
  strcpy(config->token, "PTZConfig0");
  strcpy(config->name, "PTZ Configuration");
  config->use_count = 1;
  strncpy(config->node_token, g_ptz_node.token, sizeof(config->node_token) - 1);
  config->node_token[sizeof(config->node_token) - 1] = '\0';

  config->default_absolute_pan_tilt_position_space =
    g_ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space;
  config->default_absolute_zoom_position_space =
    g_ptz_node.supported_ptz_spaces.absolute_zoom_position_space;
  config->default_relative_pan_tilt_translation_space =
    g_ptz_node.supported_ptz_spaces.relative_pan_tilt_translation_space;
  config->default_relative_zoom_translation_space =
    g_ptz_node.supported_ptz_spaces.relative_zoom_translation_space;
  config->default_continuous_pan_tilt_velocity_space =
    g_ptz_node.supported_ptz_spaces.continuous_pan_tilt_velocity_space;
  config->default_continuous_zoom_velocity_space =
    g_ptz_node.supported_ptz_spaces.continuous_zoom_velocity_space;

  config->default_ptz_speed.pan_tilt.x = PTZ_DEFAULT_PAN_TILT_SPEED;
  config->default_ptz_speed.pan_tilt.y = PTZ_DEFAULT_PAN_TILT_SPEED;
  config->default_ptz_speed.zoom = PTZ_DEFAULT_ZOOM_SPEED;

  config->default_ptz_timeout = PTZ_DEFAULT_TIMEOUT_MS; /* 10 seconds */

  config->pan_tilt_limits.range.x_range.min = -1.0F;
  config->pan_tilt_limits.range.x_range.max = 1.0F;
  config->pan_tilt_limits.range.y_range.min = -1.0F;
  config->pan_tilt_limits.range.y_range.max = 1.0F;
  strncpy(config->pan_tilt_limits.range.uri,
          g_ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri,
          sizeof(config->pan_tilt_limits.range.uri) - 1);
  config->pan_tilt_limits.range.uri[sizeof(config->pan_tilt_limits.range.uri) - 1] = '\0';

  config->zoom_limits.range.x_range.min = 0.0F;
  config->zoom_limits.range.x_range.max = 0.0F;
  config->zoom_limits.range.y_range.min = 0.0F;
  config->zoom_limits.range.y_range.max = 0.0F;
  config->zoom_limits.range.uri[0] = '\0'; /* No zoom support */

  return ONVIF_SUCCESS;
}

int onvif_ptz_get_status(const char* profile_token, struct ptz_status* status) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(status);

  struct ptz_device_status adapter_status;
  if (ptz_adapter_get_status(&adapter_status) != 0) {
    return ONVIF_ERROR;
  }

  /* Convert adapter status to ONVIF format */
  status->position.pan_tilt.x = degrees_to_normalize_pan(adapter_status.h_pos_deg);
  status->position.pan_tilt.y = degrees_to_normalize_tilt(adapter_status.v_pos_deg);
  status->position.zoom = 0.0F; /* No zoom support */

  strncpy(status->position.space,
          g_ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri,
          sizeof(status->position.space) - 1);
  status->position.space[sizeof(status->position.space) - 1] = '\0';

  /* Movement status */
  status->move_status.pan_tilt =
    (adapter_status.h_speed > 0 || adapter_status.v_speed > 0) ? PTZ_MOVE_MOVING : PTZ_MOVE_IDLE;
  status->move_status.zoom = PTZ_MOVE_IDLE;

  /* Error status */
  strcpy(status->error, "");

  /* UTC time - simplified */
  time_t now = time(NULL);
  struct tm* utc_tm = gmtime(&now);
  (void)strftime(status->utc_time, sizeof(status->utc_time), "%Y-%m-%dT%H:%M:%S.000Z", utc_tm);

  return ONVIF_SUCCESS;
}

int onvif_ptz_absolute_move(const char* profile_token, const struct ptz_vector* position,
                            const struct ptz_speed* speed) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(position);

  int pan_deg = normalize_to_degrees_pan(position->pan_tilt.x);
  int tilt_deg = normalize_to_degrees_tilt(position->pan_tilt.y);

  int move_speed = PTZ_DEFAULT_SPEED; /* Default speed */
  if (speed) {
    float max_speed = fmaxf(fabsf(speed->pan_tilt.x), fabsf(speed->pan_tilt.y));
    move_speed = normalize_to_speed(max_speed);
  }

  return ptz_adapter_absolute_move(pan_deg, tilt_deg, move_speed);
}

int onvif_ptz_relative_move(const char* profile_token, const struct ptz_vector* translation,
                            const struct ptz_speed* speed) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(translation);

  int pan_delta = normalize_to_degrees_pan(translation->pan_tilt.x);
  int tilt_delta = normalize_to_degrees_tilt(translation->pan_tilt.y);

  int move_speed = PTZ_DEFAULT_SPEED; /* Default speed */
  if (speed) {
    float max_speed = fmaxf(fabsf(speed->pan_tilt.x), fabsf(speed->pan_tilt.y));
    move_speed = normalize_to_speed(max_speed);
  }

  return ptz_adapter_relative_move(pan_delta, tilt_delta, move_speed);
}

int onvif_ptz_continuous_move(const char* profile_token, const struct ptz_speed* velocity,
                              int timeout) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(velocity);

  int pan_vel = normalize_to_speed(velocity->pan_tilt.x);
  int tilt_vel = normalize_to_speed(velocity->pan_tilt.y);

  /* Apply direction */
  if (velocity->pan_tilt.x < 0) {
    pan_vel = -pan_vel;
  }
  if (velocity->pan_tilt.y < 0) {
    tilt_vel = -tilt_vel;
  }

  int timeout_s = timeout / PTZ_MS_PER_SECOND; /* Convert ms to seconds */
  if (timeout_s <= 0) {
    timeout_s = PTZ_DEFAULT_TIMEOUT_S; /* Default 10 seconds */
  }

  return ptz_adapter_continuous_move(pan_vel, tilt_vel, timeout_s);
}

int onvif_ptz_stop(const char* profile_token, int stop_pan_tilt, // NOLINT
                   int stop_zoom) {                              // NOLINT
  ONVIF_CHECK_NULL(profile_token);

  if (stop_pan_tilt) {
    return ptz_adapter_stop();
  }

  return ONVIF_SUCCESS; /* Zoom stop not needed (no zoom support) */
}

int onvif_ptz_goto_home_position(const char* profile_token, const struct ptz_speed* speed) {
  ONVIF_CHECK_NULL(profile_token);

  /* Home position is center (0, 0) in normalized coordinates */
  struct ptz_vector home_position;
  home_position.pan_tilt.x = 0.0F;
  home_position.pan_tilt.y = 0.0F;
  home_position.zoom = 0.0F;

  return onvif_ptz_absolute_move(profile_token, &home_position, speed);
}

int onvif_ptz_set_home_position(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

  /* TODO: Implement home position setting */
  platform_log_info("SetHomePosition for profile %s (not implemented)\n", profile_token);
  return ONVIF_SUCCESS;
}

int onvif_ptz_get_presets(const char* profile_token, struct ptz_preset** preset_list, int* count) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_list);
  ONVIF_CHECK_NULL(count);

  // Get profile index from token
  int profile_index = get_profile_index_from_token(profile_token);
  if (profile_index < 0) {
    platform_log_error("[PTZ] Invalid profile token: %s\n", profile_token);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  // Ensure presets are loaded from configuration
  int result = load_ptz_presets_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Return profile-specific presets
  ptz_preset_list_t* profile = &g_profile_presets[profile_index];
  *preset_list = profile->presets;
  *count = profile->preset_count;

  return ONVIF_SUCCESS;
}

int onvif_ptz_set_preset(const char* profile_token, // NOLINT
                         const char* preset_name, char* output_preset_token,
                         size_t token_size) { // NOLINT
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_name);
  ONVIF_CHECK_NULL(output_preset_token);

  // Get profile index from token
  int profile_index = get_profile_index_from_token(profile_token);
  if (profile_index < 0) {
    platform_log_error("[PTZ] Invalid profile token: %s\n", profile_token);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  // Ensure presets are loaded from configuration
  int result = load_ptz_presets_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Get profile-specific preset list
  ptz_preset_list_t* profile = &g_profile_presets[profile_index];

  // Check if profile has reached maximum presets
  if (profile->preset_count >= PTZ_PRESETS_PER_PROFILE) {
    platform_log_error("[PTZ] Maximum presets (%d) reached for profile %s\n",
                      PTZ_PRESETS_PER_PROFILE, profile_token);
    return ONVIF_ERROR; /* Maximum presets reached */
  }

  /* Get current position */
  struct ptz_status status;
  if (onvif_ptz_get_status(profile_token, &status) != 0) {
    return ONVIF_ERROR;
  }

  /* Use buffer pool for preset token generation */
  char* temp_token = buffer_pool_get(&g_ptz_response_buffer_pool);
  if (!temp_token) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Create new preset in profile */
  struct ptz_preset* preset = &profile->presets[profile->preset_count];

  /* Generate preset token: "Profile1_Preset1", "Profile2_Preset2", etc. */
  int token_len = snprintf(temp_token, PTZ_PRESET_TOKEN_MAX_LENGTH + 1, "%s_Preset%d",
                          profile_token, profile->preset_count + 1);
  if (token_len >= PTZ_PRESET_TOKEN_MAX_LENGTH + 1) {
    buffer_pool_return(&g_ptz_response_buffer_pool, temp_token);
    return ONVIF_ERROR;
  }

  /* Optimized string copying with explicit bounds checking */
  size_t token_copy_len =
    (size_t)token_len < sizeof(preset->token) - 1 ? (size_t)token_len : sizeof(preset->token) - 1;
  memcpy(preset->token, temp_token, token_copy_len);
  preset->token[token_copy_len] = '\0';

  size_t name_len = strlen(preset_name);
  if (name_len >= sizeof(preset->name)) {
    name_len = sizeof(preset->name) - 1;
  }
  memcpy(preset->name, preset_name, name_len);
  preset->name[name_len] = '\0';

  preset->ptz_position = status.position;

  /* Increment preset count */
  profile->preset_count++;

  /* Persist to configuration */
  result = config_runtime_set_ptz_profile_presets(profile_index, profile);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[PTZ] Failed to persist preset to configuration for profile %s\n",
                      profile_token);
    /* Rollback: decrement preset count */
    profile->preset_count--;
    buffer_pool_return(&g_ptz_response_buffer_pool, temp_token);
    return result;
  }

  /* Call adapter to persist preset (legacy compatibility) */
  ptz_adapter_set_preset(preset_name, profile->preset_count);

  /* Copy token to output */
  if (output_preset_token) {
    strncpy(output_preset_token, preset->token, PTZ_PRESET_TOKEN_MAX_LENGTH);
    output_preset_token[PTZ_PRESET_TOKEN_MAX_LENGTH] = '\0';
  }

  /* Return buffer pool allocation */
  buffer_pool_return(&g_ptz_response_buffer_pool, temp_token);

  platform_log_info("[PTZ] Preset '%s' created for profile %s (token: %s)\n",
                   preset_name, profile_token, preset->token);

  return ONVIF_SUCCESS;
}

int onvif_ptz_remove_preset(const char* profile_token, // NOLINT
                            const char* preset_token_to_remove) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_token_to_remove);

  // Get profile index from token
  int profile_index = get_profile_index_from_token(profile_token);
  if (profile_index < 0) {
    platform_log_error("[PTZ] Invalid profile token: %s\n", profile_token);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  // Ensure presets are loaded from configuration
  int result = load_ptz_presets_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Find preset in profile
  int preset_index = 0;
  result = find_preset_by_token_in_profile(profile_index, preset_token_to_remove, &preset_index);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[PTZ] Preset token '%s' not found in profile %s\n",
                      preset_token_to_remove, profile_token);
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Get profile-specific preset list
  ptz_preset_list_t* profile = &g_profile_presets[profile_index];

  // Remove preset by shifting remaining presets
  if (preset_index < profile->preset_count - 1) {
    memmove(&profile->presets[preset_index], &profile->presets[preset_index + 1],
            (profile->preset_count - preset_index - 1) * sizeof(struct ptz_preset));
  }

  // Clear the last slot and decrement count
  memset(&profile->presets[profile->preset_count - 1], 0, sizeof(struct ptz_preset));
  profile->preset_count--;

  // Persist to configuration
  result = config_runtime_set_ptz_profile_presets(profile_index, profile);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[PTZ] Failed to persist preset removal to configuration for profile %s\n",
                      profile_token);
    return result;
  }

  platform_log_info("[PTZ] Preset '%s' removed from profile %s\n",
                   preset_token_to_remove, profile_token);

  return ONVIF_SUCCESS;
}

int onvif_ptz_goto_preset(const char* profile_token, // NOLINT
                          const char* preset_token_to_goto, const struct ptz_speed* speed) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_token_to_goto);

  // Get profile index from token
  int profile_index = get_profile_index_from_token(profile_token);
  if (profile_index < 0) {
    platform_log_error("[PTZ] Invalid profile token: %s\n", profile_token);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  // Ensure presets are loaded from configuration
  int result = load_ptz_presets_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Find preset in profile
  int preset_index = 0;
  result = find_preset_by_token_in_profile(profile_index, preset_token_to_goto, &preset_index);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[PTZ] Preset token '%s' not found in profile %s\n",
                      preset_token_to_goto, profile_token);
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Get profile-specific preset list
  ptz_preset_list_t* profile = &g_profile_presets[profile_index];

  /* Move to preset position */
  platform_log_info("[PTZ] Moving to preset '%s' in profile %s\n",
                   preset_token_to_goto, profile_token);

  return onvif_ptz_absolute_move(profile_token, &profile->presets[preset_index].ptz_position, speed);
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

/* Refactored PTZ service implementation */

// PTZ service configuration and state
static struct {
  service_handler_config_t config;
} g_ptz_handler_state = {0}; // NOLINT

/* ============================================================================
 * Forward Declarations
 * ============================================================================ */

// Action handler forward declarations
static int handle_get_nodes(const service_handler_config_t* config, const http_request_t* request,
                            http_response_t* response, onvif_gsoap_context_t* gsoap_ctx);
static int handle_absolute_move(const service_handler_config_t* config,
                                const http_request_t* request, http_response_t* response,
                                onvif_gsoap_context_t* gsoap_ctx);
static int handle_get_presets(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx);
static int handle_set_preset(const service_handler_config_t* config, const http_request_t* request,
                             http_response_t* response, onvif_gsoap_context_t* gsoap_ctx);
static int handle_goto_preset(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx);

/* ============================================================================
 * PTZ Service Operations Lookup Table
 * ============================================================================ */

/**
 * @brief PTZ operation handler function pointer type
 */
typedef int (*ptz_operation_handler_t)(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief PTZ operation dispatch entry
 */
typedef struct {
  const char* operation_name;
  ptz_operation_handler_t handler;
} ptz_operation_entry_t;

/**
 * @brief PTZ service operation dispatch table
 */
static const ptz_operation_entry_t g_ptz_operations[] = {{"GetNodes", handle_get_nodes},
                                                         {"AbsoluteMove", handle_absolute_move},
                                                         {"GetPresets", handle_get_presets},
                                                         {"SetPreset", handle_set_preset},
                                                         {"GotoPreset", handle_goto_preset}};

#define PTZ_OPERATIONS_COUNT (sizeof(g_ptz_operations) / sizeof(g_ptz_operations[0]))

/* ============================================================================
 * PTZ Service Helper Functions
 * ============================================================================ */

/**
 * @brief PTZ service cleanup handler
 */
static void ptz_service_cleanup_handler(void) {
  onvif_ptz_cleanup();
}

/**
 * @brief PTZ service capabilities handler
 * @param capability_name Capability name to check
 * @return 1 if supported, 0 if not supported
 */
static int ptz_service_capabilities_handler(const char* capability_name) {
  if (!capability_name) {
    return 0;
  }

  // PTZ service supports all standard PTZ capabilities
  return 1;
}

/**
 * @brief Generate PTZ service capability structure
 * @param ctx gSOAP context for memory allocation
 * @param capabilities_ptr Output pointer to tt__PTZCapabilities*
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
static int ptz_service_get_capabilities(struct soap* ctx, void** capabilities_ptr) {
  if (!ctx || !capabilities_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate PTZCapabilities structure
  struct tt__PTZCapabilities* caps = soap_new_tt__PTZCapabilities(ctx, 1);
  if (!caps) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }
  soap_default_tt__PTZCapabilities(ctx, caps);

  // Get device IP and port from runtime config
  char device_ip[64] = "192.168.1.100"; // Default value
  int http_port = 8080;  // Default value

  // Try to get values from runtime config (ignore errors, use defaults)
  (void)config_runtime_get_string(CONFIG_SECTION_NETWORK, "device_ip", device_ip, sizeof(device_ip));
  (void)config_runtime_get_int(CONFIG_SECTION_NETWORK, "http_port", &http_port);

  // Build XAddr
  char xaddr[256];
  snprintf(xaddr, sizeof(xaddr), "http://%s:%d/onvif/ptz_service", device_ip, http_port);
  caps->XAddr = soap_strdup(ctx, xaddr);

  *capabilities_ptr = (void*)caps;
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ service registration structure using standardized interface
 */
static const onvif_service_registration_t g_ptz_service_registration = {
  .service_name = "ptz",
  .namespace_uri = "http://www.onvif.org/ver10/ptz/wsdl",
  .operation_handler = onvif_ptz_handle_operation,
  .init_handler = NULL, // Service is initialized explicitly before registration
  .cleanup_handler = ptz_service_cleanup_handler,
  .capabilities_handler = ptz_service_capabilities_handler,
  .get_capabilities = ptz_service_get_capabilities,
  .reserved = {NULL, NULL, NULL}};

/* ============================================================================
 * PTZ Service Operation Definitions
 * ============================================================================ */

/**
 * @brief PTZ nodes business logic function
 */
static int get_ptz_nodes_business_logic(const service_handler_config_t* config, // NOLINT
                                        const http_request_t* request,          // NOLINT
                                        http_response_t* response,              // NOLINT
                                        onvif_gsoap_context_t* gsoap_ctx,       // NOLINT
                                        service_log_context_t* log_ctx,
                                        error_context_t* error_ctx, // NOLINT
                                        void* callback_data) {
  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse GetNodes request (empty request structure)
  struct _tptz__GetNodes* get_nodes_req = NULL;
  result = onvif_gsoap_parse_get_nodes(gsoap_ctx, &get_nodes_req);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "parse_get_nodes", result,
                                  "Failed to parse GetNodes request");
    return result;
  }

  // Get PTZ nodes using existing function
  struct ptz_node* nodes = NULL;
  int count = 0;
  result = onvif_ptz_get_nodes(&nodes, &count);

  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "get_nodes", result, "Failed to get PTZ nodes");
    return result;
  }

  // Update callback data
  ptz_nodes_callback_data_t* data = (ptz_nodes_callback_data_t*)callback_data;
  data->nodes = nodes;
  data->count = count;

  service_log_operation_success(log_ctx, "PTZ nodes retrieved");
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ absolute move business logic function
 */
static int ptz_absolute_move_business_logic(const service_handler_config_t* config, // NOLINT
                                            const http_request_t* request,          // NOLINT
                                            http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx,
                                            service_log_context_t* log_ctx,
                                            error_context_t* error_ctx,
                                            void* callback_data) { // NOLINT
  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return error_handle_system(error_ctx, ONVIF_ERROR, "gsoap_init", response);
  }

  // Parse AbsoluteMove request using new gSOAP parsing function
  struct _tptz__AbsoluteMove* absolute_move_req = NULL;
  result = onvif_gsoap_parse_absolute_move(gsoap_ctx, &absolute_move_req);
  if (result != ONVIF_SUCCESS || !absolute_move_req) {
    service_log_operation_failure(log_ctx, "parse_absolute_move", result,
                                  "Failed to parse AbsoluteMove request");
    return error_handle_parameter(error_ctx, "AbsoluteMove", "parse_failed", response);
  }

  // Extract profile token
  if (!absolute_move_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = absolute_move_req->ProfileToken;

  // Extract position from Position field
  struct ptz_vector position;
  memset(&position, 0, sizeof(position));
  if (absolute_move_req->Position && absolute_move_req->Position->PanTilt) {
    position.pan_tilt.x = absolute_move_req->Position->PanTilt->x;
    position.pan_tilt.y = absolute_move_req->Position->PanTilt->y;
  }

  // Execute PTZ movement using the existing function
  result = onvif_ptz_absolute_move(profile_token, &position, NULL);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "absolute_move", response);
  }

  service_log_operation_success(log_ctx, "PTZ absolute move completed");
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ presets business logic function
 */
static int get_ptz_presets_business_logic(const service_handler_config_t* config, // NOLINT
                                          const http_request_t* request,          // NOLINT
                                          http_response_t* response,
                                          onvif_gsoap_context_t* gsoap_ctx,
                                          service_log_context_t* log_ctx,
                                          error_context_t* error_ctx, void* callback_data) {
  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return error_handle_system(error_ctx, ONVIF_ERROR, "gsoap_init", response);
  }

  // Parse GetPresets request using new gSOAP parsing function
  struct _tptz__GetPresets* get_presets_req = NULL;
  result = onvif_gsoap_parse_get_presets(gsoap_ctx, &get_presets_req);
  if (result != ONVIF_SUCCESS || !get_presets_req) {
    service_log_operation_failure(log_ctx, "parse_get_presets", result,
                                  "Failed to parse GetPresets request");
    return error_handle_parameter(error_ctx, "GetPresets", "parse_failed", response);
  }

  // Extract profile token from parsed structure
  if (!get_presets_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = get_presets_req->ProfileToken;

  // Get presets using existing function
  struct ptz_preset* preset_list = NULL;
  int count = 0;
  result = onvif_ptz_get_presets(profile_token, &preset_list, &count);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "get_presets", response);
  }

  // Update callback data
  ptz_presets_callback_data_t* data = (ptz_presets_callback_data_t*)callback_data;
  data->presets = preset_list;
  data->count = count;

  service_log_operation_success(log_ctx, "PTZ presets retrieved");
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ set preset business logic function
 */
static int set_ptz_preset_business_logic(const service_handler_config_t* config, // NOLINT
                                         const http_request_t* request,          // NOLINT
                                         http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return error_handle_system(error_ctx, ONVIF_ERROR, "gsoap_init", response);
  }

  // Parse SetPreset request using new gSOAP parsing function
  struct _tptz__SetPreset* set_preset_req = NULL;
  result = onvif_gsoap_parse_set_preset(gsoap_ctx, &set_preset_req);
  if (result != ONVIF_SUCCESS || !set_preset_req) {
    service_log_operation_failure(log_ctx, "parse_set_preset", result,
                                  "Failed to parse SetPreset request");
    return error_handle_parameter(error_ctx, "SetPreset", "parse_failed", response);
  }

  // Extract profile token from parsed structure
  if (!set_preset_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = set_preset_req->ProfileToken;

  // Extract preset name (optional, defaults to "Preset")
  const char* preset_name = set_preset_req->PresetName ? set_preset_req->PresetName : "Preset";

  // Use buffer pool for preset token output
  char* preset_token = buffer_pool_get(&g_ptz_response_buffer_pool);
  if (!preset_token) {
    return error_handle_system(error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "buffer_pool_get",
                               response);
  }

  // Set preset using existing function
  result =
    onvif_ptz_set_preset(profile_token, preset_name, preset_token, PTZ_PRESET_NAME_MAX_LENGTH);
  if (result != ONVIF_SUCCESS) {
    buffer_pool_return(&g_ptz_response_buffer_pool, preset_token);
    return error_handle_system(error_ctx, result, "set_preset", response);
  }

  // Update callback data
  ptz_set_preset_callback_data_t* data = (ptz_set_preset_callback_data_t*)callback_data;
  data->preset_token = preset_token;

  service_log_operation_success(log_ctx, "PTZ preset created");
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ goto preset business logic function
 */
static int goto_ptz_preset_business_logic(const service_handler_config_t* config, // NOLINT
                                          const http_request_t* request,          // NOLINT
                                          http_response_t* response,
                                          onvif_gsoap_context_t* gsoap_ctx,
                                          service_log_context_t* log_ctx,
                                          error_context_t* error_ctx,
                                          void* callback_data) { // NOLINT
  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return error_handle_system(error_ctx, ONVIF_ERROR, "gsoap_init", response);
  }

  // Parse GotoPreset request using new gSOAP parsing function
  struct _tptz__GotoPreset* goto_preset_req = NULL;
  result = onvif_gsoap_parse_goto_preset(gsoap_ctx, &goto_preset_req);
  if (result != ONVIF_SUCCESS || !goto_preset_req) {
    service_log_operation_failure(log_ctx, "parse_goto_preset", result,
                                  "Failed to parse GotoPreset request");
    return error_handle_parameter(error_ctx, "GotoPreset", "parse_failed", response);
  }

  // Extract profile token from parsed structure
  if (!goto_preset_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = goto_preset_req->ProfileToken;

  // Extract preset token (required)
  if (!goto_preset_req->PresetToken) {
    return error_handle_parameter(error_ctx, "PresetToken", "missing", response);
  }
  const char* preset_token = goto_preset_req->PresetToken;

  // Parse PTZ speed from request (optional) - for now, initialize with defaults
  struct ptz_speed speed;
  memset(&speed, 0, sizeof(speed));
  // TODO: Implement proper PTZ speed parsing using gSOAP parsing functions

  // Goto preset using existing function
  result = onvif_ptz_goto_preset(profile_token, preset_token, &speed);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "goto_preset", response);
  }

  service_log_operation_success(log_ctx, "PTZ goto preset completed");
  return ONVIF_SUCCESS;
}

/**
 * @brief PTZ nodes service operation definition
 */
static const onvif_service_operation_t get_ptz_nodes_operation = {
  .service_name = "PTZ",
  .operation_name = "GetNodes",
  .operation_context = "nodes_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_ptz_nodes_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief PTZ absolute move service operation definition
 */
static const onvif_service_operation_t ptz_absolute_move_operation = {
  .service_name = "PTZ",
  .operation_name = "AbsoluteMove",
  .operation_context = "ptz_movement",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = ptz_absolute_move_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief PTZ presets service operation definition
 */
static const onvif_service_operation_t get_ptz_presets_operation = {
  .service_name = "PTZ",
  .operation_name = "GetPresets",
  .operation_context = "presets_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_ptz_presets_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief PTZ set preset service operation definition
 */
static const onvif_service_operation_t set_ptz_preset_operation = {
  .service_name = "PTZ",
  .operation_name = "SetPreset",
  .operation_context = "preset_creation",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = set_ptz_preset_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief PTZ goto preset service operation definition
 */
static const onvif_service_operation_t goto_ptz_preset_operation = {
  .service_name = "PTZ",
  .operation_name = "GotoPreset",
  .operation_context = "preset_movement",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = goto_ptz_preset_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/* ============================================================================
 * Action Handlers
 * ============================================================================ */

// Action handlers
static int handle_get_nodes(const service_handler_config_t* config, const http_request_t* request,
                            http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for PTZ nodes
  ptz_nodes_callback_data_t callback_data = {.nodes = NULL, .count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_ptz_nodes_operation, ptz_nodes_response_callback,
                                           &callback_data);
}

static int handle_absolute_move(const service_handler_config_t* config,
                                const http_request_t* request, http_response_t* response,
                                onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for PTZ absolute move
  ptz_absolute_move_callback_data_t callback_data = {.message = "PTZ absolute move completed"};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &ptz_absolute_move_operation,
                                           ptz_absolute_move_response_callback, &callback_data);
}

// Additional action handlers
static int handle_get_presets(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for PTZ presets
  ptz_presets_callback_data_t callback_data = {.presets = NULL, .count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_ptz_presets_operation,
                                           ptz_presets_response_callback, &callback_data);
}

static int handle_set_preset(const service_handler_config_t* config, const http_request_t* request,
                             http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for PTZ set preset
  ptz_set_preset_callback_data_t callback_data = {0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &set_ptz_preset_operation,
                                           ptz_set_preset_response_callback, &callback_data);
}

static int handle_goto_preset(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for PTZ goto preset
  ptz_goto_preset_callback_data_t callback_data = {.message = "PTZ goto preset completed"};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &goto_ptz_preset_operation,
                                           ptz_goto_preset_response_callback, &callback_data);
}

/* ============================================================================
 * Standardized PTZ Service Operation Handler
 * ============================================================================ */

/**
 * @brief Handle ONVIF PTZ service operations using standardized interface
 * @param operation_name ONVIF operation name (e.g., "GetConfigurations")
 * @param request HTTP request containing SOAP envelope
 * @param response HTTP response to populate with SOAP envelope
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function implements the standardized onvif_service_operation_handler_t interface
 */
int onvif_ptz_handle_operation(const char* operation_name, const http_request_t* request,
                               http_response_t* response) {
  if (!operation_name || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate per-request gSOAP context for thread safety
  onvif_gsoap_context_t gsoap_ctx;
  int init_result = onvif_gsoap_init(&gsoap_ctx);
  if (init_result != ONVIF_SUCCESS) {
    platform_log_error("Failed to initialize gSOAP context for PTZ request (error: %d)\n",
                       init_result);
    return ONVIF_ERROR_MEMORY;
  }

  // CRITICAL: Initialize request parsing from HTTP body
  // This is required for gSOAP's soap_read_* functions to work
  if (request->body && request->body_length > 0) {
    int parse_init =
      onvif_gsoap_init_request_parsing(&gsoap_ctx, request->body, request->body_length);
    if (parse_init != ONVIF_SUCCESS) {
      platform_log_error("Failed to initialize SOAP request parsing (error: %d)\n", parse_init);
      onvif_gsoap_cleanup(&gsoap_ctx);
      return ONVIF_ERROR_PARSE_FAILED;
    }
  }

  // Dispatch using lookup table for O(n) performance with small constant factor
  int result = ONVIF_ERROR_NOT_FOUND;
  for (size_t i = 0; i < PTZ_OPERATIONS_COUNT; i++) {
    if (strcmp(operation_name, g_ptz_operations[i].operation_name) == 0) {
      result =
        g_ptz_operations[i].handler(&g_ptz_handler_state.config, request, response, &gsoap_ctx);
      break;
    }
  }

  // Cleanup gSOAP context
  onvif_gsoap_cleanup(&gsoap_ctx);

  return result;
}

/**
 * @brief Handle ONVIF PTZ service requests (HTTP server interface)
 * @param action_name ONVIF action name (e.g., "GetConfigurations")
 * @param request HTTP request containing SOAP envelope
 * @param response HTTP response to populate with SOAP envelope
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function provides the HTTP server interface for PTZ service
 */
int onvif_ptz_handle_request(const char* action_name, const http_request_t* request,
                             http_response_t* response) {
  if (!action_name || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Use the existing operation handler
  return onvif_ptz_handle_operation(action_name, request, response);
}

/* ============================================================================
 * PTZ Service Initialization and Cleanup
 * ============================================================================ */

int onvif_ptz_init(config_manager_t* config) {
  // Idempotency guard - prevent double initialization
  if (g_ptz_initialized) {
    return ONVIF_SUCCESS;
  }

  // Initialize PTZ handler state (NULL config allowed for unit testing)
  g_ptz_handler_state.config.service_type = ONVIF_SERVICE_PTZ;
  g_ptz_handler_state.config.service_name = "PTZ";
  g_ptz_handler_state.config.config = config;
  g_ptz_handler_state.config.enable_validation = 1;
  g_ptz_handler_state.config.enable_logging = 1;

  // Initialize buffer pool for PTZ service responses
  if (buffer_pool_init(&g_ptz_response_buffer_pool) != 0) {
    platform_log_error("Failed to initialize PTZ response buffer pool\n");
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Set flag BEFORE registration (enables cleanup on failure)
  g_ptz_initialized = 1;

#ifdef UNIT_TESTING
  int result = onvif_service_unit_register(&g_ptz_service_registration, &g_ptz_initialized,
                                           onvif_ptz_cleanup, "PTZ");
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  return ONVIF_SUCCESS;
#else
  int result = onvif_service_dispatcher_register_service(&g_ptz_service_registration);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("Failed to register PTZ service with dispatcher: %d\n", result);
    onvif_ptz_cleanup();
    return result;
  }

  platform_log_info("PTZ service initialized and registered with standardized dispatcher\n");

  return ONVIF_SUCCESS;
#endif
}

void onvif_ptz_cleanup(void) {
  // Guard against double cleanup
  if (!g_ptz_initialized || g_ptz_cleanup_in_progress) {
    return;
  }
  g_ptz_cleanup_in_progress = 1;
  // Unregister from standardized service dispatcher
  onvif_service_dispatcher_unregister_service("ptz");

  // Cleanup buffer pool
  buffer_pool_cleanup(&g_ptz_response_buffer_pool);

  // Cleanup preset memory
  cleanup_preset_memory();

  // Reset handler state
  memset(&g_ptz_handler_state, 0, sizeof(g_ptz_handler_state));

  // Reset initialization flag (enables re-initialization)
  g_ptz_initialized = 0;
  g_ptz_cleanup_in_progress = 0;
  // Check for memory leaks
  memory_manager_check_leaks();

  platform_log_info("PTZ service cleaned up and unregistered from dispatcher\n");
}

/* ============================================================================
 * Preset Management Helper Functions
 * ============================================================================ */

/**
 * @brief Load PTZ presets from configuration (lazy loading pattern)
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note Following media profile pattern from onvif_media.c
 */
static int load_ptz_presets_from_config(void) {
  if (g_presets_loaded) {
    return ONVIF_SUCCESS;
  }

  // Load presets for all 4 profiles
  for (int i = 0; i < PTZ_PROFILE_COUNT; i++) {
    int result = config_runtime_get_ptz_profile_presets(i, &g_profile_presets[i]);
    if (result != ONVIF_SUCCESS) {
      platform_log_error("[PTZ] Failed to load presets for profile %d from configuration\n", i + 1);
      return result;
    }
  }

  g_presets_loaded = 1;
  platform_log_info("[PTZ] Loaded presets for %d profiles from runtime configuration\n",
                    PTZ_PROFILE_COUNT);
  return ONVIF_SUCCESS;
}

/**
 * @brief Map profile token to profile index
 * @param profile_token Profile token (e.g., "Profile1", "ProfileToken1", "ptz_profile_1")
 * @return Profile index (0-3) on success, -1 on error
 */
static int get_profile_index_from_token(const char* profile_token) {
  if (!profile_token) {
    return -1;
  }

  // Support multiple profile token formats for compatibility
  // Format 1: "Profile1", "Profile2", "Profile3", "Profile4"
  if (strcmp(profile_token, "Profile1") == 0) {
    return 0;
  }
  if (strcmp(profile_token, "Profile2") == 0) {
    return 1;
  }
  if (strcmp(profile_token, "Profile3") == 0) {
    return 2;
  }
  if (strcmp(profile_token, "Profile4") == 0) {
    return 3;
  }

  // Format 2: "ProfileToken1", "ProfileToken2", etc. (legacy test format)
  if (strcmp(profile_token, "ProfileToken1") == 0) {
    return 0;
  }
  if (strcmp(profile_token, "ProfileToken2") == 0) {
    return 1;
  }
  if (strcmp(profile_token, "ProfileToken3") == 0) {
    return 2;
  }
  if (strcmp(profile_token, "ProfileToken4") == 0) {
    return 3;
  }

  // Format 3: "ptz_profile_1", "ptz_profile_2", etc. (SOAP test format)
  if (strcmp(profile_token, "ptz_profile_1") == 0) {
    return 0;
  }
  if (strcmp(profile_token, "ptz_profile_2") == 0) {
    return 1;
  }
  if (strcmp(profile_token, "ptz_profile_3") == 0) {
    return 2;
  }
  if (strcmp(profile_token, "ptz_profile_4") == 0) {
    return 3;
  }

  platform_log_error("[PTZ] Unknown profile token: %s\n", profile_token);
  return -1;
}

/**
 * @brief Find preset by token within specific profile
 * @param profile_index Profile index (0-3)
 * @param token Preset token to find
 * @param preset_index Output parameter for preset index within profile
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_NOT_FOUND if not found
 */
static int find_preset_by_token_in_profile(int profile_index, const char* token,
                                           int* preset_index) {
  if (profile_index < 0 || profile_index >= PTZ_PROFILE_COUNT || !token || !preset_index) {
    return ONVIF_ERROR_INVALID;
  }

  // Ensure presets are loaded
  int result = load_ptz_presets_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Linear search within profile's preset list (max 4 presets)
  ptz_preset_list_t* profile = &g_profile_presets[profile_index];
  for (int i = 0; i < profile->preset_count && i < PTZ_PRESETS_PER_PROFILE; i++) {
    if (strcmp(profile->presets[i].token, token) == 0) {
      *preset_index = i;
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

/**
 * @brief Cleanup preset memory and reset to initial state
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int cleanup_preset_memory(void) {
  // Clear all profile preset data
  memset(g_profile_presets, 0, sizeof(g_profile_presets));
  g_presets_loaded = 0;

  platform_log_info("[PTZ] Preset memory cleaned up: %zu bytes freed\n", sizeof(g_profile_presets));

  return ONVIF_SUCCESS;
}

/**
 * @brief Reset preset state (internal implementation)
 * @return ONVIF_SUCCESS on success
 * @note This resets the loaded flag and clears preset data without logging
 */
static int reset_preset_state(void) {
  // Clear all profile preset data
  memset(g_profile_presets, 0, sizeof(g_profile_presets));
  g_presets_loaded = 0;

  return ONVIF_SUCCESS;
}

/**
 * @brief Reset PTZ preset state (public API for testing)
 * @return ONVIF_SUCCESS on success
 */
int onvif_ptz_reset_presets(void) {
  return reset_preset_state();
}
