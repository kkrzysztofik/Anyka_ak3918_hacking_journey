/**
 * @file platform_mock.c
 * @brief CMocka-based platform mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include "platform_mock.h"

#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include "platform_ptz_mock.h"

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
  va_list args;
  va_start(args, format);
  printf("[ERROR] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int __wrap_platform_log_warning(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[WARNING] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int __wrap_platform_log_notice(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[NOTICE] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int __wrap_platform_log_info(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[INFO] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int __wrap_platform_log_debug(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[DEBUG] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
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

platform_result_t __wrap_platform_vpss_effect_set(platform_vi_handle_t handle,
                                                  platform_vpss_effect_t effect, int value) {
  check_expected_ptr(handle);
  check_expected(effect);
  check_expected(value);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_vpss_effect_get(platform_vi_handle_t handle,
                                                  platform_vpss_effect_t effect, int* value) {
  check_expected_ptr(handle);
  check_expected(effect);
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

// Forward declarations for PTZ mock recording functions
void platform_ptz_mock_record_init(void);
void platform_ptz_mock_record_cleanup(void);
void platform_ptz_mock_record_absolute_move(int pan, int tilt, int speed);
void platform_ptz_mock_record_turn(platform_ptz_direction_t dir, int steps);
void platform_ptz_mock_record_turn_stop(platform_ptz_direction_t dir);

platform_result_t __wrap_platform_ptz_init(void) {
  function_called();
  platform_result_t result = (platform_result_t)mock();
  // Only record init as successful if the result is PLATFORM_SUCCESS
  if (result == PLATFORM_SUCCESS) {
    platform_ptz_mock_record_init();
  }
  return result;
}

platform_result_t __wrap_platform_ptz_cleanup(void) {
  function_called();
  platform_ptz_mock_record_cleanup();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg) {
  check_expected(pan_range_deg);
  check_expected(tilt_range_deg);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_check_self(void) {
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_move_to_position(int pan_deg, int tilt_deg) {
  check_expected(pan_deg);
  check_expected(tilt_deg);
  // Record the move (speed is 0 since it's not part of this function's signature)
  platform_ptz_mock_record_absolute_move(pan_deg, tilt_deg, 0);
  function_called();
  return (platform_result_t)mock();
}

int __wrap_platform_ptz_get_step_position(platform_ptz_axis_t axis) {
  check_expected(axis);
  function_called();
  return (int)mock();
}

platform_result_t __wrap_platform_ptz_get_status(platform_ptz_axis_t axis,
                                                 platform_ptz_status_t* status) {
  check_expected(axis);
  check_expected_ptr(status);
  function_called();
  if (status) {
    *status = (platform_ptz_status_t)mock();
  }
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_set_speed(platform_ptz_axis_t axis_type, int speed_value) {
  check_expected(axis_type);
  check_expected(speed_value);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_turn(platform_ptz_direction_t direction, int steps) {
  check_expected(direction);
  check_expected(steps);
  platform_ptz_mock_record_turn(direction, steps);
  printf("[MOCK][PTZ] turn dir=%d steps=%d\n", direction, steps);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_platform_ptz_turn_stop(platform_ptz_direction_t direction) {
  if (platform_ptz_mock_should_bypass_expectations()) {
    platform_ptz_mock_record_turn_stop(direction);
    printf("[MOCK][PTZ] turn_stop bypass dir=%d\n", direction);
    return PLATFORM_SUCCESS;
  }

  check_expected(direction);
  platform_ptz_mock_record_turn_stop(direction);
  printf("[MOCK][PTZ] turn_stop dir=%d (expectation path)\n", direction);
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * IR LED Functions
 * ============================================================================ */

platform_result_t __wrap_platform_irled_init(int level) {
  check_expected(level);
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

int __wrap_platform_config_get_int(const char* section, const char* key, int default_value) {
  check_expected_ptr(section);
  check_expected_ptr(key);
  function_called();
  return (int)mock_type(int);
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
