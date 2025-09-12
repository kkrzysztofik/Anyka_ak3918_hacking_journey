/**
 * @file onvif_ptz.h
 * @brief ONVIF PTZ service structures & API functions.
 */
#ifndef ONVIF_PTZ_H
#define ONVIF_PTZ_H

#include "../common/onvif_types.h"
#include "../common/onvif_request.h"

enum ptz_move_status {
    PTZ_MOVE_IDLE = 0,
    PTZ_MOVE_MOVING = 1,
    PTZ_MOVE_UNKNOWN = 2
};

struct ptz_vector_2d {
    float x;
    float y;
};

struct ptz_vector {
    struct ptz_vector_2d pan_tilt;
    float zoom;
    char space[128];
};

struct ptz_speed {
    struct ptz_vector_2d pan_tilt;
    float zoom;
};

struct ptz_status {
    struct ptz_vector position;
    struct {
        enum ptz_move_status pan_tilt;
        enum ptz_move_status zoom;
    } move_status;
    char error[256];
    char utc_time[32];
};

struct ptz_space {
    char uri[128];
    struct {
        float min;
        float max;
    } x_range;
    struct {
        float min;
        float max;
    } y_range;
};

struct ptz_node {
    char token[32];
    char name[64];
    struct {
        struct ptz_space absolute_pan_tilt_position_space;
        struct ptz_space absolute_zoom_position_space;
        struct ptz_space relative_pan_tilt_translation_space;
        struct ptz_space relative_zoom_translation_space;
        struct ptz_space continuous_pan_tilt_velocity_space;
        struct ptz_space continuous_zoom_velocity_space;
    } supported_ptz_spaces;
    int maximum_number_of_presets;
    int home_supported;
    char auxiliary_commands[256];
};

struct ptz_configuration_ex {
    char token[32];
    char name[64];
    int use_count;
    char node_token[32];
    struct ptz_space default_absolute_pan_tilt_position_space;
    struct ptz_space default_absolute_zoom_position_space;
    struct ptz_space default_relative_pan_tilt_translation_space;
    struct ptz_space default_relative_zoom_translation_space;
    struct ptz_space default_continuous_pan_tilt_velocity_space;
    struct ptz_space default_continuous_zoom_velocity_space;
    struct ptz_speed default_ptz_speed;
    int default_ptz_timeout;
    struct {
        struct ptz_space range;
    } pan_tilt_limits;
    struct {
        struct ptz_space range;
    } zoom_limits;
};

struct ptz_preset {
    char token[64];
    char name[64];
    struct ptz_vector ptz_position;
};

int onvif_ptz_get_node(const char *node_token, struct ptz_node *node); /**< Retrieve PTZ node capabilities. */
int onvif_ptz_get_configuration(const char *config_token, struct ptz_configuration_ex *config); /**< Fetch PTZ configuration. */
int onvif_ptz_get_status(const char *profile_token, struct ptz_status *status); /**< Get current position & status. */
int onvif_ptz_absolute_move(const char *profile_token, const struct ptz_vector *position, const struct ptz_speed *speed); /**< AbsoluteMove. */
int onvif_ptz_relative_move(const char *profile_token, const struct ptz_vector *translation, const struct ptz_speed *speed); /**< RelativeMove. */
int onvif_ptz_continuous_move(const char *profile_token, const struct ptz_speed *velocity, int timeout); /**< ContinuousMove. */
int onvif_ptz_stop(const char *profile_token, int pan_tilt, int zoom); /**< Stop motion axes. */
int onvif_ptz_goto_home_position(const char *profile_token, const struct ptz_speed *speed); /**< GotoHomePosition. */
int onvif_ptz_set_home_position(const char *profile_token); /**< SetHomePosition. */
int onvif_ptz_get_presets(const char *profile_token, struct ptz_preset **preset_list, int *count); /**< GetPresets list. */
int onvif_ptz_set_preset(const char *profile_token, const char *preset_name, char *preset_token, size_t token_size); /**< SetPreset. */
int onvif_ptz_goto_preset(const char *profile_token, const char *preset_token, const struct ptz_speed *speed); /**< GotoPreset. */
int onvif_ptz_remove_preset(const char *profile_token, const char *preset_token); /**< RemovePreset. */
int onvif_ptz_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response); /**< Handle ONVIF PTZ service requests. */

/**
 * @struct ptz_device_status
 * @brief Current device PTZ position & speed in degrees / speed units.
 * This is the low-level hardware abstraction used internally.
 */
struct ptz_device_status {
    int h_pos_deg; /**< Horizontal position (degrees). */
    int v_pos_deg; /**< Vertical position (degrees). */
    int h_speed;   /**< Current horizontal speed. */
    int v_speed;   /**< Current vertical speed. */
};

/* Low-level PTZ hardware abstraction functions (formerly ptz_adapter) */
int ptz_adapter_init(void); /**< Initialize underlying PTZ hardware or control channel. */
int ptz_adapter_shutdown(void); /**< Shutdown / release PTZ hardware resources. */
int ptz_adapter_get_status(struct ptz_device_status *status); /**< Retrieve current PTZ absolute position & speed. */
int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed); /**< Move to absolute pan/tilt with speed. */
int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed); /**< Move relative delta in pan/tilt. */
int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s); /**< Start continuous velocity move (timeout_s seconds, 0 = indefinite). */
int ptz_adapter_stop(void); /**< Stop any motion (pan & tilt). */
int ptz_adapter_set_preset(const char *name, int id); /**< Store current position as preset id with optional name. */
int ptz_adapter_goto_preset(int id); /**< Move to a previously stored preset id. */

#endif
