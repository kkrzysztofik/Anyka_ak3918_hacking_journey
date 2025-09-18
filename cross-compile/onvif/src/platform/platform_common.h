/**
 * @file platform_common.h
 * @brief Common platform definitions and shared types.
 * 
 * This file contains common platform definitions, types, and constants
 * that are shared across all platform implementations.
 */

#ifndef PLATFORM_COMMON_H
#define PLATFORM_COMMON_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Platform result codes */
typedef enum {
    PLATFORM_SUCCESS = 0,
    PLATFORM_ERROR = -1,
    PLATFORM_ERROR_NULL = -2,
    PLATFORM_ERROR_INVALID = -3,
    PLATFORM_ERROR_MEMORY = -4,
    PLATFORM_ERROR_IO = -5,
    PLATFORM_ERROR_NOT_SUPPORTED = -6,
    PLATFORM_ERROR_BUSY = -7,
    PLATFORM_ERROR_TIMEOUT = -8,
    PLATFORM_ERROR_INVALID_PARAM = -9,
    PLATFORM_ERROR_NOT_INITIALIZED = -10,
    PLATFORM_ERROR_ALREADY_INITIALIZED = -11,
    PLATFORM_ERROR_OUT_OF_MEMORY = -12,
    PLATFORM_ERROR_HARDWARE = -13,
    PLATFORM_ERROR_CONFIG = -14,
    PLATFORM_ERROR_UNKNOWN = -999
} platform_result_t;

/* Video codec types */
typedef enum {
    PLATFORM_VIDEO_CODEC_H264 = 0,
    PLATFORM_VIDEO_CODEC_H265,
    PLATFORM_VIDEO_CODEC_MJPEG
} platform_video_codec_t;

/* Audio codec types */
typedef enum {
    PLATFORM_AUDIO_CODEC_AAC = 0,
    PLATFORM_AUDIO_CODEC_G711A,
    PLATFORM_AUDIO_CODEC_G711U,
    PLATFORM_AUDIO_CODEC_PCM
} platform_audio_codec_t;

/* PTZ axis types */
typedef enum {
    PLATFORM_PTZ_AXIS_PAN = 0,
    PLATFORM_PTZ_AXIS_TILT,
    PLATFORM_PTZ_AXIS_ZOOM
} platform_ptz_axis_t;

/* PTZ direction types */
typedef enum {
    PLATFORM_PTZ_DIRECTION_UP = 0,
    PLATFORM_PTZ_DIRECTION_DOWN,
    PLATFORM_PTZ_DIRECTION_LEFT,
    PLATFORM_PTZ_DIRECTION_RIGHT,
    PLATFORM_PTZ_DIRECTION_ZOOM_IN,
    PLATFORM_PTZ_DIRECTION_ZOOM_OUT
} platform_ptz_direction_t;

/* PTZ status types */
typedef enum {
    PLATFORM_PTZ_STATUS_STOPPED = 0,
    PLATFORM_PTZ_STATUS_MOVING,
    PLATFORM_PTZ_STATUS_ERROR
} platform_ptz_status_t;

/* VPSS effect types */
typedef enum {
    PLATFORM_VPSS_EFFECT_NONE = 0,
    PLATFORM_VPSS_EFFECT_BRIGHTNESS,
    PLATFORM_VPSS_EFFECT_CONTRAST,
    PLATFORM_VPSS_EFFECT_SATURATION,
    PLATFORM_VPSS_EFFECT_SHARPNESS,
    PLATFORM_VPSS_EFFECT_HUE,
    PLATFORM_VPSS_EFFECT_SHARPEN,
    PLATFORM_VPSS_EFFECT_SMOOTH,
    PLATFORM_VPSS_EFFECT_EDGE_ENHANCE
} platform_vpss_effect_t;

/* IR LED mode types */
typedef enum {
    PLATFORM_IRLED_MODE_OFF = 0,
    PLATFORM_IRLED_MODE_ON,
    PLATFORM_IRLED_MODE_AUTO
} platform_irled_mode_t;

/* Handle types */
typedef void* platform_vi_handle_t;
typedef void* platform_venc_handle_t;
typedef void* platform_ai_handle_t;
typedef void* platform_aenc_handle_t;

/* Configuration structures */
typedef struct {
    int width;
    int height;
    int fps;
    int bitrate;
    platform_video_codec_t codec;
} platform_video_config_t;

typedef struct {
    int sample_rate;
    int channels;
    int bitrate;
    int bits_per_sample;
    platform_audio_codec_t codec;
} platform_audio_config_t;

#ifdef __cplusplus
}
#endif

#endif /* PLATFORM_COMMON_H */
