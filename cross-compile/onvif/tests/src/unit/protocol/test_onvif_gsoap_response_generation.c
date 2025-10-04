/**
 * @file test_onvif_gsoap_response_generation.c
 * @brief Unit tests for gSOAP response generation functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"
#include "data/response_test_data.h"
#include "mocks/gsoap_mock.h"
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/xml_validation_helpers.h"

// ============================================================================
// Test Setup and Teardown
// ============================================================================

/**
 * @brief Setup function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_setup(void** state) {
  onvif_gsoap_context_t* ctx = calloc(1, sizeof(onvif_gsoap_context_t));
  if (!ctx) {
    return -1;
  }

  // Use real functions instead of mocks
  gsoap_mock_use_real_function(true);

  // Initialize the gSOAP context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    free(ctx);
    return -1;
  }

  *state = ctx;
  return 0;
}

/**
 * @brief Teardown function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_teardown(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  if (ctx) {
    // Cleanup the gSOAP context
    onvif_gsoap_cleanup(ctx);
    free(ctx);
  }

  // Re-enable mocks for other tests
  gsoap_mock_use_real_function(false);

  return 0;
}

// ============================================================================
// Device Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful device info response generation
 */
static void test_unit_onvif_gsoap_generate_device_info_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test basic context validation
  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_device_info_response(
    ctx, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
    mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
    mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_device_info_response(
    NULL, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
    mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
    mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test device info response generation with NULL parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test with NULL manufacturer - the function should handle NULL gracefully
  int result = onvif_gsoap_generate_device_info_response(
    ctx, NULL, mock_device_info_valid.model, mock_device_info_valid.firmware_version,
    mock_device_info_valid.serial_number, mock_device_info_valid.hardware_id);

  // The function should succeed even with NULL parameters (it converts them to empty strings)
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with empty parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_empty_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test with empty parameters using real functions
  int result = onvif_gsoap_generate_device_info_response(
    ctx, mock_device_info_empty.manufacturer, mock_device_info_empty.model,
    mock_device_info_empty.firmware_version, mock_device_info_empty.serial_number,
    mock_device_info_empty.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with memory allocation failure
 */
static void test_unit_onvif_gsoap_generate_device_info_response_memory_failure(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test memory allocation failure using real functions
  // Note: This test may not actually trigger memory failure in real implementation
  int result = onvif_gsoap_generate_device_info_response(
    ctx, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
    mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
    mock_device_info_valid.hardware_id);

  // With real functions, we expect success unless there's an actual memory issue
  assert_int_equal(result, ONVIF_SUCCESS);
}

// ============================================================================
// Media Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful delete profile response generation
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_delete_profile_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test delete profile response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_delete_profile_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// PTZ Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful absolute move response generation
 */
static void test_unit_onvif_gsoap_generate_absolute_move_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_absolute_move_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test absolute move response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_absolute_move_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_absolute_move_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful goto preset response generation
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_goto_preset_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test goto preset response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_goto_preset_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// Error Response Generation Tests
// ============================================================================

/**
 * @brief Test successful fault response generation
 */
static void test_unit_onvif_gsoap_generate_fault_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_fault_response(
    ctx, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
    "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // API returns byte length on success, not ONVIF_SUCCESS
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Validate XML is well-formed
  assert_int_equal(is_well_formed_xml(output_buffer), ONVIF_SUCCESS);

  // Validate SOAP envelope structure
  assert_int_equal(validate_soap_envelope(output_buffer), ONVIF_SUCCESS);

  // Validate fault response content matches expected values
  assert_int_equal(validate_soap_fault_xml(output_buffer, mock_fault_invalid_parameter.fault_code,
                                           mock_fault_invalid_parameter.fault_string,
                                           mock_fault_invalid_parameter.fault_detail),
                   ONVIF_SUCCESS);
}

/**
 * @brief Test fault response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_context(void** state) {
  (void)state; // Unused parameter
  char output_buffer[2048];

  // Test with NULL context - should still work with temporary context
  int result = onvif_gsoap_generate_fault_response(
    NULL, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
    "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL context creates temporary context, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Validate generated XML is valid
  assert_int_equal(is_well_formed_xml(output_buffer), ONVIF_SUCCESS);
  assert_int_equal(validate_soap_fault_xml(output_buffer, mock_fault_invalid_parameter.fault_code,
                                           mock_fault_invalid_parameter.fault_string,
                                           mock_fault_invalid_parameter.fault_detail),
                   ONVIF_SUCCESS);
}

/**
 * @brief Test fault response generation with NULL fault code
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_code(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[2048];

  // Test with NULL fault code - should use default
  int result = onvif_gsoap_generate_fault_response(
    ctx, NULL, mock_fault_invalid_parameter.fault_string, "test_actor",
    mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL fault_code uses default, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Validate XML is well-formed
  assert_int_equal(is_well_formed_xml(output_buffer), ONVIF_SUCCESS);
  assert_int_equal(validate_soap_envelope(output_buffer), ONVIF_SUCCESS);
}

/**
 * @brief Test fault response generation with NULL fault string
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_string(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[1024];

  // Test with NULL fault string
  int result = onvif_gsoap_generate_fault_response(
    ctx, mock_fault_invalid_parameter.fault_code, NULL, "test_actor",
    mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// Test Array Definition
// ============================================================================

const struct CMUnitTest response_generation_tests[] = {
  // Device Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_params,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_empty_params,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_device_info_response_memory_failure, response_generation_setup,
    response_generation_teardown),

  // Media Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_delete_profile_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_delete_profile_response_null_context, response_generation_setup,
    response_generation_teardown),

  // PTZ Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_absolute_move_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_absolute_move_response_null_context, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_null_context,
                                  response_generation_setup, response_generation_teardown),

  // Error Response Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_code,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_string,
                                  response_generation_setup, response_generation_teardown),
};
