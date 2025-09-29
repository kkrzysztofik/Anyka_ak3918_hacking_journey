/**
 * @file platform_mock.h
 * @brief Header for platform mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PLATFORM_MOCK_H
#define PLATFORM_MOCK_H

#include <stddef.h>
#include <time.h>

// Mock function declarations
// These replace the actual platform functions during testing

/**
 * @brief Mock platform logging function
 * @param level Log level
 * @param format Format string
 * @param ... Variable arguments
 */
void mock_platform_log(int level, const char* format, ...);

/**
 * @brief Mock platform initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_init(void);

/**
 * @brief Mock platform cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_cleanup(void);

/**
 * @brief Mock platform get time function
 * @param time_val Pointer to store time value
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_time(time_t* time_val);

/**
 * @brief Mock platform get device name function
 * @param name Buffer to store device name
 * @param size Size of the buffer
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_device_name(char* name, size_t size);

/**
 * @brief Mock platform get device capabilities function
 * @param capabilities Pointer to store capabilities
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_capabilities(void* capabilities);

/**
 * @brief Mock platform video initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_video_init(void);

/**
 * @brief Mock platform video cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_video_cleanup(void);

/**
 * @brief Mock platform network initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_network_init(void);

/**
 * @brief Mock platform network cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_network_cleanup(void);

/**
 * @brief Mock platform configuration load function
 * @param config_path Path to configuration file
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_load(const char* config_path);

/**
 * @brief Mock platform configuration save function
 * @param config_path Path to configuration file
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_save(const char* config_path);

/**
 * @brief Mock platform get configuration value function
 * @param key Configuration key
 * @param value Buffer to store value
 * @param size Size of the buffer
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_get(const char* key, char* value, size_t size);

/**
 * @brief Mock platform set configuration value function
 * @param key Configuration key
 * @param value Configuration value
 * @return 0 on success, -1 on failure
 */
int mock_platform_config_set(const char* key, const char* value);

/**
 * @brief Mock platform hardware initialization function
 * @return 0 on success, -1 on failure
 */
int mock_platform_hardware_init(void);

/**
 * @brief Mock platform hardware cleanup function
 * @return 0 on success, -1 on failure
 */
int mock_platform_hardware_cleanup(void);

/**
 * @brief Mock platform get system info function
 * @param info Pointer to store system info
 * @return 0 on success, -1 on failure
 */
int mock_platform_get_system_info(void* info);

/**
 * @brief Mock platform error handling function
 * @param error_code Error code
 * @param error_msg Error message
 */
void mock_platform_handle_error(int error_code, const char* error_msg);

/**
 * @brief Mock platform error logging function
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters printed
 */
int platform_log_error(const char* format, ...);

/**
 * @brief Mock platform info logging function
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters printed
 */
int platform_log_info(const char* format, ...);

/**
 * @brief Mock platform debug logging function
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters printed
 */
int platform_log_debug(const char* format, ...);

/**
 * @brief Mock platform warning logging function
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters printed
 */
int platform_log_warning(const char* format, ...);

// System mock functions
int mock_platform_set_system_result(int result);
int mock_platform_get_system_call_count(void);

// PTZ mock functions
#include "platform_ptz_mock.h"

/**
 * @brief Mock platform initialization function
 */
void platform_mock_init(void);

/**
 * @brief Mock platform cleanup function
 */
void platform_mock_cleanup(void);

#endif // PLATFORM_MOCK_H
