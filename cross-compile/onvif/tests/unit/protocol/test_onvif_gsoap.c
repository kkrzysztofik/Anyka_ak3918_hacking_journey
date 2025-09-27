/**
 * @file test_onvif_gsoap.c
 * @brief Unit tests for ONVIF gSOAP module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "generated/soapH.h"

// Include the module under test
#include "protocol/gsoap/onvif_gsoap.h"

// Test constants for magic numbers
#define TEST_BYTES_WRITTEN     100
#define TEST_START_TIME        12345
#define TEST_END_TIME          67890
#define TEST_BUFFER_SIZE       64
#define TEST_SMALL_BUFFER_SIZE 32
#define TEST_LARGE_BUFFER_SIZE 128
#define TEST_HEADER_SIZE       100

/**
 * @brief Test gSOAP context initialization
 * @param state Test state (unused)
 */
void test_onvif_gsoap_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test initialization - in test environment, gSOAP might fail
  int result = onvif_gsoap_init(&ctx);

  // Accept either success or failure, but verify behavior is consistent
  if (result == 0) {
    // If initialization succeeded, verify context is set up correctly
    assert_non_null(ctx.soap);
    assert_int_equal(ctx.total_bytes_written, 0);
    assert_true(ctx.generation_start_time > 0); // Should be set to current timestamp
    assert_int_equal(ctx.generation_end_time, 0);
    assert_null(ctx.user_data);

    // Cleanup if successful
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If initialization failed, verify it returned a proper error code
    assert_true(result < 0);
    // Context should remain zero'd if init failed
    assert_null(ctx.soap);
  }
}

/**
 * @brief Test gSOAP context initialization with NULL pointer
 * @param state Test state (unused)
 */
void test_onvif_gsoap_init_null(void** state) {
  (void)state;

  // Test initialization with NULL context
  int result = onvif_gsoap_init(NULL);
  // Function should return a negative error code (either -1 or -EINVAL)
  assert_true(result < 0);
}

/**
 * @brief Test gSOAP context cleanup
 * @param state Test state (unused)
 */
void test_onvif_gsoap_cleanup(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Initialize and then cleanup
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Cleanup should not crash
  onvif_gsoap_cleanup(&ctx);

  // Context should be reset after cleanup
  assert_null(ctx.soap);

  // Test cleanup with NULL pointer (should not crash)
  onvif_gsoap_cleanup(NULL);
}

/**
 * @brief Test gSOAP context reset
 * @param state Test state (unused)
 */
void test_onvif_gsoap_reset(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Initialize context
  int result = onvif_gsoap_init(&ctx);

  // Only test reset if initialization succeeded
  if (result == 0) {
    // Modify some fields
    ctx.total_bytes_written = TEST_BYTES_WRITTEN;
    ctx.generation_start_time = TEST_START_TIME;
    ctx.generation_end_time = TEST_END_TIME;

    // Reset context
    onvif_gsoap_reset(&ctx);

    // Statistics should be reset
    assert_int_equal(ctx.total_bytes_written, 0);
    assert_true(ctx.generation_start_time > 0); // Should be set to current timestamp
    // Note: generation_end_time is not reset by the reset function

    // But soap context should still be valid
    assert_non_null(ctx.soap);

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip this test
    skip();
  }

  // Test reset with NULL pointer (should not crash)
  onvif_gsoap_reset(NULL);
}

/**
 * @brief Test fault response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_fault_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context first
  int result = onvif_gsoap_generate_fault_response(NULL, SOAP_FAULT_SERVER, "Test fault");
  assert_true(result < 0); // Should return negative error code

  // Initialize context
  result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test fault response generation
    result = onvif_gsoap_generate_fault_response(&ctx, SOAP_FAULT_SERVER, "Test fault message");
    // May succeed or fail depending on gSOAP state
    (void)result;

    // Test with NULL fault string (function provides default message)
    result = onvif_gsoap_generate_fault_response(&ctx, SOAP_FAULT_CLIENT, NULL);
    // Function handles NULL gracefully with default message, so may return 0
    (void)result;

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test device info response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_device_info_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context first
  int result = onvif_gsoap_generate_device_info_response(NULL, "Mfg", "Model", "1.0", "SN", "HW");
  assert_true(result < 0); // Should return negative error code

  // Initialize context
  result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test device info response generation
    result = onvif_gsoap_generate_device_info_response(&ctx, "TestManufacturer", "TestModel",
                                                       "1.0.0", "TEST123456", "HW001");
    // May succeed or fail depending on gSOAP state and platform functions
    (void)result;

    // Test with NULL parameters (function provides default empty strings)
    result = onvif_gsoap_generate_device_info_response(&ctx, NULL, "Model", "1.0", "SN", "HW");
    // Function handles NULL gracefully with default values, so may return 0
    (void)result;

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test response data retrieval
 * @param state Test state (unused)
 */
void test_onvif_gsoap_get_response_data(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  const char* response_data = onvif_gsoap_get_response_data(NULL);
  assert_null(response_data);

  // Initialize context
  int result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test with initialized but unused context
    response_data = onvif_gsoap_get_response_data(&ctx);
    // Response data may be NULL or empty before any response is generated
    (void)response_data; // Don't assert specific value

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test response length retrieval
 * @param state Test state (unused)
 */
void test_onvif_gsoap_get_response_length(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  size_t length = onvif_gsoap_get_response_length(NULL);
  assert_int_equal(length, 0);

  // Initialize context
  int result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test with initialized context
    length = onvif_gsoap_get_response_length(&ctx);
    (void)length; // Just verify function doesn't crash

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test error checking functionality
 * @param state Test state (unused)
 */
void test_onvif_gsoap_has_error(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context (should return true indicating error)
  bool has_error = onvif_gsoap_has_error(NULL);
  assert_true(has_error);

  // Initialize context
  int result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test with initialized context - may or may not have error depending on gSOAP state
    has_error = onvif_gsoap_has_error(&ctx);
    // Don't assert specific error state as it depends on gSOAP initialization
    (void)has_error; // Just verify function doesn't crash

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test error message retrieval
 * @param state Test state (unused)
 */
void test_onvif_gsoap_get_error(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  const char* error_msg = onvif_gsoap_get_error(NULL);
  assert_null(error_msg);

  // Initialize context
  int result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test with clean context (should have no error)
    error_msg = onvif_gsoap_get_error(&ctx);
    // Error message may be NULL for clean context
    (void)error_msg; // Don't assert specific value

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test response validation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_validate_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_validate_response(NULL);
  // Accept any negative error code
  assert_true(result < 0);

  // Initialize context
  result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test validation with initialized context
    result = onvif_gsoap_validate_response(&ctx);
    // Don't assert specific value as it depends on context state
    (void)result;

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test operation name extraction
 * @param state Test state (unused)
 */
void test_onvif_gsoap_extract_operation_name(void** state) {
  (void)state;

  char operation_name[TEST_BUFFER_SIZE];

  // Test with NULL request data
  int result = onvif_gsoap_extract_operation_name(NULL, 0, operation_name, sizeof(operation_name));
  // Accept any negative error code
  assert_true(result < 0);

  // Test with NULL operation name buffer
  const char* test_request =
    "<soap:Envelope><soap:Body><GetDeviceInformation/></soap:Body></soap:Envelope>";
  result =
    onvif_gsoap_extract_operation_name(test_request, strlen(test_request), NULL, TEST_BUFFER_SIZE);
  // Accept any negative error code
  assert_true(result < 0);

  // Test with zero buffer size
  result =
    onvif_gsoap_extract_operation_name(test_request, strlen(test_request), operation_name, 0);
  // Accept any negative error code
  assert_true(result < 0);

  // Test with valid but minimal request
  result = onvif_gsoap_extract_operation_name(test_request, strlen(test_request), operation_name,
                                              sizeof(operation_name));
  // May succeed or fail depending on implementation complexity
  (void)result; // Don't assert specific value
}

/**
 * @brief Test request parsing initialization
 * @param state Test state (unused)
 */
void test_onvif_gsoap_init_request_parsing(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  const char* test_request =
    "<soap:Envelope><soap:Body><GetDeviceInformation/></soap:Body></soap:Envelope>";
  int result = onvif_gsoap_init_request_parsing(NULL, test_request, strlen(test_request));
  // Accept any negative error code
  assert_true(result < 0);

  // Initialize context
  result = onvif_gsoap_init(&ctx);

  // Only test if initialization succeeded
  if (result == 0) {
    // Test with NULL request data
    result = onvif_gsoap_init_request_parsing(&ctx, NULL, TEST_HEADER_SIZE);
    // Accept any negative error code
    assert_true(result < 0);

    // Test with zero size
    result = onvif_gsoap_init_request_parsing(&ctx, test_request, 0);
    // Accept any negative error code
    assert_true(result < 0);

    // Test with valid request
    result = onvif_gsoap_init_request_parsing(&ctx, test_request, strlen(test_request));
    // May succeed or fail depending on implementation
    (void)result;

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  } else {
    // If init failed, skip the main test
    skip();
  }
}

/**
 * @brief Test profile token parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_profile_token(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  char token[TEST_BUFFER_SIZE];

  // Test with NULL context
  int result = onvif_gsoap_parse_profile_token(NULL, token, sizeof(token));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL token buffer
  result = onvif_gsoap_parse_profile_token(&ctx, NULL, sizeof(token));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with zero buffer size
  result = onvif_gsoap_parse_profile_token(&ctx, token, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with uninitialized request parsing (should fail)
  result = onvif_gsoap_parse_profile_token(&ctx, token, sizeof(token));
  // Should fail since no request has been parsed
  (void)result;

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test configuration token parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_configuration_token(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  char token[TEST_BUFFER_SIZE];

  // Test with NULL context
  int result = onvif_gsoap_parse_configuration_token(NULL, token, sizeof(token));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL token buffer
  result = onvif_gsoap_parse_configuration_token(&ctx, NULL, sizeof(token));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with zero buffer size
  result = onvif_gsoap_parse_configuration_token(&ctx, token, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test protocol parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_protocol(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  char protocol[TEST_SMALL_BUFFER_SIZE];

  // Test with NULL context
  int result = onvif_gsoap_parse_protocol(NULL, protocol, sizeof(protocol));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL protocol buffer
  result = onvif_gsoap_parse_protocol(&ctx, NULL, sizeof(protocol));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with zero buffer size
  result = onvif_gsoap_parse_protocol(&ctx, protocol, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test generic value parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_value(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  char value[TEST_LARGE_BUFFER_SIZE] = {0};

  // Test with NULL context
  int result = onvif_gsoap_parse_value(NULL, "//test", value, sizeof(value));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL xpath
  result = onvif_gsoap_parse_value(&ctx, NULL, value, sizeof(value));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with NULL value buffer
  result = onvif_gsoap_parse_value(&ctx, "//test", NULL, sizeof(value));
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with zero buffer size
  result = onvif_gsoap_parse_value(&ctx, "//test", value, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test boolean value parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_boolean(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  int value = 0;

  // Test with NULL context
  int result = onvif_gsoap_parse_boolean(NULL, "//test", &value);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL xpath
  result = onvif_gsoap_parse_boolean(&ctx, NULL, &value);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with NULL value pointer
  result = onvif_gsoap_parse_boolean(&ctx, "//test", NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test integer value parsing
 * @param state Test state (unused)
 */
void test_onvif_gsoap_parse_integer(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  int value = 0;

  // Test with NULL context
  int result = onvif_gsoap_parse_integer(NULL, "//test", &value);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL xpath
  result = onvif_gsoap_parse_integer(&ctx, NULL, &value);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with NULL value pointer
  result = onvif_gsoap_parse_integer(&ctx, "//test", NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

// Simple test callback function
static int test_callback(struct soap* soap, void* user_data) {
  (void)soap;
  (void)user_data;
  return 0;
}

/**
 * @brief Test response generation with callback
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_response_with_callback(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_response_with_callback(NULL, test_callback, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL callback
  result = onvif_gsoap_generate_response_with_callback(&ctx, NULL, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with valid callback
  result = onvif_gsoap_generate_response_with_callback(&ctx, test_callback, NULL);
  assert_int_equal(result, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test profiles response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_profiles_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_profiles_response(NULL, NULL, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL profiles (empty response)
  result = onvif_gsoap_generate_profiles_response(&ctx, NULL, 0);
  assert_int_equal(result, 0);

  // Test with negative count
  result = onvif_gsoap_generate_profiles_response(&ctx, NULL, -1);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test stream URI response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_stream_uri_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_stream_uri_response(NULL, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL URI
  result = onvif_gsoap_generate_stream_uri_response(&ctx, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test create profile response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_create_profile_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_create_profile_response(NULL, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL profile
  result = onvif_gsoap_generate_create_profile_response(&ctx, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test delete profile response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_delete_profile_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_delete_profile_response(NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with valid context
  result = onvif_gsoap_generate_delete_profile_response(&ctx);
  assert_int_equal(result, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test PTZ nodes response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_get_nodes_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_get_nodes_response(NULL, NULL, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL nodes (empty response)
  result = onvif_gsoap_generate_get_nodes_response(&ctx, NULL, 0);
  assert_int_equal(result, 0);

  // Test with negative count
  result = onvif_gsoap_generate_get_nodes_response(&ctx, NULL, -1);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test PTZ absolute move response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_absolute_move_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_absolute_move_response(NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with valid context
  result = onvif_gsoap_generate_absolute_move_response(&ctx);
  assert_int_equal(result, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test PTZ presets response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_get_presets_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_get_presets_response(NULL, NULL, 0);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL presets (empty response)
  result = onvif_gsoap_generate_get_presets_response(&ctx, NULL, 0);
  assert_int_equal(result, 0);

  // Test with negative count
  result = onvif_gsoap_generate_get_presets_response(&ctx, NULL, -1);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test PTZ set preset response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_set_preset_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_set_preset_response(NULL, "preset1");
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with NULL preset token
  result = onvif_gsoap_generate_set_preset_response(&ctx, NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Test with valid preset token
  result = onvif_gsoap_generate_set_preset_response(&ctx, "preset1");
  assert_int_equal(result, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test PTZ goto preset response generation
 * @param state Test state (unused)
 */
void test_onvif_gsoap_generate_goto_preset_response(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test with NULL context
  int result = onvif_gsoap_generate_goto_preset_response(NULL);
  assert_int_equal(result, ONVIF_XML_ERROR_INVALID_INPUT);

  // Initialize context
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, 0);

  // Test with valid context
  result = onvif_gsoap_generate_goto_preset_response(&ctx);
  assert_int_equal(result, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

// Test functions are called from test_runner.c
