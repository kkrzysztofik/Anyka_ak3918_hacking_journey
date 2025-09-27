/**
 * @file platform_mock.c
 * @brief Mock implementation of platform functions for testing
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stddef.h>
#include <stdarg.h>
#include <setjmp.h>
#include <cmocka.h>

// Mock implementations of platform functions
// These will be used to replace actual platform calls during testing

/**
 * @brief Mock platform logging function
 * @param level Log level
 * @param format Format string
 * @param ... Variable arguments
 */
void mock_platform_log(int level, const char* format, ...) {
  (void)level; // Suppress unused parameter warning

  // In a real test, you might want to capture the log output
  // For now, we'll just verify the function was called
  check_expected(level);
  check_expected(format);

  // You could also capture the formatted message for verification
  va_list args;
  va_start(args, format);
  char buffer[1024];
  vsnprintf(buffer, sizeof(buffer), format, args);
  va_end(args);

  // Store the log message for later verification if needed
  // This would require additional test infrastructure
}

/**
 * @brief Mock platform initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_init(void) {
  // Mock can simulate success or failure based on test expectations
  return mock_type(int);
}

/**
 * @brief Mock platform cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_cleanup(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform get time function
 * @param time_val Pointer to store time value
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_time(time_t* time_val) {
  check_expected_ptr(time_val);

  if (time_val) {
    *time_val = mock_type(time_t);
  }

  return mock_type(int);
}

/**
 * @brief Mock platform get device name function
 * @param name Buffer to store device name
 * @param size Size of the buffer
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_device_name(char* name, size_t size) {
  check_expected_ptr(name);
  check_expected(size);

  const char* mock_name = mock_ptr_type(char*);
  if (name && mock_name) {
    strncpy(name, mock_name, size - 1);
    name[size - 1] = '\0';
  }

  return mock_type(int);
}

/**
 * @brief Mock platform get device capabilities function
 * @param capabilities Pointer to store capabilities
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_capabilities(void* capabilities) {
  check_expected_ptr(capabilities);

  // Mock can set specific capability values based on test expectations
  // This would require knowledge of the actual capabilities structure

  return mock_type(int);
}

/**
 * @brief Mock platform video initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_video_init(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform video cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_video_cleanup(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform network initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_network_init(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform network cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_network_cleanup(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform configuration load function
 * @param config_path Path to configuration file
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_load(const char* config_path) {
  check_expected(config_path);
  return mock_type(int);
}

/**
 * @brief Mock platform configuration save function
 * @param config_path Path to configuration file
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_save(const char* config_path) {
  check_expected(config_path);
  return mock_type(int);
}

/**
 * @brief Mock platform get configuration value function
 * @param key Configuration key
 * @param value Buffer to store value
 * @param size Size of the buffer
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_get(const char* key, char* value, size_t size) {
  check_expected(key);
  check_expected_ptr(value);
  check_expected(size);

  const char* mock_value = mock_ptr_type(char*);
  if (value && mock_value) {
    strncpy(value, mock_value, size - 1);
    value[size - 1] = '\0';
  }

  return mock_type(int);
}

/**
 * @brief Mock platform set configuration value function
 * @param key Configuration key
 * @param value Configuration value
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_set(const char* key, const char* value) {
  check_expected(key);
  check_expected(value);
  return mock_type(int);
}

/**
 * @brief Mock platform hardware initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_hardware_init(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform hardware cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_hardware_cleanup(void) {
  return mock_type(int);
}

/**
 * @brief Mock platform get system info function
 * @param info Pointer to store system info
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_system_info(void* info) {
  check_expected_ptr(info);

  // Mock can set specific system info values based on test expectations
  // This would require knowledge of the actual system info structure

  return mock_type(int);
}

/**
 * @brief Mock platform error handling function
 * @param error_code Error code
 * @param error_msg Error message
 */
void mock_platform_handle_error(int error_code, const char* error_msg) {
  check_expected(error_code);
  check_expected(error_msg);

  // Mock can simulate different error handling behaviors
  // based on test expectations
}

/**
 * @brief Mock platform error logging function
 * @param format Format string
 * @param ... Variable arguments
 */
void platform_log_error(const char* format, ...) {
  (void)format; // Suppress unused parameter warning
  // In unit tests, we just ignore logging to avoid spam
  // In more advanced tests, we might want to capture the output
}

/**
 * @brief Mock platform info logging function
 * @param format Format string
 * @param ... Variable arguments
 */
void platform_log_info(const char* format, ...) {
  (void)format; // Suppress unused parameter warning
  // In unit tests, we just ignore logging to avoid spam
}

/**
 * @brief Mock platform debug logging function
 * @param format Format string
 * @param ... Variable arguments
 */
void platform_log_debug(const char* format, ...) {
  (void)format; // Suppress unused parameter warning
  // In unit tests, we just ignore logging to avoid spam
}

/**
 * @brief Mock platform warning logging function
 * @param format Format string
 * @param ... Variable arguments
 */
void platform_log_warning(const char* format, ...) {
  (void)format; // Suppress unused parameter warning
  // In unit tests, we just ignore logging to avoid spam
}

/**
 * @brief Mock platform notice logging function
 * @param format Format string
 * @param ... Variable arguments
 */
void platform_log_notice(const char* format, ...) {
  (void)format; // Suppress unused parameter warning
  // In unit tests, we just ignore logging to avoid spam
}

// Buffer pool mock functions
struct buffer_pool_stats_t {
  int total_buffers;
  int available_buffers;
  int allocated_buffers;
};

int buffer_pool_init(void) {
  return 0; // Mock success
}

void buffer_pool_cleanup(void) {
  // Mock cleanup - do nothing
}

void* buffer_pool_get(void* pool) {
  (void)pool;
  return malloc(1024); // Return a mock buffer
}

void buffer_pool_return(void* pool, void* buffer) {
  (void)pool;
  free(buffer); // Free the mock buffer
}

int buffer_pool_get_stats(struct buffer_pool_stats_t* stats) {
  if (stats) {
    stats->total_buffers = 10;
    stats->available_buffers = 8;
    stats->allocated_buffers = 2;
  }
  return 0;
}

// Service handler mock functions
int onvif_service_handler_init(void* handler) {
  (void)handler;
  return 0; // Mock success
}

void onvif_service_handler_cleanup(void* handler) {
  (void)handler;
  // Mock cleanup - do nothing
}

int onvif_service_handler_handle_request(void* handler, void* request, void* response) {
  (void)handler;
  (void)request;
  (void)response;
  return 0; // Mock success
}

// Platform PTZ mock functions
int platform_ptz_init(void) {
  return 0; // Mock success
}

void platform_ptz_cleanup(void) {
  // Mock cleanup - do nothing
}

int platform_ptz_get_position(float* pan, float* tilt, float* zoom) {
  if (pan) *pan = 0.0f;
  if (tilt) *tilt = 0.0f;
  if (zoom) *zoom = 1.0f;
  return 0;
}

int platform_ptz_absolute_move(float pan, float tilt, float zoom) {
  (void)pan;
  (void)tilt;
  (void)zoom;
  return 0; // Mock success
}

int platform_ptz_continuous_move(float pan_speed, float tilt_speed, float zoom_speed) {
  (void)pan_speed;
  (void)tilt_speed;
  (void)zoom_speed;
  return 0; // Mock success
}

int platform_ptz_stop(void) {
  return 0; // Mock success
}

int platform_ptz_preset_set(int preset_id, const char* name) {
  (void)preset_id;
  (void)name;
  return 0; // Mock success
}

int platform_ptz_preset_goto(int preset_id) {
  (void)preset_id;
  return 0; // Mock success
}

int platform_ptz_preset_remove(int preset_id) {
  (void)preset_id;
  return 0; // Mock success
}

// Platform video/imaging mock functions
int platform_video_init(void) {
  return 0; // Mock success
}

void platform_video_cleanup(void) {
  // Mock cleanup - do nothing
}

int platform_video_get_frame(void* frame) {
  (void)frame;
  return 0; // Mock success
}

int platform_vpss_effect_set(int effect_type, float value) {
  (void)effect_type;
  (void)value;
  return 0; // Mock success
}

// HTTP response mock functions
int http_response_add_header(void* response, const char* name, const char* value) {
  (void)response;
  (void)name;
  (void)value;
  return 0; // Mock success
}

