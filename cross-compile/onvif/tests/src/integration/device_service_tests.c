/**
 * @file device_service_tests.c
 * @brief Integration tests for ONVIF Device service
 * @author kkrzysztofik
 * @date 2025
 */

#define _POSIX_C_SOURCE 200809L

#include "device_service_tests.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "cmocka_wrapper.h"

// ONVIF project includes
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "services/device/onvif_device.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"

// Test mocks
#include "mocks/platform_mock.h"
#include "mocks/smart_response_mock.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "generated/soapStub.h"  // For gSOAP generated types and enums

// Test mocks
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/http_server_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/network_mock.h"

// Test constants
#define TEST_OPERATION_GET_DEVICE_INFORMATION "GetDeviceInformation"
#define TEST_OPERATION_GET_CAPABILITIES       "GetCapabilities"
#define TEST_OPERATION_GET_SYSTEM_DATE_TIME   "GetSystemDateAndTime"
#define TEST_OPERATION_GET_SERVICES           "GetServices"
#define TEST_OPERATION_SYSTEM_REBOOT          "SystemReboot"
#define TEST_OPERATION_INVALID                "InvalidOperation"

// Test concurrent operations count
#define TEST_CONCURRENT_OPS 10

// Test delay constants
#define TEST_DELAY_10MS  10
#define TEST_DELAY_100MS 100

// Test state structure to hold both app_config and config pointers
typedef struct {
  struct application_config* app_config;
  config_manager_t* config;
} device_test_state_t;

/**
 * @brief Setup function for Device service integration tests
 * @param state Test state pointer
 * @return 0 on success, -1 on failure
 *
 * This function initializes all required components for Device service testing:
 * - Memory manager for tracking allocations
 * - Device service with ONVIF protocol support
 */
int device_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Allocate test state structure
  device_test_state_t* test_state = calloc(1, sizeof(device_test_state_t));
  assert_non_null(test_state);

  // Heap-allocate application config structure (required for config_runtime_init)
  // CRITICAL: Must be heap-allocated because config_runtime_init() stores the pointer
  test_state->app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(test_state->app_config);

  // Initialize runtime configuration system
  int config_result = config_runtime_init(test_state->app_config);
  assert_int_equal(ONVIF_SUCCESS, config_result);

  // Apply default configuration values
  config_result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, config_result);

  // Enable real functions for integration testing (not platform layer)
  service_dispatcher_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);
  config_mock_use_real_function(true);
  gsoap_mock_use_real_function(true);
  http_server_mock_use_real_function(true);
  network_mock_use_real_function(true);
  smart_response_mock_use_real_function(true);

  // Setup platform mock expectations for Imaging service initialization
  // Imaging service calls platform_irled_init during onvif_imaging_init with default level=1
  expect_function_call(__wrap_platform_irled_init);
  expect_value(__wrap_platform_irled_init, level, 1);
  will_return(__wrap_platform_irled_init, PLATFORM_SUCCESS);

  // Imaging service also calls platform_vpss_effect_set 5 times for VPSS effects
  // (brightness, contrast, saturation, sharpness, hue) - all with default value 0
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, 0);  // brightness
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, 0);  // contrast
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, 0);  // saturation
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, 0);  // sharpness
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, 0);  // hue

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize Device service with mock config
  test_state->config = malloc(sizeof(config_manager_t));
  assert_non_null(test_state->config);
  memset(test_state->config, 0, sizeof(config_manager_t));

  // Initialize Device service
  result = onvif_device_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize Media service (required for GetCapabilities integration)
  result = onvif_media_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize PTZ service (required for GetCapabilities integration)
  result = onvif_ptz_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize Imaging service (required for GetCapabilities integration)
  result = onvif_imaging_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Register Imaging service with dispatcher (required for capability queries)
  result = onvif_imaging_service_init(test_state->config);
  assert_int_equal(ONVIF_SUCCESS, result);

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for Device service integration tests
 * @param state Test state pointer
 * @return 0 on success
 *
 * This function cleans up all resources allocated during setup:
 * - Device service cleanup
 * - Memory manager cleanup
 */
int device_service_teardown(void** state) {
  device_test_state_t* test_state = (device_test_state_t*)*state;

  // Free config first, before leak checking
  if (test_state && test_state->config) {
    free(test_state->config);
    test_state->config = NULL;
  }

  // Cleanup all services (in reverse order of initialization)
  onvif_imaging_service_cleanup();
  onvif_imaging_cleanup();
  onvif_ptz_cleanup();
  onvif_media_cleanup();
  onvif_device_cleanup();

  // Cleanup service dispatcher
  onvif_service_dispatcher_cleanup();

  memory_manager_cleanup();

  // Cleanup runtime configuration system
  config_runtime_cleanup();

  // CRITICAL: Free heap-allocated app_config AFTER config_runtime_cleanup()
  // config_runtime_cleanup() sets the global pointer to NULL, so we can safely free now
  if (test_state && test_state->app_config) {
    free(test_state->app_config);
    test_state->app_config = NULL;
  }

  // Free test state structure
  if (test_state) {
    free(test_state);
  }

  // Restore mock behavior for subsequent tests
  service_dispatcher_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);
  config_mock_use_real_function(false);
  gsoap_mock_use_real_function(false);
  http_server_mock_use_real_function(false);
  network_mock_use_real_function(false);
  smart_response_mock_use_real_function(false);

  return 0;
}

/**
 * @brief Test GetDeviceInformation operation with SOAP deserialization
 */
void test_integration_device_get_device_information_fields_validation(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetDeviceInformation", SOAP_DEVICE_GET_DEVICE_INFORMATION, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetDeviceInformation", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetDeviceInformationResponse* device_info = NULL;
  result = soap_test_parse_get_device_info_response(&ctx, &device_info);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(device_info);

  // Step 7: Validate response data - check actual field values
  assert_non_null(device_info->Manufacturer);
  assert_string_equal(device_info->Manufacturer, "Anyka");

  assert_non_null(device_info->Model);
  assert_string_equal(device_info->Model, "AK3918 Camera");

  assert_non_null(device_info->FirmwareVersion);
  assert_string_equal(device_info->FirmwareVersion, "1.0.0");

  assert_non_null(device_info->SerialNumber);
  assert_string_equal(device_info->SerialNumber, "AK3918-001");

  assert_non_null(device_info->HardwareId);
  assert_string_equal(device_info->HardwareId, "1.0");

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test GetCapabilities operation for specific category with SOAP deserialization
 */
void test_integration_device_get_capabilities_specific_category(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetCapabilities", SOAP_DEVICE_GET_CAPABILITIES, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetCapabilities", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetCapabilitiesResponse* caps = NULL;
  result = soap_test_parse_get_capabilities_response(&ctx, &caps);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(caps);

  // Step 7: Validate response data - Device capabilities must be present
  assert_non_null(caps->Capabilities);
  assert_non_null(caps->Capabilities->Device);
  assert_non_null(caps->Capabilities->Device->XAddr);

  // Verify XAddr contains valid URL
  assert_true(strlen(caps->Capabilities->Device->XAddr) > 0);
  assert_true(strstr(caps->Capabilities->Device->XAddr, "http") != NULL);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test GetCapabilities operation for multiple categories with SOAP deserialization
 */
void test_integration_device_get_capabilities_multiple_categories(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope for multiple categories
  http_request_t* request = soap_test_create_request(
    "GetCapabilities", SOAP_DEVICE_GET_CAPABILITIES_MULTI, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetCapabilities", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetCapabilitiesResponse* caps = NULL;
  result = soap_test_parse_get_capabilities_response(&ctx, &caps);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(caps);

  // Step 7: Validate response data - all capability categories must be present
  assert_non_null(caps->Capabilities);

  // Validate Device capabilities
  assert_non_null(caps->Capabilities->Device);
  assert_non_null(caps->Capabilities->Device->XAddr);
  assert_true(strlen(caps->Capabilities->Device->XAddr) > 0);

  // Validate Media capabilities
  assert_non_null(caps->Capabilities->Media);
  assert_non_null(caps->Capabilities->Media->XAddr);
  assert_true(strlen(caps->Capabilities->Media->XAddr) > 0);

  // Validate PTZ capabilities
  assert_non_null(caps->Capabilities->PTZ);
  assert_non_null(caps->Capabilities->PTZ->XAddr);
  assert_true(strlen(caps->Capabilities->PTZ->XAddr) > 0);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test GetSystemDateAndTime operation timezone with SOAP deserialization
 */
void test_integration_device_get_system_date_time_timezone(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetSystemDateAndTime", SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetSystemDateAndTime", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetSystemDateAndTimeResponse* datetime = NULL;
  result = soap_test_parse_get_system_date_time_response(&ctx, &datetime);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(datetime);

  // Step 7: Validate response data - focus on timezone information
  assert_non_null(datetime->SystemDateAndTime);
  assert_non_null(datetime->SystemDateAndTime->TimeZone);
  assert_non_null(datetime->SystemDateAndTime->TimeZone->TZ);

  // Verify timezone string is non-empty and valid format (e.g., "UTC", "GMT+1", etc.)
  assert_true(strlen(datetime->SystemDateAndTime->TimeZone->TZ) > 0);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test GetSystemDateAndTime DST information with SOAP deserialization
 */
void test_integration_device_get_system_date_time_dst(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetSystemDateAndTime", SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetSystemDateAndTime", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetSystemDateAndTimeResponse* datetime = NULL;
  result = soap_test_parse_get_system_date_time_response(&ctx, &datetime);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(datetime);

  // Step 7: Validate response data - focus on DST information
  assert_non_null(datetime->SystemDateAndTime);

  // Verify Daylight Savings field is present and has a valid boolean value
  // DaylightSavings is enum xsd__boolean with values: xsd__boolean__false_ or xsd__boolean__true_
  enum xsd__boolean dst_value = datetime->SystemDateAndTime->DaylightSavings;
  assert_true(dst_value == xsd__boolean__false_ || dst_value == xsd__boolean__true_);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test GetServices operation with SOAP deserialization
 */
void test_integration_device_get_services_namespaces(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GetServices", SOAP_DEVICE_GET_CAPABILITIES, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetServices", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetServicesResponse* services = NULL;
  result = soap_test_parse_get_services_response(&ctx, &services);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(services);

  // Step 7: Validate response data - services array with namespace information
  assert_true(services->__sizeService > 0);
  assert_non_null(services->Service);

  // Validate that each service has required namespace, XAddr, and version
  for (int i = 0; i < services->__sizeService; i++) {
    assert_non_null(services->Service[i].Namespace);
    assert_true(strlen(services->Service[i].Namespace) > 0);

    assert_non_null(services->Service[i].XAddr);
    assert_true(strlen(services->Service[i].XAddr) > 0);

    assert_non_null(services->Service[i].Version);
    assert_true(services->Service[i].Version->Major >= 0);
    assert_true(services->Service[i].Version->Minor >= 0);
  }

  // Verify at least the Device service is present
  int found_device = 0;
  for (int i = 0; i < services->__sizeService; i++) {
    if (strstr(services->Service[i].Namespace, "device/wsdl")) {
      found_device = 1;
      break;
    }
  }
  assert_true(found_device);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test SystemReboot operation
 */
void test_integration_device_handle_operation_null_params(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Test NULL operation name
  int result = onvif_device_handle_operation(NULL, &request, &response);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL request
  result = onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, NULL, &response);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL response
  result = onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, &request, NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test handle_operation with invalid operation name
 */
void test_integration_device_handle_operation_invalid_operation(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Test invalid operation name
  int result = onvif_device_handle_operation(TEST_OPERATION_INVALID, &request, &response);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test handle_operation when service is uninitialized
 */
void test_integration_device_handle_operation_uninitialized(void** state) {
  (void)state;

  memory_manager_init();

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Try to handle operation without initialization
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, &request, &response);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  memory_manager_cleanup();
}

/**
 * @brief Thread function for concurrent GetDeviceInformation test
 */
static void* concurrent_get_device_information_thread(void* arg) {
  (void)arg;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  // DON'T pre-allocate response.body - service will allocate it

  // Execute GetDeviceInformation
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, &request, &response);

  // Free the response body allocated by the service
  if (response.body) {
    ONVIF_FREE(response.body);
  }

  // Store result in thread argument
  int* thread_result = malloc(sizeof(int));
  *thread_result = result;

  return thread_result;
}

/**
 * @brief Test concurrent GetDeviceInformation operations
 */
void test_integration_device_concurrent_get_device_information(void** state) {
  (void)state;

  pthread_t threads[TEST_CONCURRENT_OPS];
  int i;

  // Launch concurrent operations
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    int result = pthread_create(&threads[i], NULL, concurrent_get_device_information_thread, NULL);
    assert_int_equal(0, result);
  }

  // Wait for all threads and verify results
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    void* thread_return;
    pthread_join(threads[i], &thread_return);

    int* result = (int*)thread_return;
    assert_int_equal(ONVIF_SUCCESS, *result);
    free(result);
  }
}

/**
 * @brief Thread function for concurrent GetCapabilities test
 */
static void* concurrent_get_capabilities_thread(void* arg) {
  (void)arg;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  // DON'T pre-allocate response.body - service will allocate it

  // Execute GetCapabilities
  int result = onvif_device_handle_operation(TEST_OPERATION_GET_CAPABILITIES, &request, &response);

  // Free the response body allocated by the service
  if (response.body) {
    ONVIF_FREE(response.body);
  }

  int* thread_result = malloc(sizeof(int));
  *thread_result = result;

  return thread_result;
}

/**
 * @brief Test concurrent GetCapabilities operations
 */
void test_integration_device_concurrent_get_capabilities(void** state) {
  (void)state;

  pthread_t threads[TEST_CONCURRENT_OPS];
  int i;

  // Launch concurrent operations
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    int result = pthread_create(&threads[i], NULL, concurrent_get_capabilities_thread, NULL);
    assert_int_equal(0, result);
  }

  // Wait for all threads and verify results
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    void* thread_return;
    pthread_join(threads[i], &thread_return);

    int* result = (int*)thread_return;
    assert_int_equal(ONVIF_SUCCESS, *result);
    free(result);
  }
}

/**
 * @brief Thread function for concurrent mixed operations test
 */
static void* concurrent_mixed_operations_thread(void* arg) {
  int operation_index = *(int*)arg;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  // DON'T pre-allocate response.body - service will allocate it

  const char* operations[] = {TEST_OPERATION_GET_DEVICE_INFORMATION,
                              TEST_OPERATION_GET_CAPABILITIES, TEST_OPERATION_GET_SYSTEM_DATE_TIME,
                              TEST_OPERATION_GET_SERVICES};

  const char* operation = operations[operation_index % 4];

  // Execute operation
  int result = onvif_device_handle_operation(operation, &request, &response);

  // Free the response body allocated by the service
  if (response.body) {
    ONVIF_FREE(response.body);
  }

  int* thread_result = malloc(sizeof(int));
  *thread_result = result;

  return thread_result;
}

/**
 * @brief Test concurrent mixed operations
 */
void test_integration_device_concurrent_mixed_operations(void** state) {
  (void)state;

  pthread_t threads[TEST_CONCURRENT_OPS];
  int thread_args[TEST_CONCURRENT_OPS];
  int i;

  // Launch concurrent mixed operations
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    thread_args[i] = i;
    int result =
      pthread_create(&threads[i], NULL, concurrent_mixed_operations_thread, &thread_args[i]);
    assert_int_equal(0, result);
  }

  // Wait for all threads and verify results
  for (i = 0; i < TEST_CONCURRENT_OPS; i++) {
    void* thread_return;
    pthread_join(threads[i], &thread_return);

    int* result = (int*)thread_return;
    assert_int_equal(ONVIF_SUCCESS, *result);
    free(result);
  }
}

/**
 * @brief Test Device service configuration integration
 */
void test_integration_device_config_integration(void** state) {
  device_test_state_t* test_state = (device_test_state_t*)*state;

  // Verify configuration is properly integrated
  assert_non_null(test_state);
  assert_non_null(test_state->config);
  assert_non_null(test_state->app_config);

  // Config manager structure is validated by successful initialization
  // Device service stores config pointer and uses it for operation handlers
}

/**
 * @brief Pilot SOAP test for Device GetDeviceInformation operation
 * Tests SOAP envelope parsing and response structure validation
 * Note: This is a standalone test that validates SOAP structure without full service initialization
 */
void test_integration_device_get_device_info_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetDeviceInformation", SOAP_DEVICE_GET_DEVICE_INFORMATION, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Validate request structure
  assert_non_null(request->body);
  assert_true(strstr(request->body, "GetDeviceInformation") != NULL);
  assert_true(strstr(request->body, "soap:Envelope") != NULL ||
              strstr(request->body, "Envelope") != NULL);

  // Step 3: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 4: Call actual service handler (integration test)
  int result = onvif_device_handle_operation("GetDeviceInformation", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 5: Validate HTTP response structure
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);
  assert_true(response.body_length > 0);

  // Step 6: Check for SOAP faults
  char fault_code[256] = {0};
  char fault_string[512] = {0};
  int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);
  assert_int_equal(0, has_fault);

  // Step 7: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetDeviceInformationResponse* device_info_response = NULL;
  result = soap_test_parse_get_device_info_response(&ctx, &device_info_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(device_info_response);

  // Step 8: Validate response data - check for correct field values
  assert_non_null(device_info_response->Manufacturer);
  assert_string_equal(device_info_response->Manufacturer, "Anyka");

  assert_non_null(device_info_response->Model);
  assert_string_equal(device_info_response->Model, "AK3918 Camera");

  assert_non_null(device_info_response->FirmwareVersion);
  assert_string_equal(device_info_response->FirmwareVersion, "1.0.0");

  assert_non_null(device_info_response->SerialNumber);
  assert_string_equal(device_info_response->SerialNumber, "AK3918-001");

  assert_non_null(device_info_response->HardwareId);
  assert_string_equal(device_info_response->HardwareId, "1.0");

  // Step 9: Cleanup resources
  soap_test_cleanup_response_parsing(&ctx);
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);

  // Always free response body - it was allocated by the service handler
  assert_non_null(response.body); // Ensure we have something to free
  ONVIF_FREE(response.body);
  response.body = NULL;
}

/**
 * @brief SOAP test for Device GetCapabilities operation
 */
void test_integration_device_get_capabilities_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetCapabilities", SOAP_DEVICE_GET_CAPABILITIES, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetCapabilities", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetCapabilitiesResponse* caps_response = NULL;
  result = soap_test_parse_get_capabilities_response(&ctx, &caps_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(caps_response);

  // Step 7: Validate response data - capabilities should be present
  assert_non_null(caps_response->Capabilities);

  // Validate Device capabilities
  assert_non_null(caps_response->Capabilities->Device);
  assert_non_null(caps_response->Capabilities->Device->XAddr);

  // Validate Media capabilities (should be present based on dev_caps)
  assert_non_null(caps_response->Capabilities->Media);
  assert_non_null(caps_response->Capabilities->Media->XAddr);

  // Validate PTZ capabilities (should be present based on dev_caps)
  assert_non_null(caps_response->Capabilities->PTZ);
  assert_non_null(caps_response->Capabilities->PTZ->XAddr);

  // Validate Imaging capabilities (should be present based on dev_caps)
  assert_non_null(caps_response->Capabilities->Imaging);
  assert_non_null(caps_response->Capabilities->Imaging->XAddr);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for Device GetSystemDateAndTime operation
 */
void test_integration_device_get_system_date_time_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetSystemDateAndTime", SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetSystemDateAndTime", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetSystemDateAndTimeResponse* datetime_response = NULL;
  result = soap_test_parse_get_system_date_time_response(&ctx, &datetime_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(datetime_response);

  // Step 7: Validate response data - SystemDateAndTime should be present with valid fields
  assert_non_null(datetime_response->SystemDateAndTime);

  // DateTimeType is a scalar enum (not a pointer), validate it has a valid value
  enum tt__SetDateTimeType date_time_type = datetime_response->SystemDateAndTime->DateTimeType;
  assert_true(date_time_type == tt__SetDateTimeType__Manual || date_time_type == tt__SetDateTimeType__NTP);

  // Validate UTCDateTime is present
  assert_non_null(datetime_response->SystemDateAndTime->UTCDateTime);
  assert_non_null(datetime_response->SystemDateAndTime->UTCDateTime->Time);
  assert_non_null(datetime_response->SystemDateAndTime->UTCDateTime->Date);

  // LocalDateTime is optional per ONVIF spec - only validate if present
  if (datetime_response->SystemDateAndTime->LocalDateTime) {
    assert_non_null(datetime_response->SystemDateAndTime->LocalDateTime->Time);
    assert_non_null(datetime_response->SystemDateAndTime->LocalDateTime->Date);
  }

  // Validate TimeZone is present
  assert_non_null(datetime_response->SystemDateAndTime->TimeZone);
  assert_non_null(datetime_response->SystemDateAndTime->TimeZone->TZ);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for Device GetServices operation
 */
void test_integration_device_get_services_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope (using DEVICE_GET_CAPABILITIES as placeholder)
  http_request_t* request =
    soap_test_create_request("GetServices", SOAP_DEVICE_GET_CAPABILITIES, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("GetServices", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__GetServicesResponse* services_response = NULL;
  result = soap_test_parse_get_services_response(&ctx, &services_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(services_response);

  // Step 7: Validate response data - services array should exist with services
  assert_true(services_response->__sizeService > 0);
  assert_non_null(services_response->Service);

  // Validate that at least the Device service is present
  int found_device = 0;
  for (int i = 0; i < services_response->__sizeService; i++) {
    assert_non_null(services_response->Service[i].Namespace);
    assert_non_null(services_response->Service[i].XAddr);
    assert_non_null(services_response->Service[i].Version);

    if (strstr(services_response->Service[i].Namespace, "device/wsdl")) {
      found_device = 1;
    }
  }
  assert_true(found_device);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for Device SystemReboot operation
 */
void test_integration_device_system_reboot_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("SystemReboot", SOAP_DEVICE_SYSTEM_REBOOT, "/onvif/device_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_device_handle_operation("SystemReboot", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tds__SystemRebootResponse* reboot_response = NULL;
  result = soap_test_parse_system_reboot_response(&ctx, &reboot_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(reboot_response);

  // Step 7: Validate response data - Message should be present and non-empty
  assert_non_null(reboot_response->Message);
  assert_true(strlen(reboot_response->Message) > 0);

  // Validate the message contains expected content
  assert_true(strstr(reboot_response->Message, "Rebooting") != NULL ||
              strstr(reboot_response->Message, "reboot") != NULL ||
              strstr(reboot_response->Message, "System") != NULL);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

// Test suite definition
const struct CMUnitTest device_service_tests[] = {
  // GetDeviceInformation tests
  cmocka_unit_test_setup_teardown(test_integration_device_get_device_information_fields_validation,
                                  device_service_setup, device_service_teardown),

  // GetCapabilities tests
  cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_specific_category,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_multiple_categories,
                                  device_service_setup, device_service_teardown),

  // GetSystemDateAndTime tests
  cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_timezone,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_dst,
                                  device_service_setup, device_service_teardown),

  // GetServices tests
  cmocka_unit_test_setup_teardown(test_integration_device_get_services_namespaces,
                                  device_service_setup, device_service_teardown),

  // SystemReboot test

  // Error handling tests
  cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_null_params,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_invalid_operation,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test(test_integration_device_handle_operation_uninitialized),

  // Configuration integration test
  cmocka_unit_test_setup_teardown(test_integration_device_config_integration, device_service_setup,
                                  device_service_teardown),

  // SOAP integration tests (full HTTP/SOAP layer validation)
  cmocka_unit_test_setup_teardown(test_integration_device_get_device_info_soap,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_soap,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_soap,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_services_soap, device_service_setup,
                                  device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_system_reboot_soap, device_service_setup,
                                  device_service_teardown),

  // Concurrent operations tests (may hang - placed at end)
  cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_device_information,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_capabilities,
                                  device_service_setup, device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_concurrent_mixed_operations,
                                  device_service_setup, device_service_teardown),
};
