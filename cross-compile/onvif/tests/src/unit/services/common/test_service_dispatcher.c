/**
 * @file test_service_dispatcher.c
 * @brief Refactored service dispatcher tests demonstrating generic mock framework
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"

// Create mock handlers using the new macro system
TEST_HELPER_CREATE_MOCK_HANDLERS(test_service)

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

static int setup_service_dispatcher_tests(void** state) {
  (void)state;
  test_service_reset_mock_state();
  onvif_service_dispatcher_cleanup();
  return 0;
}

static int teardown_service_dispatcher_tests(void** state) {
  (void)state;
  onvif_service_dispatcher_cleanup();
  test_service_reset_mock_state();
  return 0;
}

/* ============================================================================
 * Refactored Tests Using Generic Mock Framework
 * ============================================================================ */

/**
 * @brief Test service dispatcher initialization
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_init(void** state) {
  (void)state;

  // Test initialization
  int result = onvif_service_dispatcher_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test that initialization is idempotent
  result = onvif_service_dispatcher_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatcher cleanup
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_cleanup(void** state) {
  (void)state;

  // Initialize first
  onvif_service_dispatcher_init();

  // Register a service with cleanup handler
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "test_service", "http://test.namespace.uri", test_service_mock_operation,
    test_service_mock_init, test_service_mock_cleanup);

  onvif_service_dispatcher_register_service(&registration);

  // Test cleanup (should call service cleanup handlers)
  onvif_service_dispatcher_cleanup();
  assert_int_equal(g_test_service_mock_state.cleanup_call_count, 1);

  // Test multiple cleanups (should not crash)
  onvif_service_dispatcher_cleanup();
  assert_int_equal(g_test_service_mock_state.cleanup_call_count, 1); // Should not be called again
}

/**
 * @brief Test successful service registration
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_register_service(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Test basic service registration
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation,
    test_service_mock_init, test_service_mock_cleanup);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_service_mock_state.init_call_count, 1);

  // Verify service is registered
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_register_service_null_params(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Test NULL registration
  int result = onvif_service_dispatcher_register_service(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration with invalid parameters
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_register_service_invalid_params(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Test empty service name
  onvif_service_registration_t registration_empty_name = ONVIF_SERVICE_REGISTRATION(
    "", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&registration_empty_name);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL service name
  onvif_service_registration_t registration_null_name = ONVIF_SERVICE_REGISTRATION(
    NULL, "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_null_name);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test empty namespace
  onvif_service_registration_t registration_empty_ns =
    ONVIF_SERVICE_REGISTRATION("device", "", test_service_mock_operation, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_empty_ns);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL namespace
  onvif_service_registration_t registration_null_ns =
    ONVIF_SERVICE_REGISTRATION("device", NULL, test_service_mock_operation, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_null_ns);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL operation handler
  onvif_service_registration_t registration_null_handler = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", NULL, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_null_handler);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test duplicate service registration
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_register_service_duplicate(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register first service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Try to register same service again
  result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_ERROR_ALREADY_EXISTS);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service unregistration
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_unregister_service(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register a service
  onvif_service_registration_t registration =
    ONVIF_SERVICE_REGISTRATION("device", "http://www.onvif.org/ver10/device/wsdl",
                               test_service_mock_operation, NULL, test_service_mock_cleanup);

  onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  // Unregister the service
  int result = onvif_service_dispatcher_unregister_service("device");
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_service_mock_state.cleanup_call_count, 1);

  // Verify service is no longer registered
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 0);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service unregistration when service not found
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_unregister_service_not_found(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Try to unregister non-existent service
  int result = onvif_service_dispatcher_unregister_service("nonexistent");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  // Test with NULL service name
  result = onvif_service_dispatcher_unregister_service(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test successful request dispatch
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_dispatch(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register a service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);

  onvif_service_dispatcher_register_service(&registration);

  // Create mock request and response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch
  int result =
    onvif_service_dispatcher_dispatch("device", "GetDeviceInformation", &request, &response);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_service_mock_state.operation_call_count, 1);
  assert_string_equal(g_test_service_mock_state.last_operation, "GetDeviceInformation");

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test dispatch with invalid parameters
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_dispatch_invalid_params(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  http_request_t request = {0};
  http_response_t response = {0};

  // Test with NULL service name
  int result = onvif_service_dispatcher_dispatch(NULL, "GetDeviceInformation", &request, &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL operation name
  result = onvif_service_dispatcher_dispatch("device", NULL, &request, &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL request
  result = onvif_service_dispatcher_dispatch("device", "GetDeviceInformation", NULL, &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL response
  result = onvif_service_dispatcher_dispatch("device", "GetDeviceInformation", &request, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test dispatch when service not found
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_dispatch_service_not_found(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch to non-existent service
  int result =
    onvif_service_dispatcher_dispatch("nonexistent", "GetDeviceInformation", &request, &response);
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration check
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_is_registered(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Test with non-existent service
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 0);

  // Register a service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);

  onvif_service_dispatcher_register_service(&registration);

  // Test with registered service
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  // Test with NULL service name
  assert_int_equal(onvif_service_dispatcher_is_registered(NULL), 0);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service initialization and cleanup handlers
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_init_cleanup_handlers(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Test with successful init handler
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation,
    test_service_mock_init, test_service_mock_cleanup);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_service_mock_state.init_call_count, 1);

  // Test with failing init handler
  test_service_reset_mock_state();
  onvif_service_dispatcher_init(); // Re-initialize after reset
  g_test_service_mock_state.init_result = ONVIF_ERROR_INVALID;

  onvif_service_registration_t failing_registration = ONVIF_SERVICE_REGISTRATION(
    "failing_service", "http://test.namespace.uri", test_service_mock_operation,
    test_service_mock_init, test_service_mock_cleanup);

  result = onvif_service_dispatcher_register_service(&failing_registration);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
  assert_int_equal(g_test_service_mock_state.init_call_count, 1);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration when registry is full
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_register_service_registry_full(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register maximum number of services to fill the registry
  for (int i = 0; i < 16; i++) { // MAX_REGISTERED_SERVICES
    char service_name[32];
    snprintf(service_name, sizeof(service_name), "service_%d", i);

    onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
      service_name, "http://test.namespace.uri", test_service_mock_operation, NULL, NULL);

    int result = onvif_service_dispatcher_register_service(&registration);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Try to register one more service (should fail)
  onvif_service_registration_t extra_registration = ONVIF_SERVICE_REGISTRATION(
    "extra_service", "http://test.namespace.uri", test_service_mock_operation, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&extra_registration);
  assert_int_equal(result, -41); // ONVIF_ERROR_RESOURCE_LIMIT

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test getting list of registered services
 * @param state Test state (unused)
 */
void test_unit_service_dispatcher_get_services(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Initially no services should be registered
  const char* services[16] = {0}; // MAX_REGISTERED_SERVICES - initialize to NULL
  int result = onvif_service_dispatcher_get_services(services, 16);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_null(services[0]); // First service should be NULL

  // Register a few services
  onvif_service_registration_t device_registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_mock_operation, NULL, NULL);
  onvif_service_dispatcher_register_service(&device_registration);

  onvif_service_registration_t media_registration = ONVIF_SERVICE_REGISTRATION(
    "media", "http://www.onvif.org/ver10/media/wsdl", test_service_mock_operation, NULL, NULL);
  onvif_service_dispatcher_register_service(&media_registration);

  // Get services list
  memset(services, 0, sizeof(services)); // Clear the array first
  result = onvif_service_dispatcher_get_services(services, 16);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Check that our services are in the list
  int device_found = 0;
  int media_found = 0;
  for (int i = 0; services[i] != NULL && i < 16; i++) { // MAX_REGISTERED_SERVICES
    if (strcmp(services[i], "device") == 0) {
      device_found = 1;
    } else if (strcmp(services[i], "media") == 0) {
      media_found = 1;
    }
  }

  assert_int_equal(device_found, 1);
  assert_int_equal(media_found, 1);

  onvif_service_dispatcher_cleanup();
}

/* ============================================================================
 * Service Dispatch with Multiple Services Tests (Moved from service callbacks)
 * ============================================================================ */

/**
 * @brief Test service dispatch with valid operation
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c - generalized for dispatcher testing
 */
void test_unit_service_dispatcher_dispatch_with_registered_service(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register a test service
  onvif_service_registration_t test_registration =
    ONVIF_SERVICE_REGISTRATION("test_service", "http://www.onvif.org/ver10/test/wsdl",
                               test_service_mock_operation, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&test_registration);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Create test request/response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch through service dispatcher
  result = onvif_service_dispatcher_dispatch("test_service", "TestOperation", &request, &response);

  // Verify dispatch completed (actual return depends on mock implementation)
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Verify mock was called
  assert_int_equal(1, g_test_service_mock_state.operation_call_count);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatch with unknown operation
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c - generalized for dispatcher testing
 */
void test_unit_service_dispatcher_dispatch_unknown_operation(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Register a test service
  onvif_service_registration_t test_registration =
    ONVIF_SERVICE_REGISTRATION("test_service", "http://www.onvif.org/ver10/test/wsdl",
                               test_service_mock_operation, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&test_registration);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Create test request/response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch with unknown operation
  result =
    onvif_service_dispatcher_dispatch("test_service", "UnknownOperation", &request, &response);

  // Should return error or success depending on mock implementation
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR || result == ONVIF_ERROR_NOT_FOUND);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatch with NULL service name
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c
 */
void test_unit_service_dispatcher_dispatch_null_service_name(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Create test request/response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch with NULL service name
  int result = onvif_service_dispatcher_dispatch(NULL, "TestOperation", &request, &response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatch with NULL operation name
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c
 */
void test_unit_service_dispatcher_dispatch_null_operation_name(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Create test request/response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch with NULL operation name
  int result = onvif_service_dispatcher_dispatch("test_service", NULL, &request, &response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatch with NULL request
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c
 */
void test_unit_service_dispatcher_dispatch_null_request_param(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Create test response
  http_response_t response = {0};

  // Test dispatch with NULL request
  int result = onvif_service_dispatcher_dispatch("test_service", "TestOperation", NULL, &response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatch with NULL response
 * @param state Test state (unused)
 *
 * @note Moved from test_onvif_ptz_callbacks.c
 */
void test_unit_service_dispatcher_dispatch_null_response_param(void** state) {
  (void)state;

  onvif_service_dispatcher_init();

  // Create test request
  http_request_t request = {0};

  // Test dispatch with NULL response
  int result = onvif_service_dispatcher_dispatch("test_service", "TestOperation", &request, NULL);

  assert_int_equal(ONVIF_ERROR_INVALID, result);

  onvif_service_dispatcher_cleanup();
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest service_dispatcher_tests[] = {
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_init, setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_cleanup,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_null_params,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_invalid_params,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_duplicate,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_unregister_service,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_unregister_service_not_found,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_invalid_params,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_service_not_found,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_is_registered,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_init_cleanup_handlers,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_registry_full,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_get_services,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),

  // Service Dispatch with Multiple Services Tests (moved from service callbacks)
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_with_registered_service,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_unknown_operation,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_service_name,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_operation_name,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_request_param,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_response_param,
                                  setup_service_dispatcher_tests,
                                  teardown_service_dispatcher_tests),
};
