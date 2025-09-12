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
#include "onvif_ptz.h"
#include "platform.h"
#include "../server/http/http_parser.h"
#include "../utils/xml_utils.h"
#include "../utils/logging_utils.h"
#include "../common/onvif_types.h"

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
    if (!nodes || !count) return -1;
    
    *nodes = &ptz_node;
    *count = 1;
    return 0;
}

int onvif_ptz_get_node(const char *node_token, struct ptz_node *node) {
    if (!node_token || !node) return -1;
    
    if (strcmp(node_token, ptz_node.token) == 0) {
        *node = ptz_node;
        return 0;
    }
    
    return -1;
}

int onvif_ptz_get_configuration(const char *config_token, struct ptz_configuration_ex *config) {
    if (!config_token || !config) return -1;
    
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
    
    return 0;
}

int onvif_ptz_get_status(const char *profile_token, struct ptz_status *status) {
    if (!profile_token || !status) return -1;
    
    struct ptz_device_status adapter_status;
    if (ptz_adapter_get_status(&adapter_status) != 0) {
        return -1;
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
    
    return 0;
}

int onvif_ptz_absolute_move(const char *profile_token, const struct ptz_vector *position, 
                           const struct ptz_speed *speed) {
    if (!profile_token || !position) return -1;
    
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
    if (!profile_token || !translation) return -1;
    
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
    if (!profile_token || !velocity) return -1;
    
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
    if (!profile_token) return -1;
    
    if (pan_tilt) {
        return ptz_adapter_stop();
    }
    
    return 0;  /* Zoom stop not needed (no zoom support) */
}

int onvif_ptz_goto_home_position(const char *profile_token, const struct ptz_speed *speed) {
    if (!profile_token) return -1;
    
    /* Home position is center (0, 0) in normalized coordinates */
    struct ptz_vector home_position;
    home_position.pan_tilt.x = 0.0;
    home_position.pan_tilt.y = 0.0;
    home_position.zoom = 0.0;
    
    return onvif_ptz_absolute_move(profile_token, &home_position, speed);
}

int onvif_ptz_set_home_position(const char *profile_token) {
    if (!profile_token) return -1;
    
    /* TODO: Implement home position setting */
    printf("SetHomePosition for profile %s (not implemented)\n", profile_token);
    return 0;
}

int onvif_ptz_get_presets(const char *profile_token, struct ptz_preset **preset_list, int *count) {
    if (!profile_token || !preset_list || !count) return -1;
    
    *preset_list = presets;
    *count = preset_count;
    return 0;
}

int onvif_ptz_set_preset(const char *profile_token, const char *preset_name, char *preset_token, size_t token_size) {
    if (!profile_token || !preset_name || !preset_token) return -1;
    
    if (preset_count >= 10) {
        return -1;  /* Maximum presets reached */
    }
    
    /* Get current position */
    struct ptz_status status;
    if (onvif_ptz_get_status(profile_token, &status) != 0) {
        return -1;
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
    
    return 0;
}

int onvif_ptz_remove_preset(const char *profile_token, const char *preset_token) {
    if (!profile_token || !preset_token) return -1;
    
    /* Find and remove preset */
    for (int i = 0; i < preset_count; i++) {
        if (strcmp(presets[i].token, preset_token) == 0) {
            /* Shift remaining presets */
            for (int j = i; j < preset_count - 1; j++) {
                presets[j] = presets[j + 1];
            }
            preset_count--;
            return 0;
        }
    }
    
    return -1;
}

int onvif_ptz_goto_preset(const char *profile_token, const char *preset_token,
                         const struct ptz_speed *speed) {
    if (!profile_token || !preset_token) return -1;
    
    /* Find preset */
    struct ptz_preset *preset = NULL;
    for (int i = 0; i < preset_count; i++) {
        if (strcmp(presets[i].token, preset_token) == 0) {
            preset = &presets[i];
            break;
        }
    }
    
    if (!preset) return -1;
    
    /* Move to preset position */
    return onvif_ptz_absolute_move(profile_token, &preset->ptz_position, speed);
}

/* SOAP XML generation helpers */
static void soap_fault_response(char *response, size_t response_size, const char *fault_code, const char *fault_string) {
    snprintf(response, response_size, 
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <soap:Fault>\n"
        "      <soap:Code>\n"
        "        <soap:Value>%s</soap:Value>\n"
        "      </soap:Code>\n"
        "      <soap:Reason>\n"
        "        <soap:Text>%s</soap:Text>\n"
        "      </soap:Reason>\n"
        "    </soap:Fault>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", fault_code, fault_string);
}

static void soap_success_response(char *response, size_t response_size, const char *action, const char *body_content) {
    snprintf(response, response_size,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <tptz:%sResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
        "      %s\n"
        "    </tptz:%sResponse>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", action, body_content, action);
}

/* XML parsing helpers - now using xml_utils module */


/**
 * @brief Handle ONVIF PTZ service requests
 */
int onvif_ptz_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!request || !response) {
        return -1;
    }
    
    // Initialize response structure
    response->status_code = 200;
    response->content_type = "application/soap+xml";
    response->body = malloc(4096);
    if (!response->body) {
        return -1;
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
                    
                    snprintf(response->body, 4096, 
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:GetStatusResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "      %s\n"
                        "    </tptz:GetStatusResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>", status_xml);
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get PTZ status");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing ProfileToken");
                response->body_length = strlen(response->body);
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
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <tptz:AbsoluteMoveResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
                        "    </tptz:AbsoluteMoveResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>");
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to execute absolute move");
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
            soap_fault_response(response->body, 4096, "soap:Receiver", "Unsupported action");
            response->body_length = strlen(response->body);
            break;
    }
    
    return response->body_length;
}

/* ============================================================================
 * Low-level PTZ hardware abstraction functions (formerly ptz_adapter)
 * ============================================================================ */

static pthread_mutex_t ptz_lock = PTHREAD_MUTEX_INITIALIZER;
static int ptz_initialized = 0;
static int current_pan_pos = 0;
static int current_tilt_pos = 0;

static int simple_abs(int value) { return (value < 0) ? -value : value; }

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
            printf("PTZ adapter initialized successfully\n");
        } else {
            fprintf(stderr, "ak_drv_ptz_open failed: %d\n", ret);
        }
    }
    pthread_mutex_unlock(&ptz_lock);
    return ptz_initialized ? 0 : -1;
}

int ptz_adapter_shutdown(void) {
    pthread_mutex_lock(&ptz_lock);
    if (ptz_initialized) {
        platform_ptz_cleanup();
        ptz_initialized = 0;
    }
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_get_status(struct ptz_device_status *status) {
    if (!status) return -1;
    
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    int h = platform_ptz_get_step_position(PLATFORM_PTZ_AXIS_PAN);
    int v = platform_ptz_get_step_position(PLATFORM_PTZ_AXIS_TILT);
    
    status->h_pos_deg = h;
    status->v_pos_deg = v;
    status->h_speed = 0;
    status->v_speed = 0;
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    /* Clamp values to safe ranges - based on akipc implementation */
    if (pan_deg > 350) pan_deg = 350;
    if (pan_deg < -350) pan_deg = -350;
    if (tilt_deg > 130) tilt_deg = 130;
    if (tilt_deg < -130) tilt_deg = -130;
    
    printf("PTZ absolute move to pan=%d, tilt=%d\n", pan_deg, tilt_deg);
    
    int ret = platform_ptz_move_to_position(pan_deg, tilt_deg);
    if (ret == 0) {
        current_pan_pos = pan_deg;
        current_tilt_pos = tilt_deg;
        
        /* Wait for movement to complete */
        platform_ptz_status_t h_status, v_status;
        do {
            platform_sleep_us(5000); /* 5ms delay */
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_PAN, &h_status);
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_TILT, &v_status);
        } while ((h_status != PLATFORM_PTZ_STATUS_OK) || (v_status != PLATFORM_PTZ_STATUS_OK));
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ relative move pan_delta=%d, tilt_delta=%d\n", pan_delta_deg, tilt_delta_deg);
    
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
    
    /* Wait for movement to complete */
    if (ret == 0) {
        do {
            platform_sleep_us(5000); /* 5ms delay */
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_PAN, &h_status);
            platform_ptz_get_status(PLATFORM_PTZ_AXIS_TILT, &v_status);
        } while ((h_status != PLATFORM_PTZ_STATUS_OK) || (v_status != PLATFORM_PTZ_STATUS_OK));
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    if (pan_vel > 0) platform_ptz_set_speed(PLATFORM_PTZ_AXIS_PAN, pan_vel);
    if (tilt_vel > 0) platform_ptz_set_speed(PLATFORM_PTZ_AXIS_TILT, tilt_vel);
    
    if (pan_vel != 0) {
        platform_ptz_direction_t dir = (pan_vel > 0) ? PLATFORM_PTZ_DIRECTION_RIGHT : PLATFORM_PTZ_DIRECTION_LEFT;
        platform_ptz_turn(dir, 360);
    }
    
    if (tilt_vel != 0) {
        platform_ptz_direction_t dir = (tilt_vel > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
        platform_ptz_turn(dir, 180);
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_stop(void) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ stop all movement\n");
    
    /* Stop all directions of movement */
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_set_preset(const char *name, int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ set preset %s (id=%d) at pan=%d, tilt=%d\n", 
           name ? name : "unnamed", id, current_pan_pos, current_tilt_pos);
    
    /* For now, just store current position - could be enhanced to save to file */
    pthread_mutex_unlock(&ptz_lock);
    return 0;   /* Basic implementation */
}

int ptz_adapter_goto_preset(int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ goto preset id=%d\n", id);
    
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
            printf("Preset %d not implemented\n", id);
            break;
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}
