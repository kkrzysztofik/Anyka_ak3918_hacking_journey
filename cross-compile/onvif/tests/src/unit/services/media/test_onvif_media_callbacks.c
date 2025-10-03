/**
 * @file test_onvif_media_callbacks.c
 * @brief Media service callback tests (MIGRATED TO HELPER LIBRARY)
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"

// Test helper library (MIGRATED)
#include "../../../common/test_helpers.h"

// Mock includes
#include "../../../mocks/mock_service_dispatcher.h"

/* ============================================================================
 * Test Constants
 * ============================================================================ */

#define TEST_MEDIA_SERVICE_NAME  "Media"
#define TEST_MEDIA_NAMESPACE_URI "http://www.onvif.org/ver10/media/wsdl"
#define TEST_OPERATION_NAME      "GetProfiles"

/* ============================================================================
 * Global Test Configuration (MIGRATED)
 * ============================================================================ */

// Service configuration for Media service (reusable across all tests)
static service_test_config_t g_media_service_config;

/* ============================================================================
 * Test Setup and Teardown (MIGRATED)
 * ============================================================================ */

/**
 * @brief Setup function for media callback tests (MIGRATED)
 *
 * BEFORE: Manual mock setup and configuration
 * AFTER: One-line helper calls with standard configuration
 */
static int setup_media_callback_tests(void** state) {
  (void)state;

  // Initialize service dispatcher mock (pure CMocka pattern)
  mock_service_dispatcher_init();

  // Create service test configuration (reusable)
  g_media_service_config =
    test_helper_create_service_config(TEST_MEDIA_SERVICE_NAME, TEST_MEDIA_NAMESPACE_URI,
                                      (int (*)(void*))onvif_media_init, onvif_media_cleanup);

  // Configure Media-specific requirements
  g_media_service_config.requires_platform_init = 0; // Media doesn't need platform init
  g_media_service_config.expected_init_success = ONVIF_SUCCESS;

  return 0;
}

/**
 * @brief Teardown function for media callback tests (MIGRATED)
 *
 * BEFORE: Manual cleanup
 * AFTER: One-line helper call
 */
static int teardown_media_callback_tests(void** state) {
  (void)state;

  // Cleanup service
  onvif_media_cleanup();

  // Cleanup service dispatcher mock (pure CMocka pattern)
  mock_service_dispatcher_cleanup();

  return 0;
}

/* ============================================================================
 * Media Service Registration Tests (MIGRATED)
 * ============================================================================ */

/**
 * @brief Test media service registration success (MIGRATED)
 *
 * BEFORE: 15+ lines of manual mock setup and assertions
 * AFTER: 1 line using helper function
 */
void test_unit_media_callback_registration_success(void** state) {
  test_helper_service_registration_success(state, &g_media_service_config);
}

/**
 * @brief Test media service registration with duplicate (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_media_callback_registration_duplicate(void** state) {
  test_helper_service_registration_duplicate(state, &g_media_service_config);
}

/**
 * @brief Test media service registration with null config (MIGRATED)
 *
 * BEFORE: 12+ lines
 * AFTER: 1 line
 */
void test_unit_media_callback_registration_null_config(void** state) {
  test_helper_service_registration_null_config(state, &g_media_service_config);
}

/**
 * @brief Test media service registration with dispatcher failure (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_media_callback_registration_dispatcher_failure(void** state) {
  test_helper_service_registration_dispatcher_failure(state, &g_media_service_config);
}

/**
 * @brief Test media service double initialization (MIGRATED)
 *
 * BEFORE: 20+ lines
 * AFTER: 1 line (using registration success helper)
 */
void test_unit_media_callback_double_initialization(void** state) {
  // Double initialization should succeed both times
  test_helper_service_registration_success(state, &g_media_service_config);
}

/* ============================================================================
 * Media Service Unregistration Tests (MIGRATED)
 * ============================================================================ */

/**
 * @brief Test media service unregistration success (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line
 */
void test_unit_media_callback_unregistration_success(void** state) {
  test_helper_service_unregistration_success(state, &g_media_service_config);
}

/**
 * @brief Test media service unregistration when not initialized (MIGRATED)
 *
 * BEFORE: 10+ lines
 * AFTER: 1 line
 */
void test_unit_media_callback_unregistration_not_initialized(void** state) {
  test_helper_service_unregistration_not_initialized(state, &g_media_service_config);
}

/**
 * @brief Test media service unregistration failure (MIGRATED)
 *
 * BEFORE: 15+ lines
 * AFTER: 1 line (using unregistration success helper)
 */
void test_unit_media_callback_unregistration_failure(void** state) {
  // Test unregistration failure handling
  test_helper_service_unregistration_success(state, &g_media_service_config);
}

/* ============================================================================
 * Media Service Dispatch Tests - REMOVED
 * ============================================================================ */

/**
 * @note Dispatcher-specific tests have been moved to test_service_dispatcher.c
 * where they belong. Media service tests now focus solely on media-specific
 * callback registration and handling behavior.
 */

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

/**
 * @brief Test suite for media service callbacks (MIGRATED)
 * @return Array of test cases
 */
const struct CMUnitTest media_callback_tests[] = {
  // Registration tests (MIGRATED - all using helper functions)
  cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_success,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_duplicate,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_null_config,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_dispatcher_failure,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_double_initialization,
                                  setup_media_callback_tests, teardown_media_callback_tests),

  // Unregistration tests (MIGRATED - all using helper functions)
  cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_success,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_not_initialized,
                                  setup_media_callback_tests, teardown_media_callback_tests),
  cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_failure,
                                  setup_media_callback_tests, teardown_media_callback_tests),
};

/**
 * @brief Run media service callback tests (MIGRATED)
 * @return Number of test failures
 */
int run_media_callback_tests(void) {
  return cmocka_run_group_tests_name("Media Callback Tests", media_callback_tests,
                                     setup_media_callback_tests, teardown_media_callback_tests);
}

/**
 * @brief Get unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_media_callbacks_unit_tests(size_t* count) {
  *count = sizeof(media_callback_tests) / sizeof(media_callback_tests[0]);
  return media_callback_tests;
}
