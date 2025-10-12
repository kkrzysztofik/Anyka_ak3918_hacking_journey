/**
 * @file onvif_ptz.h
 * @brief ONVIF PTZ service structures & API functions
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef ONVIF_PTZ_H
#define ONVIF_PTZ_H

#include "core/config/config.h"
#include "networking/http/http_parser.h"

/* PTZ Constants */
#define PTZ_DEFAULT_PAN_TILT_SPEED 0.5F
#define PTZ_DEFAULT_ZOOM_SPEED     0.0F

/* PTZ Buffer Sizes */
#define PTZ_SPACE_URI_SIZE    128
#define PTZ_ERROR_MSG_SIZE    256
#define PTZ_UTC_TIME_SIZE     32
#define PTZ_TOKEN_SIZE        32
#define PTZ_NAME_SIZE         64
#define PTZ_AUX_COMMANDS_SIZE 256
#define PTZ_PRESET_TOKEN_SIZE 64

enum ptz_move_status { PTZ_MOVE_IDLE = 0, PTZ_MOVE_MOVING = 1, PTZ_MOVE_UNKNOWN = 2 };

struct ptz_vector_2d {
  float x;
  float y;
};

struct ptz_vector {
  struct ptz_vector_2d pan_tilt;
  float zoom;
  char space[PTZ_SPACE_URI_SIZE];
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
  char error[PTZ_ERROR_MSG_SIZE];
  char utc_time[PTZ_UTC_TIME_SIZE];
};

struct ptz_space {
  char uri[PTZ_SPACE_URI_SIZE];
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
  char token[PTZ_TOKEN_SIZE];
  char name[PTZ_NAME_SIZE];
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
  char auxiliary_commands[PTZ_AUX_COMMANDS_SIZE];
};

struct ptz_configuration_ex {
  char token[PTZ_TOKEN_SIZE];
  char name[PTZ_NAME_SIZE];
  int use_count;
  char node_token[PTZ_TOKEN_SIZE];
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
  char token[PTZ_PRESET_TOKEN_SIZE];
  char name[PTZ_NAME_SIZE];
  struct ptz_vector ptz_position;
};

/**
 * @brief PTZ preset list for a single media profile
 * Each profile has its own isolated preset storage (max 4 presets per profile)
 */
typedef struct ptz_preset_list {
  int preset_count;            /**< Number of active presets (0-4) */
  struct ptz_preset presets[4]; /**< Preset array (max 4 per profile) */
} ptz_preset_list_t;

int onvif_ptz_get_nodes(struct ptz_node** nodes, int* count); /**< Get all PTZ nodes. */
int onvif_ptz_get_node(const char* node_token,
                       struct ptz_node* node); /**< Retrieve PTZ node capabilities. */
int onvif_ptz_get_configuration(
  const char* config_token, struct ptz_configuration_ex* config); /**< Fetch PTZ configuration. */
int onvif_ptz_get_status(const char* profile_token,
                         struct ptz_status* status); /**< Get current position & status. */
int onvif_ptz_absolute_move(const char* profile_token, const struct ptz_vector* position,
                            const struct ptz_speed* speed); /**< AbsoluteMove. */
int onvif_ptz_relative_move(const char* profile_token, const struct ptz_vector* translation,
                            const struct ptz_speed* speed); /**< RelativeMove. */
int onvif_ptz_continuous_move(const char* profile_token, const struct ptz_speed* velocity,
                              int timeout);                            /**< ContinuousMove. */
int onvif_ptz_stop(const char* profile_token, int pan_tilt, int zoom); /**< Stop motion axes. */
int onvif_ptz_goto_home_position(const char* profile_token,
                                 const struct ptz_speed* speed); /**< GotoHomePosition. */
int onvif_ptz_set_home_position(const char* profile_token);      /**< SetHomePosition. */
int onvif_ptz_get_presets(const char* profile_token, struct ptz_preset** preset_list,
                          int* count); /**< GetPresets list. */
int onvif_ptz_set_preset(const char* profile_token, const char* preset_name, char* preset_token,
                         size_t token_size); /**< SetPreset. */
int onvif_ptz_goto_preset(const char* profile_token, const char* preset_token,
                          const struct ptz_speed* speed); /**< GotoPreset. */
int onvif_ptz_remove_preset(const char* profile_token,
                            const char* preset_token); /**< RemovePreset. */
int onvif_ptz_handle_operation(
  const char* operation_name, const http_request_t* request,
  http_response_t* response); /**< Handle ONVIF PTZ service operations (standardized). */

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

/* PTZ service functions */

/**
 * @brief Initialize PTZ service
 * @param config Centralized configuration
 * @return 0 on success, negative error code on failure
 */
int onvif_ptz_init(config_manager_t* config);

/**
 * @brief Clean up PTZ service
 */
void onvif_ptz_cleanup(void);

/**
 * @brief Reset PTZ preset state (for testing)
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function is primarily used for test isolation between test cases
 */
int onvif_ptz_reset_presets(void);

/**
 * @brief Handle ONVIF PTZ service requests (HTTP server interface)
 * @param action_name ONVIF action name (e.g., "GetConfigurations")
 * @param request HTTP request containing SOAP envelope
 * @param response HTTP response to populate with SOAP envelope
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function provides the HTTP server interface for PTZ service
 */
int onvif_ptz_handle_request(const char* action_name, const http_request_t* request,
                             http_response_t* response);

#endif
