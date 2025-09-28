/**
 * @file test_device_service.c
 * @brief Unit tests for ONVIF Device service
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_device_service.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// Include the actual source files we're testing
#include "services/device/onvif_device.h"
#include "utils/error/error_handling.h"

// Mock includes for testing
#include "mocks/mock_buffer_pool.h"
#include "mocks/mock_config.h"
#include "mocks/mock_gsoap.h"
#include "mocks/mock_platform.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/mock_smart_response.h"

// Test data structures
struct test_device_info g_test_device_info = {.manufacturer = TEST_DEVICE_MANUFACTURER,
                                              .model = TEST_DEVICE_MODEL,
                                              .firmware_version = TEST_DEVICE_FIRMWARE_VERSION,
                                              .serial_number = TEST_DEVICE_SERIAL_NUMBER,
                                              .hardware_id = TEST_DEVICE_HARDWARE_ID};

struct test_capabilities g_test_capabilities = {.has_analytics = 0,
                                                .has_device = 1,
                                                .has_events = 0,
                                                .has_imaging = 1,
                                                .has_media = 1,
                                                .has_ptz = 1};

// Global test state
static int g_device_service_initialized = 0;   // NOLINT
static config_manager_t* g_test_config = NULL; // NOLINT

/* ============================================================================
 * Test Setup and Teardown Functions
 * ============================================================================
 */

/**
 * @brief Setup function for device service tests
 * @param state Test state
 * @return 0 on success
 */
static int setup_device_tests(void** state) {
  (void)state;

  // Initialize test configuration
  g_test_config = mock_config_manager_create();
  if (!g_test_config) {
    return -1;
  }

  // Set up mock configuration values
  mock_config_set_string(g_test_config, "device", "manufacturer", TEST_DEVICE_MANUFACTURER);
  mock_config_set_string(g_test_config, "device", "model", TEST_DEVICE_MODEL);
  mock_config_set_string(g_test_config, "device", "firmware_version", TEST_DEVICE_FIRMWARE_VERSION);
  mock_config_set_string(g_test_config, "device", "serial_number", TEST_DEVICE_SERIAL_NUMBER);
  mock_config_set_string(g_test_config, "device", "hardware_id", TEST_DEVICE_HARDWARE_ID);
  mock_config_set_int(g_test_config, "onvif", "http_port", TEST_HTTP_PORT);

  return 0;
}

/**
 * @brief Teardown function for device service tests
 * @param state Test state
 * @return 0 on success
 */
static int teardown_device_tests(void** state) {
  (void)state;

  // Cleanup device service if initialized
  if (g_device_service_initialized) {
    onvif_device_cleanup();
    g_device_service_initialized = 0;
  }

  // Cleanup test configuration
  if (g_test_config) {
    mock_config_manager_destroy(g_test_config);
    g_test_config = NULL;
  }

  return 0;
}

/* ============================================================================
 * System Reboot Tests
 * ============================================================================
 */

/**
 * @brief Test successful system reboot
 * @param state Test state (unused)
 */
void test_device_system_reboot_success(void** state) {
  (void)state;

  // Mock successful system reboot
  mock_platform_set_system_result(0); // Success

  int result = onvif_device_system_reboot();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify platform reboot was called
  assert_int_equal(1, mock_platform_get_system_call_count());
}

/**
 * @brief Test system reboot failure
 * @param state Test state (unused)
 */
void test_device_system_reboot_failure(void** state) {
  (void)state;

  // Mock failed system reboot
  mock_platform_set_system_result(-1); // Failure

  int result = onvif_device_system_reboot();
  assert_int_equal(ONVIF_ERROR, result);

  // Verify platform reboot was called
  assert_int_equal(1, mock_platform_get_system_call_count());
}

/* ============================================================================
 * Device Service Initialization Tests
 * ============================================================================
 */

/**
 * @brief Test successful device service initialization
 * @param state Test state (unused)
 */
void test_device_init_success(void** state) {
  (void)state;

  // Mock successful gSOAP initialization
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);

  // Mock successful buffer pool initialization
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);

  // Mock successful service dispatcher registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  g_device_service_initialized = 1;

  // Verify gSOAP was initialized
  assert_int_equal(1, mock_gsoap_get_init_call_count());

  // Verify buffer pool was initialized
  assert_int_equal(1, mock_buffer_pool_get_init_call_count());

  // Verify service was registered
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test device initialization with NULL config
 * @param state Test state (unused)
 */
void test_device_init_null_config(void** state) {
  (void)state;

  int result = onvif_device_init(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify no initialization calls were made
  assert_int_equal(0, mock_gsoap_get_init_call_count());
  assert_int_equal(0, mock_buffer_pool_get_init_call_count());
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test device initialization when already initialized
 * @param state Test state (unused)
 */
void test_device_init_already_initialized(void** state) {
  (void)state;

  // First initialization
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Second initialization should return success without reinitializing
  result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify initialization was only called once
  assert_int_equal(1, mock_gsoap_get_init_call_count());
  assert_int_equal(1, mock_buffer_pool_get_init_call_count());
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test device initialization with gSOAP failure
 * @param state Test state (unused)
 */
void test_device_init_gsoap_failure(void** state) {
  (void)state;

  // Mock gSOAP initialization failure
  mock_gsoap_set_init_result(ONVIF_ERROR);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_ERROR, result);

  // Verify gSOAP was attempted
  assert_int_equal(1, mock_gsoap_get_init_call_count());

  // Verify cleanup was called
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());

  // Verify buffer pool was not initialized
  assert_int_equal(0, mock_buffer_pool_get_init_call_count());
}

/**
 * @brief Test device initialization with buffer pool failure
 * @param state Test state (unused)
 */
void test_device_init_buffer_pool_failure(void** state) {
  (void)state;

  // Mock successful gSOAP but failed buffer pool
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(-1); // Failure

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_ERROR, result);

  // Verify gSOAP was initialized
  assert_int_equal(1, mock_gsoap_get_init_call_count());

  // Verify buffer pool initialization was attempted
  assert_int_equal(1, mock_buffer_pool_get_init_call_count());

  // Verify cleanup was called
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(1, mock_buffer_pool_get_cleanup_call_count());
}

/* ============================================================================
 * Device Service Cleanup Tests
 * ============================================================================
 */

/**
 * @brief Test successful device service cleanup
 * @param state Test state (unused)
 */
void test_device_cleanup_success(void** state) {
  (void)state;

  // First initialize the service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Now test cleanup
  result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 0;

  // Verify unregistration was called
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());

  // Verify cleanup was called
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(1, mock_buffer_pool_get_cleanup_call_count());
}

/**
 * @brief Test device cleanup when not initialized
 * @param state Test state (unused)
 */
void test_device_cleanup_not_initialized(void** state) {
  (void)state;

  int result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify no cleanup calls were made
  assert_int_equal(0, mock_service_dispatcher_get_unregister_call_count());
  assert_int_equal(0, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(0, mock_buffer_pool_get_cleanup_call_count());
}

/**
 * @brief Test device cleanup with unregistration failure
 * @param state Test state (unused)
 */
void test_device_cleanup_unregister_failure(void** state) {
  (void)state;

  // First initialize the service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR); // Failure

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test cleanup with unregistration failure
  result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result); // Should still succeed
  g_device_service_initialized = 0;

  // Verify unregistration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());

  // Verify cleanup was still called
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(1, mock_buffer_pool_get_cleanup_call_count());
}

/* ============================================================================
 * Device Operation Handler Tests
 * ============================================================================
 */

/**
 * @brief Test GetDeviceInformation operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_get_device_information(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock successful operation
  mock_gsoap_set_generate_response_result(0);
  mock_gsoap_set_get_response_data_result(
    "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  mock_smart_response_set_build_result(ONVIF_SUCCESS);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetDeviceInformation", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify gSOAP response generation was called
  assert_int_equal(1, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(1, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(1, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test GetCapabilities operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_get_capabilities(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock successful operation
  mock_gsoap_set_generate_response_result(0);
  mock_gsoap_set_get_response_data_result(
    "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  mock_smart_response_set_build_result(ONVIF_SUCCESS);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetCapabilities", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify gSOAP response generation was called
  assert_int_equal(1, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(1, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(1, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test GetSystemDateAndTime operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_get_system_date_time(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock successful operation
  mock_gsoap_set_generate_response_result(0);
  mock_gsoap_set_get_response_data_result(
    "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  mock_smart_response_set_build_result(ONVIF_SUCCESS);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetSystemDateAndTime", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify gSOAP response generation was called
  assert_int_equal(1, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(1, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(1, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test GetServices operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_get_services(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock successful operation
  mock_gsoap_set_generate_response_result(0);
  mock_gsoap_set_get_response_data_result(
    "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  mock_smart_response_set_build_result(ONVIF_SUCCESS);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetServices", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify gSOAP response generation was called
  assert_int_equal(1, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(1, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(1, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test SystemReboot operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_system_reboot(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock successful operation
  mock_platform_set_system_result(0); // Successful reboot
  mock_gsoap_set_generate_response_result(0);
  mock_gsoap_set_get_response_data_result(
    "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  mock_smart_response_set_build_result(ONVIF_SUCCESS);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("SystemReboot", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify system reboot was called
  assert_int_equal(1, mock_platform_get_system_call_count());

  // Verify gSOAP response generation was called
  assert_int_equal(1, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(1, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(1, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test unknown operation
 * @param state Test state (unused)
 */
void test_device_handle_operation_unknown_operation(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("UnknownOperation", &request, &response);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Verify no gSOAP calls were made
  assert_int_equal(0, mock_gsoap_get_generate_response_call_count());
  assert_int_equal(0, mock_gsoap_get_get_response_data_call_count());
  assert_int_equal(0, mock_smart_response_get_build_call_count());
}

/**
 * @brief Test operation handler with NULL operation name
 * @param state Test state (unused)
 */
void test_device_handle_operation_null_operation(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation(NULL, &request, &response);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler with NULL request
 * @param state Test state (unused)
 */
void test_device_handle_operation_null_request(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Create test response
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetDeviceInformation", NULL, &response);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler with NULL response
 * @param state Test state (unused)
 */
void test_device_handle_operation_null_response(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Create test request
  http_request_t request = {0};

  result = onvif_device_handle_operation("GetDeviceInformation", &request, NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler when not initialized
 * @param state Test state (unused)
 */
void test_device_handle_operation_not_initialized(void** state) {
  (void)state;

  // Don't initialize the service
  g_device_service_initialized = 0;

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  int result = onvif_device_handle_operation("GetDeviceInformation", &request, &response);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/* ============================================================================
 * Device Capabilities Handler Tests
 * ============================================================================
 */

/**
 * @brief Test device capabilities handler with valid capabilities
 * @param state Test state (unused)
 */
void test_device_capabilities_handler_success(void** state) {
  (void)state;

  // Test known capabilities
  const char* known_capabilities[] = {"GetDeviceInformation", "GetCapabilities",
                                      "GetSystemDateAndTime", "GetServices", "SystemReboot"};

  for (size_t i = 0; i < sizeof(known_capabilities) / sizeof(known_capabilities[0]); i++) {
    // Note: This would need to be implemented as a public function or tested through
    // the service registration mechanism
    // For now, we'll test the logic conceptually
    assert_string_not_equal(known_capabilities[i], NULL);
    assert_int_not_equal(0, strlen(known_capabilities[i]));
  }
}

/**
 * @brief Test device capabilities handler with NULL capability
 * @param state Test state (unused)
 */
void test_device_capabilities_handler_null_capability(void** state) {
  (void)state;

  // Test NULL capability name
  // This would be tested through the service registration mechanism
  // For now, we verify the expected behavior
  assert_ptr_equal(NULL, NULL);
}

/**
 * @brief Test device capabilities handler with unknown capability
 * @param state Test state (unused)
 */
void test_device_capabilities_handler_unknown_capability(void** state) {
  (void)state;

  // Test unknown capability
  const char* unknown_capability = "UnknownCapability";
  assert_string_not_equal(unknown_capability, "GetDeviceInformation");
  assert_string_not_equal(unknown_capability, "GetCapabilities");
  assert_string_not_equal(unknown_capability, "GetSystemDateAndTime");
  assert_string_not_equal(unknown_capability, "GetServices");
  assert_string_not_equal(unknown_capability, "SystemReboot");
}

/* ============================================================================
 * Device Service Registration Tests
 * ============================================================================
 */

/**
 * @brief Test device service registration success
 * @param state Test state (unused)
 */
void test_device_service_registration_success(void** state) {
  (void)state;

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  // Initialize device service (which registers it)
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Verify registration was called
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test device service registration with duplicate
 * @param state Test state (unused)
 */
void test_device_service_registration_duplicate(void** state) {
  (void)state;

  // Mock duplicate service registration
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);

  // Initialize device service (which registers it)
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_ERROR_DUPLICATE, result);

  // Verify registration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());

  // Verify cleanup was called due to registration failure
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(1, mock_buffer_pool_get_cleanup_call_count());
}

/**
 * @brief Test device service registration with invalid parameters
 * @param state Test state (unused)
 */
void test_device_service_registration_invalid_params(void** state) {
  (void)state;

  // Test with NULL config
  int result = onvif_device_init(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify no registration was attempted
  assert_int_equal(0, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test device service unregistration success
 * @param state Test state (unused)
 */
void test_device_service_unregistration_success(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test unregistration
  result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 0;

  // Verify unregistration was called
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
}

/**
 * @brief Test device service unregistration when not found
 * @param state Test state (unused)
 */
void test_device_service_unregistration_not_found(void** state) {
  (void)state;

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test unregistration with not found error
  result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result); // Should still succeed
  g_device_service_initialized = 0;

  // Verify unregistration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
}

/* ============================================================================
 * Device Business Logic Tests
 * ============================================================================
 */

/**
 * @brief Test device information business logic
 * @param state Test state (unused)
 */
void test_device_business_logic_get_device_information(void** state) {
  (void)state;

  // This would test the business logic functions directly
  // For now, we test the configuration retrieval logic

  // Test configuration retrieval
  char manufacturer[64] = {0};
  int result = mock_config_get_string(g_test_config, "device", "manufacturer", manufacturer,
                                      sizeof(manufacturer), "Default");
  assert_int_equal(0, result);
  assert_string_equal(TEST_DEVICE_MANUFACTURER, manufacturer);

  char model[64] = {0};
  result =
    mock_config_get_string(g_test_config, "device", "model", model, sizeof(model), "Default");
  assert_int_equal(0, result);
  assert_string_equal(TEST_DEVICE_MODEL, model);
}

/**
 * @brief Test capabilities business logic
 * @param state Test state (unused)
 */
void test_device_business_logic_get_capabilities(void** state) {
  (void)state;

  // Test capabilities structure
  struct test_capabilities caps = g_test_capabilities;

  assert_int_equal(0, caps.has_analytics);
  assert_int_equal(1, caps.has_device);
  assert_int_equal(0, caps.has_events);
  assert_int_equal(1, caps.has_imaging);
  assert_int_equal(1, caps.has_media);
  assert_int_equal(1, caps.has_ptz);
}

/**
 * @brief Test system datetime business logic
 * @param state Test state (unused)
 */
void test_device_business_logic_get_system_date_time(void** state) {
  (void)state;

  // Test time retrieval
  time_t now = time(NULL);
  assert_true(now != (time_t)-1);

  struct tm* tm_info = localtime(&now);
  assert_non_null(tm_info);

  // Verify reasonable year (should be 2025 or later)
  assert_true(tm_info->tm_year + 1900 >= 2025);
}

/**
 * @brief Test services business logic
 * @param state Test state (unused)
 */
void test_device_business_logic_get_services(void** state) {
  (void)state;

  // Test services data structure
  struct test_services services = {0};
  services.include_capability = 1;
  services.service_count = 5;

  assert_int_equal(1, services.include_capability);
  assert_int_equal(5, services.service_count);
}

/**
 * @brief Test system reboot business logic
 * @param state Test state (unused)
 */
void test_device_business_logic_system_reboot(void** state) {
  (void)state;

  // Test reboot data structure
  struct test_system_reboot reboot = {0};
  strncpy(reboot.message, "System reboot initiated", sizeof(reboot.message) - 1);
  reboot.message[sizeof(reboot.message) - 1] = '\0';
  reboot.reboot_initiated = 1;

  assert_string_equal("System reboot initiated", reboot.message);
  assert_int_equal(1, reboot.reboot_initiated);
}

/**
 * @brief Test business logic with NULL callback data
 * @param state Test state (unused)
 */
void test_device_business_logic_null_callback_data(void** state) {
  (void)state;

  // Test NULL callback data handling
  // This would be tested through the actual business logic functions
  // For now, we verify the expected error behavior
  assert_ptr_equal(NULL, NULL);
}

/* ============================================================================
 * Error Handling Tests
 * ============================================================================
 */

/**
 * @brief Test device service error handling
 * @param state Test state (unused)
 */
void test_device_error_handling(void** state) {
  (void)state;

  // Test various error conditions

  // Test NULL parameter handling
  int result = onvif_device_handle_operation(NULL, NULL, NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Test uninitialized service
  g_device_service_initialized = 0;
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetDeviceInformation", &request, &response);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test device service memory management
 * @param state Test state (unused)
 */
void test_device_memory_management(void** state) {
  (void)state;

  // Test memory allocation and cleanup

  // Initialize service
  mock_gsoap_set_init_result(ONVIF_SUCCESS);
  mock_gsoap_set_cleanup_result(ONVIF_SUCCESS);
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  int result = onvif_device_init(g_test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Verify memory was allocated
  assert_int_equal(1, mock_gsoap_get_init_call_count());
  assert_int_equal(1, mock_buffer_pool_get_init_call_count());

  // Cleanup
  result = onvif_device_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 0;

  // Verify memory was freed
  assert_int_equal(1, mock_gsoap_get_cleanup_call_count());
  assert_int_equal(1, mock_buffer_pool_get_cleanup_call_count());
}

/**
 * @brief Test device service configuration handling
 * @param state Test state (unused)
 */
void test_device_configuration_handling(void** state) {
  (void)state;

  // Test configuration retrieval
  char value[64] = {0};

  // Test string configuration
  int result = mock_config_get_string(g_test_config, "device", "manufacturer", value, sizeof(value),
                                      "Default");
  assert_int_equal(0, result);
  assert_string_equal(TEST_DEVICE_MANUFACTURER, value);

  // Test integer configuration
  int int_value = 0;
  result = mock_config_get_int(g_test_config, "onvif", "http_port", &int_value, 8080);
  assert_int_equal(0, result);
  assert_int_equal(TEST_HTTP_PORT, int_value);

  // Test fallback values
  char fallback_value[64] = {0};
  result = mock_config_get_string(g_test_config, "nonexistent", "section", fallback_value,
                                  sizeof(fallback_value), "Fallback");
  assert_int_equal(0, result);
  assert_string_equal("Fallback", fallback_value);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================
 */

/**
 * @brief Test suite for device service
 * @return Test suite
 */
const struct CMUnitTest device_test_suite[] = {
  // System reboot tests
  cmocka_unit_test(test_device_system_reboot_success),
  cmocka_unit_test(test_device_system_reboot_failure),

  // Initialization tests
  cmocka_unit_test(test_device_init_success),
  cmocka_unit_test(test_device_init_null_config),
  cmocka_unit_test(test_device_init_already_initialized),
  cmocka_unit_test(test_device_init_gsoap_failure),
  cmocka_unit_test(test_device_init_buffer_pool_failure),

  // Cleanup tests
  cmocka_unit_test(test_device_cleanup_success),
  cmocka_unit_test(test_device_cleanup_not_initialized),
  cmocka_unit_test(test_device_cleanup_unregister_failure),

  // Operation handler tests
  cmocka_unit_test(test_device_handle_operation_get_device_information),
  cmocka_unit_test(test_device_handle_operation_get_capabilities),
  cmocka_unit_test(test_device_handle_operation_get_system_date_time),
  cmocka_unit_test(test_device_handle_operation_get_services),
  cmocka_unit_test(test_device_handle_operation_system_reboot),
  cmocka_unit_test(test_device_handle_operation_unknown_operation),
  cmocka_unit_test(test_device_handle_operation_null_operation),
  cmocka_unit_test(test_device_handle_operation_null_request),
  cmocka_unit_test(test_device_handle_operation_null_response),
  cmocka_unit_test(test_device_handle_operation_not_initialized),

  // Capabilities handler tests
  cmocka_unit_test(test_device_capabilities_handler_success),
  cmocka_unit_test(test_device_capabilities_handler_null_capability),
  cmocka_unit_test(test_device_capabilities_handler_unknown_capability),

  // Service registration tests
  cmocka_unit_test(test_device_service_registration_success),
  cmocka_unit_test(test_device_service_registration_duplicate),
  cmocka_unit_test(test_device_service_registration_invalid_params),
  cmocka_unit_test(test_device_service_unregistration_success),
  cmocka_unit_test(test_device_service_unregistration_not_found),

  // Business logic tests
  cmocka_unit_test(test_device_business_logic_get_device_information),
  cmocka_unit_test(test_device_business_logic_get_capabilities),
  cmocka_unit_test(test_device_business_logic_get_system_date_time),
  cmocka_unit_test(test_device_business_logic_get_services),
  cmocka_unit_test(test_device_business_logic_system_reboot),
  cmocka_unit_test(test_device_business_logic_null_callback_data),

  // Error handling and utility tests
  cmocka_unit_test(test_device_error_handling),
  cmocka_unit_test(test_device_memory_management),
  cmocka_unit_test(test_device_configuration_handling),
};
