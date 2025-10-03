/**
 * @file test_logging_utils.c
 * @brief Unit tests for logging utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/logging/logging_utils.h"

/**
 * @brief Test service initialization logging
 * @param state Test state (unused)
 */
void test_unit_logging_init(void** state) {
  (void)state;

  // Test service initialization success logging (should not crash)
  log_service_init_success("TestService");
  log_service_init_success("DEVICE");
  log_service_init_success("MEDIA");

  // Test with NULL service name (should handle gracefully)
  log_service_init_success(NULL);
}

/**
 * @brief Test service cleanup logging
 * @param state Test state (unused)
 */
void test_unit_logging_cleanup(void** state) {
  (void)state;

  // Test service cleanup logging (should not crash)
  log_service_cleanup("TestService");
  log_service_cleanup("DEVICE");
  log_service_cleanup("MEDIA");

  // Test with NULL service name (should handle gracefully)
  log_service_cleanup(NULL);
}

/**
 * @brief Test error logging functions
 * @param state Test state (unused)
 */
void test_unit_log_level(void** state) {
  (void)state;

  // Test service initialization failure logging
  log_service_init_failure("TestService", "Initialization failed");
  log_service_init_failure("DEVICE", "Hardware not available");

  // Test with NULL parameters (should handle gracefully)
  log_service_init_failure(NULL, "Error message");
  log_service_init_failure("TestService", NULL);
  log_service_init_failure(NULL, NULL);
}

/**
 * @brief Test parameter validation logging
 * @param state Test state (unused)
 */
void test_unit_basic_logging(void** state) {
  (void)state;

  // Test invalid parameters logging
  log_invalid_parameters("test_function");
  log_invalid_parameters("onvif_device_init");

  // Test with NULL function name (should handle gracefully)
  log_invalid_parameters(NULL);

  // Test service not initialized logging
  log_service_not_initialized("TestService");
  log_service_not_initialized("DEVICE");

  // Test with NULL service name (should handle gracefully)
  log_service_not_initialized(NULL);
}

/**
 * @brief Test operation logging functions
 * @param state Test state (unused)
 */
void test_unit_service_logging(void** state) {
  (void)state;

  // Test operation success logging
  log_operation_success("Device initialization");
  log_operation_success("Media profile creation");
  log_operation_success("PTZ movement");

  // Test with NULL operation (should handle gracefully)
  log_operation_success(NULL);

  // Test operation failure logging
  log_operation_failure("Device initialization", "Hardware not found");
  log_operation_failure("Media profile creation", "Invalid parameters");

  // Test with NULL parameters (should handle gracefully)
  log_operation_failure(NULL, "Error message");
  log_operation_failure("Operation", NULL);
  log_operation_failure(NULL, NULL);
}

/**
 * @brief Test configuration and platform logging
 * @param state Test state (unused)
 */
void test_unit_platform_logging(void** state) {
  (void)state;

  // Test configuration update logging
  log_config_updated("video_settings");
  log_config_updated("network_config");
  log_config_updated("ptz_presets");

  // Test with NULL config type (should handle gracefully)
  log_config_updated(NULL);

  // Test platform operation failure logging
  log_platform_operation_failure("video_init", "Driver not loaded");
  log_platform_operation_failure("network_setup", "Interface not available");

  // Test with NULL parameters (should handle gracefully)
  log_platform_operation_failure(NULL, "Error message");
  log_platform_operation_failure("Operation", NULL);
  log_platform_operation_failure(NULL, NULL);
}

/**
 * @brief Get logging utils unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_logging_utils_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_logging_init),
    cmocka_unit_test(test_unit_logging_cleanup),
    cmocka_unit_test(test_unit_log_level),
    cmocka_unit_test(test_unit_basic_logging),
    cmocka_unit_test(test_unit_service_logging),
    cmocka_unit_test(test_unit_platform_logging),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
