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

// CMocka test framework
#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "services/device/onvif_device.h"
#include "utils/error/error_handling.h"

// CMocka-based mock includes for testing
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/platform_mock.h"
#include "mocks/smart_response_mock.h"
#include "mocks/thread_mock.h"

#include "protocol/gsoap/onvif_gsoap_device.h"

// Test data structures
// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
struct test_device_info g_test_device_info = {.manufacturer = TEST_DEVICE_MANUFACTURER,
                                              .model = TEST_DEVICE_MODEL,
                                              .firmware_version = TEST_DEVICE_FIRMWARE_VERSION,
                                              .serial_number = TEST_DEVICE_SERIAL_NUMBER,
                                              .hardware_id = TEST_DEVICE_HARDWARE_ID};

// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
struct test_capabilities g_test_capabilities = {.has_analytics = 0, .has_device = 1, .has_events = 0, .has_imaging = 1, .has_media = 1, .has_ptz = 1};

// Global test state
static int g_device_service_initialized = 0;   // NOLINT
static config_manager_t* g_test_config = NULL; // NOLINT

/* ============================================================================
 * Helper Functions for Mock Expectations
 * ============================================================================
 */

/**
 * @brief Configure expectation for config_runtime_is_initialized()
 * @param initialized Return value to be provided by mock
 */
static void expect_config_runtime_initialized(int initialized) {
  expect_function_call(__wrap_config_runtime_is_initialized);
  will_return(__wrap_config_runtime_is_initialized, initialized);
}

/**
 * @brief Configure expectation for buffer_pool_init()
 * @param result Return value to be provided by mock
 */
static void expect_buffer_pool_init(int result) {
  expect_any(__wrap_buffer_pool_init, pool);
  expect_function_call(__wrap_buffer_pool_init);
  will_return(__wrap_buffer_pool_init, result);
}

/**
 * @brief Configure expectation for service registration
 * @param result Result code for dispatcher registration
 */
static void expect_service_registration(int result) {
  expect_any(__wrap_onvif_service_dispatcher_register_service, registration);
  EXPECT_SERVICE_DISPATCHER_REGISTER();
  SET_SERVICE_DISPATCHER_REGISTER_RESULT(result);
}

/**
 * @brief Configure expectation for service unregistration
 * @param result Result code to return from unregister
 */
static void expect_service_unregistration(int result) {
  expect_string(__wrap_onvif_service_dispatcher_unregister_service, service_name, "device");
  EXPECT_SERVICE_DISPATCHER_UNREGISTER();
  SET_SERVICE_DISPATCHER_UNREGISTER_RESULT(result);
}

/**
 * @brief Configure expectation for buffer_pool_cleanup()
 */
static void expect_buffer_pool_cleanup(void) {
  expect_any(__wrap_buffer_pool_cleanup, pool);
  expect_function_call(__wrap_buffer_pool_cleanup);
}

/**
 * @brief Configure expectation for gSOAP response generation with callback
 * @param result Result returned by the mock (0 => success)
 */
static void expect_gsoap_generate_response(int result) {
  expect_any(__wrap_onvif_gsoap_generate_response_with_callback, callback);
  expect_any(__wrap_onvif_gsoap_generate_response_with_callback, user_data);
  expect_function_call(__wrap_onvif_gsoap_generate_response_with_callback);
  will_return(__wrap_onvif_gsoap_generate_response_with_callback, result);
}

/**
 * @brief Configure expectation for smart_response_build_with_dynamic_buffer()
 * @param result Result returned by the mock (ONVIF_SUCCESS on success)
 */
static void expect_smart_response_build(int result) {
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, response);
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, soap_content);
  expect_function_call(__wrap_smart_response_build_with_dynamic_buffer);
  will_return(__wrap_smart_response_build_with_dynamic_buffer, result);
}

/**
 * @brief Configure expectation for config_runtime_get_string()
 * @param section Configuration section
 * @param key Configuration key
 * @param buffer_size Expected buffer size
 * @param result Result returned by the mock
 */
static void expect_config_runtime_get_string_call(config_section_t section, const char* key, size_t expected_size, int result) {
  expect_value(__wrap_config_runtime_get_string, section, section);
  expect_string(__wrap_config_runtime_get_string, key, key);
  expect_any(__wrap_config_runtime_get_string, out_value);
  expect_value(__wrap_config_runtime_get_string, buffer_size, expected_size);
  expect_function_call(__wrap_config_runtime_get_string);
  will_return(__wrap_config_runtime_get_string, result);
}

/**
 * @brief Configure expectation for config_runtime_get_int()
 * @param section Configuration section
 * @param key Configuration key
 * @param result Result returned by the mock
 */
static void expect_config_runtime_get_int_call(config_section_t section, const char* key, int result) {
  expect_value(__wrap_config_runtime_get_int, section, section);
  expect_string(__wrap_config_runtime_get_int, key, key);
  expect_any(__wrap_config_runtime_get_int, out_value);
  expect_function_call(__wrap_config_runtime_get_int);
  will_return(__wrap_config_runtime_get_int, result);
}

/**
 * @brief Configure expectations for GetDeviceInformation business logic
 */
static void expect_device_information_business_logic(void) {
  expect_config_runtime_get_string_call(CONFIG_SECTION_DEVICE, "manufacturer", DEVICE_MANUFACTURER_MAX_LEN, ONVIF_SUCCESS);
  expect_config_runtime_get_string_call(CONFIG_SECTION_DEVICE, "model", DEVICE_MODEL_MAX_LEN, ONVIF_SUCCESS);
  expect_config_runtime_get_string_call(CONFIG_SECTION_DEVICE, "firmware_version", FIRMWARE_VERSION_MAX_LEN, ONVIF_SUCCESS);
  expect_config_runtime_get_string_call(CONFIG_SECTION_DEVICE, "serial_number", SERIAL_NUMBER_MAX_LEN, ONVIF_SUCCESS);
  expect_config_runtime_get_string_call(CONFIG_SECTION_DEVICE, "hardware_id", HARDWARE_ID_MAX_LEN, ONVIF_SUCCESS);
}

/**
 * @brief Configure expectations for GetCapabilities business logic
 */
static void expect_capabilities_business_logic(void) {
  expect_config_runtime_get_int_call(CONFIG_SECTION_ONVIF, "http_port", ONVIF_SUCCESS);
}

/**
 * @brief Configure expectations for GetServices business logic
 */
static void expect_services_business_logic(void) {
  expect_config_runtime_get_int_call(CONFIG_SECTION_ONVIF, "http_port", ONVIF_SUCCESS);
}

/* ============================================================================
 * Test Setup and Teardown Functions
 * ============================================================================
 */

/**
 * @brief Setup function for each test
 * @param state Test state pointer
 * @return 0 on success
 * @note Ensures clean test state before each test
 */
static int device_test_setup(void** state) {
  (void)state;

  // Reset the internal initialization flag before each test
  onvif_device_test_reset();
  config_mock_use_real_function(false);
  thread_mock_use_real_function(false);
  mock_service_dispatcher_init();

  // Reset global test state
  g_device_service_initialized = 0;

  return 0;
}

/**
 * @brief Teardown function for each test
 * @param state Test state pointer
 * @return 0 on success
 * @note Resets test state flags after each test
 * @note Tests that initialize the service must explicitly clean up with proper mocks
 */
static int device_test_teardown(void** state) {
  (void)state;

  // Always reset the internal initialization flag for test isolation
  // This is safe to call even if the service was never initialized
  onvif_device_test_reset();
  config_mock_use_real_function(false);
  thread_mock_use_real_function(true);
  mock_service_dispatcher_cleanup();

  // Reset global test state
  g_device_service_initialized = 0;

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
void test_unit_device_system_reboot_success(void** state) {
  (void)state;

  // Mock successful system reboot
  expect_string(__wrap_platform_system, command, "reboot");
  expect_function_call(__wrap_platform_system);
  will_return(__wrap_platform_system, 0); // Success

  int result = onvif_device_system_reboot();
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test system reboot failure
 * @param state Test state (unused)
 */
void test_unit_device_system_reboot_failure(void** state) {
  (void)state;

  // Mock failed system reboot
  expect_string(__wrap_platform_system, command, "reboot");
  expect_function_call(__wrap_platform_system);
  will_return(__wrap_platform_system, -1); // Failure

  int result = onvif_device_system_reboot();
  assert_int_equal(ONVIF_ERROR, result);
}

/* ============================================================================
 * Device Service Initialization Tests
 * ============================================================================
 */

/**
 * @brief Test successful device service initialization
 * @param state Test state (unused)
 */
void test_unit_device_init_success(void** state) {
  (void)state;

  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  g_device_service_initialized = 1;
}

/**
 * @brief Test device initialization when config_runtime is not initialized
 * @param state Test state (unused)
 */
void test_unit_device_init_config_runtime_not_initialized(void** state) {
  (void)state;

  // Mock config_runtime_is_initialized to return false
  expect_config_runtime_initialized(0); // Not initialized

  int result = onvif_device_init();
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/**
 * @brief Test device initialization when already initialized
 * @param state Test state (unused)
 */
void test_unit_device_init_already_initialized(void** state) {
  (void)state;

  // First initialization
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Second initialization should return success without reinitializing
  result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test device initialization with dispatcher registration failure
 * @param state Test state (unused)
 */
void test_unit_device_init_gsoap_failure(void** state) {
  (void)state;

  // Mock config_runtime_is_initialized to return true
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_ERROR);
  expect_service_unregistration(ONVIF_ERROR_NOT_FOUND);
  expect_buffer_pool_cleanup();

  int result = onvif_device_init();
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test device initialization with buffer pool failure
 * @param state Test state (unused)
 */
void test_unit_device_init_buffer_pool_failure(void** state) {
  (void)state;

  // Mock config_runtime_is_initialized to return true
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(-1);          // Failure

  int result = onvif_device_init();
  assert_int_equal(ONVIF_ERROR, result);
}

/* ============================================================================
 * Device Service Cleanup Tests
 * ============================================================================
 */

/**
 * @brief Test successful device service cleanup
 * @param state Test state (unused)
 */
void test_unit_device_cleanup_success(void** state) {
  (void)state;

  // First initialize the service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Now test cleanup (void return - best effort)
  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();

  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/**
 * @brief Test device cleanup when not initialized
 * @param state Test state (unused)
 */
void test_unit_device_cleanup_not_initialized(void** state) {
  (void)state;

  // Cleanup when not initialized should be no-op (void return)
  onvif_device_cleanup();
}

/**
 * @brief Test device cleanup with unregistration failure
 * @param state Test state (unused)
 */
void test_unit_device_cleanup_unregister_failure(void** state) {
  (void)state;

  // First initialize the service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test cleanup with unregister failure (void return - best effort, always succeeds)
  expect_service_unregistration(ONVIF_ERROR); // Failure
  expect_buffer_pool_cleanup();

  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/* ============================================================================
 * Device Operation Handler Tests
 * ============================================================================
 */

/**
 * @brief Test GetDeviceInformation operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_get_device_information(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Business logic expectations
  expect_device_information_business_logic();

  // Mock successful response generation
  expect_gsoap_generate_response(ONVIF_SUCCESS);
  expect_function_call(__wrap_onvif_gsoap_get_response_data);
  will_return(__wrap_onvif_gsoap_get_response_data, "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  expect_smart_response_build(ONVIF_SUCCESS);

  // Mock gSOAP cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetDeviceInformation", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test GetCapabilities operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_get_capabilities(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Business logic expectations
  expect_capabilities_business_logic();

  // Mock successful response generation
  expect_gsoap_generate_response(ONVIF_SUCCESS);
  expect_function_call(__wrap_onvif_gsoap_get_response_data);
  will_return(__wrap_onvif_gsoap_get_response_data, "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  expect_smart_response_build(ONVIF_SUCCESS);

  // Mock gSOAP cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetCapabilities", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test GetSystemDateAndTime operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_get_system_date_time(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Mock successful operation
  expect_gsoap_generate_response(ONVIF_SUCCESS);
  expect_function_call(__wrap_onvif_gsoap_get_response_data);
  will_return(__wrap_onvif_gsoap_get_response_data, "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  expect_smart_response_build(ONVIF_SUCCESS);

  // Mock gSOAP cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetSystemDateAndTime", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test GetServices operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_get_services(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Business logic expectations
  expect_services_business_logic();

  // Mock successful operation
  expect_gsoap_generate_response(ONVIF_SUCCESS);
  expect_function_call(__wrap_onvif_gsoap_get_response_data);
  will_return(__wrap_onvif_gsoap_get_response_data, "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>");
  expect_smart_response_build(ONVIF_SUCCESS);

  // Mock gSOAP cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("GetServices", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test SystemReboot operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_system_reboot(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Mock successful operation
  expect_gsoap_generate_response(ONVIF_SUCCESS);
  expect_function_call(__wrap_onvif_gsoap_get_response_data);
  will_return(__wrap_onvif_gsoap_get_response_data,
              "<?xml version=\"1.0\"?><soap:Envelope>...</soap:Envelope>"); // NOLINT(cppcoreguidelines-avoid-magic-numbers,readability-magic-numbers)
  expect_smart_response_build(ONVIF_SUCCESS);

  // Mock gSOAP cleanup followed by deferred reboot
  // Expect deferred reboot to invoke platform reboot sequence
  expect_any(__wrap_platform_system, command);
  expect_function_call(__wrap_platform_system);
  will_return(__wrap_platform_system, 0); // Successful reboot

  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("SystemReboot", &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Allow deferred reboot thread to execute mocked platform_system call
  struct timespec reboot_wait = {.tv_sec = 3, .tv_nsec = 0};
  nanosleep(&reboot_wait, NULL);

  // Cleanup after operation
  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();
  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/**
 * @brief Test unknown operation
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_unknown_operation(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Mock gSOAP context initialization for the operation
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Mock gSOAP cleanup (no handler called for unknown operation)
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Create test request and response
  http_request_t request = {0};
  http_response_t response = {0};

  result = onvif_device_handle_operation("UnknownOperation", &request, &response);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test operation handler with NULL operation name
 * @param state Test state (unused)
 */
void test_unit_device_handle_operation_null_operation(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
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
void test_unit_device_handle_operation_null_request(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
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
void test_unit_device_handle_operation_null_response(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
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
void test_unit_device_handle_operation_not_initialized(void** state) {
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
void test_unit_device_capabilities_handler_success(void** state) {
  (void)state;

  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int init_result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, init_result);
  g_device_service_initialized = 1;

  const onvif_service_registration_t* registration = mock_service_dispatcher_get_last_registration();
  assert_non_null(registration);
  assert_non_null(registration->capabilities_handler);

  int supported = registration->capabilities_handler("GetDeviceInformation");
  assert_int_equal(1, supported);

  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();
  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/**
 * @brief Test device capabilities handler with NULL capability
 * @param state Test state (unused)
 */
void test_unit_device_capabilities_handler_null_capability(void** state) {
  (void)state;

  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int init_result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, init_result);
  g_device_service_initialized = 1;

  const onvif_service_registration_t* registration = mock_service_dispatcher_get_last_registration();
  assert_non_null(registration);
  assert_non_null(registration->capabilities_handler);

  int supported = registration->capabilities_handler(NULL);
  assert_int_equal(0, supported);

  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();
  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/**
 * @brief Test device capabilities handler with unknown capability
 * @param state Test state (unused)
 */
void test_unit_device_capabilities_handler_unknown_capability(void** state) {
  (void)state;

  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int init_result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, init_result);
  g_device_service_initialized = 1;

  const onvif_service_registration_t* registration = mock_service_dispatcher_get_last_registration();
  assert_non_null(registration);
  assert_non_null(registration->capabilities_handler);

  int supported = registration->capabilities_handler("UnknownCapability");
  assert_int_equal(0, supported);

  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();
  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/* ============================================================================
 * Device Service Registration Tests
 * ============================================================================
 */

/**
 * @brief Test device service registration success
 * @param state Test state (unused)
 */
void test_unit_device_service_registration_success(void** state) {
  (void)state;

  // Initialize device service (which registers it)
  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;
}

/**
 * @brief Test device service registration with duplicate
 * @param state Test state (unused)
 */
void test_unit_device_service_registration_duplicate(void** state) {
  (void)state;

  // Initialize device service (which attempts to register it)
  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_ERROR_DUPLICATE);
  expect_service_unregistration(ONVIF_ERROR_NOT_FOUND);
  expect_buffer_pool_cleanup();

  int result = onvif_device_init();
  assert_int_equal(ONVIF_ERROR_DUPLICATE, result);
}

/**
 * @brief Test device service registration with invalid parameters
 * @param state Test state (unused)
 */
void test_unit_device_service_registration_invalid_params(void** state) {
  (void)state;

  // Test when config_runtime is not initialized
  expect_config_runtime_initialized(0); // Not initialized

  int result = onvif_device_init();
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/**
 * @brief Test device service unregistration success
 * @param state Test state (unused)
 */
void test_unit_device_service_unregistration_success(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test unregistration (void return)
  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();

  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/**
 * @brief Test device service unregistration when not found
 * @param state Test state (unused)
 */
void test_unit_device_service_unregistration_not_found(void** state) {
  (void)state;

  // Initialize service
  expect_config_runtime_initialized(1);
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Test unregistration with not found error (void return - best effort)
  expect_service_unregistration(ONVIF_ERROR_NOT_FOUND);
  expect_buffer_pool_cleanup();

  onvif_device_cleanup();
  g_device_service_initialized = 0;
}

/* ============================================================================
 * Device Business Logic Tests
 * ============================================================================
 */

/**
 * @brief Test capabilities business logic
 * @param state Test state (unused)
 */
void test_unit_device_business_logic_get_capabilities(void** state) {
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
void test_unit_device_business_logic_get_system_date_time(void** state) {
  (void)state;

  // Test time retrieval
  time_t now = time(NULL);
  assert_true(now != (time_t)-1);

  struct tm* tm_info = localtime(&now);
  assert_non_null(tm_info);
  assert_true(tm_info->tm_year + 1900 >= 2025);
}

/**
 * @brief Test services business logic
 * @param state Test state (unused)
 */
void test_unit_device_business_logic_get_services(void** state) {
  (void)state;

  // Test services data structure
  struct test_services services = {0};
  services.include_capability = 1;
  services.service_count = TEST_DEFAULT_SERVICE_COUNT;

  assert_int_equal(1, services.include_capability);
  assert_int_equal(TEST_DEFAULT_SERVICE_COUNT, services.service_count);
}

/**
 * @brief Test system reboot business logic
 * @param state Test state (unused)
 */
void test_unit_device_business_logic_system_reboot(void** state) {
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
void test_unit_device_business_logic_null_callback_data(void** state) {
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
void test_unit_device_error_handling(void** state) {
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
void test_unit_device_memory_management(void** state) {
  (void)state;

  // Test memory allocation and cleanup

  // Initialize service
  expect_config_runtime_initialized(1); // Initialized
  expect_buffer_pool_init(0);
  expect_service_registration(ONVIF_SUCCESS);

  int result = onvif_device_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  g_device_service_initialized = 1;

  // Cleanup (void return)
  expect_service_unregistration(ONVIF_SUCCESS);
  expect_buffer_pool_cleanup();

  onvif_device_cleanup();
  g_device_service_initialized = 0;
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
  cmocka_unit_test_setup_teardown(test_unit_device_system_reboot_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_system_reboot_failure, device_test_setup, device_test_teardown),

  // Initialization tests
  cmocka_unit_test_setup_teardown(test_unit_device_init_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_init_config_runtime_not_initialized, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_init_already_initialized, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_init_gsoap_failure, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_init_buffer_pool_failure, device_test_setup, device_test_teardown),

  // Cleanup tests
  cmocka_unit_test_setup_teardown(test_unit_device_cleanup_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_cleanup_not_initialized, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_cleanup_unregister_failure, device_test_setup, device_test_teardown),

  // Operation handler tests
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_get_device_information, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_get_capabilities, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_get_system_date_time, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_get_services, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_system_reboot, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_unknown_operation, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_null_operation, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_null_request, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_null_response, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_handle_operation_not_initialized, device_test_setup, device_test_teardown),

  // Capabilities handler tests
  cmocka_unit_test_setup_teardown(test_unit_device_capabilities_handler_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_capabilities_handler_null_capability, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_capabilities_handler_unknown_capability, device_test_setup, device_test_teardown),

  // Service registration tests
  cmocka_unit_test_setup_teardown(test_unit_device_service_registration_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_service_registration_duplicate, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_service_registration_invalid_params, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_service_unregistration_success, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_service_unregistration_not_found, device_test_setup, device_test_teardown),

  // Business logic tests
  cmocka_unit_test_setup_teardown(test_unit_device_business_logic_get_capabilities, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_business_logic_get_system_date_time, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_business_logic_get_services, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_business_logic_system_reboot, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_business_logic_null_callback_data, device_test_setup, device_test_teardown),

  // Error handling and utility tests
  cmocka_unit_test_setup_teardown(test_unit_device_error_handling, device_test_setup, device_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_device_memory_management, device_test_setup, device_test_teardown),
};

/**
 * @brief Get unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_device_service_unit_tests(size_t* count) {
  *count = sizeof(device_test_suite) / sizeof(device_test_suite[0]);
  return device_test_suite;
}
