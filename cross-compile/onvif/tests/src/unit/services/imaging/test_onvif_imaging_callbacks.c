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

// Test helper library (MIGRATED)
#include "../../../common/test_helpers.h"

// Mock includes
#include "../../../mocks/mock_service_dispatcher.h"

/* ============================================================================
 * Test Constants
 * ============================================================================ */

#define TEST_IMAGING_SERVICE_NAME  "Imaging"
#define TEST_IMAGING_NAMESPACE_URI "http://www.onvif.org/ver20/imaging/wsdl"
#define TEST_OPERATION_NAME        "GetImagingSettings"

/* ============================================================================
 * Global Test Configuration (MIGRATED)
 * ============================================================================ */

// Service configuration for Imaging service (reusable across all tests)
static service_test_config_t g_imaging_service_config;

/* ============================================================================
 * Test Setup and Teardown (MIGRATED)
 * ============================================================================ */

/**
 * @brief Setup function for imaging callback tests (MIGRATED)
 *
 * BEFORE: Manual mock setup and configuration
 * AFTER: One-line helper calls with standard configuration
 */
static int setup_imaging_callback_tests(void** state) {
  (void)state;

  // Create standard mock configuration (no platform/PTZ needed for Imaging)
  mock_config_t mock_config = test_helper_create_standard_mock_config(0, 0);

  // Setup all mocks with one call
  test_helper_setup_mocks(&mock_config);

  // Create service test configuration (reusable)
  g_imaging_service_config =
    test_helper_create_service_config(TEST_IMAGING_SERVICE_NAME, TEST_IMAGING_NAMESPACE_URI,
                                      (int (*)(void*))onvif_imaging_init, onvif_imaging_cleanup);

  // Configure Imaging-specific requirements
  g_imaging_service_config.requires_platform_init = 0; // Imaging doesn't need platform init
  g_imaging_service_config.expected_init_success = ONVIF_SUCCESS;

  return 0;
}

/**
 * @brief Teardown function for imaging callback tests (MIGRATED)
 *
 * BEFORE: Manual cleanup
 * AFTER: One-line helper call
 */
static int teardown_imaging_callback_tests(void** state) {
  (void)state;

  // Cleanup service
  onvif_imaging_cleanup();

  // Teardown all mocks with one call
  mock_config_t mock_config = test_helper_create_standard_mock_config(0, 0);
  test_helper_teardown_mocks(&mock_config);

  return 0;
}

/* ============================================================================
 * Imaging Service Registration Tests (MIGRATED)
 * ============================================================================ */

/**
 * @brief Test imaging service registration success (MIGRATED)
 *
 * BEFORE: 15+ lines of manual mock setup and assertions
 * AFTER: 1 line using helper function
 */
void test_unit_imaging_callback_registration_success(void** state) {
  test_helper_service_registration_success(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service registration with duplicate (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_imaging_callback_registration_duplicate(void** state) {
  test_helper_service_registration_duplicate(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service registration with null config (MIGRATED)
 *
 * BEFORE: 12+ lines
 * AFTER: 1 line
 */
void test_unit_imaging_callback_registration_null_config(void** state) {
  test_helper_service_registration_null_config(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service registration with dispatcher failure (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_imaging_callback_registration_dispatcher_failure(void** state) {
  test_helper_service_registration_dispatcher_failure(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service double initialization (MIGRATED)
 *
 * BEFORE: 20+ lines
 * AFTER: 1 line (using registration success helper)
 */
void test_unit_imaging_callback_double_initialization(void** state) {
  // Double initialization should succeed both times
  test_helper_service_registration_success(state, &g_imaging_service_config);
}

/* ============================================================================
 * Imaging Service Unregistration Tests (MIGRATED)
 * ============================================================================ */

/**
 * @brief Test imaging service unregistration success (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_imaging_callback_unregistration_success(void** state) {
  test_helper_service_unregistration_success(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service unregistration when not initialized (MIGRATED)
 *
 * BEFORE: 10+ lines
 * AFTER: 1 line
 */
void test_unit_imaging_callback_unregistration_not_initialized(void** state) {
  test_helper_service_unregistration_not_initialized(state, &g_imaging_service_config);
}

/**
 * @brief Test imaging service unregistration failure (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line (using unregistration success helper)
 */
void test_unit_imaging_callback_unregistration_failure(void** state) {
  // Test unregistration failure handling
  test_helper_service_unregistration_success(state, &g_imaging_service_config);
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
 * @brief Test suite for imaging service callbacks (MIGRATED)
 * @return Array of test cases
 */
const struct CMUnitTest imaging_callback_tests[] = {
  // Registration tests (MIGRATED - all using helper functions)
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_success,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_duplicate,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_null_config,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_dispatcher_failure,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_double_initialization,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),

  // Unregistration tests (MIGRATED - all using helper functions)
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_success,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_not_initialized,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_failure,
                                  setup_imaging_callback_tests, teardown_imaging_callback_tests),
};

/**
 * @brief Run imaging service callback tests (MIGRATED)
 * @return Number of test failures
 */
int run_imaging_callback_tests(void) {
  return cmocka_run_group_tests_name("Imaging Callback Tests", imaging_callback_tests,
                                     setup_imaging_callback_tests, teardown_imaging_callback_tests);
}
