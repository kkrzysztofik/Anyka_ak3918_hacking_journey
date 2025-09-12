/*
 * onvif_ptz.c - ONVIF PTZ service implementation
 * 
 * This file implements the ONVIF PTZ Web Service endpoints including
 * PTZ movement, presets, and status operations.
 */

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>
#include <time.h>
#include <pthread.h>
#include "onvif_ptz.h"
#include "platform/platform.h"
#include "utils/xml_utils.h"
#include "utils/logging_utils.h"
#include "utils/error_handling.h"
#include "utils/soap_helpers.h"
#include "utils/response_helpers.h"
#include "utils/service_handler.h"
#include "common/onvif_types.h"

/* PTZ Node configuration */
static struct ptz_node ptz_node = {
    .token = "PTZNode0",
    .name = "PTZ Node",
    .supported_ptz_spaces = {
        .absolute_pan_tilt_position_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
            .x_range = {-180.0, 180.0},
            .y_range = {-90.0, 90.0}
        },
        .absolute_zoom_position_space = {
            .uri = "",
            .x_range = {0.0, 0.0},
            .y_range = {0.0, 0.0}
        },
        .relative_pan_tilt_translation_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace",
            .x_range = {-180.0, 180.0},
            .y_range = {-90.0, 90.0}
        },
        .relative_zoom_translation_space = {
            .uri = "",
            .x_range = {0.0, 0.0},
            .y_range = {0.0, 0.0}
        },
        .continuous_pan_tilt_velocity_space = {
            .uri = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace",
            .x_range = {-1.0, 1.0},
            .y_range = {-1.0, 1.0}
        },
        .continuous_zoom_velocity_space = {
            .uri = "",
            .x_range = {0.0, 0.0},
            .y_range = {0.0, 0.0}
        }
    },
    .maximum_number_of_presets = 10,
    .home_supported = 1,
    .auxiliary_commands = {0}  /* No auxiliary commands */
};

/* Static preset storage */
static struct ptz_preset presets[10];
static int preset_count = 0;

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
    strncpy(config->token, "PTZConfig0", sizeof(config->token) - 1);
    strncpy(config->name, "PTZ Configuration", sizeof(config->name) - 1);
    config->use_count = 1;
    strncpy(config->node_token, ptz_node.token, sizeof(config->node_token) - 1);
    
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
    strncpy(config->pan_tilt_limits.range.uri, ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri, 
            sizeof(config->pan_tilt_limits.range.uri) - 1);
    
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
    
    strncpy(status->position.space, ptz_node.supported_ptz_spaces.absolute_pan_tilt_position_space.uri,
            sizeof(status->position.space) - 1);
    
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
    
    int move_speed = 50;  /* Default speed */
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
    
    int move_speed = 50;  /* Default speed */
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
    
    if (preset_count >= 10) {
        return ONVIF_ERROR;  /* Maximum presets reached */
    }
    
    /* Get current position */
    struct ptz_status status;
    if (onvif_ptz_get_status(profile_token, &status) != 0) {
        return ONVIF_ERROR;
    }
    
    /* Create new preset */
    struct ptz_preset *preset = &presets[preset_count];
    snprintf(preset->token, sizeof(preset->token), "Preset%d", preset_count + 1);
    strncpy(preset->name, preset_name, sizeof(preset->name) - 1);
    preset->ptz_position = status.position;
    
    /* Call adapter to persist preset */
    ptz_adapter_set_preset(preset_name, preset_count + 1);
    
    preset_count++;
    
    if (preset_token) {
        strncpy(preset_token, preset->token, 64);
    }
    
    return ONVIF_SUCCESS;
}

int onvif_ptz_remove_preset(const char *profile_token, const char *preset_token) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_token);
    
    /* Find and remove preset */
    for (int i = 0; i < preset_count; i++) {
        if (strcmp(presets[i].token, preset_token) == 0) {
            /* Shift remaining presets */
            for (int j = i; j < preset_count - 1; j++) {
                presets[j] = presets[j + 1];
            }
            preset_count--;
            return ONVIF_SUCCESS;
        }
    }
    
    return ONVIF_ERROR;
}

int onvif_ptz_goto_preset(const char *profile_token, const char *preset_token,
                         const struct ptz_speed *speed) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(preset_token);
    
    /* Find preset */
    struct ptz_preset *preset = NULL;
    for (int i = 0; i < preset_count; i++) {
        if (strcmp(presets[i].token, preset_token) == 0) {
            preset = &presets[i];
            break;
        }
    }
    
    ONVIF_CHECK_NULL(preset);
    
    /* Move to preset position */
    return onvif_ptz_absolute_move(profile_token, &preset->ptz_position, speed);
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */


/**
 * @brief Internal PTZ service handler
 */
static int ptz_service_handler(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    // Initialize response structure
    response->status_code = 200;
    response->content_type = "application/soap+xml";
    response->body = malloc(4096);
    if (!response->body) {
        return ONVIF_ERROR;
    }
    response->body_length = 0;
    
    switch (action) {
        case ONVIF_ACTION_GET_STATUS: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            
            if (profile_token) {
                struct ptz_status status;
                if (onvif_ptz_get_status(profile_token, &status) == 0) {
                    char status_xml[1024];
                    snprintf(status_xml, sizeof(status_xml),
                        "<tptz:PTZStatus>\n"
                        "  <tt:Position>\n"
                        "    <tt:PanTilt x=\"%.3f\" y=\"%.3f\" space=\"%s\" />\n"
                        "    <tt:Zoom x=\"%.3f\" space=\"%s\" />\n"
                        "  </tt:Position>\n"
                        "  <tt:MoveStatus>\n"
                        "    <tt:PanTilt>%s</tt:PanTilt>\n"
                        "    <tt:Zoom>%s</tt:Zoom>\n"
                        "  </tt:MoveStatus>\n"
                        "  <tt:Error>%s</tt:Error>\n"
                        "  <tt:UtcTime>%s</tt:UtcTime>\n"
                        "</tptz:PTZStatus>",
                        status.position.pan_tilt.x, status.position.pan_tilt.y, status.position.space,
                        status.position.zoom, status.position.space,
                        status.move_status.pan_tilt == PTZ_MOVE_MOVING ? "MOVING" : "IDLE",
                        status.move_status.zoom == PTZ_MOVE_MOVING ? "MOVING" : "IDLE",
                        status.error, status.utc_time);
                    
                    onvif_response_ptz_success(response, "GetStatus", status_xml);
                } else {
                    onvif_response_soap_fault(response, "soap:Receiver", "Failed to get PTZ status");
                }
            } else {
                onvif_handle_missing_parameter(response, "ProfileToken");
            }
            
            if (profile_token) free(profile_token);
            break;
        }
        
        case ONVIF_ACTION_ABSOLUTE_MOVE: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *x_str = xml_extract_value(request->body, "<tt:PanTilt><tt:x>", "</tt:x></tt:PanTilt>");
            char *y_str = xml_extract_value(request->body, "<tt:PanTilt><tt:y>", "</tt:y></tt:PanTilt>");
            
            if (profile_token && x_str && y_str) {
                struct ptz_vector position;
                position.pan_tilt.x = atof(x_str);
                position.pan_tilt.y = atof(y_str);
                position.zoom = 0.0;
                strcpy(position.space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace");
                
                if (onvif_ptz_absolute_move(profile_token, &position, NULL) == 0) {
                    onvif_response_ptz_success(response, "AbsoluteMove", "");
                } else {
                    onvif_response_soap_fault(response, "soap:Receiver", "Failed to execute absolute move");
                }
            } else {
                onvif_handle_missing_parameter(response, "required parameters");
            }
            
            if (profile_token) free(profile_token);
            if (x_str) free(x_str);
            if (y_str) free(y_str);
            break;
        }
        
        case ONVIF_ACTION_RELATIVE_MOVE: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *x_str = xml_extract_value(request->body, "<tt:Translation><tt:PanTilt><tt:x>", "</tt:x></tt:PanTilt></tt:Translation>");
            char *y_str = xml_extract_value(request->body, "<tt:Translation><tt:PanTilt><tt:y>", "</tt:y></tt:PanTilt></tt:Translation>");
            
            if (profile_token && x_str && y_str) {
                struct ptz_vector translation;
                translation.pan_tilt.x = atof(x_str);
                translation.pan_tilt.y = atof(y_str);
                translation.zoom = 0.0;
                strcpy(translation.space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace");
                
                if (onvif_ptz_relative_move(profile_token, &translation, NULL) == 0) {
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:RelativeMoveResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "    </tptz:RelativeMoveResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>");
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to execute relative move");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing required parameters");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            if (x_str) free(x_str);
            if (y_str) free(y_str);
            break;
        }
        
        case ONVIF_ACTION_CONTINUOUS_MOVE: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *x_str = xml_extract_value(request->body, "<tt:Velocity><tt:PanTilt><tt:x>", "</tt:x></tt:PanTilt></tt:Velocity>");
            char *y_str = xml_extract_value(request->body, "<tt:Velocity><tt:PanTilt><tt:y>", "</tt:y></tt:PanTilt></tt:Velocity>");
            char *timeout_str = xml_extract_value(request->body, "<tptz:Timeout>", "</tptz:Timeout>");
            
            if (profile_token && x_str && y_str) {
                struct ptz_speed velocity;
                velocity.pan_tilt.x = atof(x_str);
                velocity.pan_tilt.y = atof(y_str);
                velocity.zoom = 0.0;
                
                int timeout = timeout_str ? atoi(timeout_str) : 10000; // Default 10 seconds
                
                if (onvif_ptz_continuous_move(profile_token, &velocity, timeout) == 0) {
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:ContinuousMoveResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "    </tptz:ContinuousMoveResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>");
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to execute continuous move");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing required parameters");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            if (x_str) free(x_str);
            if (y_str) free(y_str);
            if (timeout_str) free(timeout_str);
            break;
        }
        
        case ONVIF_ACTION_STOP: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *pan_tilt_str = xml_extract_value(request->body, "<tptz:PanTilt>", "</tptz:PanTilt>");
            char *zoom_str = xml_extract_value(request->body, "<tptz:Zoom>", "</tptz:Zoom>");
            
            if (profile_token) {
                int pan_tilt = pan_tilt_str ? 1 : 0;
                int zoom = zoom_str ? 1 : 0;
                
                if (onvif_ptz_stop(profile_token, pan_tilt, zoom) == 0) {
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:StopResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "    </tptz:StopResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>");
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to stop PTZ");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing ProfileToken");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            if (pan_tilt_str) free(pan_tilt_str);
            if (zoom_str) free(zoom_str);
            break;
        }
        
        case ONVIF_ACTION_GET_PRESETS: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            
            if (profile_token) {
                struct ptz_preset *presets = NULL;
                int count = 0;
                if (onvif_ptz_get_presets(profile_token, &presets, &count) == 0) {
                    char presets_xml[2048];
                    strcpy(presets_xml, "<tptz:Preset>\n");
                    
                    for (int i = 0; i < count; i++) {
                        char preset_xml[256];
                        snprintf(preset_xml, sizeof(preset_xml),
                            "  <tt:Preset token=\"%s\">\n"
                            "    <tt:Name>%s</tt:Name>\n"
                            "    <tt:PTZPosition>\n"
                            "      <tt:PanTilt x=\"%.3f\" y=\"%.3f\" space=\"%s\" />\n"
                            "      <tt:Zoom x=\"%.3f\" space=\"%s\" />\n"
                            "    </tt:PTZPosition>\n"
                            "  </tt:Preset>\n",
                            presets[i].token, presets[i].name,
                            presets[i].ptz_position.pan_tilt.x, presets[i].ptz_position.pan_tilt.y,
                            presets[i].ptz_position.space,
                            presets[i].ptz_position.zoom, presets[i].ptz_position.space);
                        strcat(presets_xml, preset_xml);
                    }
                    strcat(presets_xml, "</tptz:Preset>");
                    
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:GetPresetsResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "      %s\n"
                        "    </tptz:GetPresetsResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>", presets_xml);
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get presets");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing ProfileToken");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            break;
        }
        
        case ONVIF_ACTION_SET_PRESET: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *preset_name = xml_extract_value(request->body, "<tptz:PresetName>", "</tptz:PresetName>");
            
            if (profile_token && preset_name) {
                char preset_token[64];
                if (onvif_ptz_set_preset(profile_token, preset_name, preset_token, sizeof(preset_token)) == 0) {
                    char preset_xml[256];
                    snprintf(preset_xml, sizeof(preset_xml),
                        "<tptz:PresetToken>%s</tptz:PresetToken>", preset_token);
                    
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:SetPresetResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "      %s\n"
                        "    </tptz:SetPresetResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>", preset_xml);
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to set preset");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing required parameters");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            if (preset_name) free(preset_name);
            break;
        }
        
        case ONVIF_ACTION_GOTO_PRESET: {
            char *profile_token = xml_extract_value(request->body, "<tptz:ProfileToken>", "</tptz:ProfileToken>");
            char *preset_token = xml_extract_value(request->body, "<tptz:PresetToken>", "</tptz:PresetToken>");
            
            if (profile_token && preset_token) {
                if (onvif_ptz_goto_preset(profile_token, preset_token, NULL) == 0) {
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:GotoPresetResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "    </tptz:GotoPresetResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>");
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to goto preset");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing required parameters");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            if (preset_token) free(preset_token);
            break;
        }
        
        default:
            onvif_handle_unsupported_action(response);
            break;
    }
    
    return response->body_length;
}

/**
 * @brief Handle ONVIF PTZ service requests
 */
int onvif_ptz_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    return onvif_handle_service_request(action, request, response, ptz_service_handler);
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
            platform_ptz_set_degree(350, 130);
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

int ptz_adapter_get_status(struct ptz_device_status *status) {
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
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return ONVIF_ERROR;
    }
    
    /* Clamp values to safe ranges - based on akipc implementation */
    if (pan_deg > 350) pan_deg = 350;
    if (pan_deg < -350) pan_deg = -350;
    if (tilt_deg > 130) tilt_deg = 130;
    if (tilt_deg < -130) tilt_deg = -130;
    
    platform_log_info("PTZ absolute move to pan=%d, tilt=%d\n", pan_deg, tilt_deg);
    
    int ret = platform_ptz_move_to_position(pan_deg, tilt_deg);
    if (ret == 0) {
        current_pan_pos = pan_deg;
        current_tilt_pos = tilt_deg;
        
        /* Wait for movement to complete with timeout */
        platform_ptz_status_t h_status, v_status;
        uint32_t timeout_ms = 5000; /* 5 second timeout */
        uint32_t elapsed_ms = 0;
        uint32_t check_interval_ms = 5; /* Check every 5ms */
        
        do {
            platform_sleep_us(check_interval_ms * 1000); /* Convert to microseconds */
            elapsed_ms += check_interval_ms;
            
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_PAN, &h_status);
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_TILT, &v_status);
            
            if (elapsed_ms >= timeout_ms) {
                platform_log_error("PTZ absolute move timeout after %dms\n", timeout_ms);
                ret = ONVIF_ERROR_TIMEOUT;
                break;
            }
        } while ((h_status != PLATFORM_PTZ_STATUS_OK) || (v_status != PLATFORM_PTZ_STATUS_OK));
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
        if (steps > 16) steps = 16; /* Limit step size like in akipc */
        
        ret = platform_ptz_turn(dir, steps);
        if (ret == 0) {
            current_pan_pos += (dir == PLATFORM_PTZ_DIRECTION_LEFT) ? steps : -steps;
        }
    }
    
    /* Vertical movement - based on akipc implementation with step size 8 */
    if (tilt_delta_deg != 0) {
        platform_ptz_direction_t dir = (tilt_delta_deg > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
        int steps = simple_abs(tilt_delta_deg);
        if (steps > 8) steps = 8; /* Limit step size like in akipc */
        
        ret |= platform_ptz_turn(dir, steps);
        if (ret == 0) current_tilt_pos += (dir == PLATFORM_PTZ_DIRECTION_DOWN) ? steps : -steps;
    }
    
    /* Wait for movement to complete with timeout */
    if (ret == 0) {
        uint32_t timeout_ms = 5000; /* 5 second timeout */
        uint32_t elapsed_ms = 0;
        uint32_t check_interval_ms = 5; /* Check every 5ms */
        
        do {
            platform_sleep_us(check_interval_ms * 1000); /* Convert to microseconds */
            elapsed_ms += check_interval_ms;
            
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_PAN, &h_status);
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_TILT, &v_status);
            
            if (elapsed_ms >= timeout_ms) {
                platform_log_error("PTZ relative move timeout after %dms\n", timeout_ms);
                ret = ONVIF_ERROR_TIMEOUT;
                break;
            }
        } while ((h_status != PLATFORM_PTZ_STATUS_OK) || (v_status != PLATFORM_PTZ_STATUS_OK));
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
        platform_ptz_turn(dir, 360); /* Large number for continuous movement */
    }
    
    if (tilt_vel != 0) {
        platform_ptz_direction_t dir = (tilt_vel > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
        platform_ptz_turn(dir, 180); /* Large number for continuous movement */
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
