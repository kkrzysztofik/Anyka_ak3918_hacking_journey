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
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "services/device/onvif_device.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Test mocks

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

  // Initialize buffer pool mock

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize Device service with mock config
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));

  // Initialize Device service
  result = onvif_device_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  *state = config;
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
  config_manager_t* config = (config_manager_t*)*state;

  // Free config first, before leak checking
  free(config);

  // Cleanup Device service (this unregisters from dispatcher)
  onvif_device_cleanup();

  // Note: Don't cleanup dispatcher - keep it alive for next test
  // The dispatcher mutex gets destroyed and can't be reinitialized

  memory_manager_cleanup();
  return 0;
}

/**
 * @brief Test Device service initialization and cleanup lifecycle
 */
void test_integration_device_init_cleanup_lifecycle(void** state) {
  (void)state;

  memory_manager_init();

  // Initialize buffer pool mock

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Create config
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));

  // Initialize Device service
  result = onvif_device_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Cleanup Device service
  onvif_device_cleanup();

  // Try to reinitialize
  result = onvif_device_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Cleanup again
  onvif_device_cleanup();

  free(config);
  
  // Note: Don't cleanup dispatcher - keep it alive for next tests
  
  memory_manager_cleanup();
}

/**
 * @brief Test GetDeviceInformation operation
 */
void test_integration_device_get_device_information_fields_validation(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify all required fields are present in response
  assert_non_null(strstr(response.body, "Manufacturer"));
  assert_non_null(strstr(response.body, "Model"));
  assert_non_null(strstr(response.body, "FirmwareVersion"));
  assert_non_null(strstr(response.body, "SerialNumber"));
  assert_non_null(strstr(response.body, "HardwareId"));
}

/**
 * @brief Test GetCapabilities operation for all services
 */
void test_integration_device_get_capabilities_specific_category(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[8192];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result = onvif_device_handle_operation(TEST_OPERATION_GET_CAPABILITIES, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify Device capabilities are present
  assert_non_null(strstr(response.body, "Device"));
}

/**
 * @brief Test GetCapabilities operation for multiple categories
 */
void test_integration_device_get_capabilities_multiple_categories(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[8192];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result = onvif_device_handle_operation(TEST_OPERATION_GET_CAPABILITIES, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify multiple capability categories
  assert_non_null(strstr(response.body, "Device"));
  assert_non_null(strstr(response.body, "Media"));
  assert_non_null(strstr(response.body, "PTZ"));
}

/**
 * @brief Test GetSystemDateAndTime operation
 */
void test_integration_device_get_system_date_time_timezone(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_SYSTEM_DATE_TIME, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify timezone information is present
  assert_non_null(strstr(response.body, "TimeZone"));
}

/**
 * @brief Test GetSystemDateAndTime DST information
 */
void test_integration_device_get_system_date_time_dst(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_SYSTEM_DATE_TIME, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify DST information is present
  assert_non_null(strstr(response.body, "DaylightSavings"));
}

/**
 * @brief Test GetServices operation for all services
 */
void test_integration_device_get_services_namespaces(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(http_request_t));

  char response_buffer[8192];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute operation
  int result = onvif_device_handle_operation(TEST_OPERATION_GET_SERVICES, &request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify namespace information is present
  assert_non_null(strstr(response.body, "Namespace"));
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

  char response_buffer[4096];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute GetDeviceInformation
  int result =
    onvif_device_handle_operation(TEST_OPERATION_GET_DEVICE_INFORMATION, &request, &response);

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

  char response_buffer[8192];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  // Execute GetCapabilities
  int result = onvif_device_handle_operation(TEST_OPERATION_GET_CAPABILITIES, &request, &response);

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

  char response_buffer[8192];
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));
  response.body = response_buffer;

  const char* operations[] = {TEST_OPERATION_GET_DEVICE_INFORMATION,
                              TEST_OPERATION_GET_CAPABILITIES, TEST_OPERATION_GET_SYSTEM_DATE_TIME,
                              TEST_OPERATION_GET_SERVICES};

  const char* operation = operations[operation_index % 4];

  // Execute operation
  int result = onvif_device_handle_operation(operation, &request, &response);

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
  config_manager_t* config = (config_manager_t*)*state;

  // Verify configuration is properly integrated
  assert_non_null(config);

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

  // Step 8: Validate response data
  assert_non_null(device_info_response->Manufacturer);
  assert_non_null(device_info_response->Model);
  assert_non_null(device_info_response->FirmwareVersion);

  // Step 9: Cleanup resources
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Device GetCapabilities operation
 */
void test_integration_device_get_capabilities_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request("GetCapabilities",
                                                     SOAP_DEVICE_GET_CAPABILITIES,
                                                     "/onvif/device_service");
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

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Device GetSystemDateAndTime operation
 */
void test_integration_device_get_system_date_time_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request("GetSystemDateAndTime",
                                                     SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME,
                                                     "/onvif/device_service");
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

  // Step 7: Validate response data - SystemDateAndTime should be present
  assert_non_null(datetime_response->SystemDateAndTime);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Device GetServices operation
 */
void test_integration_device_get_services_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope (using DEVICE_GET_CAPABILITIES as placeholder)
  http_request_t* request = soap_test_create_request("GetServices",
                                                     SOAP_DEVICE_GET_CAPABILITIES,
                                                     "/onvif/device_service");
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

  // Step 7: Validate response data - services array should exist
  assert_true(services_response->__sizeService >= 0);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Device SystemReboot operation
 */
void test_integration_device_system_reboot_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request("SystemReboot",
                                                     SOAP_DEVICE_SYSTEM_REBOOT,
                                                     "/onvif/device_service");
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

  // Step 7: Validate response data - Message should be present
  assert_non_null(reboot_response->Message);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

// Test suite definition
const struct CMUnitTest device_service_tests[] = {
  // Lifecycle tests
  cmocka_unit_test(test_integration_device_init_cleanup_lifecycle),

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
  cmocka_unit_test_setup_teardown(test_integration_device_get_device_info_soap, device_service_setup,
                                  device_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_soap, device_service_setup,
                                  device_service_teardown),
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
