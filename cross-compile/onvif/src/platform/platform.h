/**
 * @file platform.h
 * @brief Unified platform abstraction layer for ONVIF daemon
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_PLATFORM_H
#define ONVIF_PLATFORM_H

#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "platform_common.h"

/* Platform detection */
#ifdef ANYKA_PLATFORM
#define PLATFORM_ANYKA 1
#else
#define PLATFORM_ANYKA 0
#endif

/* Video encoding constants */
#define PLATFORM_H264_ENC_TYPE  0
#define PLATFORM_HEVC_ENC_TYPE  1
#define PLATFORM_MJPEG_ENC_TYPE 2

/* Video profile constants */
#define PLATFORM_PROFILE_MAIN     0
#define PLATFORM_PROFILE_BASELINE 1
#define PLATFORM_PROFILE_HIGH     2
#define PLATFORM_PROFILE_MAX      3

/* Bitrate mode constants */
#define PLATFORM_BR_MODE_CBR 0
#define PLATFORM_BR_MODE_VBR 1
#define PLATFORM_BR_MODE_MAX 2

/* Frame type constants */
#define PLATFORM_FRAME_TYPE_I 0
#define PLATFORM_FRAME_TYPE_P 1
#define PLATFORM_FRAME_TYPE_B 2

typedef enum {
  PLATFORM_DAYNIGHT_DAY = 0,
  PLATFORM_DAYNIGHT_NIGHT = 1,
  PLATFORM_DAYNIGHT_AUTO = 2
} platform_daynight_mode_t;

/* Additional video resolution type */
typedef struct {
  int width;
  int height;
} platform_video_resolution_t;

/* PTZ status constants */
#define PLATFORM_PTZ_STATUS_OK    0
#define PLATFORM_PTZ_STATUS_BUSY  1
#define PLATFORM_PTZ_STATUS_ERROR 2

/* IR LED constants */
#define PLATFORM_IRLED_OFF  0
#define PLATFORM_IRLED_ON   1
#define PLATFORM_IRLED_AUTO 2

/* Opaque handles - now defined in platform_common.h */

/** @defgroup platform_init Platform Initialization
 * @brief Platform initialization and cleanup functions
 * @{
 */

/**
 * @brief Initialize the platform abstraction layer
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_init(void);

/**
 * @brief Cleanup platform resources and shutdown
 */
void platform_cleanup(void);

/** @} */

/** @defgroup platform_vi Video Input (VI) Functions
 * @brief Video input device management and configuration
 * @{
 */

/**
 * @brief Match sensor configuration
 * @param isp_cfg_path Path to ISP configuration directory
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_match_sensor(const char* isp_cfg_path);

/**
 * @brief Open video input device
 * @param handle Pointer to store video input handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_open(platform_vi_handle_t* handle);

/**
 * @brief Close video input device
 * @param handle Video input handle to close
 */
void platform_vi_close(platform_vi_handle_t handle);

/**
 * @brief Get sensor resolution information
 * @param handle Video input handle
 * @param resolution Pointer to store resolution information
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_get_sensor_resolution(platform_vi_handle_t handle,
                                                    platform_video_resolution_t* resolution);

/**
 * @brief Switch between day and night modes
 * @param handle Video input handle
 * @param mode Day/night mode to switch to
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_switch_day_night(platform_vi_handle_t handle,
                                               platform_daynight_mode_t mode);

/**
 * @brief Set video flip and mirror settings
 * @param handle Video input handle
 * @param flip Enable vertical flip
 * @param mirror Enable horizontal mirror
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_set_flip_mirror(platform_vi_handle_t handle, bool flip, bool mirror);

/**
 * @brief Start video input capture
 * @param handle Video input handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_capture_on(platform_vi_handle_t handle);

/**
 * @brief Start global video capture (called once during platform init)
 * @param handle Video input handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_start_global_capture(platform_vi_handle_t handle);

/**
 * @brief Stop video input capture
 * @param handle Video input handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vi_capture_off(platform_vi_handle_t handle);

/**
 * @brief Set video input channel attributes
 * @param handle Video input handle
 * @param attr Channel attributes to set
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note This must be called after platform_vi_open() and before
 * platform_vi_capture_on()
 */
platform_result_t platform_vi_set_channel_attr(platform_vi_handle_t handle,
                                               const platform_video_channel_attr_t* attr);

/**
 * @brief Get current sensor frame rate
 * @param handle Video input handle
 * @param fps Output parameter for frame rate (frames per second)
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note This returns the actual sensor frame rate, which may differ from
 * encoder frame rate
 */
platform_result_t platform_vi_get_fps(platform_vi_handle_t handle, int* fps);

/** @} */

/** @defgroup platform_vpss Video Processing Subsystem (VPSS) Functions
 * @brief Video processing effects and filters
 * @{
 */

/**
 * @brief Set video processing effect value
 * @param handle Video input handle
 * @param effect Effect type to modify
 * @param value New effect value
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vpss_effect_set(platform_vi_handle_t handle,
                                           platform_vpss_effect_t effect, int value);

/**
 * @brief Get current video processing effect value
 * @param handle Video input handle
 * @param effect Effect type to query
 * @param value Pointer to store current effect value
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_vpss_effect_get(platform_vi_handle_t handle,
                                           platform_vpss_effect_t effect, int* value);

/** @} */

/** @defgroup platform_venc Video Encoder Functions
 * @brief Video encoding and frame management
 * @{
 */

/**
 * @brief Initialize video encoder with configuration
 * @param handle Pointer to store video encoder handle
 * @param config Video encoding configuration
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_venc_init(platform_venc_handle_t* handle,
                                     const platform_video_config_t* config);

/**
 * @brief Validate video encoder configuration parameters
 * @param config Video encoder configuration to validate
 * @return PLATFORM_SUCCESS if valid, error code on failure
 * @note Performs comprehensive validation of all encoder parameters
 */
platform_result_t platform_validate_venc_config(const platform_video_config_t* config);

/**
 * @brief Cleanup video encoder resources
 * @param handle Video encoder handle to cleanup
 */
void platform_venc_cleanup(platform_venc_handle_t handle);

/**
 * @brief Get encoded video frame
 * @param handle Video encoder handle
 * @param data Pointer to store frame data pointer
 * @param size Pointer to store frame size
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_venc_get_frame(platform_venc_handle_t handle, uint8_t** data,
                                          uint32_t* size);

/**
 * @brief Release encoded video frame
 * @param handle Video encoder handle
 * @param data Frame data to release
 */
void platform_venc_release_frame(platform_venc_handle_t handle, uint8_t* data);

/** @} */

/* Video Encoder Stream functions (for RTSP) */
typedef struct {
  uint8_t* data;
  uint32_t len;
  uint32_t timestamp;
  bool is_keyframe;
} platform_venc_stream_t;

platform_result_t platform_venc_get_stream(platform_venc_handle_t handle,
                                           platform_venc_stream_t* stream, uint32_t timeout_ms);
void platform_venc_release_stream(platform_venc_handle_t handle, platform_venc_stream_t* stream);

/**
 * @brief Request a bound encoder stream between VI and VENC
 * @param vi_handle Video input handle
 * @param venc_handle Video encoder handle
 * @param stream_handle Pointer to store requested stream handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_venc_request_stream(platform_vi_handle_t vi_handle,
                                               platform_venc_handle_t venc_handle,
                                               platform_venc_stream_handle_t* stream_handle);

/**
 * @brief Cancel a previously requested encoder stream
 * @param stream_handle Stream handle to cancel
 */
void platform_venc_cancel_stream(platform_venc_stream_handle_t stream_handle);

/**
 * @brief Get encoded video stream using a stream handle
 * @param stream_handle Stream handle
 * @param stream Output stream struct
 * @param timeout_ms Timeout in milliseconds
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_venc_get_stream_by_handle(platform_venc_stream_handle_t stream_handle,
                                                     platform_venc_stream_t* stream,
                                                     uint32_t timeout_ms);

/**
 * @brief Release encoded video stream using a stream handle
 * @param stream_handle Stream handle
 * @param stream Stream struct to release
 */
void platform_venc_release_stream_by_handle(platform_venc_stream_handle_t stream_handle,
                                            platform_venc_stream_t* stream);

/**
 * @brief Get encoder stream buffer status (for debugging)
 * @param stream_handle Stream handle
 * @param buffer_count Output parameter for current buffer count
 * @param max_buffers Output parameter for maximum buffers
 * @param overflow_count Output parameter for overflow count
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note This function provides debugging information about the encoder stream
 * buffer status
 */
platform_result_t platform_venc_get_buffer_status(platform_venc_stream_handle_t stream_handle,
                                                  uint32_t* buffer_count, uint32_t* max_buffers,
                                                  uint32_t* overflow_count);

/* Audio Input (AI) functions */
platform_result_t platform_ai_open(platform_ai_handle_t* handle);
void platform_ai_close(platform_ai_handle_t handle);

/* Audio Encoder functions */
platform_result_t platform_aenc_init(platform_aenc_stream_handle_t* handle,
                                     const platform_audio_config_t* config);
void platform_aenc_cleanup(platform_aenc_stream_handle_t handle);
platform_result_t platform_aenc_get_frame(platform_aenc_stream_handle_t handle, uint8_t** data,
                                          uint32_t* size);
void platform_aenc_release_frame(platform_aenc_stream_handle_t handle, const uint8_t* data);

/* Audio Encoder Stream functions (for RTSP) */
typedef struct {
  uint8_t* data;
  uint32_t len;
  uint32_t timestamp;
} platform_aenc_stream_t;

platform_result_t platform_aenc_get_stream(platform_aenc_stream_handle_t handle,
                                           platform_aenc_stream_t* stream, uint32_t timeout_ms);
void platform_aenc_release_stream(platform_aenc_stream_handle_t handle,
                                  platform_aenc_stream_t* stream);

/* PTZ functions */
platform_result_t platform_ptz_init(void);
void platform_ptz_cleanup(void);
platform_result_t platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg);
platform_result_t platform_ptz_check_self(void);
platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg);
int platform_ptz_get_step_position(platform_ptz_axis_t axis);
platform_result_t platform_ptz_get_status(platform_ptz_axis_t axis, platform_ptz_status_t* status);
platform_result_t platform_ptz_set_speed(platform_ptz_axis_t axis, int speed);
platform_result_t platform_ptz_turn(platform_ptz_direction_t direction, int steps);
platform_result_t platform_ptz_turn_stop(platform_ptz_direction_t direction);

/* IR LED functions */
platform_result_t platform_irled_init(int level);
void platform_irled_cleanup(void);
platform_result_t platform_irled_set_mode(platform_irled_mode_t mode);
platform_result_t platform_irled_get_status(void);

/* Snapshot functions */
typedef struct {
  uint8_t* data;      /**< JPEG data buffer */
  size_t len;         /**< JPEG data length */
  uint64_t timestamp; /**< Capture timestamp */
} platform_snapshot_t;

typedef void* platform_snapshot_handle_t;

platform_result_t platform_snapshot_init(platform_snapshot_handle_t* handle,
                                         platform_vi_handle_t vi_handle, int width, int height);
void platform_snapshot_cleanup(platform_snapshot_handle_t handle);
platform_result_t platform_snapshot_capture(platform_snapshot_handle_t handle,
                                            platform_snapshot_t* snapshot, uint32_t timeout_ms);
void platform_snapshot_release(platform_snapshot_handle_t handle, platform_snapshot_t* snapshot);

/* Configuration functions */
platform_result_t platform_config_load(const char* filename);
platform_result_t platform_config_save(const char* filename);
const char* platform_config_get_string(const char* section, const char* key,
                                       const char* default_value);
int platform_config_get_int(const char* section, const char* key, int default_value);

/* Include enhanced logging utilities */

/* Legacy logging functions - now use enhanced logging internally */
int platform_log_error(const char* format, ...);
int platform_log_warning(const char* format, ...);
int platform_log_notice(const char* format, ...);
int platform_log_info(const char* format, ...);
int platform_log_debug(const char* format, ...);

/* System monitoring functions */
typedef struct {
  float cpu_usage;       /**< CPU usage percentage (0-100) */
  float cpu_temperature; /**< CPU temperature in Celsius */
  uint64_t total_memory; /**< Total system memory in bytes */
  uint64_t free_memory;  /**< Free system memory in bytes */
  uint64_t uptime_ms;    /**< System uptime in milliseconds */
} platform_system_info_t;

/**
 * @brief Get current system utilization information
 * @param info Pointer to store system information
 * @return PLATFORM_SUCCESS on success, error code on failure
 */
platform_result_t platform_get_system_info(platform_system_info_t* info);

#endif /* ONVIF_PLATFORM_H */
