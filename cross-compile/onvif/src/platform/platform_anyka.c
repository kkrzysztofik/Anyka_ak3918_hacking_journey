/**
 * @file platform_anyka.c
 * @brief Anyka-specific implementation of unified platform abstraction layer.
 * 
 * This file consolidates the HAL and platform abstraction functionality
 * into a single implementation for the Anyka AK3918 platform.
 */

#include "platform.h"
#include "ak_common.h"
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#include "ak_drv_ptz.h"
#include "ak_drv_irled.h"
#include "ak_vpss.h"
#include "ak_global.h"
#include "ak_thread.h"
#include "list.h"
#include "utils/safe_string.h"
#include "utils/error_context.h"
#include "utils/memory_manager.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <pthread.h>

/* Platform state */
static bool g_platform_initialized = false;
static platform_vi_handle_t g_vi_handle = NULL;
static platform_venc_handle_t g_venc_handle = NULL;
static platform_ai_handle_t g_ai_handle = NULL;
static platform_aenc_handle_t g_aenc_handle = NULL;
static platform_video_config_t g_video_config;
static platform_audio_config_t g_audio_config;

/* Internal helper functions */
static int map_video_codec(platform_video_codec_t codec) {
    switch (codec) {
        case PLATFORM_VIDEO_CODEC_H264: return H264_ENC_TYPE;
        case PLATFORM_VIDEO_CODEC_H265: return HEVC_ENC_TYPE;
        case PLATFORM_VIDEO_CODEC_MJPEG: return MJPEG_ENC_TYPE;
        default: return -1;
    }
}

static int map_platform_enc_type(int platform_enc_type) {
    switch (platform_enc_type) {
        case PLATFORM_H264_ENC_TYPE: return H264_ENC_TYPE;
        case PLATFORM_HEVC_ENC_TYPE: return HEVC_ENC_TYPE;
        case PLATFORM_MJPEG_ENC_TYPE: return MJPEG_ENC_TYPE;
        default: return -1;
    }
}

static int map_platform_profile(int platform_profile) {
    switch (platform_profile) {
        case PLATFORM_PROFILE_MAIN: return PROFILE_MAIN;
        case PLATFORM_PROFILE_BASELINE: return PROFILE_MAIN; // Use MAIN as fallback
        case PLATFORM_PROFILE_HIGH: return PROFILE_MAIN; // Use MAIN as fallback
        default: return -1;
    }
}

static int map_platform_br_mode(int platform_br_mode) {
    switch (platform_br_mode) {
        case PLATFORM_BR_MODE_CBR: return BR_MODE_CBR;
        case PLATFORM_BR_MODE_VBR: return BR_MODE_VBR;
        default: return -1;
    }
}

static int map_platform_frame_type(int platform_frame_type) {
    switch (platform_frame_type) {
        case PLATFORM_FRAME_TYPE_I: return FRAME_TYPE_I;
        case PLATFORM_FRAME_TYPE_P: return FRAME_TYPE_P;
        case PLATFORM_FRAME_TYPE_B: return FRAME_TYPE_B;
        default: return -1;
    }
}

static int map_audio_codec(platform_audio_codec_t codec) {
    switch (codec) {
        case PLATFORM_AUDIO_CODEC_PCM: return AK_AUDIO_TYPE_UNKNOWN; // Use PCM as default
        case PLATFORM_AUDIO_CODEC_AAC: return AK_AUDIO_TYPE_AAC;
        case PLATFORM_AUDIO_CODEC_G711A: return AK_AUDIO_TYPE_UNKNOWN; // G711 not directly supported
        case PLATFORM_AUDIO_CODEC_G711U: return AK_AUDIO_TYPE_UNKNOWN; // G711 not directly supported
        default: return -1;
    }
}

static int map_vpss_effect(platform_vpss_effect_t effect) {
    switch (effect) {
        case PLATFORM_VPSS_EFFECT_BRIGHTNESS: return VPSS_EFFECT_BRIGHTNESS;
        case PLATFORM_VPSS_EFFECT_CONTRAST: return VPSS_EFFECT_CONTRAST;
        case PLATFORM_VPSS_EFFECT_SATURATION: return VPSS_EFFECT_SATURATION;
        case PLATFORM_VPSS_EFFECT_SHARPNESS: return VPSS_EFFECT_SHARP;
        case PLATFORM_VPSS_EFFECT_HUE: return VPSS_EFFECT_HUE;
        default: return -1;
    }
}

static int map_daynight_mode(platform_daynight_mode_t mode) {
    switch (mode) {
        case PLATFORM_DAYNIGHT_DAY: return VI_MODE_DAY;
        case PLATFORM_DAYNIGHT_NIGHT: return VI_MODE_NIGHT;
        case PLATFORM_DAYNIGHT_AUTO: return VI_MODE_DAY; // Use DAY as default for AUTO
        default: return -1;
    }
}

static int map_ptz_axis(platform_ptz_axis_t axis) {
    switch (axis) {
        case PLATFORM_PTZ_AXIS_PAN: return PTZ_DEV_H;
        case PLATFORM_PTZ_AXIS_TILT: return PTZ_DEV_V;
        default: return -1;
    }
}

static int map_ptz_direction(platform_ptz_direction_t direction) {
    switch (direction) {
        case PLATFORM_PTZ_DIRECTION_LEFT: return PTZ_TURN_LEFT;
        case PLATFORM_PTZ_DIRECTION_RIGHT: return PTZ_TURN_RIGHT;
        case PLATFORM_PTZ_DIRECTION_UP: return PTZ_TURN_UP;
        case PLATFORM_PTZ_DIRECTION_DOWN: return PTZ_TURN_DOWN;
        default: return -1;
    }
}

/* Platform initialization */
platform_result_t platform_init(void) {
    if (g_platform_initialized) {
        return PLATFORM_SUCCESS;
    }
    
    // Initialize memory manager
    if (memory_manager_init() != 0) {
        platform_log_error("Failed to initialize memory manager\n");
        return PLATFORM_ERROR;
    }
    
    g_platform_initialized = true;
    platform_log_info("Unified platform abstraction initialized for Anyka\n");
    return PLATFORM_SUCCESS;
}

void platform_cleanup(void) {
    if (!g_platform_initialized) {
        return;
    }
    
    platform_venc_cleanup(g_venc_handle);
    platform_aenc_cleanup(g_aenc_handle);
    platform_vi_close(g_vi_handle);
    platform_ai_close(g_ai_handle);
    platform_ptz_cleanup();
    platform_irled_cleanup();
    
    g_vi_handle = NULL;
    g_venc_handle = NULL;
    g_ai_handle = NULL;
    g_aenc_handle = NULL;
    g_platform_initialized = false;
    
    // Cleanup memory manager
    memory_manager_cleanup();
}

/* Video Input (VI) functions */
platform_result_t platform_vi_open(platform_vi_handle_t *handle) {
    if (!handle) return PLATFORM_ERROR_NULL;
    
    void *h = ak_vi_open(VIDEO_DEV0);
    if (!h) return PLATFORM_ERROR;
    
    *handle = h;
    return PLATFORM_SUCCESS;
}

void platform_vi_close(platform_vi_handle_t handle) {
    if (handle) {
        ak_vi_close(handle);
    }
}

platform_result_t platform_vi_get_sensor_resolution(platform_vi_handle_t handle, 
                                                   platform_video_resolution_t *resolution) {
    if (!handle || !resolution) return PLATFORM_ERROR_NULL;
    
    struct video_resolution r;
    if (ak_vi_get_sensor_resolution(handle, &r) != 0) {
        return PLATFORM_ERROR;
    }
    
    resolution->width = r.width;
    resolution->height = r.height;
    return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_switch_day_night(platform_vi_handle_t handle, 
                                              platform_daynight_mode_t mode) {
    if (!handle) return PLATFORM_ERROR_NULL;
    
    int vi_mode = map_daynight_mode(mode);
    if (vi_mode < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_vi_switch_mode(handle, vi_mode) != 0) {
        return PLATFORM_ERROR;
    }
    
    return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_set_flip_mirror(platform_vi_handle_t handle, 
                                             bool flip, bool mirror) {
    if (!handle) return PLATFORM_ERROR_NULL;
    
    if (ak_vi_set_flip_mirror(handle, flip ? 1 : 0, mirror ? 1 : 0) != 0) {
        return PLATFORM_ERROR;
    }
    
    return PLATFORM_SUCCESS;
}

/* VPSS (Video Processing Subsystem) functions */
platform_result_t platform_vpss_effect_set(platform_vi_handle_t handle, 
                                          platform_vpss_effect_t effect, int value) {
    if (!handle) return PLATFORM_ERROR_NULL;
    
    int vpss_effect = map_vpss_effect(effect);
    if (vpss_effect < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_vpss_effect_set(handle, vpss_effect, value) != 0) {
        return PLATFORM_ERROR;
    }
    
    return PLATFORM_SUCCESS;
}

platform_result_t platform_vpss_effect_get(platform_vi_handle_t handle, 
                                          platform_vpss_effect_t effect, int *value) {
    if (!handle || !value) return PLATFORM_ERROR_NULL;
    
    int vpss_effect = map_vpss_effect(effect);
    if (vpss_effect < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_vpss_effect_get(handle, vpss_effect, value) != 0) {
        return PLATFORM_ERROR;
    }
    
    return PLATFORM_SUCCESS;
}

/* Video Encoder functions */
platform_result_t platform_venc_init(platform_venc_handle_t *handle, 
                                    const platform_video_config_t *config) {
    if (!handle || !config) return PLATFORM_ERROR_NULL;
    
    struct encode_param param;
    memset(&param, 0, sizeof(param));
    
    param.width = config->width;
    param.height = config->height;
    param.fps = config->fps;
    param.bps = config->bitrate;
    param.enc_out_type = map_video_codec(config->codec);
    param.use_chn = ENCODE_MAIN_CHN;
    param.enc_grp = ENCODE_MAINCHN_NET;
    param.br_mode = BR_MODE_CBR;
    param.profile = PROFILE_MAIN;
    param.goplen = 30;
    param.minqp = 20;
    param.maxqp = 45;
    
    if (param.enc_out_type < 0) return PLATFORM_ERROR_INVALID;
    
    void *h = ak_venc_open(&param);
    if (!h) return PLATFORM_ERROR;
    
    *handle = h;
    g_video_config = *config;
    return PLATFORM_SUCCESS;
}

void platform_venc_cleanup(platform_venc_handle_t handle) {
    if (handle) {
        ak_venc_close(handle);
    }
}

platform_result_t platform_venc_get_frame(platform_venc_handle_t handle, 
                                         uint8_t **data, uint32_t *size) {
    if (!handle || !data || !size) return PLATFORM_ERROR_NULL;
    
    struct video_stream stream;
    if (ak_venc_get_stream(handle, &stream) != 0) {
        return PLATFORM_ERROR;
    }
    
    *data = stream.data;
    *size = stream.len;
    return PLATFORM_SUCCESS;
}

void platform_venc_release_frame(platform_venc_handle_t handle, uint8_t *data) {
    if (handle && data) {
        struct video_stream stream = {.data = data};
        ak_venc_release_stream(handle, &stream);
    }
}

/* Audio Input (AI) functions */
platform_result_t platform_ai_open(platform_ai_handle_t *handle) {
    if (!handle) return PLATFORM_ERROR_NULL;
    
    void *h = ak_ai_open(0); // Use device 0
    if (!h) return PLATFORM_ERROR;
    
    *handle = h;
    return PLATFORM_SUCCESS;
}

void platform_ai_close(platform_ai_handle_t handle) {
    if (handle) {
        ak_ai_close(handle);
    }
}

/* Audio Encoder functions */
platform_result_t platform_aenc_init(platform_aenc_handle_t *handle, 
                                    const platform_audio_config_t *config) {
    if (!handle || !config) return PLATFORM_ERROR_NULL;
    
    struct audio_param param;
    memset(&param, 0, sizeof(param));
    
    param.sample_rate = config->sample_rate;
    param.channel_num = config->channels;
    param.sample_bits = config->bits_per_sample;
    param.type = map_audio_codec(config->codec);
    
    if (param.type < 0) return PLATFORM_ERROR_INVALID;
    
    void *h = ak_aenc_open(&param);
    if (!h) return PLATFORM_ERROR;
    
    *handle = h;
    g_audio_config = *config;
    return PLATFORM_SUCCESS;
}

void platform_aenc_cleanup(platform_aenc_handle_t handle) {
    if (handle) {
        ak_aenc_close(handle);
    }
}

platform_result_t platform_aenc_get_frame(platform_aenc_handle_t handle, 
                                         uint8_t **data, uint32_t *size) {
    if (!handle || !data || !size) return PLATFORM_ERROR_NULL;
    
    struct list_head stream_head;
    INIT_LIST_HEAD(&stream_head);
    
    if (ak_aenc_get_stream(handle, &stream_head) != 0) {
        return PLATFORM_ERROR;
    }
    
    if (list_empty(&stream_head)) {
        return PLATFORM_ERROR;
    }
    
    struct aenc_entry *entry = list_first_entry(&stream_head, struct aenc_entry, list);
    *data = entry->stream.data;
    *size = entry->stream.len;
    return PLATFORM_SUCCESS;
}

void platform_aenc_release_frame(platform_aenc_handle_t handle, uint8_t *data) {
    if (handle && data) {
        // Note: Audio encoder doesn't have a separate release function
        // The stream is managed internally
    }
}

/* PTZ functions */
platform_result_t platform_ptz_init(void) {
    if (ak_drv_ptz_open() != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

void platform_ptz_cleanup(void) {
    ak_drv_ptz_close();
}

platform_result_t platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg) {
    if (ak_drv_ptz_set_degree(pan_range_deg, tilt_range_deg) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_check_self(void) {
    if (ak_drv_ptz_check_self(PTZ_FEEDBACK_PIN_NONE) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg) {
    if (ak_drv_ptz_turn_to_pos(pan_deg, tilt_deg) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

int platform_ptz_get_step_position(platform_ptz_axis_t axis) {
    int anyka_axis = map_ptz_axis(axis);
    if (anyka_axis < 0) return PLATFORM_ERROR_INVALID;
    
    int pos = ak_drv_ptz_get_step_pos(anyka_axis);
    if (pos < 0) return PLATFORM_ERROR;
    
    return pos;
}

platform_result_t platform_ptz_get_status(platform_ptz_axis_t axis, 
                                        platform_ptz_status_t *status) {
    if (!status) return PLATFORM_ERROR_NULL;
    
    int anyka_axis = map_ptz_axis(axis);
    if (anyka_axis < 0) return PLATFORM_ERROR_INVALID;
    
    enum ptz_status anyka_status;
    if (ak_drv_ptz_get_status(anyka_axis, &anyka_status) != 0) {
        return PLATFORM_ERROR;
    }
    
    switch (anyka_status) {
        case PTZ_INIT_OK: *status = PLATFORM_PTZ_STATUS_OK; break;
        case PTZ_WAIT_INIT: *status = PLATFORM_PTZ_STATUS_BUSY; break;
        case PTZ_SYS_WAIT: *status = PLATFORM_PTZ_STATUS_BUSY; break;
        default: *status = PLATFORM_PTZ_STATUS_ERROR; break;
    }
    
    return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_set_speed(platform_ptz_axis_t axis, int speed) {
    int anyka_axis = map_ptz_axis(axis);
    if (anyka_axis < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_drv_ptz_set_speed(anyka_axis, speed) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn(platform_ptz_direction_t direction, int steps) {
    int anyka_direction = map_ptz_direction(direction);
    if (anyka_direction < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_drv_ptz_turn(anyka_direction, steps) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn_stop(platform_ptz_direction_t direction) {
    int anyka_direction = map_ptz_direction(direction);
    if (anyka_direction < 0) return PLATFORM_ERROR_INVALID;
    
    if (ak_drv_ptz_turn_stop(anyka_direction) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

/* IR LED functions */
platform_result_t platform_irled_init(int level) {
    struct ak_drv_irled_hw_param param;
    param.irled_working_level = level;
    
    if (ak_drv_irled_init(&param) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

void platform_irled_cleanup(void) {
    // No cleanup function available in Anyka SDK
}

platform_result_t platform_irled_set_mode(platform_irled_mode_t mode) {
    int anyka_mode;
    switch (mode) {
        case PLATFORM_IRLED_OFF: anyka_mode = 0; break;
        case PLATFORM_IRLED_ON: anyka_mode = 1; break;
        case PLATFORM_IRLED_AUTO: anyka_mode = 1; break; // Use ON for AUTO
        default: return PLATFORM_ERROR_INVALID;
    }
    
    if (ak_drv_irled_set_working_stat(anyka_mode) != 0) {
        return PLATFORM_ERROR;
    }
    return PLATFORM_SUCCESS;
}

platform_result_t platform_irled_get_status(void) {
    int status = ak_drv_irled_get_working_stat();
    if (status < 0) return PLATFORM_ERROR;
    return status;
}

/* Configuration functions */
static char g_config_buffer[4096] = {0};
static bool g_config_loaded = false;

platform_result_t platform_config_load(const char *filename) {
    if (!filename) return PLATFORM_ERROR_NULL;
    
    FILE *fp = fopen(filename, "r");
    if (!fp) {
        platform_log_warning("Failed to open config file: %s\n", filename);
        return PLATFORM_ERROR_IO;
    }
    
    size_t bytes_read = fread(g_config_buffer, 1, sizeof(g_config_buffer) - 1, fp);
    fclose(fp);
    
    if (bytes_read == 0) {
        platform_log_warning("Config file is empty: %s\n", filename);
        return PLATFORM_ERROR_IO;
    }
    
    g_config_buffer[bytes_read] = '\0';
    g_config_loaded = true;
    
    platform_log_info("Configuration loaded from: %s\n", filename);
    return PLATFORM_SUCCESS;
}

platform_result_t platform_config_save(const char *filename) {
    if (!filename) return PLATFORM_ERROR_NULL;
    
    FILE *fp = fopen(filename, "w");
    if (!fp) {
        platform_log_error("Failed to create config file: %s\n", filename);
        return PLATFORM_ERROR_IO;
    }
    
    size_t bytes_written = fwrite(g_config_buffer, 1, strlen(g_config_buffer), fp);
    fclose(fp);
    
    if (bytes_written != strlen(g_config_buffer)) {
        platform_log_error("Failed to write complete config to: %s\n", filename);
        return PLATFORM_ERROR_IO;
    }
    
    platform_log_info("Configuration saved to: %s\n", filename);
    return PLATFORM_SUCCESS;
}

const char* platform_config_get_string(const char *section, const char *key, 
                                      const char *default_value) {
    if (!section || !key || !g_config_loaded) return default_value;
    
    // Simple INI-style parsing
    char search_pattern[256];
    snprintf(search_pattern, sizeof(search_pattern), "[%s]", section);
    
    char *section_start = strstr(g_config_buffer, search_pattern);
    if (!section_start) return default_value;
    
    char *line_start = strchr(section_start, '\n');
    if (!line_start) return default_value;
    line_start++; // Skip the newline
    
    while (*line_start && *line_start != '[') {
        char *line_end = strchr(line_start, '\n');
        if (!line_end) line_end = g_config_buffer + strlen(g_config_buffer);
        
        char *equals = strchr(line_start, '=');
        if (equals && equals < line_end) {
            // Check if this is our key
            char *key_end = equals;
            while (key_end > line_start && (*key_end == ' ' || *key_end == '\t')) key_end--;
            
            size_t key_len = key_end - line_start;
            if (strncmp(line_start, key, key_len) == 0) {
                // Found our key, return the value
                char *value_start = equals + 1;
                while (*value_start == ' ' || *value_start == '\t') value_start++;
                
                char *value_end = line_end;
                while (value_end > value_start && (value_end[-1] == ' ' || value_end[-1] == '\t' || value_end[-1] == '\r')) value_end--;
                
                static char value_buffer[256];
                size_t value_len = value_end - value_start;
                if (value_len >= sizeof(value_buffer)) value_len = sizeof(value_buffer) - 1;
                
                strncpy(value_buffer, value_start, value_len);
                value_buffer[value_len] = '\0';
                
                return value_buffer;
            }
        }
        
        line_start = line_end;
        if (*line_start == '\n') line_start++;
    }
    
    return default_value;
}

int platform_config_get_int(const char *section, const char *key, int default_value) {
    const char *str_value = platform_config_get_string(section, key, NULL);
    if (!str_value) return default_value;
    
    char *endptr;
    long int_value = strtol(str_value, &endptr, 10);
    if (endptr == str_value || *endptr != '\0') {
        return default_value;
    }
    
    return (int)int_value;
}

/* Logging functions */
int platform_log_error(const char *format, ...) {
    va_list args;
    char buffer[1024];
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (len >= sizeof(buffer)) {
        // Truncated, add ellipsis
        buffer[sizeof(buffer) - 4] = '.';
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 2] = '.';
        buffer[sizeof(buffer) - 1] = '\0';
    }
    
    return ak_print(LOG_LEVEL_ERROR, "\n%s", buffer);
}

int platform_log_warning(const char *format, ...) {
    va_list args;
    char buffer[1024];
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (len >= sizeof(buffer)) {
        // Truncated, add ellipsis
        buffer[sizeof(buffer) - 4] = '.';
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 2] = '.';
        buffer[sizeof(buffer) - 1] = '\0';
    }
    
    return ak_print(LOG_LEVEL_WARNING, "\n%s", buffer);
}

int platform_log_notice(const char *format, ...) {
    va_list args;
    char buffer[1024];
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (len >= sizeof(buffer)) {
        // Truncated, add ellipsis
        buffer[sizeof(buffer) - 4] = '.';
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 2] = '.';
        buffer[sizeof(buffer) - 1] = '\0';
    }
    
    return ak_print(LOG_LEVEL_NOTICE, "%s", buffer);
}

int platform_log_info(const char *format, ...) {
    va_list args;
    char buffer[1024];
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (len >= sizeof(buffer)) {
        // Truncated, add ellipsis
        buffer[sizeof(buffer) - 4] = '.';
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 2] = '.';
        buffer[sizeof(buffer) - 1] = '\0';
    }
    
    return ak_print(LOG_LEVEL_INFO, "%s", buffer);
}

int platform_log_debug(const char *format, ...) {
    va_list args;
    char buffer[1024];
    va_start(args, format);
    int len = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (len >= sizeof(buffer)) {
        // Truncated, add ellipsis
        buffer[sizeof(buffer) - 4] = '.';
        buffer[sizeof(buffer) - 3] = '.';
        buffer[sizeof(buffer) - 2] = '.';
        buffer[sizeof(buffer) - 1] = '\0';
    }
    
    return ak_print(LOG_LEVEL_DEBUG, "%s", buffer);
}

/* Utility functions */
void platform_sleep_ms(uint32_t milliseconds) {
    struct timespec ts;
    ts.tv_sec = milliseconds / 1000;
    ts.tv_nsec = (milliseconds % 1000) * 1000000;
    nanosleep(&ts, NULL);
}

void platform_sleep_us(uint32_t microseconds) {
    struct timespec ts;
    ts.tv_sec = microseconds / 1000000;
    ts.tv_nsec = (microseconds % 1000000) * 1000;
    nanosleep(&ts, NULL);
}

uint64_t platform_get_time_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

/* Video Encoder Stream functions (for RTSP) */
platform_result_t platform_venc_get_stream(platform_venc_handle_t handle, 
                                          platform_venc_stream_t *stream, 
                                          uint32_t timeout_ms) {
    if (!handle || !stream) return PLATFORM_ERROR_NULL;
    
    struct video_stream anyka_stream;
    int result = ak_venc_get_stream(handle, &anyka_stream);
    
    if (result != 0) {
        return PLATFORM_ERROR;
    }
    
    stream->data = anyka_stream.data;
    stream->len = anyka_stream.len;
    stream->timestamp = anyka_stream.ts;
    stream->is_keyframe = (anyka_stream.frame_type == FRAME_TYPE_I);
    
    return PLATFORM_SUCCESS;
}

void platform_venc_release_stream(platform_venc_handle_t handle, 
                                 platform_venc_stream_t *stream) {
    if (!handle || !stream) return;
    
    struct video_stream anyka_stream;
    anyka_stream.data = stream->data;
    anyka_stream.len = stream->len;
    anyka_stream.ts = stream->timestamp;
    
    ak_venc_release_stream(handle, &anyka_stream);
}

/* Audio Encoder Stream functions (for RTSP) */
platform_result_t platform_aenc_get_stream(platform_aenc_handle_t handle, 
                                          platform_aenc_stream_t *stream, 
                                          uint32_t timeout_ms) {
    if (!handle || !stream) return PLATFORM_ERROR_NULL;
    
    struct list_head stream_head;
    INIT_LIST_HEAD(&stream_head);
    
    int result = ak_aenc_get_stream(handle, &stream_head);
    if (result != 0) {
        return PLATFORM_ERROR;
    }
    
    if (list_empty(&stream_head)) {
        return PLATFORM_ERROR;
    }
    
    struct aenc_entry *entry = list_first_entry(&stream_head, struct aenc_entry, list);
    stream->data = entry->stream.data;
    stream->len = entry->stream.len;
    stream->timestamp = entry->stream.ts;
    
    return PLATFORM_SUCCESS;
}

void platform_aenc_release_stream(platform_aenc_handle_t handle, 
                                 platform_aenc_stream_t *stream) {
    if (!handle || !stream) return;
    
    // Note: Audio encoder stream is managed internally by Anyka SDK
    // The stream data is released automatically when the next frame is retrieved
}