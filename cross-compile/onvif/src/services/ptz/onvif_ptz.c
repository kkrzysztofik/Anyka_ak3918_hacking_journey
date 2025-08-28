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
#include "ptz_adapter.h"

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
    /* Convert from [-1, 1] to [0, 360] */
    return (int)((normalized_value + 1.0) * 180.0);
}

static int normalize_to_degrees_tilt(float normalized_value) {
    /* Convert from [-1, 1] to [0, 180] */
    return (int)((normalized_value + 1.0) * 90.0);
}

/* Convert device degrees to ONVIF normalized coordinates */
static float degrees_to_normalize_pan(int degrees) {
    /* Convert from [0, 360] to [-1, 1] */
    return (degrees / 180.0) - 1.0;
}

static float degrees_to_normalize_tilt(int degrees) {
    /* Convert from [0, 180] to [-1, 1] */
    return (degrees / 90.0) - 1.0;
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
