/**
 * @file onvif_ptz.c
 * @brief ONVIF PTZ service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "core/config/config.h"
#include "onvif_ptz.h"
#include "platform/platform.h"
#include "protocol/soap/onvif_soap.h"
#include "protocol/xml/unified_xml.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/memory/memory_manager.h"
#include "utils/string/string_shims.h"

/* PTZ Movement Constants */
#define PTZ_MOVEMENT_TIMEOUT_MS        5000
#define PTZ_MOVEMENT_CHECK_INTERVAL_MS 5
#define PTZ_MAX_PAN_DEGREES           350
#define PTZ_MIN_PAN_DEGREES          -350
#define PTZ_MAX_TILT_DEGREES          130
#define PTZ_MIN_TILT_DEGREES         -130
#define PTZ_MAX_STEP_SIZE_PAN         16
#define PTZ_MAX_STEP_SIZE_TILT        8
#define PTZ_DEFAULT_SPEED             50
#define PTZ_MAX_PRESETS               10

/* PTZ Node configuration */
static struct ptz_node ptz_node = {
    .token = "PTZNode0",
    .name = "PTZ Node",
    .supported_ptz_spaces = {
        .absolute_pan_tilt_position_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
            .x_range = {.min = -180.0, .max = 180.0},
            .y_range = {.min = -90.0, .max = 90.0}
        },
        .absolute_zoom_position_space = {
            .uri = "",
            .x_range = {.min = 0.0, .max = 0.0},
            .y_range = {.min = 0.0, .max = 0.0}
        },
        .relative_pan_tilt_translation_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace",
            .x_range = {.min = -180.0, .max = 180.0},
            .y_range = {.min = -90.0, .max = 90.0}
        },
        .relative_zoom_translation_space = {
            .uri = "",
            .x_range = {.min = 0.0, .max = 0.0},
            .y_range = {.min = 0.0, .max = 0.0}
        },
        .continuous_pan_tilt_velocity_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace",
            .x_range = {.min = -1.0, .max = 1.0},
            .y_range = {.min = -1.0, .max = 1.0}
        },
        .continuous_zoom_velocity_space = {
            .uri = "",
            .x_range = {.min = 0.0, .max = 0.0},
            .y_range = {.min = 0.0, .max = 0.0}
        }
    },
    .maximum_number_of_presets = PTZ_MAX_PRESETS,
    .home_supported = 1,
    .auxiliary_commands = {0}  /* No auxiliary commands */
};

/* Static preset storage */
static struct ptz_preset presets[PTZ_MAX_PRESETS];
static int preset_count = 0;

/* Preset management helper functions */
static int find_preset_by_token(const char *token, int *index);
static int remove_preset_at_index(int index);

/* Error handler functions */
static int handle_ptz_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);
static int handle_ptz_system_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);

/* Convert ONVIF normalized coordinates to device degrees */
static int normalize_to_degrees_pan(float normalized_value) {
    /* Convert from [-1, 1] to [-180, 180] */
    return (int)(normalized_value * 180.0);
}

static int normalize_to_degrees_tilt(float normalized_value) {
    /* Convert from [-1, 1] to [-90, 90] */
    return (int)(normalized_value * 90.0);
}

/* Convert device degrees to ONVIF normalized coordinates */
static float degrees_to_normalize_pan(int degrees) {
    /* Convert from [-180, 180] to [-1, 1] */
    return (float)degrees / 180.0;
}

static float degrees_to_normalize_tilt(int degrees) {
    /* Convert from [-90, 90] to [-1, 1] */
    return (float)degrees / 90.0;
}

/* Convert ONVIF normalized velocity to driver speed */
static int normalize_to_speed(float normalized_velocity) {
    /* Convert from [-1, 1] to [15, 100] driver speed range */
    float abs_vel = fabs(normalized_velocity);
    return (int)(15 + abs_vel * 85);  /* 15 + (0-1) * 85 = 15-100 */
}

int onvif_ptz_get_nodes(struct ptz_node **nodes, int *count) {
    ONVIF_CHECK_NULL(nodes);
    ONVIF_CHECK_NULL(count);
    
    *nodes = &ptz_node;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_ptz_get_node(const char *node_token, struct ptz_node *node) {
    ONVIF_CHECK_NULL(node_token);
    ONVIF_CHECK_NULL(node);
    
    if (strcmp(node_token, ptz_node.token) == 0) {
        *node = ptz_node;
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_ptz_get_configuration(const char *config_token, struct ptz_configuration_ex *config) {
    ONVIF_CHECK_NULL(config_token);
    ONVIF_CHECK_NULL(config);
    
    /* Default PTZ configuration */
    strcpy(config->token, "PTZConfig0");
    strcpy(config->name, "PTZ Configuration");
    config->use_count = 1;
    strncpy(config->node_token, ptz_node.token, sizeof(config->node_token) - 1);
    config->node_token[sizeof(config->node_token) - 1] = '\0';
    
    config->default_absolute_pan_tilt_position_space = ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space;
    config->default_absolute_zoom_position_space = ptz_node.supported_ptz_spaces.absolute_zoom_position_space;
    config->default_relative_pan_tilt_translation_space = ptz_node.supported_ptz_spaces.relative_pan_tilt_translation_space;
    config->default_relative_zoom_translation_space = ptz_node.supported_ptz_spaces.relative_zoom_translation_space;
    config->default_continuous_pan_tilt_velocity_space = ptz_node.supported_ptz_spaces.continuous_pan_tilt_velocity_space;
    config->default_continuous_zoom_velocity_space = ptz_node.supported_ptz_spaces.continuous_zoom_velocity_space;
    
    config->default_ptz_speed.pan_tilt.x = 0.5;
    config->default_ptz_speed.pan_tilt.y = 0.5;
    config->default_ptz_speed.zoom = 0.0;
    
    config->default_ptz_timeout = 10000;  /* 10 seconds */
    
    config->pan_tilt_limits.range.x_range.min = -1.0;
    config->pan_tilt_limits.range.x_range.max = 1.0;
    config->pan_tilt_limits.range.y_range.min = -1.0;
    config->pan_tilt_limits.range.y_range.max = 1.0;
    strncpy(config->pan_tilt_limits.range.uri, 
            ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri, 
            sizeof(config->pan_tilt_limits.range.uri) - 1);
    config->pan_tilt_limits.range.uri[sizeof(config->pan_tilt_limits.range.uri) - 1] = '\0';
    
    config->zoom_limits.range.x_range.min = 0.0;
    config->zoom_limits.range.x_range.max = 0.0;
    config->zoom_limits.range.y_range.min = 0.0;
    config->zoom_limits.range.y_range.max = 0.0;
    config->zoom_limits.range.uri[0] = '\0';  /* No zoom support */
    
    return ONVIF_SUCCESS;
}

int onvif_ptz_get_status(const char *profile_token, struct ptz_status *status) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(status);
    
    struct ptz_device_status adapter_status;
    if (ptz_adapter_get_status(&adapter_status) != 0) {
        return ONVIF_ERROR;
    }
    
    /* Convert adapter status to ONVIF format */
    status->position.pan_tilt.x = degrees_to_normalize_pan(adapter_status.h_pos_deg);
    status->position.pan_tilt.y = degrees_to_normalize_tilt(adapter_status.v_pos_deg);
    status->position.zoom = 0.0;  /* No zoom support */
    
    strncpy(status->position.space, 
            ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri,
            sizeof(status->position.space) - 1);
    status->position.space[sizeof(status->position.space) - 1] = '\0';
    
    /* Movement status */
    status->move_status.pan_tilt = (adapter_status.h_speed > 0 || adapter_status.v_speed > 0) ? 
                                   PTZ_MOVE_MOVING : PTZ_MOVE_IDLE;
    status->move_status.zoom = PTZ_MOVE_IDLE;
    
    /* Error status */
    strcpy(status->error, "");
    
    /* UTC time - simplified */
    time_t now = time(NULL);
    struct tm *utc_tm = gmtime(&now);
    strftime(status->utc_time, sizeof(status->utc_time), "%Y-%m-%dT%H:%M:%S.000Z", utc_tm);
    
    return ONVIF_SUCCESS;
}

int onvif_ptz_absolute_move(const char *profile_token, const struct ptz_vector *position, 
                           const struct ptz_speed *speed) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(position);
    
    int pan_deg = normalize_to_degrees_pan(position->pan_tilt.x);
    int tilt_deg = normalize_to_degrees_tilt(position->pan_tilt.y);
    
    int move_speed = PTZ_DEFAULT_SPEED;  /* Default speed */
    if (speed) {
        float max_speed = fmax(fabs(speed->pan_tilt.x), fabs(speed->pan_tilt.y));
        move_speed = normalize_to_speed(max_speed);
    }
    
    return ptz_adapter_absolute_move(pan_deg, tilt_deg, move_speed);
}

int onvif_ptz_relative_move(const char *profile_token, const struct ptz_vector *translation,
                           const struct ptz_speed *speed) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(translation);
    
    int pan_delta = normalize_to_degrees_pan(translation->pan_tilt.x);
    int tilt_delta = normalize_to_degrees_tilt(translation->pan_tilt.y);
    
    int move_speed = PTZ_DEFAULT_SPEED;  /* Default speed */
    if (speed) {
        float max_speed = fmax(fabs(speed->pan_tilt.x), fabs(speed->pan_tilt.y));
        move_speed = normalize_to_speed(max_speed);
    }
    
    return ptz_adapter_relative_move(pan_delta, tilt_delta, move_speed);
}

int onvif_ptz_continuous_move(const char *profile_token, const struct ptz_speed *velocity, 
                             int timeout) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(velocity);
    
    int pan_vel = normalize_to_speed(velocity->pan_tilt.x);
    int tilt_vel = normalize_to_speed(velocity->pan_tilt.y);
    
    /* Apply direction */
    if (velocity->pan_tilt.x < 0) pan_vel = -pan_vel;
    if (velocity->pan_tilt.y < 0) tilt_vel = -tilt_vel;
    
    int timeout_s = timeout / 1000;  /* Convert ms to seconds */
    if (timeout_s <= 0) timeout_s = 10;  /* Default 10 seconds */
    
    return ptz_adapter_continuous_move(pan_vel, tilt_vel, timeout_s);
}

int onvif_ptz_stop(const char *profile_token, int pan_tilt, int zoom) {
    ONVIF_CHECK_NULL(profile_token);
    
    if (pan_tilt) {
        return ptz_adapter_stop();
    }
    
    return ONVIF_SUCCESS;  /* Zoom stop not needed (no zoom support) */
}

int onvif_ptz_goto_home_position(const char *profile_token, const struct ptz_speed *speed) {
    ONVIF_CHECK_NULL(profile_token);
    
    /* Home position is center (0, 0) in normalized coordinates */
    struct ptz_vector home_position;
    home_position.pan_tilt.x = 0.0;
    home_position.pan_tilt.y = 0.0;
    home_position.zoom = 0.0;
    
    return onvif_ptz_absolute_move(profile_token, &home_position, speed);
}

int onvif_ptz_set_home_position(const char *profile_token) {
    ONVIF_CHECK_NULL(profile_token);
    
    /* TODO: Implement home position setting */
    platform_log_info("SetHomePosition for profile %s (not implemented)\n", profile_token);
    return ONVIF_SUCCESS;
}

int onvif_ptz_get_presets(const char *profile_token, struct ptz_preset **preset_list, int *count) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_list);
    ONVIF_CHECK_NULL(count);
    
    *preset_list = presets;
    *count = preset_count;
    return ONVIF_SUCCESS;
}

int onvif_ptz_set_preset(const char *profile_token, const char *preset_name, char *preset_token, size_t token_size) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_name);
    ONVIF_CHECK_NULL(preset_token);
    
    if (preset_count >= PTZ_MAX_PRESETS) {
        return ONVIF_ERROR;  /* Maximum presets reached */
    }
    
    /* Get current position */
    struct ptz_status status;
    if (onvif_ptz_get_status(profile_token, &status) != 0) {
        return ONVIF_ERROR;
    }
    
    /* Create new preset */
    struct ptz_preset *preset = &presets[preset_count];
    if (snprintf(preset->token, sizeof(preset->token), "Preset%d", preset_count + 1) >= sizeof(preset->token)) {
        return ONVIF_ERROR;
    }
    strncpy(preset->name, preset_name, sizeof(preset->name) - 1);
    preset->name[sizeof(preset->name) - 1] = '\0';
    preset->ptz_position = status.position;
    
    /* Call adapter to persist preset */
    ptz_adapter_set_preset(preset_name, preset_count + 1);
    
    preset_count++;
    
    if (preset_token) {
        strncpy(preset_token, preset->token, 63);
        preset_token[63] = '\0';
    }
    
    return ONVIF_SUCCESS;
}

int onvif_ptz_remove_preset(const char *profile_token, const char *preset_token) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_token);
    
    /* Find and remove preset using helper function */
    int index;
    if (find_preset_by_token(preset_token, &index) == ONVIF_SUCCESS) {
        return remove_preset_at_index(index);
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_ptz_goto_preset(const char *profile_token, const char *preset_token,
                         const struct ptz_speed *speed) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_token);
    
    /* Find preset using helper function */
    int index;
    if (find_preset_by_token(preset_token, &index) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_NOT_FOUND;
    }
    
    /* Move to preset position */
    return onvif_ptz_absolute_move(profile_token, &presets[index].ptz_position, speed);
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

/* Refactored PTZ service implementation */

// Service handler instance
static onvif_service_handler_instance_t g_ptz_handler;
static int g_handler_initialized = 0;

// Action handlers
static int handle_get_nodes(const service_handler_config_t *config,
                           const onvif_request_t *request,
                           onvif_response_t *response,
                           onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GetNodes", "nodes_retrieval");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Build PTZ nodes XML using XML builder
  onvif_xml_builder_start_element(xml_builder, "tptz:PTZNode", NULL);
  
  onvif_xml_builder_element_with_text(xml_builder, "tt:Name", ptz_node.name, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tt:SupportedPTZSpaces", "true", NULL);
  
  // Supported spaces
  onvif_xml_builder_start_element(xml_builder, "tt:SupportedSpaces", NULL);
  
  // Absolute position space
  onvif_xml_builder_start_element(xml_builder, "tt:AbsolutePanTiltPositionSpace", NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tt:URI", ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri, NULL);
  onvif_xml_builder_start_element(xml_builder, "tt:XRange", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Min", "%.1f", ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.min);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Max", "%.1f", ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.max);
  onvif_xml_builder_end_element(xml_builder, "tt:XRange");
  onvif_xml_builder_start_element(xml_builder, "tt:YRange", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Min", "%.1f", ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.min);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Max", "%.1f", ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.max);
  onvif_xml_builder_end_element(xml_builder, "tt:YRange");
  onvif_xml_builder_end_element(xml_builder, "tt:AbsolutePanTiltPositionSpace");
  
  // Absolute zoom position space
  onvif_xml_builder_start_element(xml_builder, "tt:AbsoluteZoomPositionSpace", NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tt:URI", ptz_node.supported_ptz_spaces.absolute_zoom_position_space.uri, NULL);
  onvif_xml_builder_start_element(xml_builder, "tt:XRange", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Min", "%.1f", ptz_node.supported_ptz_spaces.absolute_zoom_position_space.x_range.min);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Max", "%.1f", ptz_node.supported_ptz_spaces.absolute_zoom_position_space.x_range.max);
  onvif_xml_builder_end_element(xml_builder, "tt:XRange");
  onvif_xml_builder_end_element(xml_builder, "tt:AbsoluteZoomPositionSpace");
  
  onvif_xml_builder_end_element(xml_builder, "tt:SupportedSpaces");
  
  // Maximum number of presets
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:MaximumNumberOfPresets", "%d", ptz_node.maximum_number_of_presets);
  
  // Home supported
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:HomeSupported", "%s", ptz_node.home_supported ? "true" : "false");
  
  onvif_xml_builder_end_element(xml_builder, "tptz:PTZNode");
  
  // Generate success response using consistent SOAP response utility
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, "GetNodes", xml_content);
}

static int handle_absolute_move(const service_handler_config_t *config,
                               const onvif_request_t *request,
                               onvif_response_t *response,
                               onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "AbsoluteMove", "ptz_movement");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Parse profile token from request using common XML parser
  char profile_token[32];
  if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  // Parse PTZ position from request using common XML parser
  struct ptz_vector position;
  onvif_xml_parser_t parser;
  if (onvif_xml_parser_init(&parser, request->body, strlen(request->body), NULL) != ONVIF_SUCCESS) {
    return error_handle_parameter(&error_ctx, "xml_parser", "init_failed", response);
  }
  
  if (onvif_xml_parse_ptz_position(&parser, &position) != 0) {
    onvif_xml_parser_cleanup(&parser);
    return error_handle_parameter(&error_ctx, "ptz_position", "invalid", response);
  }
  
  onvif_xml_parser_cleanup(&parser);
  
  // Execute PTZ movement using the existing function
  int result = onvif_ptz_absolute_move(profile_token, &position, NULL);
  
  if (result == ONVIF_SUCCESS) {
    // Generate success response using consistent SOAP response utility
    return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, "AbsoluteMove", "");
  } else {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
}

// Additional action handlers
static int handle_get_presets(const service_handler_config_t *config,
                             const onvif_request_t *request,
                             onvif_response_t *response,
                             onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GetPresets", "presets_retrieval");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Parse profile token from request using common XML parser
  char profile_token[32];
  if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  // Get presets using existing function
  struct ptz_preset *preset_list;
  int count;
  int result = onvif_ptz_get_presets(profile_token, &preset_list, &count);
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Build presets XML
  onvif_xml_builder_start_element(xml_builder, "tptz:Preset", NULL);
  
  for (int i = 0; i < count; i++) {
    onvif_xml_builder_start_element(xml_builder, "tt:Preset", "token", preset_list[i].token, NULL);
    onvif_xml_builder_element_with_text(xml_builder, "tt:Name", preset_list[i].name, NULL);
    onvif_xml_builder_end_element(xml_builder, "tt:Preset");
  }
  
  onvif_xml_builder_end_element(xml_builder, "tptz:Preset");
  
  // Generate success response using consistent SOAP response utility
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, "GetPresets", xml_content);
}

static int handle_set_preset(const service_handler_config_t *config,
                            const onvif_request_t *request,
                            onvif_response_t *response,
                            onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "SetPreset", "preset_creation");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Parse profile token from request using common XML parser
  char profile_token[32];
  if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  // Parse preset name from request
  char preset_name[64] = "Preset";
  onvif_xml_extract_string_value(request->body, "<trt:Name>", "</trt:Name>", preset_name, sizeof(preset_name));
  
  // Set preset using existing function
  char preset_token[64];
  int result = onvif_ptz_set_preset(profile_token, preset_name, preset_token, sizeof(preset_token));
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Build response XML
  onvif_xml_builder_element_with_text(xml_builder, "tptz:PresetToken", preset_token, NULL);
  
  // Generate success response using consistent SOAP response utility
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, "SetPreset", xml_content);
}

static int handle_goto_preset(const service_handler_config_t *config,
                             const onvif_request_t *request,
                             onvif_response_t *response,
                             onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "PTZ", "GotoPreset", "preset_movement");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Parse profile token from request using common XML parser
  char profile_token[32];
  if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  // Parse preset token from request
  char preset_token[64];
  if (onvif_xml_extract_string_value(request->body, "<trt:PresetToken>", "</trt:PresetToken>", 
                                   preset_token, sizeof(preset_token)) != 0) {
    return error_handle_parameter(&error_ctx, "preset_token", "missing", response);
  }
  
  // Parse PTZ speed from request (optional)
  struct ptz_speed speed;
  onvif_xml_parser_t parser;
  if (onvif_xml_parser_init(&parser, request->body, strlen(request->body), NULL) != ONVIF_SUCCESS) {
    return error_handle_parameter(&error_ctx, "xml_parser", "init_failed", response);
  }
  
  onvif_xml_parse_ptz_speed(&parser, &speed);
  onvif_xml_parser_cleanup(&parser);
  
  // Goto preset using existing function
  int result = onvif_ptz_goto_preset(profile_token, preset_token, &speed);
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Generate success response using consistent SOAP response utility
  return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, "GotoPreset", "");
}

// Action definitions
static const service_action_def_t ptz_actions[] = {
  {ONVIF_ACTION_GET_CONFIGURATIONS, "GetConfigurations", handle_get_nodes, 0},
  {ONVIF_ACTION_ABSOLUTE_MOVE, "AbsoluteMove", handle_absolute_move, 1},
  {ONVIF_ACTION_GET_PRESETS, "GetPresets", handle_get_presets, 1},
  {ONVIF_ACTION_SET_PRESET, "SetPreset", handle_set_preset, 1},
  {ONVIF_ACTION_GOTO_PRESET, "GotoPreset", handle_goto_preset, 1}
};

int onvif_ptz_init(config_manager_t *config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }
  
  service_handler_config_t handler_config = {
    .service_type = ONVIF_SERVICE_PTZ,
    .service_name = "PTZ",
    .config = config,
    .enable_validation = 1,
    .enable_logging = 1
  };
  
  int result = onvif_service_handler_init(&g_ptz_handler, &handler_config,
                                   ptz_actions, sizeof(ptz_actions) / sizeof(ptz_actions[0]));
  
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
int onvif_ptz_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!g_handler_initialized) {
        return ONVIF_ERROR;
    }
    
    return onvif_service_handler_handle_request(&g_ptz_handler, action, request, response);
}

/* ============================================================================
 * Low-level PTZ hardware abstraction functions (formerly ptz_adapter)
 * ============================================================================ */

static pthread_mutex_t ptz_lock = PTHREAD_MUTEX_INITIALIZER;
static int ptz_initialized = 0;
static int current_pan_pos = 0;
static int current_tilt_pos = 0;

/* Continuous move timeout handling */
static pthread_t continuous_move_timer_thread = 0;
static int continuous_move_timeout_s = 0;
static volatile int continuous_move_active = 0;

static int simple_abs(int value) { return (value < 0) ? -value : value; }

/* Common movement completion checking function */
static int wait_for_movement_completion(uint32_t timeout_ms) {
    platform_ptz_status_t h_status, v_status;
    uint32_t elapsed_ms = 0;
    uint32_t check_interval_ms = PTZ_MOVEMENT_CHECK_INTERVAL_MS;
    
    do {
        platform_sleep_us(check_interval_ms * 1000); /* Convert to microseconds */
        elapsed_ms += check_interval_ms;
        
        platform_ptz_get_status(PLATFORM_PTZ_AXIS_PAN, &h_status);
        platform_ptz_get_status(PLATFORM_PTZ_AXIS_TILT, &v_status);
        
        if (elapsed_ms >= timeout_ms) {
            platform_log_error("PTZ movement timeout after %dms\n", timeout_ms);
            return ONVIF_ERROR_TIMEOUT;
        }
    } while ((h_status != PLATFORM_PTZ_STATUS_OK) || (v_status != PLATFORM_PTZ_STATUS_OK));
    
    return ONVIF_SUCCESS;
}

/* Preset management helper functions */
static int find_preset_by_token(const char *token, int *index) {
    for (int i = 0; i < preset_count; i++) {
        if (strcmp(presets[i].token, token) == 0) {
            *index = i;
            return ONVIF_SUCCESS;
        }
    }
    return ONVIF_ERROR_NOT_FOUND;
}

static int remove_preset_at_index(int index) {
    if (index < 0 || index >= preset_count) {
        return ONVIF_ERROR_INVALID;
    }
    
    /* Shift remaining presets */
    for (int j = index; j < preset_count - 1; j++) {
        presets[j] = presets[j + 1];
    }
    preset_count--;
    return ONVIF_SUCCESS;
}

/* Continuous move timeout thread function */
static void* continuous_move_timeout_thread(void* arg) {
    (void)arg; /* Suppress unused parameter warning */
    
    platform_sleep_ms(continuous_move_timeout_s * 1000);
    
    if (continuous_move_active) {
        platform_log_info("PTZ continuous move timeout after %ds, stopping movement\n", continuous_move_timeout_s);
        ptz_adapter_stop();
        continuous_move_active = 0;
    }
    
    return NULL;
}

int ptz_adapter_init(void) {
    int ret = 0;
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        ret = platform_ptz_init();
        if (ret == 0) {
            /* Initialize PTZ with proper motor parameters */
            platform_ptz_set_degree(PTZ_MAX_PAN_DEGREES, PTZ_MAX_TILT_DEGREES);
            platform_ptz_check_self();
            
            /* Reset to center position */
            current_pan_pos = 0;
            current_tilt_pos = 0;
            platform_ptz_move_to_position(current_pan_pos, current_tilt_pos);
            
            ptz_initialized = 1;
            platform_log_notice("PTZ adapter initialized successfully\n");
        } else {
            platform_log_error("ak_drv_ptz_open failed: %d\n", ret);
        }
    }
    pthread_mutex_unlock(&ptz_lock);
    return ptz_initialized ? 0 : -1;
}

int ptz_adapter_shutdown(void) {
    pthread_mutex_lock(&ptz_lock);
    if (ptz_initialized) {
        /* Stop any ongoing continuous movement */
        if (continuous_move_active) {
            platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
            platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
            platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
            platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
            continuous_move_active = 0;
        }
        
        /* Wait for timeout thread to finish if it exists */
        if (continuous_move_timer_thread != 0) {
            pthread_t timer_thread = continuous_move_timer_thread;
            continuous_move_timer_thread = 0;
            pthread_mutex_unlock(&ptz_lock);
            pthread_join(timer_thread, NULL);
            pthread_mutex_lock(&ptz_lock);
        }
        
        platform_ptz_cleanup();
        ptz_initialized = 0;
    }
    pthread_mutex_unlock(&ptz_lock);
    return ONVIF_SUCCESS;
}

platform_result_t ptz_adapter_get_status(struct ptz_device_status *status) {
    ONVIF_CHECK_NULL(status);
    
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    int h = platform_ptz_get_step_position(PLATFORM_PTZ_AXIS_PAN);
    int v = platform_ptz_get_step_position(PLATFORM_PTZ_AXIS_TILT);
    
    status->h_pos_deg = h;
    status->v_pos_deg = v;
    status->h_speed = 0;
    status->v_speed = 0;
    
    pthread_mutex_unlock(&ptz_lock);
    return ONVIF_SUCCESS;
}

int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed) {
    int ret = ONVIF_ERROR;
    
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    /* Clamp values to safe ranges - based on akipc implementation */
    if (pan_deg > PTZ_MAX_PAN_DEGREES) pan_deg = PTZ_MAX_PAN_DEGREES;
    if (pan_deg < PTZ_MIN_PAN_DEGREES) pan_deg = PTZ_MIN_PAN_DEGREES;
    if (tilt_deg > PTZ_MAX_TILT_DEGREES) tilt_deg = PTZ_MAX_TILT_DEGREES;
    if (tilt_deg < PTZ_MIN_TILT_DEGREES) tilt_deg = PTZ_MIN_TILT_DEGREES;
    
    platform_log_info("PTZ absolute move to pan=%d, tilt=%d\n", pan_deg, tilt_deg);
    
    ret = platform_ptz_move_to_position(pan_deg, tilt_deg);
    if (ret == 0) {
        current_pan_pos = pan_deg;
        current_tilt_pos = tilt_deg;
        
        /* Wait for movement to complete with timeout */
        ret = wait_for_movement_completion(PTZ_MOVEMENT_TIMEOUT_MS);
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    platform_log_info("PTZ relative move pan_delta=%d, tilt_delta=%d\n", pan_delta_deg, tilt_delta_deg);
    
    int ret = 0;
    platform_ptz_status_t h_status, v_status;
    
    /* Horizontal movement - based on akipc implementation with step size 16 */
    if (pan_delta_deg != 0) {
        platform_ptz_direction_t dir = (pan_delta_deg > 0) ? PLATFORM_PTZ_DIRECTION_LEFT : PLATFORM_PTZ_DIRECTION_RIGHT;
        int steps = simple_abs(pan_delta_deg);
        if (steps > PTZ_MAX_STEP_SIZE_PAN) steps = PTZ_MAX_STEP_SIZE_PAN; /* Limit step size like in akipc */
        
        ret = platform_ptz_turn(dir, steps);
        if (ret == 0) {
            current_pan_pos += (dir == PLATFORM_PTZ_DIRECTION_LEFT) ? steps : -steps;
        }
    }
    
    /* Vertical movement - based on akipc implementation with step size 8 */
    if (tilt_delta_deg != 0) {
        platform_ptz_direction_t dir = (tilt_delta_deg > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
        int steps = simple_abs(tilt_delta_deg);
        if (steps > PTZ_MAX_STEP_SIZE_TILT) steps = PTZ_MAX_STEP_SIZE_TILT; /* Limit step size like in akipc */
        
        ret |= platform_ptz_turn(dir, steps);
        if (ret == 0) current_tilt_pos += (dir == PLATFORM_PTZ_DIRECTION_DOWN) ? steps : -steps;
    }
    
    /* Wait for movement to complete with timeout */
    if (ret == 0) {
        ret = wait_for_movement_completion(PTZ_MOVEMENT_TIMEOUT_MS);
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    /* Stop any existing continuous movement */
    if (continuous_move_active) {
        platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
        platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
        platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
        platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
        continuous_move_active = 0;
        
        /* Wait for existing timer thread to finish */
        if (continuous_move_timer_thread != 0) {
            pthread_mutex_unlock(&ptz_lock);
            pthread_join(continuous_move_timer_thread, NULL);
            pthread_mutex_lock(&ptz_lock);
            continuous_move_timer_thread = 0;
        }
    }
    
    /* Handle negative speeds by using absolute value and proper direction */
    int pan_speed = simple_abs(pan_vel);
    int tilt_speed = simple_abs(tilt_vel);
    
    /* Set speed for both axes if movement is requested */
    if (pan_vel != 0) {
        platform_ptz_set_speed(PLATFORM_PTZ_AXIS_PAN, pan_speed);
    }
    if (tilt_vel != 0) {
        platform_ptz_set_speed(PLATFORM_PTZ_AXIS_TILT, tilt_speed);
    }
    
    /* Start movement in appropriate directions */
    if (pan_vel != 0) {
        platform_ptz_direction_t dir = (pan_vel > 0) ? PLATFORM_PTZ_DIRECTION_RIGHT : PLATFORM_PTZ_DIRECTION_LEFT;
        platform_ptz_turn(dir, PTZ_MAX_PAN_DEGREES); /* Large number for continuous movement */
    }
    
    if (tilt_vel != 0) {
        platform_ptz_direction_t dir = (tilt_vel > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
        platform_ptz_turn(dir, PTZ_MAX_TILT_DEGREES); /* Large number for continuous movement */
    }
    
    /* Start timeout timer if timeout is specified and > 0 */
    if (timeout_s > 0) {
        continuous_move_timeout_s = timeout_s;
        continuous_move_active = 1;
        
        if (pthread_create(&continuous_move_timer_thread, NULL, continuous_move_timeout_thread, NULL) != 0) {
            platform_log_error("Failed to create continuous move timeout thread\n");
            continuous_move_active = 0;
            pthread_mutex_unlock(&ptz_lock);
            return ONVIF_ERROR;
        }
        
        platform_log_info("PTZ continuous move started with %ds timeout\n", timeout_s);
    } else {
        platform_log_info("PTZ continuous move started (no timeout)\n");
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ONVIF_SUCCESS;
}

int ptz_adapter_stop(void) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    platform_log_info("PTZ stop all movement\n");
    
    /* Stop all directions of movement */
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
    
    /* Clear continuous move state */
    continuous_move_active = 0;
    
    /* Wait for timeout thread to finish if it exists */
    if (continuous_move_timer_thread != 0) {
        pthread_t timer_thread = continuous_move_timer_thread;
        continuous_move_timer_thread = 0;
        pthread_mutex_unlock(&ptz_lock);
        pthread_join(timer_thread, NULL);
        pthread_mutex_lock(&ptz_lock);
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ONVIF_SUCCESS;
}

int ptz_adapter_set_preset(const char *name, int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    platform_log_info("PTZ set preset %s (id=%d) at pan=%d, tilt=%d\n", 
           name ? name : "unnamed", id, current_pan_pos, current_tilt_pos);
    
    /* For now, just store current position - could be enhanced to save to file */
    pthread_mutex_unlock(&ptz_lock);
    return ONVIF_SUCCESS;   /* Basic implementation */
}

int ptz_adapter_goto_preset(int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    platform_log_info("PTZ goto preset id=%d\n", id);
    
    /* Basic implementation - could be enhanced to load from saved presets */
    int ret = -1;
    switch (id) {
        case 1: /* Home position */
            ret = platform_ptz_move_to_position(0, 0);
            if (ret == 0) {
                current_pan_pos = 0;
                current_tilt_pos = 0;
            }
            break;
        default:
            platform_log_info("Preset %d not implemented\n", id);
            break;
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

/* Error handler implementations */
static int handle_ptz_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("PTZ validation failed: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}

static int handle_ptz_system_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("PTZ system error: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}
