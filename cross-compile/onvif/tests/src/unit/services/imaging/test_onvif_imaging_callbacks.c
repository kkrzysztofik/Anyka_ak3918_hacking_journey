/**
 * @file test_onvif_imaging_callbacks.c
 * @brief Imaging service callback tests (MIGRATED TO HELPER LIBRARY)
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

// CMocka mock includes
#include "../../../mocks/mock_service_dispatcher.h"
#include "../../../mocks/platform_mock.h"

/* ============================================================================
 * Test Constants
 * ============================================================================ */

#define TEST_IMAGING_SERVICE_NAME  "Imaging"
#define TEST_IMAGING_NAMESPACE_URI "http://www.onvif.org/ver20/imaging/wsdl"
#define TEST_OPERATION_NAME        "GetImagingSettings"

/* ============================================================================
 * Imaging Service Registration Tests (CMOCKA PATTERNS)
 * ============================================================================ */

/**
 * @brief Test imaging service registration success (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_success(void** state) {
  (void)state;

  // Expect service dispatcher registration
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);

  // Initialize imaging service
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test imaging service registration with duplicate (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_duplicate(void** state) {
  (void)state;

  // Expect service dispatcher registration to fail with duplicate
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_ERROR_DUPLICATE);

  // Initialize imaging service should fail
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_ERROR_DUPLICATE, result);
}

/**
 * @brief Test imaging service registration with null config (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_null_config(void** state) {
  (void)state;

  // Expect service dispatcher registration
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);

  // Initialize imaging service with NULL config should succeed
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test imaging service registration with dispatcher failure (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_dispatcher_failure(void** state) {
  (void)state;

  // Expect service dispatcher registration to fail
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_ERROR_INVALID);

  // Initialize imaging service should fail
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test imaging service double initialization (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_double_initialization(void** state) {
  (void)state;

  // First initialization
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Second initialization should also succeed
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);
  result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/* ============================================================================
 * Imaging Service Unregistration Tests (CMOCKA PATTERNS)
 * ============================================================================ */

/**
 * @brief Test imaging service unregistration success (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_unregistration_success(void** state) {
  (void)state;

  // First register the service
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Then unregister
  expect_function_call(__wrap_onvif_service_dispatcher_unregister_service);
  will_return(__wrap_onvif_service_dispatcher_unregister_service, ONVIF_SUCCESS);
  onvif_imaging_service_cleanup();
}

/**
 * @brief Test imaging service unregistration when not initialized (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_unregistration_not_initialized(void** state) {
  (void)state;

  // Cleanup when not initialized should not crash
  onvif_imaging_service_cleanup();
}

/**
 * @brief Test imaging service unregistration failure (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_unregistration_failure(void** state) {
  (void)state;

  // First register the service
  expect_function_call(__wrap_onvif_service_dispatcher_register_service);
  will_return(__wrap_onvif_service_dispatcher_register_service, ONVIF_SUCCESS);
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Then unregister with failure
  expect_function_call(__wrap_onvif_service_dispatcher_unregister_service);
  will_return(__wrap_onvif_service_dispatcher_unregister_service, ONVIF_ERROR_INVALID);
  onvif_imaging_service_cleanup();
}

/* ============================================================================
 * Imaging Service Dispatch Tests - REMOVED
 * ============================================================================ */

/**
 * @note Dispatcher-specific tests have been moved to test_service_dispatcher.c
 * where they belong. Imaging service tests now focus solely on imaging-specific
 * callback registration and handling behavior.
 */

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

/**
 * @brief Test suite for imaging service callbacks (CMOCKA PATTERNS)
 * @return Array of test cases
 */
const struct CMUnitTest imaging_callback_tests[] = {
  // Registration tests (CMOCKA PATTERNS)
  cmocka_unit_test(test_unit_imaging_callback_registration_success),
  cmocka_unit_test(test_unit_imaging_callback_registration_duplicate),
  cmocka_unit_test(test_unit_imaging_callback_registration_null_config),
  cmocka_unit_test(test_unit_imaging_callback_registration_dispatcher_failure),
  cmocka_unit_test(test_unit_imaging_callback_double_initialization),

  // Unregistration tests (CMOCKA PATTERNS)
  cmocka_unit_test(test_unit_imaging_callback_unregistration_success),
  cmocka_unit_test(test_unit_imaging_callback_unregistration_not_initialized),
  cmocka_unit_test(test_unit_imaging_callback_unregistration_failure),
};

/**
 * @brief Run imaging service callback tests (CMOCKA PATTERNS)
 * @return Number of test failures
 */
int run_imaging_callback_tests(void) {
  return cmocka_run_group_tests_name("Imaging Callback Tests", imaging_callback_tests, NULL, NULL);
}

/**
 * @brief Get unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_imaging_callbacks_unit_tests(size_t* count) {
  *count = sizeof(imaging_callback_tests) / sizeof(imaging_callback_tests[0]);
  return imaging_callback_tests;
}
