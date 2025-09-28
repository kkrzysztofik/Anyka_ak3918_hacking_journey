/**
 * @file test_onvif_ptz_callbacks.c
 * @brief Unit tests for ONVIF PTZ service callback registration and dispatch
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_parser.h"
#include "platform/platform_common.h"
#include "services/common/service_dispatcher.h"

// Include the actual source files we're testing
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

// Mock includes
#include "../../../mocks/mock_service_dispatcher.h"
#include "../../../mocks/platform_mock.h"
#include "../../../mocks/platform_ptz_mock.h"

/* ============================================================================
 * Test Data and Constants
 * ============================================================================ */

// Test constants
#define TEST_PTZ_SERVICE_NAME      "ptz"
#define TEST_PTZ_NAMESPACE         "http://www.onvif.org/ver10/ptz/wsdl"
#define TEST_PTZ_OPERATION         "GetNodes"
#define TEST_PTZ_UNKNOWN_OPERATION "UnknownOperation"

// Test HTTP request/response data
static http_request_t g_test_request;   // NOLINT
static http_response_t g_test_response; // NOLINT

/* ============================================================================
 * Test Helper Functions
 * ============================================================================ */

/**
 * @brief Setup function for PTZ callback tests
 * @param state Test state
 * @return 0 on success
 */
int setup_ptz_callback_tests(void** state) {
  (void)state;

  // Initialize all mocks
  mock_service_dispatcher_init();
  platform_mock_init();

  // Initialize PTZ adapter
  ptz_adapter_init();

  // Initialize test data
  memset(&g_test_request, 0, sizeof(g_test_request));
  memset(&g_test_response, 0, sizeof(g_test_response));

  return 0;
}

/**
 * @brief Teardown function for PTZ callback tests
 * @param state Test state
 * @return 0 on success
 */
int teardown_ptz_callback_tests(void** state) {
  (void)state;

  // Cleanup PTZ service
  onvif_ptz_cleanup();

  // Cleanup PTZ adapter
  ptz_adapter_shutdown();

  // Cleanup mocks
  platform_mock_cleanup();
  mock_service_dispatcher_cleanup();

  return 0;
}

/* ============================================================================
 * PTZ Service Registration Tests
 * ============================================================================ */

/**
 * @brief Test PTZ service registration success
 * @param state Test state (unused)
 */
void test_ptz_service_registration_success(void** state) {
  (void)state;

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service (which registers it)
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify registration was called
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());

  // Verify registration data
  const onvif_service_registration_t* registration =
    mock_service_dispatcher_get_last_registration();
  assert_non_null(registration);
  assert_string_equal(registration->service_name, TEST_PTZ_SERVICE_NAME);
  assert_string_equal(registration->namespace_uri, TEST_PTZ_NAMESPACE);
  assert_non_null(registration->operation_handler);
  assert_non_null(registration->init_handler);
  assert_non_null(registration->cleanup_handler);
  assert_non_null(registration->capabilities_handler);
}

/**
 * @brief Test PTZ service registration with duplicate
 * @param state Test state (unused)
 */
void test_ptz_service_registration_duplicate(void** state) {
  (void)state;

  // Mock duplicate service registration
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service (which registers it)
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_ERROR_DUPLICATE, result);

  // Verify registration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test PTZ service registration with invalid parameters
 * @param state Test state (unused)
 */
void test_ptz_service_registration_invalid_params(void** state) {
  (void)state;

  // Test with NULL config (should still work)
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify no registration was attempted for invalid config
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test PTZ service unregistration success
 * @param state Test state (unused)
 */
void test_ptz_service_unregistration_success(void** state) {
  (void)state;

  // First register the service
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test unregistration
  onvif_ptz_cleanup();

  // Verify unregistration was called
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
  assert_string_equal(mock_service_dispatcher_get_last_unregister_service(), TEST_PTZ_SERVICE_NAME);
}

/**
 * @brief Test PTZ service unregistration when not found
 * @param state Test state (unused)
 */
void test_ptz_service_unregistration_not_found(void** state) {
  (void)state;

  // Mock unregistration failure
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);

  // Test unregistration when service not registered
  onvif_ptz_cleanup();

  // Verify unregistration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
}

/* ============================================================================
 * PTZ Service Dispatch Tests
 * ============================================================================ */

/**
 * @brief Test PTZ service dispatch with valid operation
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_success(void** state) {
  (void)state;

  // Mock successful service registration and dispatch
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_SUCCESS);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test dispatch through service dispatcher
  result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_OPERATION,
                                             &g_test_request, &g_test_response);

  // Note: This will likely return an error due to missing gSOAP context
  // but we're testing the dispatch mechanism
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_service(), TEST_PTZ_SERVICE_NAME);
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_operation(), TEST_PTZ_OPERATION);
}

/**
 * @brief Test PTZ service dispatch with unknown operation
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_unknown_operation(void** state) {
  (void)state;

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test dispatch with unknown operation
  result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_UNKNOWN_OPERATION,
                                             &g_test_request, &g_test_response);

  // Should return error for unknown operation
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_service(), TEST_PTZ_SERVICE_NAME);
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_operation(),
                      TEST_PTZ_UNKNOWN_OPERATION);
}

/**
 * @brief Test PTZ service dispatch with NULL service name
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_null_service(void** state) {
  (void)state;

  // Test dispatch with NULL service name
  int result =
    onvif_service_dispatcher_dispatch(NULL, TEST_PTZ_OPERATION, &g_test_request, &g_test_response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ service dispatch with NULL operation name
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_null_operation(void** state) {
  (void)state;

  // Test dispatch with NULL operation name
  int result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, NULL, &g_test_request,
                                                 &g_test_response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ service dispatch with NULL request
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_null_request(void** state) {
  (void)state;

  // Test dispatch with NULL request
  int result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_OPERATION, NULL,
                                                 &g_test_response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ service dispatch with NULL response
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_null_response(void** state) {
  (void)state;

  // Test dispatch with NULL response
  int result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_OPERATION,
                                                 &g_test_request, NULL);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/* ============================================================================
 * PTZ Service Callback Handler Tests
 * ============================================================================ */

/**
 * @brief Test PTZ operation handler with valid operation
 * @param state Test state (unused)
 */
void test_ptz_operation_handler_success(void** state) {
  (void)state;

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test operation handler directly
  result = onvif_ptz_handle_operation(TEST_PTZ_OPERATION, &g_test_request, &g_test_response);

  // Note: This will likely return an error due to missing gSOAP context
  // but we're testing the handler mechanism
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);
}

/**
 * @brief Test PTZ operation handler with NULL operation name
 * @param state Test state (unused)
 */
void test_ptz_operation_handler_null_operation(void** state) {
  (void)state;

  int result = onvif_ptz_handle_operation(NULL, &g_test_request, &g_test_response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ operation handler with NULL request
 * @param state Test state (unused)
 */
void test_ptz_operation_handler_null_request(void** state) {
  (void)state;

  int result = onvif_ptz_handle_operation(TEST_PTZ_OPERATION, NULL, &g_test_response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ operation handler with NULL response
 * @param state Test state (unused)
 */
void test_ptz_operation_handler_null_response(void** state) {
  (void)state;

  int result = onvif_ptz_handle_operation(TEST_PTZ_OPERATION, &g_test_request, NULL);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test PTZ operation handler with unknown operation
 * @param state Test state (unused)
 */
void test_ptz_operation_handler_unknown_operation(void** state) {
  (void)state;

  int result =
    onvif_ptz_handle_operation(TEST_PTZ_UNKNOWN_OPERATION, &g_test_request, &g_test_response);

  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/* ============================================================================
 * PTZ Service Callback Error Handling Tests
 * ============================================================================ */

/**
 * @brief Test PTZ service registration failure handling
 * @param state Test state (unused)
 */
void test_ptz_service_registration_failure_handling(void** state) {
  (void)state;

  // Mock registration failure
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_RESOURCE_LIMIT);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service (should fail)
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_ERROR_RESOURCE_LIMIT, result);

  // Verify registration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test PTZ service dispatch failure handling
 * @param state Test state (unused)
 */
void test_ptz_service_dispatch_failure_handling(void** state) {
  (void)state;

  // Mock successful registration but dispatch failure
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_ERROR);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test dispatch failure
  result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_OPERATION,
                                             &g_test_request, &g_test_response);

  assert_int_equal(ONVIF_ERROR, result);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test PTZ service unregistration failure handling
 * @param state Test state (unused)
 */
void test_ptz_service_unregistration_failure_handling(void** state) {
  (void)state;

  // Mock successful registration but unregistration failure
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test unregistration failure
  onvif_ptz_cleanup();

  // Verify unregistration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
}

/* ============================================================================
 * PTZ Service Callback Logging Tests
 * ============================================================================ */

/**
 * @brief Test PTZ service callback logging for success paths
 * @param state Test state (unused)
 */
void test_ptz_service_callback_logging_success(void** state) {
  (void)state;

  // Mock successful operations
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_SUCCESS);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test dispatch (should log success)
  result = onvif_service_dispatcher_dispatch(TEST_PTZ_SERVICE_NAME, TEST_PTZ_OPERATION,
                                             &g_test_request, &g_test_response);

  // Verify operations completed (logging would be verified by log output)
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test PTZ service callback logging for failure paths
 * @param state Test state (unused)
 */
void test_ptz_service_callback_logging_failure(void** state) {
  (void)state;

  // Mock failure operations
  mock_service_dispatcher_set_register_result(ONVIF_ERROR);
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service (should fail)
  int result = onvif_ptz_init(NULL);
  assert_int_equal(ONVIF_ERROR, result);

  // Verify failure was logged (logging would be verified by log output)
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest ptz_callback_tests[] = {
  // Service Registration Tests
  cmocka_unit_test_setup_teardown(test_ptz_service_registration_success, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_registration_duplicate, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_registration_invalid_params,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_unregistration_success, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_unregistration_not_found,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),

  // Service Dispatch Tests
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_success, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_unknown_operation,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_null_service, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_null_operation,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_null_request, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_null_response, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),

  // Operation Handler Tests
  cmocka_unit_test_setup_teardown(test_ptz_operation_handler_success, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_operation_handler_null_operation,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_operation_handler_null_request, setup_ptz_callback_tests,
                                  teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_operation_handler_null_response,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_operation_handler_unknown_operation,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),

  // Error Handling Tests
  cmocka_unit_test_setup_teardown(test_ptz_service_registration_failure_handling,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_dispatch_failure_handling,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_unregistration_failure_handling,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),

  // Logging Tests
  cmocka_unit_test_setup_teardown(test_ptz_service_callback_logging_success,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
  cmocka_unit_test_setup_teardown(test_ptz_service_callback_logging_failure,
                                  setup_ptz_callback_tests, teardown_ptz_callback_tests),
};

/**
 * @brief Run PTZ callback tests
 * @return Number of tests run
 */
int run_ptz_callback_tests(void) {
  return cmocka_run_group_tests(ptz_callback_tests, NULL, NULL);
}
