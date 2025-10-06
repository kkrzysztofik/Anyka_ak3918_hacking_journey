/**
 * @file test_onvif_imaging_callbacks.c
 * @brief Imaging service callback tests (MIGRATED TO HELPER LIBRARY)
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/error/error_handling.h"

// CMocka mock includes
#include "../../../mocks/buffer_pool_mock.h"
#include "../../../mocks/gsoap_mock.h"
#include "../../../mocks/mock_service_dispatcher.h"
#include "../../../mocks/platform_mock.h"

/* ============================================================================
 * Test Constants
 * ============================================================================ */

#define TEST_IMAGING_SERVICE_NAME  "imaging"
#define TEST_IMAGING_NAMESPACE_URI "http://www.onvif.org/ver20/imaging/wsdl"
#define TEST_OPERATION_NAME        "GetImagingSettings"

/* ============================================================================
 * Test State and Helper Functions
 * ============================================================================ */

static config_manager_t g_mock_config;

static int dummy_operation_handler(const char* operation_name, const http_request_t* request,
                                   http_response_t* response) {
  (void)operation_name;
  (void)request;
  (void)response;
  return ONVIF_SUCCESS;
}

static void imaging_dependencies_set_real(bool enable) {
  service_dispatcher_mock_use_real_function(enable);
  buffer_pool_mock_use_real_function(enable);
  gsoap_mock_use_real_function(enable);
}

static void imaging_reset_state(void) {
  onvif_imaging_service_cleanup();
  memset(&g_mock_config, 0, sizeof(g_mock_config));
}

/**
 * @brief Setup function for imaging callback tests
 */
int setup_imaging_unit_tests(void** state) {
  (void)state;

  mock_service_dispatcher_init();
  imaging_dependencies_set_real(true);

  onvif_service_dispatcher_init();
  imaging_reset_state();

  return 0;
}

/**
 * @brief Teardown function for imaging callback tests
 */
int teardown_imaging_unit_tests(void** state) {
  (void)state;

  onvif_imaging_service_cleanup();
  onvif_service_dispatcher_cleanup();

  imaging_dependencies_set_real(false);
  mock_service_dispatcher_cleanup();

  return 0;
}

/* ============================================================================
 * Imaging Service Registration Tests (CMOCKA PATTERNS)
 * ============================================================================ */

/**
 * @brief Test imaging service registration success (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_success(void** state) {
  (void)state;

  // Initialize imaging service with real dispatcher
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify service is registered with dispatcher
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/**
 * @brief Test imaging service registration with duplicate (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_duplicate(void** state) {
  (void)state;

  // First initialization should succeed
  assert_int_equal(ONVIF_SUCCESS, onvif_imaging_service_init(NULL));

  // Second initialization should also succeed (idempotent)
  assert_int_equal(ONVIF_SUCCESS, onvif_imaging_service_init(NULL));

  // Verify service is still registered
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/**
 * @brief Test imaging service registration with null config (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_null_config(void** state) {
  (void)state;

  // Initialize imaging service with NULL config should succeed
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify service is registered
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
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
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));

  // Then unregister
  onvif_imaging_service_cleanup();

  // Verify service is no longer registered
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/**
 * @brief Test imaging service unregistration when not initialized (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_unregistration_not_initialized(void** state) {
  (void)state;

  // Cleanup when not initialized should not crash
  onvif_imaging_service_cleanup();

  // Verify no service is registered
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
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
 * Test Suite Registration
 * ============================================================================ */

/**
 * @brief Get imaging callbacks unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_imaging_callbacks_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_success,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_duplicate,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_null_config,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_success,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_not_initialized,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
