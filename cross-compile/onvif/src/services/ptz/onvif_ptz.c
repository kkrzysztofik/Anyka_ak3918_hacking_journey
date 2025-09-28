/**
 * @file onvif_ptz.c
 * @brief ONVIF PTZ service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_ptz.h"

#include <bits/pthreadtypes.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "platform/adapters/ptz_adapter.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"

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
#define PTZ_MAX_PRESETS                10

/* PTZ Node configuration */
static const struct ptz_node g_ptz_node = {
  .token = "PTZNode0",
  .name = "PTZ Node",
  .supported_ptz_spaces =
    {.absolute_pan_tilt_position_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                 "PositionGenericSpace",
                                          .x_range = {.min = -180.0f, .max = 180.0f},
                                          .y_range = {.min = -90.0f, .max = 90.0f}},
     .absolute_zoom_position_space = {.uri = "",
                                      .x_range = {.min = 0.0f, .max = 0.0f},
                                      .y_range = {.min = 0.0f, .max = 0.0f}},
     .relative_pan_tilt_translation_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                    "TranslationGenericSpace",
                                             .x_range = {.min = -180.0f, .max = 180.0f},
                                             .y_range = {.min = -90.0f, .max = 90.0f}},
     .relative_zoom_translation_space = {.uri = "",
                                         .x_range = {.min = 0.0f, .max = 0.0f},
                                         .y_range = {.min = 0.0f, .max = 0.0f}},
     .continuous_pan_tilt_velocity_space = {.uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                   "VelocityGenericSpace",
                                            .x_range = {.min = -1.0f, .max = 1.0f},
                                            .y_range = {.min = -1.0f, .max = 1.0f}},
     .continuous_zoom_velocity_space = {.uri = "",
                                        .x_range = {.min = 0.0f, .max = 0.0f},
                                        .y_range = {.min = 0.0f, .max = 0.0f}}},
  .maximum_number_of_presets = PTZ_MAX_PRESETS,
  .home_supported = 1,
  .auxiliary_commands = {0} /* No auxiliary commands */
};

/* Static preset storage */
static struct ptz_preset g_ptz_presets[PTZ_MAX_PRESETS]; // NOLINT
static int g_ptz_preset_count = 0;                       // NOLINT

/* Global PTZ state variables */
static pthread_mutex_t g_ptz_lock = PTHREAD_MUTEX_INITIALIZER; // NOLINT
static int g_ptz_initialized = 0;                              // NOLINT
static int g_ptz_current_pan_pos = 0;                          // NOLINT
static int g_ptz_current_tilt_pos = 0;                         // NOLINT
static pthread_t g_ptz_continuous_move_timer_thread = 0;       // NOLINT
static int g_ptz_continuous_move_timeout_s = 0;                // NOLINT
static volatile int g_ptz_continuous_move_active = 0;          // NOLINT

/* Preset management helper functions */
static int find_preset_by_token(const char* token, int* index);
static int remove_preset_at_index(int index);

/* Convert ONVIF normalized coordinates to device degrees */
static int normalize_to_degrees_pan(float normalized_value) {
  /* Convert from [-1, 1] to [-180, 180] */
  return (int)(normalized_value * 180.0f);
}

static int normalize_to_degrees_tilt(float normalized_value) {
  /* Convert from [-1, 1] to [-90, 90] */
  return (int)(normalized_value * 90.0f);
}

/* Convert device degrees to ONVIF normalized coordinates */
static float degrees_to_normalize_pan(int degrees) {
  /* Convert from [-180, 180] to [-1, 1] */
  return (float)degrees / 180.0f;
}

static float degrees_to_normalize_tilt(int degrees) {
  /* Convert from [-90, 90] to [-1, 1] */
  return (float)degrees / 90.0f;
}

/* Convert ONVIF normalized velocity to driver speed */
static int normalize_to_speed(float normalized_velocity) {
  /* Convert from [-1, 1] to [15, 100] driver speed range */
  float abs_vel = fabsf(normalized_velocity);
  return (int)(15 + abs_vel * 85); /* 15 + (0-1) * 85 = 15-100 */
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

  config->default_ptz_speed.pan_tilt.x = 0.5f;
  config->default_ptz_speed.pan_tilt.y = 0.5f;
  config->default_ptz_speed.zoom = 0.0f;

  config->default_ptz_timeout = 10000; /* 10 seconds */

  config->pan_tilt_limits.range.x_range.min = -1.0f;
  config->pan_tilt_limits.range.x_range.max = 1.0f;
  config->pan_tilt_limits.range.y_range.min = -1.0f;
  config->pan_tilt_limits.range.y_range.max = 1.0f;
  strncpy(config->pan_tilt_limits.range.uri,
          g_ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri,
          sizeof(config->pan_tilt_limits.range.uri) - 1);
  config->pan_tilt_limits.range.uri[sizeof(config->pan_tilt_limits.range.uri) - 1] = '\0';

  config->zoom_limits.range.x_range.min = 0.0f;
  config->zoom_limits.range.x_range.max = 0.0f;
  config->zoom_limits.range.y_range.min = 0.0f;
  config->zoom_limits.range.y_range.max = 0.0f;
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
  status->position.zoom = 0.0f; /* No zoom support */

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

  int timeout_s = timeout / 1000; /* Convert ms to seconds */
  if (timeout_s <= 0) {
    timeout_s = 10; /* Default 10 seconds */
  }

  return ptz_adapter_continuous_move(pan_vel, tilt_vel, timeout_s);
}

int onvif_ptz_stop(const char* profile_token, int stop_pan_tilt, // NOLINT
                   int stop_zoom) {
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
  home_position.pan_tilt.x = 0.0f;
  home_position.pan_tilt.y = 0.0f;
  home_position.zoom = 0.0f;

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

  *preset_list = g_ptz_presets;
  *count = g_ptz_preset_count;
  return ONVIF_SUCCESS;
}

int onvif_ptz_set_preset(const char* profile_token, // NOLINT
                         const char* preset_name, char* output_preset_token, size_t token_size) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_name);
  ONVIF_CHECK_NULL(output_preset_token);

  if (g_ptz_preset_count >= PTZ_MAX_PRESETS) {
    return ONVIF_ERROR; /* Maximum presets reached */
  }

  /* Get current position */
  struct ptz_status status;
  if (onvif_ptz_get_status(profile_token, &status) != 0) {
    return ONVIF_ERROR;
  }

  /* Create new preset */
  struct ptz_preset* preset = &g_ptz_presets[g_ptz_preset_count];
  if (snprintf(preset->token, sizeof(preset->token), "Preset%d", g_ptz_preset_count + 1) >=
      sizeof(preset->token)) {
    return ONVIF_ERROR;
  }
  strncpy(preset->name, preset_name, sizeof(preset->name) - 1);
  preset->name[sizeof(preset->name) - 1] = '\0';
  preset->ptz_position = status.position;

  /* Call adapter to persist preset */
  ptz_adapter_set_preset(preset_name, g_ptz_preset_count + 1);

  g_ptz_preset_count++;

  if (output_preset_token) {
    strncpy(output_preset_token, preset->token, 63);
    output_preset_token[63] = '\0';
  }

  return ONVIF_SUCCESS;
}

int onvif_ptz_remove_preset(const char* profile_token, // NOLINT
                            const char* preset_token_to_remove) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_token_to_remove);

  /* Find and remove preset using helper function */
  int index = 0;
  if (find_preset_by_token(preset_token_to_remove, &index) == ONVIF_SUCCESS) {
    return remove_preset_at_index(index);
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_ptz_goto_preset(const char* profile_token, // NOLINT
                          const char* preset_token_to_goto, const struct ptz_speed* speed) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(preset_token_to_goto);

  /* Find preset using helper function */
  int index = 0;
  if (find_preset_by_token(preset_token_to_goto, &index) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Move to preset position */
  return onvif_ptz_absolute_move(profile_token, &g_ptz_presets[index].ptz_position, speed);
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

/* Refactored PTZ service implementation */

// Service handler instance
static onvif_service_handler_instance_t g_ptz_handler; // NOLINT
static int g_handler_initialized = 0;                  // NOLINT

// Action handlers
static int handle_get_nodes(const service_handler_config_t* config, const http_request_t* request,
                            http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GetNodes", "nodes_retrieval");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "PTZ", "GetNodes", SERVICE_LOG_INFO);

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

  // Get PTZ nodes using existing function
  struct ptz_node* nodes = NULL;
  int count = 0;
  int result = onvif_ptz_get_nodes(&nodes, &count);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "get_nodes", response);
  }

  // Generate PTZ nodes response using callback infrastructure
  // Prepare callback data
  ptz_nodes_callback_data_t callback_data = {.nodes = nodes, .count = count};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, ptz_nodes_response_callback,
                                                       &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR_NOT_FOUND, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "allocate_response",
                                 response);
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
  service_log_operation_success(&log_ctx, "PTZ nodes retrieved");

  return ONVIF_SUCCESS;
}

static int handle_absolute_move(const service_handler_config_t* config,
                                const http_request_t* request, http_response_t* response,
                                onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "AbsoluteMove", "ptz_movement");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "PTZ", "AbsoluteMove", SERVICE_LOG_INFO);

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

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "gsoap_parser", "init_failed", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[32];
  result = onvif_gsoap_parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token));
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }

  // Parse PTZ position from request using gSOAP
  struct ptz_vector position;
  memset(&position, 0, sizeof(position));

  // Parse pan position
  char pan_str[32];
  result =
    onvif_gsoap_parse_value(gsoap_ctx, "//tptz:Position/tt:PanTilt/tt:x", pan_str, sizeof(pan_str));
  if (result == 0) {
    position.pan_tilt.x = atof(pan_str);
  }

  // Parse tilt position
  char tilt_str[32];
  result = onvif_gsoap_parse_value(gsoap_ctx, "//tptz:Position/tt:PanTilt/tt:y", tilt_str,
                                   sizeof(tilt_str));
  if (result == 0) {
    position.pan_tilt.y = atof(tilt_str);
  }

  // Execute PTZ movement using the existing function
  result = onvif_ptz_absolute_move(profile_token, &position, NULL);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "absolute_move", response);
  }

  // Generate PTZ absolute move response using callback infrastructure
  // Prepare callback data
  ptz_absolute_move_callback_data_t callback_data = {.message = "PTZ absolute move completed"};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, ptz_absolute_move_response_callback, &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR_NOT_FOUND, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "allocate_response",
                                 response);
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
  service_log_operation_success(&log_ctx, "PTZ absolute move completed");

  return ONVIF_SUCCESS;
}

// Additional action handlers
static int handle_get_presets(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GetPresets", "presets_retrieval");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "PTZ", "GetPresets", SERVICE_LOG_INFO);

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

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "gsoap_parser", "init_failed", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[32];
  result = onvif_gsoap_parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token));
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }

  // Get presets using existing function
  struct ptz_preset* preset_list = NULL;
  int count = 0;
  result = onvif_ptz_get_presets(profile_token, &preset_list, &count);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "get_presets", response);
  }

  // Generate PTZ presets response using callback infrastructure
  // Prepare callback data
  ptz_presets_callback_data_t callback_data = {.presets = preset_list, .count = count};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, ptz_presets_response_callback,
                                                       &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR_NOT_FOUND, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "allocate_response",
                                 response);
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
  service_log_operation_success(&log_ctx, "PTZ presets retrieved");

  return ONVIF_SUCCESS;
}

static int handle_set_preset(const service_handler_config_t* config, const http_request_t* request,
                             http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "SetPreset", "preset_creation");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "PTZ", "SetPreset", SERVICE_LOG_INFO);

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

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "gsoap_parser", "init_failed", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[32];
  result = onvif_gsoap_parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token));
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }

  // Parse preset name from request using gSOAP
  char preset_name[64] = "Preset";
  result = onvif_gsoap_parse_value(gsoap_ctx, "//tptz:Name", preset_name, sizeof(preset_name));
  // Note: preset_name defaults to "Preset" if parsing fails

  // Set preset using existing function
  char preset_token[64];
  result = onvif_ptz_set_preset(profile_token, preset_name, preset_token, sizeof(preset_token));
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "set_preset", response);
  }

  // Generate PTZ set preset response using callback infrastructure
  // Prepare callback data
  ptz_set_preset_callback_data_t callback_data = {.preset_token = preset_token};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, ptz_set_preset_response_callback,
                                                       &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR_NOT_FOUND, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "allocate_response",
                                 response);
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
  service_log_operation_success(&log_ctx, "PTZ preset created");

  return ONVIF_SUCCESS;
}

static int handle_goto_preset(const service_handler_config_t* config, const http_request_t* request,
                              http_response_t* response, onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize error and logging context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GotoPreset", "preset_movement");

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "PTZ", "GotoPreset", SERVICE_LOG_INFO);

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

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, request->body_length);
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "gsoap_parser", "init_failed", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[32];
  result = onvif_gsoap_parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token));
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }

  // Parse preset token from request using gSOAP
  char preset_token[64];
  result =
    onvif_gsoap_parse_value(gsoap_ctx, "//tptz:PresetToken", preset_token, sizeof(preset_token));
  if (result != 0) {
    return error_handle_parameter(&error_ctx, "preset_token", "missing", response);
  }

  // Parse PTZ speed from request (optional) - for now, initialize with defaults
  struct ptz_speed speed;
  memset(&speed, 0, sizeof(speed));
  // TODO: Implement proper PTZ speed parsing using gSOAP parsing functions

  // Goto preset using existing function
  result = onvif_ptz_goto_preset(profile_token, preset_token, &speed);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "goto_preset", response);
  }

  // Generate PTZ goto preset response using callback infrastructure
  // Prepare callback data
  ptz_goto_preset_callback_data_t callback_data = {.message = "PTZ goto preset completed"};

  // Use the generic response generation with callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, ptz_goto_preset_response_callback,
                                                       &callback_data);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "generate_response", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR_NOT_FOUND, "get_response_data", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR_MEMORY_ALLOCATION, "allocate_response",
                                 response);
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
  service_log_operation_success(&log_ctx, "PTZ goto preset completed");

  return ONVIF_SUCCESS;
}

// Action definitions
static const service_action_def_t ptz_actions[] = {{"GetConfigurations", handle_get_nodes, 0},
                                                   {"AbsoluteMove", handle_absolute_move, 1},
                                                   {"GetPresets", handle_get_presets, 1},
                                                   {"SetPreset", handle_set_preset, 1},
                                                   {"GotoPreset", handle_goto_preset, 1}};

int onvif_ptz_init(config_manager_t* config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {.service_type = ONVIF_SERVICE_PTZ,
                                             .service_name = "PTZ",
                                             .config = config,
                                             .enable_validation = 1,
                                             .enable_logging = 1};

  int result = onvif_service_handler_init(&g_ptz_handler, &handler_config, ptz_actions,
                                          sizeof(ptz_actions) / sizeof(ptz_actions[0]));

  if (result == ONVIF_SUCCESS) {
    // Register PTZ-specific error handlers
    // Error handler registration not implemented yet

    g_handler_initialized = 1;
  }

  return result;
}

void onvif_ptz_cleanup(void) {
  if (g_handler_initialized) {
    error_context_t error_ctx;
    error_context_init(&error_ctx, "PTZ", "Cleanup", "service_cleanup");

    onvif_service_handler_cleanup(&g_ptz_handler);

    // Unregister error handlers
    // Error handler unregistration not implemented yet

    g_handler_initialized = 0;

    // Check for memory leaks
    memory_manager_check_leaks();
  }
}

/**
 * @brief Handle ONVIF PTZ service requests
 */
int onvif_ptz_handle_request(const char* action_name, const http_request_t* request,
                             http_response_t* response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }

  return onvif_service_handler_handle_request(&g_ptz_handler, action_name, request, response);
}

/* ============================================================================
 * Low-level PTZ hardware abstraction functions (formerly ptz_adapter)
 * ============================================================================
 */

/* Continuous move timeout handling */

static int simple_abs(int value) {
  return (value < 0) ? -value : value;
}

/* Movement completion is handled by the Anyka driver internally */

/* Preset management helper functions */
static int find_preset_by_token(const char* token, int* index) {
  for (int i = 0; i < g_ptz_preset_count; i++) {
    if (strcmp(g_ptz_presets[i].token, token) == 0) {
      *index = i;
      return ONVIF_SUCCESS;
    }
  }
  return ONVIF_ERROR_NOT_FOUND;
}

static int remove_preset_at_index(int index) {
  if (index < 0 || index >= g_ptz_preset_count) {
    return ONVIF_ERROR_INVALID;
  }

  /* Shift remaining presets */
  for (int j = index; j < g_ptz_preset_count - 1; j++) {
    g_ptz_presets[j] = g_ptz_presets[j + 1];
  }
  g_ptz_preset_count--;
  return ONVIF_SUCCESS;
}

/* Continuous move timeout thread function */
static void* continuous_move_timeout_thread(void* unused_arg) {
  (void)unused_arg; /* Suppress unused parameter warning */

  platform_sleep_ms(g_ptz_continuous_move_timeout_s * 1000);

  if (g_ptz_continuous_move_active) {
    platform_log_info("PTZ continuous move timeout after %ds, stopping movement\n",
                      g_ptz_continuous_move_timeout_s);
    ptz_adapter_stop();
    g_ptz_continuous_move_active = 0;
  }

  return NULL;
}

int ptz_adapter_init(void) {
  int ret = 0;
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    ret = platform_ptz_init();
    if (ret == PLATFORM_SUCCESS) {
      /* PTZ is already initialized with proper parameters in
       * platform_ptz_init() */
      /* Reset to center position */
      g_ptz_current_pan_pos = 0;
      g_ptz_current_tilt_pos = 0;
      platform_ptz_move_to_position(g_ptz_current_pan_pos, g_ptz_current_tilt_pos);

      g_ptz_initialized = 1;
      platform_log_notice("PTZ adapter initialized successfully\n");
    } else {
      platform_log_error("PTZ initialization failed: %d\n", ret);
    }
  }
  pthread_mutex_unlock(&g_ptz_lock);
  return g_ptz_initialized ? 0 : -1;
}

int ptz_adapter_shutdown(void) {
  pthread_mutex_lock(&g_ptz_lock);
  if (g_ptz_initialized) {
    /* Stop any ongoing continuous movement */
    if (g_ptz_continuous_move_active) {
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
      g_ptz_continuous_move_active = 0;
    }

    /* Wait for timeout thread to finish if it exists */
    if (g_ptz_continuous_move_timer_thread != 0) {
      pthread_t timer_thread = g_ptz_continuous_move_timer_thread;
      g_ptz_continuous_move_timer_thread = 0;
      pthread_mutex_unlock(&g_ptz_lock);
      pthread_join(timer_thread, NULL);
      pthread_mutex_lock(&g_ptz_lock);
    }

    platform_ptz_cleanup();
    g_ptz_initialized = 0;
  }
  pthread_mutex_unlock(&g_ptz_lock);
  return ONVIF_SUCCESS;
}

platform_result_t ptz_adapter_get_status(struct ptz_device_status* status) {
  ONVIF_CHECK_NULL(status);

  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  // Use current position tracking instead of platform function
  status->h_pos_deg = g_ptz_current_pan_pos;
  status->v_pos_deg = g_ptz_current_tilt_pos;
  status->h_speed = 0;
  status->v_speed = 0;

  pthread_mutex_unlock(&g_ptz_lock);
  return ONVIF_SUCCESS;
}

int ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees, // NOLINT
                              int move_speed) {
  int ret = ONVIF_ERROR;

  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  /* Clamp values to safe ranges - based on akipc implementation */
  if (pan_degrees > PTZ_MAX_PAN_DEGREES) {
    pan_degrees = PTZ_MAX_PAN_DEGREES;
  }
  if (pan_degrees < PTZ_MIN_PAN_DEGREES) {
    pan_degrees = PTZ_MIN_PAN_DEGREES;
  }
  if (tilt_degrees > PTZ_MAX_TILT_DEGREES) {
    tilt_degrees = PTZ_MAX_TILT_DEGREES;
  }
  if (tilt_degrees < PTZ_MIN_TILT_DEGREES) {
    tilt_degrees = PTZ_MIN_TILT_DEGREES;
  }

  platform_log_info("PTZ absolute move to pan=%d, tilt=%d\n", pan_degrees, tilt_degrees);

  ret = platform_ptz_move_to_position(pan_degrees, tilt_degrees);
  if (ret == PLATFORM_SUCCESS) {
    g_ptz_current_pan_pos = pan_degrees;
    g_ptz_current_tilt_pos = tilt_degrees;
    ret = ONVIF_SUCCESS; // Convert platform success to ONVIF success
  }

  pthread_mutex_unlock(&g_ptz_lock);
  return ret;
}

int ptz_adapter_relative_move(int pan_delta_degrees,  // NOLINT
                              int tilt_delta_degrees, // NOLINT
                              int move_speed) {       // NOLINT
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  platform_log_info("PTZ relative move pan_delta=%d, tilt_delta=%d\n", pan_delta_degrees,
                    tilt_delta_degrees);

  int ret = ONVIF_SUCCESS;

  /* Horizontal movement - based on akipc implementation with step size 16 */
  if (pan_delta_degrees != 0) {
    platform_ptz_direction_t dir =
      (pan_delta_degrees > 0) ? PLATFORM_PTZ_DIRECTION_LEFT : PLATFORM_PTZ_DIRECTION_RIGHT;
    int steps = simple_abs(pan_delta_degrees);
    if (steps > PTZ_MAX_STEP_SIZE_PAN) {
      steps = PTZ_MAX_STEP_SIZE_PAN; /* Limit step size like in akipc */
    }

    ret = platform_ptz_turn(dir, steps);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_current_pan_pos += (dir == PLATFORM_PTZ_DIRECTION_LEFT) ? steps : -steps;
    }
  }

  /* Vertical movement - based on akipc implementation with step size 8 */
  if (tilt_delta_degrees != 0) {
    platform_ptz_direction_t dir =
      (tilt_delta_degrees > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
    int steps = simple_abs(tilt_delta_degrees);
    if (steps > PTZ_MAX_STEP_SIZE_TILT) {
      steps = PTZ_MAX_STEP_SIZE_TILT; /* Limit step size like in akipc */
    }

    ret = platform_ptz_turn(dir, steps);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_current_tilt_pos += (dir == PLATFORM_PTZ_DIRECTION_DOWN) ? steps : -steps;
    }
  }

  pthread_mutex_unlock(&g_ptz_lock);
  return ret;
}

int ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity, // NOLINT
                                int timeout_seconds) {
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  /* Stop any existing continuous movement */
  if (g_ptz_continuous_move_active) {
    g_ptz_continuous_move_active = 0;

    /* Wait for existing timer thread to finish */
    if (g_ptz_continuous_move_timer_thread != 0) {
      pthread_mutex_unlock(&g_ptz_lock);
      pthread_join(g_ptz_continuous_move_timer_thread, NULL);
      pthread_mutex_lock(&g_ptz_lock);
      g_ptz_continuous_move_timer_thread = 0;
    }
  }

  /* Start movement in appropriate directions */
  if (pan_velocity != 0) {
    platform_ptz_direction_t dir =
      (pan_velocity > 0) ? PLATFORM_PTZ_DIRECTION_RIGHT : PLATFORM_PTZ_DIRECTION_LEFT;
    platform_ptz_turn(dir, PTZ_MAX_PAN_DEGREES); /* Large number for continuous movement */
  }

  if (tilt_velocity != 0) {
    platform_ptz_direction_t dir =
      (tilt_velocity > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
    platform_ptz_turn(dir, PTZ_MAX_TILT_DEGREES); /* Large number for continuous movement */
  }

  /* Start timeout timer if timeout is specified and > 0 */
  if (timeout_seconds > 0) {
    g_ptz_continuous_move_timeout_s = timeout_seconds;
    g_ptz_continuous_move_active = 1;

    if (pthread_create(&g_ptz_continuous_move_timer_thread, NULL, continuous_move_timeout_thread,
                       NULL) != 0) {
      platform_log_error("Failed to create continuous move timeout thread\n");
      g_ptz_continuous_move_active = 0;
      pthread_mutex_unlock(&g_ptz_lock);
      return ONVIF_ERROR;
    }

    platform_log_info("PTZ continuous move started with %ds timeout\n", timeout_seconds);
  } else {
    platform_log_info("PTZ continuous move started (no timeout)\n");
  }

  pthread_mutex_unlock(&g_ptz_lock);
  return ONVIF_SUCCESS;
}

int ptz_adapter_stop(void) {
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  platform_log_info("PTZ stop all movement\n");

  /* Clear continuous move state */
  g_ptz_continuous_move_active = 0;

  /* Wait for timeout thread to finish if it exists */
  if (g_ptz_continuous_move_timer_thread != 0) {
    pthread_t timer_thread = g_ptz_continuous_move_timer_thread;
    g_ptz_continuous_move_timer_thread = 0;
    pthread_mutex_unlock(&g_ptz_lock);
    pthread_join(timer_thread, NULL);
    pthread_mutex_lock(&g_ptz_lock);
  }

  pthread_mutex_unlock(&g_ptz_lock);
  return ONVIF_SUCCESS;
}

int ptz_adapter_set_preset(const char* name, int preset_id) {
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  platform_log_info("PTZ set preset %s (id=%d) at pan=%d, tilt=%d\n", name ? name : "unnamed",
                    preset_id, g_ptz_current_pan_pos, g_ptz_current_tilt_pos);

  /* For now, just store current position - could be enhanced to save to file */
  pthread_mutex_unlock(&g_ptz_lock);
  return ONVIF_SUCCESS; /* Basic implementation */
}

int ptz_adapter_goto_preset(int preset_id) {
  pthread_mutex_lock(&g_ptz_lock);
  if (!g_ptz_initialized) {
    pthread_mutex_unlock(&g_ptz_lock);
    return ONVIF_ERROR;
  }

  platform_log_info("PTZ goto preset id=%d\n", preset_id);

  /* Basic implementation - could be enhanced to load from saved presets */
  int ret = ONVIF_ERROR;
  switch (preset_id) {
  case 1: /* Home position */
    ret = platform_ptz_move_to_position(0, 0);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_current_pan_pos = 0;
      g_ptz_current_tilt_pos = 0;
      ret = ONVIF_SUCCESS;
    }
    break;
  default:
    platform_log_info("Preset %d not implemented\n", preset_id);
    break;
  }

  pthread_mutex_unlock(&g_ptz_lock);
  return ret;
}
