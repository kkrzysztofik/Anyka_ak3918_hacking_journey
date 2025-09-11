/**
 * @file platform.h
 * @brief Unified platform abstraction layer for ONVIF daemon.
 * 
 * This file consolidates the HAL (Hardware Abstraction Layer) and platform
 * abstraction functionality into a single, cohesive interface that provides
 * hardware abstraction while maintaining platform independence.
 */

#ifndef ONVIF_PLATFORM_H
#define ONVIF_PLATFORM_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Platform detection */
#ifdef ANYKA_PLATFORM
#define PLATFORM_ANYKA 1
#else
#define PLATFORM_ANYKA 0
#endif

/* Common result codes */
typedef enum {
    PLATFORM_SUCCESS = 0,
    PLATFORM_ERROR = -1,
    PLATFORM_ERROR_NULL = -2,
    PLATFORM_ERROR_INVALID = -3,
    PLATFORM_ERROR_MEMORY = -4,
    PLATFORM_ERROR_IO = -5,
    PLATFORM_ERROR_NOT_SUPPORTED = -6,
    PLATFORM_ERROR_BUSY = -7,
    PLATFORM_ERROR_TIMEOUT = -8
} platform_result_t;

/* Video types */
typedef enum {
    PLATFORM_VIDEO_CODEC_H264 = 0,
    PLATFORM_VIDEO_CODEC_H265 = 1,
    PLATFORM_VIDEO_CODEC_MJPEG = 2
} platform_video_codec_t;

typedef enum {
    PLATFORM_DAYNIGHT_DAY = 0,
    PLATFORM_DAYNIGHT_NIGHT = 1,
    PLATFORM_DAYNIGHT_AUTO = 2
} platform_daynight_mode_t;

typedef enum {
    PLATFORM_VPSS_EFFECT_BRIGHTNESS = 0,
    PLATFORM_VPSS_EFFECT_CONTRAST = 1,
    PLATFORM_VPSS_EFFECT_SATURATION = 2,
    PLATFORM_VPSS_EFFECT_SHARPNESS = 3,
    PLATFORM_VPSS_EFFECT_HUE = 4
} platform_vpss_effect_t;

typedef struct {
    uint32_t width;
    uint32_t height;
    uint32_t fps;
    uint32_t bitrate;
    platform_video_codec_t codec;
} platform_video_config_t;

typedef struct {
    int width;
    int height;
} platform_video_resolution_t;

/* Audio types */
typedef enum {
    PLATFORM_AUDIO_CODEC_PCM = 0,
    PLATFORM_AUDIO_CODEC_AAC = 1,
    PLATFORM_AUDIO_CODEC_G711A = 2,
    PLATFORM_AUDIO_CODEC_G711U = 3
} platform_audio_codec_t;

typedef struct {
    uint32_t sample_rate;
    uint32_t channels;
    uint32_t bits_per_sample;
    platform_audio_codec_t codec;
} platform_audio_config_t;

/* PTZ types */
typedef enum {
    PLATFORM_PTZ_AXIS_PAN = 0,
    PLATFORM_PTZ_AXIS_TILT = 1
} platform_ptz_axis_t;

typedef enum {
    PLATFORM_PTZ_STATUS_OK = 0,
    PLATFORM_PTZ_STATUS_BUSY = 1,
    PLATFORM_PTZ_STATUS_ERROR = 2
} platform_ptz_status_t;

typedef enum {
    PLATFORM_PTZ_DIRECTION_LEFT = 0,
    PLATFORM_PTZ_DIRECTION_RIGHT = 1,
    PLATFORM_PTZ_DIRECTION_UP = 2,
    PLATFORM_PTZ_DIRECTION_DOWN = 3
} platform_ptz_direction_t;

/* IR LED types */
typedef enum {
    PLATFORM_IRLED_OFF = 0,
    PLATFORM_IRLED_ON = 1,
    PLATFORM_IRLED_AUTO = 2
} platform_irled_mode_t;

/* Opaque handles */
typedef void* platform_vi_handle_t;
typedef void* platform_venc_handle_t;
typedef void* platform_ai_handle_t;
typedef void* platform_aenc_handle_t;

/* Platform initialization */
platform_result_t platform_init(void);
void platform_cleanup(void);

/* Video Input (VI) functions */
platform_result_t platform_vi_open(platform_vi_handle_t *handle);
void platform_vi_close(platform_vi_handle_t handle);
platform_result_t platform_vi_get_sensor_resolution(platform_vi_handle_t handle, 
                                                   platform_video_resolution_t *resolution);
platform_result_t platform_vi_switch_day_night(platform_vi_handle_t handle, 
                                              platform_daynight_mode_t mode);
platform_result_t platform_vi_set_flip_mirror(platform_vi_handle_t handle, 
                                             bool flip, bool mirror);

/* VPSS (Video Processing Subsystem) functions */
platform_result_t platform_vpss_effect_set(platform_vi_handle_t handle, 
                                          platform_vpss_effect_t effect, int value);
platform_result_t platform_vpss_effect_get(platform_vi_handle_t handle, 
                                          platform_vpss_effect_t effect, int *value);

/* Video Encoder functions */
platform_result_t platform_venc_init(platform_venc_handle_t *handle, 
                                    const platform_video_config_t *config);
void platform_venc_cleanup(platform_venc_handle_t handle);
platform_result_t platform_venc_get_frame(platform_venc_handle_t handle, 
                                         uint8_t **data, uint32_t *size);
void platform_venc_release_frame(platform_venc_handle_t handle, uint8_t *data);

/* Audio Input (AI) functions */
platform_result_t platform_ai_open(platform_ai_handle_t *handle);
void platform_ai_close(platform_ai_handle_t handle);

/* Audio Encoder functions */
platform_result_t platform_aenc_init(platform_aenc_handle_t *handle, 
                                    const platform_audio_config_t *config);
void platform_aenc_cleanup(platform_aenc_handle_t handle);
platform_result_t platform_aenc_get_frame(platform_aenc_handle_t handle, 
                                         uint8_t **data, uint32_t *size);
void platform_aenc_release_frame(platform_aenc_handle_t handle, uint8_t *data);

/* PTZ functions */
platform_result_t platform_ptz_init(void);
void platform_ptz_cleanup(void);
platform_result_t platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg);
platform_result_t platform_ptz_check_self(void);
platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg);
platform_result_t platform_ptz_get_step_position(platform_ptz_axis_t axis);
platform_result_t platform_ptz_get_status(platform_ptz_axis_t axis, 
                                        platform_ptz_status_t *status);
platform_result_t platform_ptz_set_speed(platform_ptz_axis_t axis, int speed);
platform_result_t platform_ptz_turn(platform_ptz_direction_t direction, int steps);
platform_result_t platform_ptz_turn_stop(platform_ptz_direction_t direction);

/* IR LED functions */
platform_result_t platform_irled_init(int level);
void platform_irled_cleanup(void);
platform_result_t platform_irled_set_mode(platform_irled_mode_t mode);
platform_result_t platform_irled_get_status(void);

/* Configuration functions */
platform_result_t platform_config_load(const char *filename);
platform_result_t platform_config_save(const char *filename);
const char* platform_config_get_string(const char *section, const char *key, 
                                      const char *default_value);
int platform_config_get_int(const char *section, const char *key, int default_value);

/* Logging functions */
int platform_log_error(const char *format, ...);
int platform_log_warning(const char *format, ...);
int platform_log_notice(const char *format, ...);
int platform_log_info(const char *format, ...);
int platform_log_debug(const char *format, ...);

/* Utility functions */
void platform_sleep_ms(uint32_t milliseconds);
void platform_sleep_us(uint32_t microseconds);
uint64_t platform_get_time_ms(void);

#endif /* ONVIF_PLATFORM_H */