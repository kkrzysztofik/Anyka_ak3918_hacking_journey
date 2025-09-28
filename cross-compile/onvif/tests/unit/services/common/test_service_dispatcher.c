/**
 * @file test_service_dispatcher.c
 * @brief Unit tests for ONVIF service dispatcher implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"


// Error constants from service_dispatcher.c
#define ONVIF_ERROR_NOT_INITIALIZED -40
#define ONVIF_ERROR_RESOURCE_LIMIT  -41

// Test constants
#define MAX_SERVICES_TEST       16
#define MAX_SERVICE_NAME_LENGTH 32

// Mock data for testing
static int g_test_init_called = 0;             // NOLINT
static int g_test_cleanup_called = 0;          // NOLINT
static int g_test_operation_called = 0;        // NOLINT
static int g_test_init_result = ONVIF_SUCCESS; // NOLINT

// Mock service handlers
static int test_service_init_handler(void) {
  g_test_init_called++;
  return g_test_init_result;
}

static void test_service_cleanup_handler(void) {
  g_test_cleanup_called++;
}

static int test_service_operation_handler(const char* operation_name, const http_request_t* request,
                                          http_response_t* response) {
  (void)operation_name;
  (void)request;
  (void)response;
  g_test_operation_called++;
  return ONVIF_SUCCESS;
}

// Reset test state before each test
static void reset_test_state(void) {
  g_test_init_called = 0;
  g_test_cleanup_called = 0;
  g_test_operation_called = 0;
  g_test_init_result = ONVIF_SUCCESS;

  // Ensure clean state
  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service dispatcher initialization
 * @param state Test state (unused)
 */
void test_service_dispatcher_init(void** state) {
  (void)state;
  reset_test_state();

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
void test_service_dispatcher_cleanup(void** state) {
  (void)state;
  reset_test_state();

  // Initialize first
  onvif_service_dispatcher_init();

  // Register a service with cleanup handler
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "test_service", "http://test.namespace.uri", test_service_operation_handler,
    test_service_init_handler, test_service_cleanup_handler);

  onvif_service_dispatcher_register_service(&registration);

  // Test cleanup (should call service cleanup handlers)
  onvif_service_dispatcher_cleanup();
  assert_int_equal(g_test_cleanup_called, 1);

  // Test multiple cleanups (should not crash)
  onvif_service_dispatcher_cleanup();
  assert_int_equal(g_test_cleanup_called, 1); // Should not be called again
}

/**
 * @brief Test successful service registration
 * @param state Test state (unused)
 */
void test_service_dispatcher_register_service(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Test basic service registration
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler,
    test_service_init_handler, test_service_cleanup_handler);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_init_called, 1);

  // Verify service is registered
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration with NULL parameters
 * @param state Test state (unused)
 */
void test_service_dispatcher_register_service_null_params(void** state) {
  (void)state;
  reset_test_state();

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
void test_service_dispatcher_register_service_invalid_params(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Test empty service name
  onvif_service_registration_t registration_empty_name = ONVIF_SERVICE_REGISTRATION(
    "", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&registration_empty_name);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL service name
  onvif_service_registration_t registration_null_name = ONVIF_SERVICE_REGISTRATION(
    NULL, "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_null_name);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test empty namespace
  onvif_service_registration_t registration_empty_ns =
    ONVIF_SERVICE_REGISTRATION("device", "", test_service_operation_handler, NULL, NULL);

  result = onvif_service_dispatcher_register_service(&registration_empty_ns);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL namespace
  onvif_service_registration_t registration_null_ns =
    ONVIF_SERVICE_REGISTRATION("device", NULL, test_service_operation_handler, NULL, NULL);

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
void test_service_dispatcher_register_service_duplicate(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Register first service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Try to register same service again
  result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_ERROR_ALREADY_EXISTS);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service registration when registry is full
 * @param state Test state (unused)
 */
void test_service_dispatcher_register_service_registry_full(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Register maximum number of services (MAX_SERVICES_TEST)
  static char service_names[MAX_SERVICES_TEST][MAX_SERVICE_NAME_LENGTH];
  for (int i = 0; i < MAX_SERVICES_TEST; i++) {
    (void)snprintf(service_names[i], sizeof(service_names[i]), "service_%d", i);

    onvif_service_registration_t registration = {
      .service_name = service_names[i],
      .namespace_uri = "http://test.namespace.uri",
      .operation_handler = test_service_operation_handler,
      .init_handler = NULL,
      .cleanup_handler = NULL,
      .capabilities_handler = NULL,
      .reserved = {NULL, NULL, NULL, NULL}};

    int result = onvif_service_dispatcher_register_service(&registration);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Try to register one more service (should fail)
  onvif_service_registration_t overflow_registration = ONVIF_SERVICE_REGISTRATION(
    "overflow_service", "http://test.namespace.uri", test_service_operation_handler, NULL, NULL);

  int result = onvif_service_dispatcher_register_service(&overflow_registration);
  assert_int_equal(result, ONVIF_ERROR_RESOURCE_LIMIT);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service unregistration
 * @param state Test state (unused)
 */
void test_service_dispatcher_unregister_service(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Register a service
  onvif_service_registration_t registration =
    ONVIF_SERVICE_REGISTRATION("device", "http://www.onvif.org/ver10/device/wsdl",
                               test_service_operation_handler, NULL, test_service_cleanup_handler);

  onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  // Unregister the service
  int result = onvif_service_dispatcher_unregister_service("device");
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_cleanup_called, 1);

  // Verify service is no longer registered
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 0);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service unregistration when service not found
 * @param state Test state (unused)
 */
void test_service_dispatcher_unregister_service_not_found(void** state) {
  (void)state;
  reset_test_state();

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
void test_service_dispatcher_dispatch(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Register a service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  onvif_service_dispatcher_register_service(&registration);

  // Create mock request and response
  http_request_t request = {0};
  http_response_t response = {0};

  // Test dispatch
  int result =
    onvif_service_dispatcher_dispatch("device", "GetDeviceInformation", &request, &response);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_operation_called, 1);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test dispatch with invalid parameters
 * @param state Test state (unused)
 */
void test_service_dispatcher_dispatch_invalid_params(void** state) {
  (void)state;
  reset_test_state();

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
void test_service_dispatcher_dispatch_service_not_found(void** state) {
  (void)state;
  reset_test_state();

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
void test_service_dispatcher_is_registered(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Test with non-existent service
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 0);

  // Register a service
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  onvif_service_dispatcher_register_service(&registration);

  // Test with registered service
  assert_int_equal(onvif_service_dispatcher_is_registered("device"), 1);

  // Test with NULL service name
  assert_int_equal(onvif_service_dispatcher_is_registered(NULL), 0);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test getting list of registered services
 * @param state Test state (unused)
 */
void test_service_dispatcher_get_services(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  const char* services[MAX_SERVICES_TEST];

  // Test with no services registered
  int count = onvif_service_dispatcher_get_services(services, MAX_SERVICES_TEST);
  assert_int_equal(count, 0);

  // Register some services
  onvif_service_registration_t device_reg = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler, NULL, NULL);

  onvif_service_registration_t media_reg = ONVIF_SERVICE_REGISTRATION(
    "media", "http://www.onvif.org/ver10/media/wsdl", test_service_operation_handler, NULL, NULL);

  onvif_service_dispatcher_register_service(&device_reg);
  onvif_service_dispatcher_register_service(&media_reg);

  // Test getting services
  count = onvif_service_dispatcher_get_services(services, MAX_SERVICES_TEST);
  assert_int_equal(count, 2);

  // Verify service names are returned
  int found_device = 0;
  int found_media = 0;
  for (int i = 0; i < count; i++) {
    if (strcmp(services[i], "device") == 0) {
      found_device = 1;
    }
    if (strcmp(services[i], "media") == 0) {
      found_media = 1;
    }
  }
  assert_int_equal(found_device, 1);
  assert_int_equal(found_media, 1);

  // Test with limited buffer
  count = onvif_service_dispatcher_get_services(services, 1);
  assert_int_equal(count, 1);

  // Test with NULL services array
  count = onvif_service_dispatcher_get_services(NULL, MAX_SERVICES_TEST);
  assert_int_equal(count, ONVIF_ERROR_INVALID);

  // Test with zero max_services
  count = onvif_service_dispatcher_get_services(services, 0);
  assert_int_equal(count, ONVIF_ERROR_INVALID);

  onvif_service_dispatcher_cleanup();
}

/**
 * @brief Test service initialization and cleanup handlers
 * @param state Test state (unused)
 */
void test_service_dispatcher_init_cleanup_handlers(void** state) {
  (void)state;
  reset_test_state();

  onvif_service_dispatcher_init();

  // Test with successful init handler
  onvif_service_registration_t registration = ONVIF_SERVICE_REGISTRATION(
    "device", "http://www.onvif.org/ver10/device/wsdl", test_service_operation_handler,
    test_service_init_handler, test_service_cleanup_handler);

  int result = onvif_service_dispatcher_register_service(&registration);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_test_init_called, 1);

  // Test with failing init handler
  reset_test_state();
  onvif_service_dispatcher_init(); // Re-initialize after reset
  g_test_init_result = ONVIF_ERROR_INVALID;

  onvif_service_registration_t failing_registration = ONVIF_SERVICE_REGISTRATION(
    "failing_service", "http://test.namespace.uri", test_service_operation_handler,
    test_service_init_handler, test_service_cleanup_handler);

  result = onvif_service_dispatcher_register_service(&failing_registration);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
  assert_int_equal(g_test_init_called, 1);

  // Service should not be registered due to init failure
  assert_int_equal(onvif_service_dispatcher_is_registered("failing_service"), 0);

  onvif_service_dispatcher_cleanup();
}
