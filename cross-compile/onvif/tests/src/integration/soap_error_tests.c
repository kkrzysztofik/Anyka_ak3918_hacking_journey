/**
 * @file soap_error_tests.c
 * @brief SOAP error handling integration tests for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#define _POSIX_C_SOURCE 200809L

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Test framework
#include "cmocka_wrapper.h"
#include "soap_error_tests.h"

// ONVIF project includes
#include "services/common/service_dispatcher.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Mock control headers for integration testing
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/http_server_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/network_mock.h"
#include "mocks/smart_response_mock.h"

/**
 * @brief Setup function for SOAP error tests
 */
int soap_error_tests_setup(void** state) {
  (void)state;

  /* Enable real functions for integration testing (not platform layer) */
  service_dispatcher_mock_use_real_function(true);
  gsoap_mock_use_real_function(true);
  network_mock_use_real_function(true);
  http_server_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);
  smart_response_mock_use_real_function(true);
  config_mock_use_real_function(true);

  // Initialize memory manager for tracking
  memory_manager_init();

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  if (result != ONVIF_SUCCESS) {
    return -1;
  }

  // Initialize media service (as representative service for error testing)
  result = onvif_media_init();
  if (result != ONVIF_SUCCESS) {
    onvif_service_dispatcher_cleanup();
    return -1;
  }

  return 0;
}

/**
 * @brief Teardown function for SOAP error tests
 */
int soap_error_tests_teardown(void** state) {
  (void)state;

  // Cleanup media service
  onvif_media_cleanup();

  // Cleanup service dispatcher
  onvif_service_dispatcher_cleanup();

  /* Reset mock functions to default state */
  service_dispatcher_mock_use_real_function(false);
  gsoap_mock_use_real_function(false);
  network_mock_use_real_function(false);
  http_server_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);
  smart_response_mock_use_real_function(false);
  config_mock_use_real_function(false);

  return 0;
}

/**
 * @brief Test SOAP error handling for invalid XML
 */
void test_integration_soap_error_invalid_xml(void** state) {
  (void)state;

  // Step 1: Create SOAP request with invalid XML
  http_request_t* request = soap_test_create_request("GetProfiles", SOAP_INVALID_XML, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler - should handle error gracefully
  int result = onvif_media_handle_request("GetProfiles", request, &response);

  // Step 4: Validate that we got an error response (not a success)
  // The service may return an error code or generate a fault response
  // Accept both error return code and fault response
  if (result == ONVIF_SUCCESS) {
    // If success, should have generated a fault response
    assert_non_null(response.body);
    assert_true(response.body_length > 0);

    // Step 5: Check for SOAP fault
    char fault_code[256] = {0};
    char fault_string[512] = {0};
    int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);
    assert_int_equal(1, has_fault); // Should have a fault

    // Step 6: Validate fault contains error information
    assert_true(strlen(fault_code) > 0 || strlen(fault_string) > 0);
  } else {
    // Error return code is also acceptable for invalid XML
    assert_true(result != ONVIF_SUCCESS);
  }

  // Step 7: Cleanup resources
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test SOAP error handling for missing required parameter
 */
void test_integration_soap_error_missing_param(void** state) {
  (void)state;

  // Step 1: Create SOAP request with missing required parameter
  // GetStreamUri requires ProfileToken, but SOAP_MISSING_REQUIRED_PARAM omits it
  http_request_t* request = soap_test_create_request("GetStreamUri", SOAP_MISSING_REQUIRED_PARAM, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Validate request was created
  assert_non_null(request->body);
  assert_true(strstr(request->body, "GetStreamUri") != NULL);

  // Step 3: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 4: Call service handler
  int result = onvif_media_handle_request("GetStreamUri", request, &response);

  // Step 5: Should get error or fault response
  if (result == ONVIF_SUCCESS && response.body) {
    // Check for SOAP fault
    char fault_code[256] = {0};
    char fault_string[512] = {0};
    int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);

    // Should have a fault for missing parameter
    assert_int_equal(1, has_fault);

    // Step 6: Validate fault indicates client error (missing parameter)
    // Fault code should indicate client-side error
    if (strlen(fault_code) > 0) {
      assert_true(strstr(fault_code, "Client") != NULL || strstr(fault_code, "Sender") != NULL);
    }
  } else {
    // Error return code is acceptable
    assert_true(result != ONVIF_SUCCESS);
  }

  // Step 7: Cleanup resources
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test SOAP error handling for wrong operation name
 */
void test_integration_soap_error_wrong_operation(void** state) {
  (void)state;

  // Step 1: Create SOAP request with non-existent operation
  http_request_t* request = soap_test_create_request("NonExistentOperation", SOAP_WRONG_OPERATION, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Validate request contains wrong operation
  assert_non_null(request->body);
  assert_true(strstr(request->body, "NonExistentOperation") != NULL);

  // Step 3: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 4: Call service handler with wrong operation name
  int result = onvif_media_handle_request("NonExistentOperation", request, &response);

  // Step 5: Should get error or fault response
  if (result == ONVIF_SUCCESS && response.body) {
    // Check for SOAP fault
    char fault_code[256] = {0};
    char fault_string[512] = {0};
    int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);

    // Should have a fault for unknown operation
    assert_int_equal(1, has_fault);

    // Step 6: Validate fault indicates unknown operation
    if (strlen(fault_string) > 0) {
      // Fault string should mention unknown/unsupported operation
      assert_true(strstr(fault_string, "unknown") != NULL || strstr(fault_string, "Unknown") != NULL || strstr(fault_string, "unsupported") != NULL ||
                  strstr(fault_string, "Unsupported") != NULL || strstr(fault_string, "not found") != NULL);
    }
  } else {
    // Error return code is acceptable
    assert_true(result != ONVIF_SUCCESS);
  }

  // Step 7: Cleanup resources
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief Test SOAP error handling for malformed envelope
 */
void test_integration_soap_error_malformed_envelope(void** state) {
  (void)state;

  // Step 1: Create SOAP request with empty body
  http_request_t* request = soap_test_create_request("GetProfiles", SOAP_EMPTY_BODY, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Validate request has empty body
  assert_non_null(request->body);
  // Empty body should just have envelope header/footer
  assert_true(strstr(request->body, "<s:Body>") != NULL);
  assert_true(strstr(request->body, "</s:Body>") != NULL);

  // Step 3: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 4: Call service handler with empty body
  int result = onvif_media_handle_request("GetProfiles", request, &response);

  // Step 5: Should get error or fault response
  if (result == ONVIF_SUCCESS && response.body) {
    // Check for SOAP fault
    char fault_code[256] = {0};
    char fault_string[512] = {0};
    int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);

    // Should have a fault for empty/malformed envelope
    assert_int_equal(1, has_fault);

    // Step 6: Validate fault information is present
    assert_true(strlen(fault_code) > 0 || strlen(fault_string) > 0);
  } else {
    // Error return code is acceptable for malformed envelope
    assert_true(result != ONVIF_SUCCESS);
  }

  // Step 7: Cleanup resources
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP error test suite
 */
static const struct CMUnitTest soap_error_tests[] = {
  cmocka_unit_test(test_integration_soap_error_invalid_xml),
  cmocka_unit_test(test_integration_soap_error_missing_param),
  cmocka_unit_test(test_integration_soap_error_wrong_operation),
  cmocka_unit_test(test_integration_soap_error_malformed_envelope),
};

/**
 * @brief Get SOAP error integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_soap_error_integration_tests(size_t* count) {
  *count = sizeof(soap_error_tests) / sizeof(soap_error_tests[0]);
  return soap_error_tests;
}
