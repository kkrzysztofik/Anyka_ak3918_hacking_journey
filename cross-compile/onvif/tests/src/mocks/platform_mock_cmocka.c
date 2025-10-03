/**
 * @file platform_mock_cmocka.c
 * @brief CMocka-based platform mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include "platform_mock_cmocka.h"

/* ============================================================================
 * Core Platform Functions
 * ============================================================================ */

platform_result_t __wrap_platform_init(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_cleanup(void) {
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * Logging Functions
 * ============================================================================ */

int __wrap_platform_log_error(const char* format, ...) {
  (void)format;
  // Logging functions don't require expectations - they're informational only
  return 0;
}

int __wrap_platform_log_warning(const char* format, ...) {
  (void)format;
  // Logging functions don't require expectations - they're informational only
  return 0;
}

int __wrap_platform_log_notice(const char* format, ...) {
  (void)format;
  // Logging functions don't require expectations - they're informational only
  return 0;
}

int __wrap_platform_log_info(const char* format, ...) {
  (void)format;
  // Logging functions don't require expectations - they're informational only
  return 0;
}

int __wrap_platform_log_debug(const char* format, ...) {
  (void)format;
  // Logging functions don't require expectations - they're informational only
  return 0;
}

/* ============================================================================
 * Video Input (VI) Functions
 * ============================================================================ */

platform_result_t __wrap_platform_vi_match_sensor(void* sensor) {
  check_expected_ptr(sensor);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_open(void** vi_handle) {
  check_expected_ptr(vi_handle);
  function_called();
  if (vi_handle) {
    *vi_handle = (void*)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_close(void* vi_handle) {
  check_expected_ptr(vi_handle);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_get_sensor_resolution(
  platform_video_resolution_t* resolution) {
  check_expected_ptr(resolution);
  function_called();
  if (resolution) {
    resolution->width = (int)mock();
    resolution->height = (int)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_switch_day_night(platform_daynight_mode_t mode) {
  check_expected(mode);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_set_flip_mirror(int flip, int mirror) {
  check_expected(flip);
  check_expected(mirror);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_capture_on(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_start_global_capture(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_capture_off(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_set_channel_attr(int channel, void* attr) {
  check_expected(channel);
  check_expected_ptr(attr);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vi_get_fps(int* fps) {
  check_expected_ptr(fps);
  function_called();
  if (fps) {
    *fps = (int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * Video Processing (VPSS) Functions
 * ============================================================================ */

platform_result_t __wrap_platform_vpss_effect_set(int effect_type, int value) {
  check_expected(effect_type);
  check_expected(value);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vpss_effect_get(int effect_type, int* value) {
  check_expected(effect_type);
  check_expected_ptr(value);
  function_called();
  if (value) {
    *value = (int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * Video Encoding (VENC) Functions
 * ============================================================================ */

platform_result_t __wrap_platform_venc_init(void** venc_handle, void* config) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(config);
  function_called();
  if (venc_handle) {
    *venc_handle = (void*)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_validate_venc_config(void* config) {
  check_expected_ptr(config);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_cleanup(void* venc_handle) {
  check_expected_ptr(venc_handle);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_get_frame(void* venc_handle, void* frame) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(frame);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_release_frame(void* venc_handle, void* frame) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(frame);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_get_stream(void* venc_handle,
                                                  platform_venc_stream_t* stream) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(stream);
  function_called();
  if (stream) {
    stream->data = (unsigned char*)mock();
    stream->len = (unsigned int)mock();
    stream->timestamp = (unsigned long long)mock();
    stream->is_keyframe = (int)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_release_stream(void* venc_handle,
                                                      platform_venc_stream_t* stream) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(stream);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_request_stream(void* venc_handle, void** stream_handle) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(stream_handle);
  function_called();
  if (stream_handle) {
    *stream_handle = (void*)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_cancel_stream(void* stream_handle) {
  check_expected_ptr(stream_handle);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_get_stream_by_handle(void* stream_handle,
                                                            platform_venc_stream_t* stream) {
  check_expected_ptr(stream_handle);
  check_expected_ptr(stream);
  function_called();
  if (stream) {
    stream->data = (unsigned char*)mock();
    stream->len = (unsigned int)mock();
    stream->timestamp = (unsigned long long)mock();
    stream->is_keyframe = (int)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_release_stream_by_handle(void* stream_handle,
                                                                platform_venc_stream_t* stream) {
  check_expected_ptr(stream_handle);
  check_expected_ptr(stream);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_venc_get_buffer_status(void* venc_handle, int* buffer_count,
                                                         int* max_buffers, int* overflow_count) {
  check_expected_ptr(venc_handle);
  check_expected_ptr(buffer_count);
  check_expected_ptr(max_buffers);
  check_expected_ptr(overflow_count);
  function_called();
  if (buffer_count) {
    *buffer_count = (int)mock();
  }
  if (max_buffers) {
    *max_buffers = (int)mock();
  }
  if (overflow_count) {
    *overflow_count = (int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * Audio Input (AI) Functions
 * ============================================================================ */

platform_result_t __wrap_platform_ai_open(void** ai_handle) {
  check_expected_ptr(ai_handle);
  function_called();
  if (ai_handle) {
    *ai_handle = (void*)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ai_close(void* ai_handle) {
  check_expected_ptr(ai_handle);
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * Audio Encoding (AENC) Functions
 * ============================================================================ */

platform_result_t __wrap_platform_aenc_init(void** aenc_handle, void* config) {
  check_expected_ptr(aenc_handle);
  check_expected_ptr(config);
  function_called();
  if (aenc_handle) {
    *aenc_handle = (void*)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_aenc_cleanup(void* aenc_handle) {
  check_expected_ptr(aenc_handle);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_aenc_get_frame(void* aenc_handle, void* frame) {
  check_expected_ptr(aenc_handle);
  check_expected_ptr(frame);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_aenc_release_frame(void* aenc_handle, void* frame) {
  check_expected_ptr(aenc_handle);
  check_expected_ptr(frame);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_aenc_get_stream(void* aenc_handle,
                                                  platform_aenc_stream_t* stream) {
  check_expected_ptr(aenc_handle);
  check_expected_ptr(stream);
  function_called();
  if (stream) {
    stream->data = (unsigned char*)mock();
    stream->len = (unsigned int)mock();
    stream->timestamp = (unsigned long long)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_aenc_release_stream(void* aenc_handle,
                                                      platform_aenc_stream_t* stream) {
  check_expected_ptr(aenc_handle);
  check_expected_ptr(stream);
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * PTZ Functions
 * ============================================================================ */

platform_result_t __wrap_platform_ptz_init(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_cleanup(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_set_degree(float pan_degree, float tilt_degree) {
  check_expected(pan_degree);
  check_expected(tilt_degree);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_check_self(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_move_to_position(float pan, float tilt, float zoom) {
  check_expected(pan);
  check_expected(tilt);
  check_expected(zoom);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_get_step_position(int* pan, int* tilt) {
  check_expected_ptr(pan);
  check_expected_ptr(tilt);
  function_called();
  if (pan) {
    *pan = (int)mock();
  }
  if (tilt) {
    *tilt = (int)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_get_status(void* status) {
  check_expected_ptr(status);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_set_speed(float pan_speed, float tilt_speed) {
  check_expected(pan_speed);
  check_expected(tilt_speed);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_turn(int direction) {
  check_expected(direction);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_turn_stop(void) {
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * IR LED Functions
 * ============================================================================ */

platform_result_t __wrap_platform_irled_init(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_irled_cleanup(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_irled_set_mode(int mode) {
  check_expected(mode);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_irled_get_status(int* status) {
  check_expected_ptr(status);
  function_called();
  if (status) {
    *status = (int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * Snapshot Functions
 * ============================================================================ */

platform_result_t __wrap_platform_snapshot_init(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_snapshot_cleanup(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_snapshot_capture(platform_snapshot_handle_t handle,
                                                   platform_snapshot_t* snapshot) {
  check_expected(handle);
  check_expected_ptr(snapshot);
  function_called();
  if (snapshot) {
    snapshot->data = (unsigned char*)mock();
    snapshot->len = (unsigned int)mock();
    snapshot->timestamp = (unsigned long long)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_snapshot_release(platform_snapshot_handle_t handle,
                                                   platform_snapshot_t* snapshot) {
  check_expected(handle);
  check_expected_ptr(snapshot);
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * Configuration Functions
 * ============================================================================ */

platform_result_t __wrap_platform_config_load(const char* filename) {
  check_expected_ptr(filename);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_config_save(const char* filename) {
  check_expected_ptr(filename);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_config_get_string(const char* key, char* value, size_t size) {
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(size);
  function_called();
  if (value && size > 0) {
    const char* mock_value = (const char*)mock();
    if (mock_value) {
      strncpy(value, mock_value, size - 1);
      value[size - 1] = '\0';
    }
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_config_get_int(const char* key, int* value) {
  check_expected_ptr(key);
  check_expected_ptr(value);
  function_called();
  if (value) {
    *value = (int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * System Information Functions
 * ============================================================================ */

platform_result_t __wrap_platform_get_system_info(platform_system_info_t* info) {
  check_expected_ptr(info);
  function_called();
  if (info) {
    info->cpu_usage = (float)mock();
    info->cpu_temperature = (float)mock();
    info->total_memory = (unsigned int)mock();
    info->free_memory = (unsigned int)mock();
    info->uptime_ms = (unsigned int)mock();
  }
  return (platform_result_t)mock();
}

/* ============================================================================
 * Platform Utility Functions
 * ============================================================================ */

void __wrap_platform_sleep_ms(unsigned int ms) {
  check_expected(ms);
  function_called();
}
