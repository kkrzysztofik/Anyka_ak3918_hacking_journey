/**
 * @file test_error_handling.c
 * @brief Unit tests for error handling utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/error/error_handling.h"

/**
 * @brief Test error handling initialization
 * @param state Test state (unused)
 */
static void test_error_handling_init(void** state) {
  (void)state;

  // Test error handling system initialization
  int result = onvif_error_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test that initialization is idempotent
  result = onvif_error_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_error_cleanup();
}

/**
 * @brief Test error handling cleanup
 * @param state Test state (unused)
 */
static void test_error_handling_cleanup(void** state) {
  (void)state;

  // Initialize first
  onvif_error_init();

  // Test cleanup (should not crash)
  onvif_error_cleanup();

  // Test multiple cleanups (should not crash)
  onvif_error_cleanup();
}

/**
 * @brief Test error code validation
 * @param state Test state (unused)
 */
static void test_error_code_validation(void** state) {
  (void)state;

  onvif_error_init();

  // Test valid error codes
  assert_true(onvif_is_valid_error_code(ONVIF_SUCCESS));
  assert_true(onvif_is_valid_error_code(ONVIF_ERROR_INVALID));
  assert_true(onvif_is_valid_error_code(ONVIF_ERROR_MEMORY));
  assert_true(onvif_is_valid_error_code(ONVIF_ERROR_NOT_FOUND));
  assert_true(onvif_is_valid_error_code(ONVIF_ERROR_AUTH_FAILED));

  // Test invalid error codes
  assert_false(onvif_is_valid_error_code(-9999));
  assert_false(onvif_is_valid_error_code(9999));

  onvif_error_cleanup();
}

/**
 * @brief Test error message retrieval
 * @param state Test state (unused)
 */
static void test_error_message_retrieval(void** state) {
  (void)state;

  onvif_error_init();

  // Test getting error messages for valid codes
  const char* msg = onvif_get_error_message(ONVIF_SUCCESS);
  assert_non_null(msg);
  assert_true(strlen(msg) > 0);

  msg = onvif_get_error_message(ONVIF_ERROR_INVALID);
  assert_non_null(msg);
  assert_true(strlen(msg) > 0);

  msg = onvif_get_error_message(ONVIF_ERROR_MEMORY);
  assert_non_null(msg);
  assert_true(strlen(msg) > 0);

  msg = onvif_get_error_message(ONVIF_ERROR_NOT_FOUND);
  assert_non_null(msg);
  assert_true(strlen(msg) > 0);

  // Test getting error message for invalid code
  msg = onvif_get_error_message(-9999);
  assert_non_null(msg); // Should return a default message
  assert_true(strlen(msg) > 0);

  onvif_error_cleanup();
}

/**
 * @brief Test error context setting and getting
 * @param state Test state (unused)
 */
static void test_error_context(void** state) {
  (void)state;

  onvif_error_init();

  // Test setting error context
  onvif_set_error_context("test_function", "test_file.c", 123);

  // Test getting error context
  const char* function = onvif_get_error_function();
  const char* file = onvif_get_error_file();
  int line = onvif_get_error_line();

  assert_non_null(function);
  assert_string_equal(function, "test_function");
  assert_non_null(file);
  assert_string_equal(file, "test_file.c");
  assert_int_equal(line, 123);

  // Test setting with NULL values
  onvif_set_error_context(NULL, NULL, 0);

  function = onvif_get_error_function();
  file = onvif_get_error_file();
  line = onvif_get_error_line();

  // Should handle NULL gracefully
  (void)function;
  (void)file;
  (void)line;

  onvif_error_cleanup();
}

/**
 * @brief Test error logging
 * @param state Test state (unused)
 */
static void test_error_logging(void** state) {
  (void)state;

  onvif_error_init();

  // Test logging errors with different codes
  onvif_log_error(ONVIF_ERROR_INVALID, "Test invalid parameter error");
  onvif_log_error(ONVIF_ERROR_MEMORY, "Test memory allocation error");
  onvif_log_error(ONVIF_ERROR_NOT_FOUND, "Test resource not found error");

  // Test logging with NULL message (should handle gracefully)
  onvif_log_error(ONVIF_ERROR_INVALID, NULL);

  // Test logging with invalid error code
  onvif_log_error(-9999, "Test invalid error code");

  onvif_error_cleanup();
}

/**
 * @brief Test last error tracking
 * @param state Test state (unused)
 */
static void test_last_error_tracking(void** state) {
  (void)state;

  onvif_error_init();

  // Initially should have no error
  int last_error = onvif_get_last_error();
  assert_int_equal(last_error, ONVIF_SUCCESS);

  // Set an error
  onvif_set_last_error(ONVIF_ERROR_INVALID);
  last_error = onvif_get_last_error();
  assert_int_equal(last_error, ONVIF_ERROR_INVALID);

  // Set another error
  onvif_set_last_error(ONVIF_ERROR_MEMORY);
  last_error = onvif_get_last_error();
  assert_int_equal(last_error, ONVIF_ERROR_MEMORY);

  // Clear the error
  onvif_clear_last_error();
  last_error = onvif_get_last_error();
  assert_int_equal(last_error, ONVIF_SUCCESS);

  onvif_error_cleanup();
}

/**
 * @brief Test error string formatting
 * @param state Test state (unused)
 */
static void test_error_string_formatting(void** state) {
  (void)state;

  onvif_error_init();

  char buffer[256];

  // Test formatting error string with context
  onvif_set_error_context("test_func", "test.c", 42);
  onvif_set_last_error(ONVIF_ERROR_INVALID);

  int result = onvif_format_error_string(buffer, sizeof(buffer));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(strlen(buffer) > 0);

  // Test formatting with NULL buffer
  result = onvif_format_error_string(NULL, sizeof(buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test formatting with zero buffer size
  result = onvif_format_error_string(buffer, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test formatting with small buffer
  char small_buffer[4];
  result = onvif_format_error_string(small_buffer, sizeof(small_buffer));
  assert_int_equal(result, ONVIF_ERROR_BUFFER_TOO_SMALL);

  onvif_error_cleanup();
}

// Global variables for callback testing
static int g_callback_called = 0;
static int g_callback_error_code = 0;

// Simple test callback function
static void test_error_callback(int error_code, const char* message, void* user_data) {
  (void)message;
  (void)user_data;
  g_callback_called++;
  g_callback_error_code = error_code;
}

/**
 * @brief Test error callback registration
 * @param state Test state (unused)
 */
static void test_error_callback_registration(void** state) {
  (void)state;

  onvif_error_init();

  // Reset callback state
  g_callback_called = 0;
  g_callback_error_code = 0;

  // Register callback
  int result = onvif_register_error_callback(test_error_callback, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Trigger an error
  onvif_log_error(ONVIF_ERROR_MEMORY, "Test callback error");

  // Check if callback was called
  assert_int_equal(g_callback_called, 1);
  assert_int_equal(g_callback_error_code, ONVIF_ERROR_MEMORY);

  // Test registering NULL callback
  result = onvif_register_error_callback(NULL, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_error_cleanup();
}

/**
 * @brief Test error statistics
 * @param state Test state (unused)
 */
static void test_error_statistics(void** state) {
  (void)state;

  onvif_error_init();

  // Reset statistics
  onvif_reset_error_statistics();

  // Initially should have no errors
  int total_errors = onvif_get_total_error_count();
  assert_int_equal(total_errors, 0);

  int specific_errors = onvif_get_error_count(ONVIF_ERROR_INVALID);
  assert_int_equal(specific_errors, 0);

  // Log some errors
  onvif_log_error(ONVIF_ERROR_INVALID, "Test error 1");
  onvif_log_error(ONVIF_ERROR_INVALID, "Test error 2");
  onvif_log_error(ONVIF_ERROR_MEMORY, "Test error 3");

  // Check statistics
  total_errors = onvif_get_total_error_count();
  assert_int_equal(total_errors, 3);

  specific_errors = onvif_get_error_count(ONVIF_ERROR_INVALID);
  assert_int_equal(specific_errors, 2);

  specific_errors = onvif_get_error_count(ONVIF_ERROR_MEMORY);
  assert_int_equal(specific_errors, 1);

  specific_errors = onvif_get_error_count(ONVIF_ERROR_NOT_FOUND);
  assert_int_equal(specific_errors, 0);

  // Reset and check
  onvif_reset_error_statistics();
  total_errors = onvif_get_total_error_count();
  assert_int_equal(total_errors, 0);

  onvif_error_cleanup();
}

/**
 * @brief Test error macros
 * @param state Test state (unused)
 */
static void test_error_macros(void** state) {
  (void)state;

  onvif_error_init();

  // Test ONVIF_RETURN_IF_ERROR macro
  int test_result = ONVIF_ERROR_INVALID;

// This should return early due to the error
#define TEST_FUNC_WITH_MACRO()                                                                     \
  do {                                                                                             \
    ONVIF_RETURN_IF_ERROR(test_result);                                                            \
    assert_fail(); /* Should not reach here */                                                     \
  } while (0)

  // For now, just verify the error codes are properly defined
  assert_true(ONVIF_SUCCESS == 0);
  assert_true(ONVIF_ERROR_INVALID < 0);
  assert_true(ONVIF_ERROR_MEMORY < 0);
  assert_true(ONVIF_ERROR_NOT_FOUND < 0);

  onvif_error_cleanup();
}
