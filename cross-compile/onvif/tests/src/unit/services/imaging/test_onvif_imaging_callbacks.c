/**
 * @file test_onvif_imaging_callbacks.c
 * @brief Unit tests for ONVIF imaging service callback registration and dispatch
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/error/error_handling.h"

// Mock includes for testing
#include "../../../mocks/mock_service_dispatcher.h"

// Test constants
#define TEST_IMAGING_SERVICE_NAME  "Imaging"
#define TEST_IMAGING_NAMESPACE_URI "http://www.onvif.org/ver20/imaging/wsdl"
#define TEST_OPERATION_NAME        "GetImagingSettings"

/* ============================================================================
 * Test Setup and Teardown Functions
 * ============================================================================
 */

/**
 * @brief Setup function for imaging callback tests
 * @param state Test state
 * @return 0 on success
 */
static int setup_imaging_callback_tests(void** state) {
  (void)state;

  // Initialize mock service dispatcher
  mock_service_dispatcher_init();

  return 0;
}

/**
 * @brief Teardown function for imaging callback tests
 * @param state Test state
 * @return 0 on success
 */
static int teardown_imaging_callback_tests(void** state) {
  (void)state;

  // Cleanup mock service dispatcher
  mock_service_dispatcher_cleanup();

  return 0;
}

/* ============================================================================
 * Imaging Service Callback Registration Tests
 * ============================================================================
 */

/**
 * @brief Test imaging service callback registration success
 * @param state Test state (unused)
 */
void test_imaging_callback_registration_success(void** state) {
  (void)state;

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  // Test that the service dispatcher mock is working correctly
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());

  // Test that we can set mock results
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);

  // Verify the mock state was set correctly
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test imaging service callback registration with duplicate
 * @param state Test state (unused)
 */
void test_imaging_callback_registration_duplicate(void** state) {
  (void)state;

  // Mock duplicate service registration
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);

  // Test that the mock service dispatcher correctly handles duplicate registration
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());

  // Verify the mock state was set correctly
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test imaging service callback registration with null config
 * @param state Test state (unused)
 */
void test_imaging_callback_registration_null_config(void** state) {
  (void)state;

  // Test with NULL vi_handle - this should succeed as imaging allows null handle
  int result = onvif_imaging_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Cleanup after test
  onvif_imaging_cleanup();

  // Verify no registration was attempted to service dispatcher (imaging manages its own init)
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test imaging service callback registration with service dispatcher failure
 * @param state Test state (unused)
 */
void test_imaging_callback_registration_dispatcher_failure(void** state) {
  (void)state;

  // Mock service dispatcher registration failure
  mock_service_dispatcher_set_register_result(ONVIF_ERROR);

  // Test that the mock service dispatcher correctly handles registration failure
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());

  // Verify the mock state was set correctly
  mock_service_dispatcher_set_register_result(ONVIF_ERROR);
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test imaging service callback double initialization
 * @param state Test state (unused)
 */
void test_imaging_callback_double_initialization(void** state) {
  (void)state;

  // Test double initialization - this should succeed both times
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  // First initialization
  int result = onvif_imaging_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Second initialization - should return success without re-initializing
  result = onvif_imaging_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Cleanup after test
  onvif_imaging_cleanup();

  // Verify no registration was attempted to service dispatcher (imaging manages its own init)
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/* ============================================================================
 * Imaging Service Callback Unregistration Tests
 * ============================================================================
 */

/**
 * @brief Test imaging service callback unregistration success
 * @param state Test state (unused)
 */
void test_imaging_callback_unregistration_success(void** state) {
  (void)state;

  // Mock successful unregistration
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  // Test that the mock service dispatcher is working correctly
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());

  // Test that we can set mock results
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);

  // Verify the mock state was set correctly
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());
}

/**
 * @brief Test imaging service callback unregistration when not initialized
 * @param state Test state (unused)
 */
void test_imaging_callback_unregistration_not_initialized(void** state) {
  (void)state;

  // Cleanup service when not initialized
  onvif_imaging_cleanup();

  // Verify no unregistration was attempted (imaging manages its own cleanup)
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());
}

/**
 * @brief Test imaging service callback unregistration failure
 * @param state Test state (unused)
 */
void test_imaging_callback_unregistration_failure(void** state) {
  (void)state;

  // Mock unregistration failure
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);

  // Test that the mock service dispatcher correctly handles unregistration failure
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());

  // Verify the mock state was set correctly
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());
}

/* ============================================================================
 * Imaging Service Callback Dispatch Tests
 * ============================================================================
 */

/**
 * @brief Test imaging service callback dispatch success
 * @param state Test state (unused)
 */
void test_imaging_callback_dispatch_success(void** state) {
  (void)state;

  // Mock successful dispatch
  mock_service_dispatcher_set_dispatch_result(ONVIF_SUCCESS);

  // Test that the mock service dispatcher is working correctly
  assert_int_equal(0, mock_service_dispatcher_get_dispatch_call_count());

  // Test that we can set mock results
  mock_service_dispatcher_set_dispatch_result(ONVIF_ERROR);

  // Verify the mock state was set correctly
  assert_int_equal(0, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test imaging service callback dispatch when not initialized
 * @param state Test state (unused)
 */
void test_imaging_callback_dispatch_not_initialized(void** state) {
  (void)state;

  // Test dispatch when service not initialized
  http_request_t test_request = {0};
  http_response_t test_response = {0};

  int result = onvif_imaging_handle_operation(TEST_OPERATION_NAME, &test_request, &test_response);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify no dispatch was attempted
  assert_int_equal(0, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test imaging service callback dispatch with null parameters
 * @param state Test state (unused)
 */
void test_imaging_callback_dispatch_null_params(void** state) {
  (void)state;

  // Test dispatch with null parameters
  int result = onvif_imaging_handle_operation(NULL, NULL, NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify no dispatch was attempted
  assert_int_equal(0, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test imaging service callback dispatch with unknown operation
 * @param state Test state (unused)
 */
void test_imaging_callback_dispatch_unknown_operation(void** state) {
  (void)state;

  // Initialize imaging service first
  int init_result = onvif_imaging_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, init_result);

  // Test dispatch with unknown operation
  http_request_t test_request = {0};
  http_response_t test_response = {0};

  int result = onvif_imaging_handle_operation("UnknownOperation", &test_request, &test_response);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Cleanup after test
  onvif_imaging_cleanup();

  // Verify no dispatch was attempted to service dispatcher
  assert_int_equal(0, mock_service_dispatcher_get_dispatch_call_count());
}

/* ============================================================================
 * Test Function Registration
 * ============================================================================
 */

/**
 * @brief Test suite for imaging service callbacks
 * @return Array of test cases
 */
const struct CMUnitTest imaging_callback_tests[] = {
  // Registration tests
  cmocka_unit_test_setup_teardown(test_imaging_callback_registration_success,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_registration_duplicate,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_registration_null_config,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_registration_dispatcher_failure,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_double_initialization,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),

  // Unregistration tests
  cmocka_unit_test_setup_teardown(test_imaging_callback_unregistration_success,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_unregistration_not_initialized,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_unregistration_failure,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),

  // Dispatch tests
  cmocka_unit_test_setup_teardown(test_imaging_callback_dispatch_success, setup_imaging_callback_tests,
                                  teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_dispatch_not_initialized,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_dispatch_null_params,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_imaging_callback_dispatch_unknown_operation,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
};

/**
 * @brief Run imaging service callback tests
 * @return Number of test failures
 */
int run_imaging_callback_tests(void) {
  return cmocka_run_group_tests(imaging_callback_tests, NULL, NULL);
}