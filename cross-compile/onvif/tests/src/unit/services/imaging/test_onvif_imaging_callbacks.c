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
#include "../../../mocks/config_mock.h"
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

// Test configuration file path
#define TEST_IMAGING_CONFIG_PATH "configs/imaging_test_config.ini"

// Test state structure to hold config pointers
typedef struct {
  config_manager_t* config;
  struct application_config* app_config;
} imaging_test_state_t;

static int dummy_operation_handler(const char* operation_name, const http_request_t* request, http_response_t* response) {
  (void)operation_name;
  (void)request;
  (void)response;
  return ONVIF_SUCCESS;
}

static void imaging_dependencies_set_real(bool enable) {
  service_dispatcher_mock_use_real_function(enable);
  buffer_pool_mock_use_real_function(enable);
  gsoap_mock_use_real_function(enable);
  config_mock_use_real_function(enable);
}

/**
 * @brief Setup function for imaging callback tests
 */
int setup_imaging_unit_tests(void** state) {
  mock_service_dispatcher_init();
  imaging_dependencies_set_real(true);

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Heap-allocate application config structure
  struct application_config* app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(app_config);

  // Allocate pointer members for application_config
  memset(&app_config->imaging, 0, sizeof(struct imaging_settings));
  memset(&app_config->auto_daynight, 0, sizeof(struct auto_daynight_config));
  memset(&app_config->network, 0, sizeof(struct network_settings));
  memset(&app_config->device, 0, sizeof(struct device_info));
  memset(&app_config->logging, 0, sizeof(struct logging_settings));
  memset(&app_config->server, 0, sizeof(struct server_settings));

  // Initialize runtime configuration system
  result = config_runtime_init(app_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Load configuration from test INI file
  result = config_storage_load(TEST_IMAGING_CONFIG_PATH, NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize config manager
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));
  config->app_config = app_config;

  // Store config pointers in test state for cleanup
  imaging_test_state_t* test_state = calloc(1, sizeof(imaging_test_state_t));
  assert_non_null(test_state);
  test_state->config = config;
  test_state->app_config = app_config;

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for imaging callback tests
 */
int teardown_imaging_unit_tests(void** state) {
  imaging_test_state_t* test_state = (imaging_test_state_t*)*state;
  config_manager_t* config = test_state->config;
  struct application_config* app_config = test_state->app_config;

  // Free application_config members first
  if (app_config) {
    // All struct members are direct, no free needed
    free(app_config);
  }

  // Free config
  free(config);

  // Free test state structure
  free(test_state);

  // Clean up runtime configuration system
  config_runtime_cleanup();

  // Cleanup imaging service
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
  imaging_test_state_t* test_state = (imaging_test_state_t*)*state;

  // Initialize imaging service with real config
  int result = onvif_imaging_service_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify service is registered with dispatcher
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/**
 * @brief Test imaging service registration with duplicate (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_duplicate(void** state) {
  imaging_test_state_t* test_state = (imaging_test_state_t*)*state;

  // First initialization should succeed
  assert_int_equal(ONVIF_SUCCESS, onvif_imaging_service_init(test_state->config));

  // Second initialization should also succeed (idempotent)
  assert_int_equal(ONVIF_SUCCESS, onvif_imaging_service_init(test_state->config));

  // Verify service is still registered
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/**
 * @brief Test imaging service registration with null config (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_registration_null_config(void** state) {
  (void)state;

  // Initialize imaging service with NULL config should fail (unified config required)
  int result = onvif_imaging_service_init(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify service is NOT registered (init failed)
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_IMAGING_SERVICE_NAME));
}

/* ============================================================================
 * Imaging Service Unregistration Tests (CMOCKA PATTERNS)
 * ============================================================================ */

/**
 * @brief Test imaging service unregistration success (CMOCKA PATTERNS)
 */
void test_unit_imaging_callback_unregistration_success(void** state) {
  imaging_test_state_t* test_state = (imaging_test_state_t*)*state;

  // First register the service
  int result = onvif_imaging_service_init(test_state->config);
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
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_success, setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_duplicate, setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_null_config, setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_success, setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_not_initialized, setup_imaging_unit_tests, teardown_imaging_unit_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
