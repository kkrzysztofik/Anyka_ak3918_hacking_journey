/**
 * @file platform_mock.c
 * @brief Mock implementation for platform functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "platform_mock.h"

#include <pthread.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "../../src/platform/platform.h"

/* ============================================================================
 * Mock Constants
 * ============================================================================ */

#define MOCK_DEVICE_NAME_MAX_LEN       256
#define MOCK_BUFFER_SIZE               1024
#define MOCK_CPU_USAGE_DEFAULT         25.5f
#define MOCK_CPU_TEMPERATURE_DEFAULT   45.0f
#define MOCK_MEMORY_1GB                (1024U * 1024U * 1024U)
#define MOCK_MEMORY_512MB              (512U * 1024U * 1024U)
#define MOCK_UPTIME_1HOUR_MS           3600000U
#define MOCK_VIDEO_WIDTH_DEFAULT       1920
#define MOCK_VIDEO_HEIGHT_DEFAULT      1080
#define MOCK_FPS_DEFAULT               30
#define MOCK_VPSS_EFFECT_VALUE_DEFAULT 50
#define MOCK_BUFFER_COUNT_DEFAULT      5
#define MOCK_MAX_BUFFERS_DEFAULT       10
#define MOCK_OVERFLOW_COUNT_DEFAULT    0
#define MOCK_HANDLE_VI                 0x12345678U
#define MOCK_HANDLE_VENC               0x87654321U
#define MOCK_HANDLE_VENC_STREAM        0x11111111U
#define MOCK_HANDLE_AI                 0x22222222U
#define MOCK_HANDLE_AENC_STREAM        0x33333333U
#define MOCK_HANDLE_SNAPSHOT           0x44444444U
#define MOCK_MS_PER_SECOND             1000U

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

// Global mock state
static struct {
  int initialized;
  int init_call_count;
  int cleanup_call_count;
  int error_simulation_enabled;
  platform_result_t error_code;
  char device_name[MOCK_DEVICE_NAME_MAX_LEN];
  time_t mock_time;
  void* mock_capabilities;
  void* mock_system_info;
  pthread_mutex_t mutex;
} g_platform_mock_state = {0}; // NOLINT

/* ============================================================================
 * Mock Initialization and Cleanup
 * ============================================================================ */

void platform_mock_init(void) {
  pthread_mutex_init(&g_platform_mock_state.mutex, NULL);
  g_platform_mock_state.initialized = 1;
  g_platform_mock_state.init_call_count = 0;
  g_platform_mock_state.cleanup_call_count = 0;
  g_platform_mock_state.error_simulation_enabled = 0;
  g_platform_mock_state.error_code = PLATFORM_SUCCESS;
  strcpy(g_platform_mock_state.device_name, "MockCamera");
  g_platform_mock_state.mock_time = time(NULL);
  g_platform_mock_state.mock_capabilities = NULL;
  g_platform_mock_state.mock_system_info = NULL;
}

void platform_mock_cleanup(void) {
  if (g_platform_mock_state.mock_capabilities) {
    free(g_platform_mock_state.mock_capabilities);
    g_platform_mock_state.mock_capabilities = NULL;
  }
  if (g_platform_mock_state.mock_system_info) {
    free(g_platform_mock_state.mock_system_info);
    g_platform_mock_state.mock_system_info = NULL;
  }
  pthread_mutex_destroy(&g_platform_mock_state.mutex);
  memset(&g_platform_mock_state, 0, sizeof(g_platform_mock_state));
}

void platform_mock_reset(void) {
  platform_mock_cleanup();
  platform_mock_init();
}

/* ============================================================================
 * Mock Configuration Functions
 * ============================================================================ */

void platform_mock_set_device_name(const char* name) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  if (name) {
    strncpy(g_platform_mock_state.device_name, name, MOCK_DEVICE_NAME_MAX_LEN - 1);
    g_platform_mock_state.device_name[MOCK_DEVICE_NAME_MAX_LEN - 1] = '\0';
  }
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
}

void platform_mock_set_time(time_t time_val) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  g_platform_mock_state.mock_time = time_val;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
}

void platform_mock_enable_error(platform_result_t error_code) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  g_platform_mock_state.error_simulation_enabled = 1;
  g_platform_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
}

void platform_mock_disable_error(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  g_platform_mock_state.error_simulation_enabled = 0;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
}

int platform_mock_is_error_enabled(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  int enabled = g_platform_mock_state.error_simulation_enabled;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return enabled;
}

int platform_mock_get_init_call_count(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  int count = g_platform_mock_state.init_call_count;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return count;
}

int platform_mock_get_cleanup_call_count(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  int count = g_platform_mock_state.cleanup_call_count;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return count;
}

/* ============================================================================
 * Platform Function Mocks
 * ============================================================================ */

// Mock platform initialization
platform_result_t platform_init(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  g_platform_mock_state.init_call_count++;

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

// Mock platform cleanup
void platform_cleanup(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  g_platform_mock_state.cleanup_call_count++;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
}

// Mock platform logging functions
int platform_log_error(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[ERROR] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int platform_log_warning(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[WARNING] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int platform_log_notice(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[NOTICE] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int platform_log_info(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[INFO] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

int platform_log_debug(const char* format, ...) {
  va_list args;
  va_start(args, format);
  printf("[DEBUG] ");
  int result = vprintf(format, args);
  printf("\n");
  va_end(args);
  return result;
}

// Mock platform utility functions
void platform_sleep_ms(uint32_t milliseconds) {
  // Mock implementation - just return immediately
  (void)milliseconds;
}

void platform_sleep_us(uint32_t microseconds) {
  // Mock implementation - just return immediately
  (void)microseconds;
}

uint64_t platform_get_time_ms(void) {
  pthread_mutex_lock(&g_platform_mock_state.mutex);
  uint64_t time_ms = (uint64_t)g_platform_mock_state.mock_time * MOCK_MS_PER_SECOND;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return time_ms;
}

// Mock platform configuration functions
platform_result_t platform_config_load(const char* filename) {
  (void)filename;
  pthread_mutex_lock(&g_platform_mock_state.mutex);

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t platform_config_save(const char* filename) {
  (void)filename;
  pthread_mutex_lock(&g_platform_mock_state.mutex);

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

const char* platform_config_get_string(const char* section, const char* key, // NOLINT
                                       const char* default_value) {
  (void)section;
  (void)key;
  return default_value;
}

int platform_config_get_int(const char* section, const char* key, int default_value) { // NOLINT
  (void)section;
  (void)key;
  return default_value;
}

// Mock platform system info
platform_result_t platform_get_system_info(platform_system_info_t* info) {
  if (!info) {
    return PLATFORM_ERROR_NULL;
  }

  pthread_mutex_lock(&g_platform_mock_state.mutex);

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  // Set mock system info
  info->cpu_usage = MOCK_CPU_USAGE_DEFAULT;
  info->cpu_temperature = MOCK_CPU_TEMPERATURE_DEFAULT;
  info->total_memory = (uint64_t)MOCK_MEMORY_1GB;
  info->free_memory = (uint64_t)MOCK_MEMORY_512MB;
  info->uptime_ms = MOCK_UPTIME_1HOUR_MS;

  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

/* ============================================================================
 * Video Input (VI) Mock Functions
 * ============================================================================ */

platform_result_t platform_vi_match_sensor(const char* isp_cfg_path) {
  (void)isp_cfg_path;
  pthread_mutex_lock(&g_platform_mock_state.mutex);

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_open(platform_vi_handle_t* handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  pthread_mutex_lock(&g_platform_mock_state.mutex);

  if (g_platform_mock_state.error_simulation_enabled) {
    platform_result_t error = g_platform_mock_state.error_code;
    pthread_mutex_unlock(&g_platform_mock_state.mutex);
    return error;
  }

  *handle = (platform_vi_handle_t)MOCK_HANDLE_VI;
  pthread_mutex_unlock(&g_platform_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

void platform_vi_close(platform_vi_handle_t handle) {
  (void)handle;
  // Mock implementation - no action needed
}

platform_result_t platform_vi_get_sensor_resolution(platform_vi_handle_t handle,
                                                    platform_video_resolution_t* resolution) {
  if (!resolution) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  resolution->width = MOCK_VIDEO_WIDTH_DEFAULT;
  resolution->height = MOCK_VIDEO_HEIGHT_DEFAULT;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_switch_day_night(platform_vi_handle_t handle,
                                               platform_daynight_mode_t mode) {
  (void)handle;
  (void)mode;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_set_flip_mirror(platform_vi_handle_t handle, bool flip, bool mirror) {
  (void)handle;
  (void)flip;
  (void)mirror;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_capture_on(platform_vi_handle_t handle) {
  (void)handle;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_start_global_capture(platform_vi_handle_t handle) {
  (void)handle;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_capture_off(platform_vi_handle_t handle) {
  (void)handle;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_set_channel_attr(platform_vi_handle_t handle,
                                               const platform_video_channel_attr_t* attr) {
  (void)handle;
  (void)attr;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_get_fps(platform_vi_handle_t handle, int* fps) {
  if (!fps) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  *fps = MOCK_FPS_DEFAULT;
  return PLATFORM_SUCCESS;
}

/* ============================================================================
 * Video Processing Subsystem (VPSS) Mock Functions
 * ============================================================================ */

platform_result_t platform_vpss_effect_set(platform_vi_handle_t handle,
                                           platform_vpss_effect_t effect, int value) { // NOLINT
  (void)handle;
  (void)effect;
  (void)value;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vpss_effect_get(platform_vi_handle_t handle,
                                           platform_vpss_effect_t effect, int* value) {
  if (!value) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  (void)effect;
  *value = MOCK_VPSS_EFFECT_VALUE_DEFAULT;
  return PLATFORM_SUCCESS;
}

/* ============================================================================
 * Video Encoder Mock Functions
 * ============================================================================ */

platform_result_t platform_venc_init(platform_venc_handle_t* handle,
                                     const platform_video_config_t* config) {
  if (!handle || !config) {
    return PLATFORM_ERROR_NULL;
  }

  *handle = (platform_venc_handle_t)MOCK_HANDLE_VENC;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_validate_venc_config(const platform_video_config_t* config) {
  if (!config) {
    return PLATFORM_ERROR_NULL;
  }

  return PLATFORM_SUCCESS;
}

void platform_venc_cleanup(platform_venc_handle_t handle) {
  (void)handle;
  // Mock implementation - no action needed
}

platform_result_t platform_venc_get_frame(platform_venc_handle_t handle, uint8_t** data,
                                          uint32_t* size) {
  if (!data || !size) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  *data = NULL; // Mock - no frame data
  *size = 0;
  return PLATFORM_SUCCESS;
}

void platform_venc_release_frame(platform_venc_handle_t handle, uint8_t* data) {
  (void)handle;
  (void)data;
  // Mock implementation - no action needed
}

platform_result_t platform_venc_get_stream(platform_venc_handle_t handle,
                                           platform_venc_stream_t* stream, uint32_t timeout_ms) {
  if (!stream) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  (void)timeout_ms;
  stream->data = NULL;
  stream->len = 0;
  stream->timestamp = 0;
  stream->is_keyframe = false;
  return PLATFORM_SUCCESS;
}

void platform_venc_release_stream(platform_venc_handle_t handle, platform_venc_stream_t* stream) {
  (void)handle;
  (void)stream;
  // Mock implementation - no action needed
}

platform_result_t platform_venc_request_stream(platform_vi_handle_t vi_handle, // NOLINT
                                               platform_venc_handle_t venc_handle,
                                               platform_venc_stream_handle_t* stream_handle) {
  if (!stream_handle) {
    return PLATFORM_ERROR_NULL;
  }

  (void)vi_handle;
  (void)venc_handle;
  *stream_handle = (platform_venc_stream_handle_t)MOCK_HANDLE_VENC_STREAM;
  return PLATFORM_SUCCESS;
}

void platform_venc_cancel_stream(platform_venc_stream_handle_t stream_handle) {
  (void)stream_handle;
  // Mock implementation - no action needed
}

platform_result_t platform_venc_get_stream_by_handle(platform_venc_stream_handle_t stream_handle,
                                                     platform_venc_stream_t* stream,
                                                     uint32_t timeout_ms) {
  if (!stream) {
    return PLATFORM_ERROR_NULL;
  }

  (void)stream_handle;
  (void)timeout_ms;
  stream->data = NULL;
  stream->len = 0;
  stream->timestamp = 0;
  stream->is_keyframe = false;
  return PLATFORM_SUCCESS;
}

void platform_venc_release_stream_by_handle(platform_venc_stream_handle_t stream_handle,
                                            platform_venc_stream_t* stream) {
  (void)stream_handle;
  (void)stream;
  // Mock implementation - no action needed
}

platform_result_t platform_venc_get_buffer_status(platform_venc_stream_handle_t stream_handle,
                                                  uint32_t* buffer_count, uint32_t* max_buffers,
                                                  uint32_t* overflow_count) {
  if (!buffer_count || !max_buffers || !overflow_count) {
    return PLATFORM_ERROR_NULL;
  }

  (void)stream_handle;
  *buffer_count = MOCK_BUFFER_COUNT_DEFAULT;
  *max_buffers = MOCK_MAX_BUFFERS_DEFAULT;
  *overflow_count = MOCK_OVERFLOW_COUNT_DEFAULT;
  return PLATFORM_SUCCESS;
}

/* ============================================================================
 * Audio Input (AI) Mock Functions
 * ============================================================================ */

platform_result_t platform_ai_open(platform_ai_handle_t* handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  *handle = (platform_ai_handle_t)MOCK_HANDLE_AI;
  return PLATFORM_SUCCESS;
}

void platform_ai_close(platform_ai_handle_t handle) {
  (void)handle;
  // Mock implementation - no action needed
}

/* ============================================================================
 * Audio Encoder Mock Functions
 * ============================================================================ */

platform_result_t platform_aenc_init(platform_aenc_stream_handle_t* handle,
                                     const platform_audio_config_t* config) {
  if (!handle || !config) {
    return PLATFORM_ERROR_NULL;
  }

  *handle = (platform_aenc_stream_handle_t)MOCK_HANDLE_AENC_STREAM;
  return PLATFORM_SUCCESS;
}

void platform_aenc_cleanup(platform_aenc_stream_handle_t handle) {
  (void)handle;
  // Mock implementation - no action needed
}

platform_result_t platform_aenc_get_frame(platform_aenc_stream_handle_t handle, uint8_t** data,
                                          uint32_t* size) {
  if (!data || !size) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  *data = NULL; // Mock - no frame data
  *size = 0;
  return PLATFORM_SUCCESS;
}

void platform_aenc_release_frame(platform_aenc_stream_handle_t handle, const uint8_t* data) {
  (void)handle;
  (void)data;
  // Mock implementation - no action needed
}

platform_result_t platform_aenc_get_stream(platform_aenc_stream_handle_t handle,
                                           platform_aenc_stream_t* stream, uint32_t timeout_ms) {
  if (!stream) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  (void)timeout_ms;
  stream->data = NULL;
  stream->len = 0;
  stream->timestamp = 0;
  return PLATFORM_SUCCESS;
}

void platform_aenc_release_stream(platform_aenc_stream_handle_t handle,
                                  platform_aenc_stream_t* stream) {
  (void)handle;
  (void)stream;
  // Mock implementation - no action needed
}

/* ============================================================================
 * Buffer Pool Mock Functions
 * ============================================================================ */

void* buffer_pool_get(buffer_pool_t* pool) {
  (void)pool;
  return malloc(MOCK_BUFFER_SIZE);
}

void buffer_pool_return(buffer_pool_t* pool, void* buffer) {
  (void)pool;
  if (buffer) {
    free(buffer);
  }
}

/* ============================================================================
 * PTZ Mock Functions (delegated to platform_ptz_mock.c)
 * ============================================================================ */
// PTZ functions are implemented in platform_ptz_mock.c

/* ============================================================================
 * IR LED Mock Functions
 * ============================================================================ */

platform_result_t platform_irled_init(int level) {
  (void)level;
  return PLATFORM_SUCCESS;
}

void platform_irled_cleanup(void) {
  // Mock implementation - no action needed
}

platform_result_t platform_irled_set_mode(platform_irled_mode_t mode) {
  (void)mode;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_irled_get_status(void) {
  return PLATFORM_SUCCESS;
}

/* ============================================================================
 * Snapshot Mock Functions
 * ============================================================================ */

platform_result_t platform_snapshot_init(platform_snapshot_handle_t* handle,
                                         platform_vi_handle_t vi_handle, int width,
                                         int height) { // NOLINT
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  (void)vi_handle;
  (void)width;
  (void)height;
  *handle = (platform_snapshot_handle_t)MOCK_HANDLE_SNAPSHOT;
  return PLATFORM_SUCCESS;
}

void platform_snapshot_cleanup(platform_snapshot_handle_t handle) {
  (void)handle;
  // Mock implementation - no action needed
}

platform_result_t platform_snapshot_capture(platform_snapshot_handle_t handle,
                                            platform_snapshot_t* snapshot, uint32_t timeout_ms) {
  if (!snapshot) {
    return PLATFORM_ERROR_NULL;
  }

  (void)handle;
  (void)timeout_ms;
  snapshot->data = NULL;
  snapshot->len = 0;
  snapshot->timestamp = 0;
  return PLATFORM_SUCCESS;
}

void platform_snapshot_release(platform_snapshot_handle_t handle, platform_snapshot_t* snapshot) {
  (void)handle;
  (void)snapshot;
  // Mock implementation - no action needed
}
