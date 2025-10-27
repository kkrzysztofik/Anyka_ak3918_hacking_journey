/**
 * @file test_helpers.h
 * @brief Common test helper functions to reduce code duplication in unit tests
 * @author kkrzysztofik
 * @date 2025
 *
 * This library provides reusable test helpers for:
 * - Service callback registration testing
 * - NULL parameter validation testing
 * - Mock setup/teardown patterns
 * - Common assertion patterns
 *
 * Benefits:
 * - Eliminates ~500 lines of duplicated test code
 * - Standardizes test patterns across services
 * - Makes tests more maintainable and easier to write
 */

#ifndef TEST_HELPERS_H
#define TEST_HELPERS_H

#include <pthread.h>
#include <stddef.h>
#include <sys/time.h>

// Forward declarations for types used in function parameters
struct http_auth_config;
struct ptz_vector;
struct ptz_speed;
struct ptz_preset;

#include "cmocka_wrapper.h"
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "services/ptz/onvif_ptz.h" // Added for struct ptz_vector

/* ============================================================================
 * Constants
 * ============================================================================ */

// Buffer sizes for test data
#define TEST_PARAM_DESCRIPTION_SIZE 128
#define TEST_OPERATION_NAME_SIZE    64

/* ============================================================================
 * Type Definitions
 * ============================================================================ */

/**
 * @brief Configuration for service callback testing
 *
 * This structure encapsulates all the information needed to test a service's
 * callback registration and dispatch behavior.
 */
typedef struct {
  /** Service name (e.g., "ptz", "Media", "Imaging") */
  const char* service_name;

  /** Service namespace URI */
  const char* namespace_uri;

  /** Service initialization function */
  int (*init_func)(void* config);

  /** Service cleanup function */
  void (*cleanup_func)(void);

  /** Whether the service requires platform initialization */
  int requires_platform_init;

  /** Expected result from successful initialization */
  int expected_init_success;

  /** Expected result from failed initialization */
  int expected_init_failure;
} service_test_config_t;

/**
 * @brief Configuration for NULL parameter testing
 *
 * Describes which parameters to test with NULL values and what results to expect.
 */
typedef struct {
  /** Description of the parameter being tested (for error messages) */
  char param_description[TEST_PARAM_DESCRIPTION_SIZE];

  /** Index of the parameter to set to NULL (0-based) */
  int param_index;

  /** Expected return code when parameter is NULL */
  int expected_result;
} null_param_test_t;

/* ============================================================================
 * Service Callback Test Helpers
 * ============================================================================ */

/**
 * @brief Test successful service registration
 *
 * This helper tests that a service correctly registers with the service
 * dispatcher, including verifying all registration parameters.
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_registration_success(void** state, const service_test_config_t* config);

/**
 * @brief Test service registration with duplicate error
 *
 * This helper tests that a service correctly handles the case where
 * the service dispatcher returns a duplicate registration error.
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_registration_duplicate(void** state, const service_test_config_t* config);

/**
 * @brief Test service registration with NULL config
 *
 * This helper tests that a service can handle NULL config (if supported).
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_registration_null_config(void** state, const service_test_config_t* config);

/**
 * @brief Test service registration with dispatcher failure
 *
 * This helper tests that a service correctly handles dispatcher failures.
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_registration_dispatcher_failure(void** state, const service_test_config_t* config);

/**
 * @brief Test service unregistration success
 *
 * This helper tests that a service correctly unregisters from the service
 * dispatcher.
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_unregistration_success(void** state, const service_test_config_t* config);

/**
 * @brief Test service unregistration when not initialized
 *
 * This helper tests that unregistering a service that was never initialized
 * is handled correctly.
 *
 * @param state CMocka test state
 * @param config Service test configuration
 */
void test_helper_service_unregistration_not_initialized(void** state, const service_test_config_t* config);

/**
 * @brief Verify service registration data
 *
 * This helper verifies that a service's registration data matches expectations.
 *
 * @param registration The registration data to verify
 * @param config Service test configuration
 */
void test_helper_verify_service_registration(const onvif_service_registration_t* registration, const service_test_config_t* config);

/* ============================================================================
 * NULL Parameter Test Helpers
 * ============================================================================ */

/**
 * @brief Test a function with NULL parameters
 *
 * This helper runs a series of tests where individual parameters are set to
 * NULL to verify proper NULL handling.
 *
 * @param state CMocka test state
 * @param function_name Name of the function being tested (for error messages)
 * @param test_func Function pointer to the test function
 * @param tests Array of NULL parameter test configurations
 * @param test_count Number of tests in the array
 */
void test_helper_null_parameters(void** state, const char* function_name, void (*test_func)(void**, const null_param_test_t*),
                                 const null_param_test_t* tests, int test_count);

// Generic NULL parameter test wrappers
void test_helper_null_param_2_args(void** state, const null_param_test_t* test_config);
void test_helper_null_param_3_args(void** state, const null_param_test_t* test_config);
void test_helper_null_param_4_args(void** state, const null_param_test_t* test_config);

// Helper to create NULL parameter test configurations
// Helper to create NULL parameter test configurations
// Helper to create NULL parameter test configurations
null_param_test_t test_helper_create_null_test(const char* description, int param_index, int expected_result);

/* ============================================================================
 * Test State Management System
 * ============================================================================ */

/**
 * @brief Test state configuration structure
 */
typedef struct {
  void (*reset_func)(void);   // Custom reset function
  void (*cleanup_func)(void); // Cleanup function to call
  int** counters;             // Array of counter pointers
  int counter_count;          // Number of counters
} test_state_config_t;

// Test state management functions
void test_helper_reset_state(const test_state_config_t* config);
test_state_config_t test_helper_create_state_config(void (*reset_func)(void), void (*cleanup_func)(void), int* counters, int counter_count);

/* ============================================================================
 * Service-Specific Test Helper Functions
 * ============================================================================ */

// HTTP test helpers
int test_helper_http_build_basic_auth_header(const char* username, const char* password, char* header_value, size_t header_size);
int test_helper_http_init_auth_config(struct http_auth_config* config, int auth_type, int enabled);
int test_helper_http_create_request(const char* method, const char* uri, http_request_t* request);
int test_helper_http_create_response(int status_code, http_response_t* response);

// PTZ test helpers
int test_helper_ptz_create_test_position(struct ptz_vector* position, float pan, float tilt, float zoom);
int test_helper_ptz_create_test_speed(struct ptz_speed* speed, float pan_tilt, float zoom);
int test_helper_ptz_create_test_preset(struct ptz_preset* preset, const char* token, const char* name);

// Macro to simplify state management
#define TEST_HELPER_DECLARE_COUNTERS(name, ...)                                                                                                      \
  static int g_##name##_counter_values[] = {__VA_ARGS__};                                                                                            \
  static int* g_##name##_counters[] = {&g_##name##_counter_values[0], &g_##name##_counter_values[1], &g_##name##_counter_values[2],                  \
                                       &g_##name##_counter_values[3]};                                                                               \
  static void reset_##name##_state(void) {                                                                                                           \
    test_state_config_t config = {                                                                                                                   \
      .counters = g_##name##_counters,                                                                                                               \
      .counter_count = sizeof(g_##name##_counter_values) / sizeof(int),                                                                              \
    };                                                                                                                                               \
    test_helper_reset_state(&config);                                                                                                                \
  };

/* ============================================================================
 * Generic Mock Handler System
 * ============================================================================ */

/**
 * @brief Generic mock handler state structure
 */
typedef struct {
  int init_call_count;
  int cleanup_call_count;
  int operation_call_count;
  int init_result;
  void* last_request;
  void* last_response;
  char last_operation[TEST_OPERATION_NAME_SIZE];
} generic_mock_handler_state_t;

// Generic mock handler functions
int test_helper_generic_init_handler(generic_mock_handler_state_t* state);
void test_helper_generic_cleanup_handler(generic_mock_handler_state_t* state);
int test_helper_generic_operation_handler(generic_mock_handler_state_t* state, const char* operation, const void* request, void* response);
void test_helper_reset_generic_mock_state(generic_mock_handler_state_t* state);

// Macro to create service-specific mock handlers easily
#define TEST_HELPER_CREATE_MOCK_HANDLERS(service_name)                                                                                               \
  static generic_mock_handler_state_t g_##service_name##_mock_state = {0};                                                                           \
                                                                                                                                                     \
  static int service_name##_mock_init(void) {                                                                                                        \
    return test_helper_generic_init_handler(&g_##service_name##_mock_state);                                                                         \
  }                                                                                                                                                  \
                                                                                                                                                     \
  static void service_name##_mock_cleanup(void) {                                                                                                    \
    test_helper_generic_cleanup_handler(&g_##service_name##_mock_state);                                                                             \
  }                                                                                                                                                  \
                                                                                                                                                     \
  static int service_name##_mock_operation(const char* operation_name, const http_request_t* request, http_response_t* response) {                   \
    return test_helper_generic_operation_handler(&g_##service_name##_mock_state, operation_name, (const void*)request, (void*)response);             \
  }                                                                                                                                                  \
                                                                                                                                                     \
  static void service_name##_reset_mock_state(void) {                                                                                                \
    test_helper_reset_generic_mock_state(&g_##service_name##_mock_state);                                                                            \
  }

/* ============================================================================
 * Memory and Performance Measurement Helpers
 * ============================================================================ */

/**
 * @brief Get current memory usage in bytes
 *
 * Reads the VmRSS (Resident Set Size) from /proc/self/status on Linux systems.
 * On non-Linux systems or if reading fails, returns 0.
 *
 * @return Current memory usage in bytes, or 0 if unavailable
 * @note This is Linux-specific and may not work on other platforms
 */
size_t test_helper_get_memory_usage(void);

/* ============================================================================
 * Mock Setup/Teardown Helpers
 * ============================================================================ */

/* NOTE: Custom mock framework helpers have been removed as part of CMocka migration.
 * Use CMocka's built-in mocking patterns (will_return, expect_function_call) directly.
 */

/* ============================================================================
 * Common Assertion Helpers
 * ============================================================================ */

/**
 * @brief Assert that a pointer is non-NULL with descriptive message
 *
 * @param ptr Pointer to check
 * @param description Description of what the pointer represents
 */
void test_helper_assert_non_null(const void* ptr, const char* description);

/**
 * @brief Assert that two strings match with descriptive message
 *
 * @param actual Actual string value
 * @param expected Expected string value
 * @param description Description of what is being compared
 */
void test_helper_assert_string_equal(const char* actual, const char* expected, const char* description);

/**
 * @brief Assert that an integer matches expected value with descriptive message
 *
 * @param actual Actual integer value
 * @param expected Expected integer value
 * @param description Description of what is being compared
 */
void test_helper_assert_int_equal(int actual, int expected, const char* description);

/**
 * @brief Assert that a mock function was called expected number of times
 *
 * @param actual_count Actual call count
 * @param expected_count Expected call count
 * @param function_name Name of the mocked function
 */
void test_helper_assert_mock_called(int actual_count, int expected_count, const char* function_name);

/* ============================================================================
 * Test Data Initialization Helpers
 * ============================================================================ */

/**
 * @brief Initialize test HTTP request structure
 *
 * @param request Pointer to HTTP request structure to initialize
 */
void test_helper_init_http_request(void* request);

/**
 * @brief Initialize test HTTP response structure
 *
 * @param response Pointer to HTTP response structure to initialize
 */
void test_helper_init_http_response(void* response);

/**
 * @brief Create a simple service test configuration
 *
 * This is a convenience function for creating basic service test configs.
 *
 * @param service_name Service name
 * @param namespace_uri Namespace URI
 * @param init_func Initialization function
 * @param cleanup_func Cleanup function
 * @return Initialized service_test_config_t structure
 */
service_test_config_t test_helper_create_service_config(const char* service_name, const char* namespace_uri, int (*init_func)(void*),
                                                        void (*cleanup_func)(void));

// Service Dispatch Helpers
void test_helper_service_dispatch_success(void** state, const service_test_config_t* config, const char* operation, http_request_t* request,
                                          http_response_t* response);
void test_helper_service_dispatch_unknown_operation(void** state, const service_test_config_t* config, const char* operation, http_request_t* request,
                                                    http_response_t* response);
void test_helper_service_dispatch_null_service(void** state, const char* operation, http_request_t* request, http_response_t* response);
void test_helper_service_dispatch_null_operation(void** state, const char* service_name, http_request_t* request, http_response_t* response);
void test_helper_service_dispatch_null_request(void** state, const char* service_name, const char* operation, http_response_t* response);
void test_helper_service_dispatch_null_response(void** state, const char* service_name, const char* operation, http_request_t* request);

// Operation Handler Helpers
void test_helper_operation_handler_success(void** state, const service_test_config_t* config, const char* operation, http_request_t* request,
                                           http_response_t* response);
void test_helper_operation_handler_null_operation(void** state, http_request_t* request, http_response_t* response);
void test_helper_operation_handler_null_request(void** state, const char* operation, http_response_t* response);
void test_helper_operation_handler_null_response(void** state, const char* operation, http_request_t* request);
void test_helper_operation_handler_unknown_operation(void** state, const char* operation, http_request_t* request, http_response_t* response);

// Error Handling Helpers
void test_helper_service_registration_failure_handling(void** state, const service_test_config_t* config, int error_code);
void test_helper_service_dispatch_failure_handling(void** state, const service_test_config_t* config, const char* operation, http_request_t* request,
                                                   http_response_t* response);
void test_helper_service_unregistration_failure_handling(void** state, const service_test_config_t* config);

// Logging Helpers
void test_helper_service_callback_logging_success(void** state, const service_test_config_t* config, const char* operation, http_request_t* request,
                                                  http_response_t* response);
void test_helper_service_callback_logging_failure(void** state, const service_test_config_t* config);

/**
 * @brief Get the absolute path of a test resource file
 * @param relative_path Relative path from tests directory (e.g., "configs/test.ini")
 * @param output_buffer Buffer to store the absolute path
 * @param buffer_size Size of the output buffer
 * @return 0 on success, -1 on failure
 */
int test_helper_get_test_resource_path(const char* relative_path, char* output_buffer, size_t buffer_size);

/* ============================================================================
 * Generic Mock Framework Helpers
 * ============================================================================ */

#endif /* TEST_HELPERS_H */
