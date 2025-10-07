/**
 * @file test_onvif_gsoap_core.c
 * @brief Unit tests for ONVIF gSOAP core module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "data/soap_test_envelopes.h"
#include "generated/soapH.h"
#include "mocks/gsoap_mock.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "utils/error/error_handling.h"
#include "utils/test_gsoap_utils.h"

/* ============================================================================
 * Test Suite Setup/Teardown
 * ============================================================================ */

// Setup/teardown functions are now defined in utils/test_gsoap_utils.c

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

// setup_parsing_test is now defined in test_gsoap_utils.c and shared across all test files

/* ============================================================================
 * Core Context Tests
 * ============================================================================ */

/**
 * @brief Test gSOAP context initialization
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Test successful initialization
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify request_state is initialized to false
  assert_false(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, 0);

  // Verify error_context is clear
  assert_int_equal(ctx.error_context.last_error_code, ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test gSOAP context initialization with NULL pointer
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_null(void** state) {
  (void)state;

  // Test initialization with NULL context
  int result = onvif_gsoap_init(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test gSOAP context cleanup
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_cleanup(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize and then cleanup
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup should not crash
  onvif_gsoap_cleanup(&ctx);

  // Verify state is cleared
  assert_false(ctx.request_state.is_initialized);

  // Test cleanup with NULL pointer (should not crash)
  onvif_gsoap_cleanup(NULL);
}

/* ============================================================================
 * Error Handling Tests
 * ============================================================================ */

/**
 * @brief Test parsing with invalid XML
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_invalid_xml(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Setup parsing test with invalid XML
  int result = setup_parsing_test(&ctx, SOAP_INVALID_XML);
  // Should fail during parsing initialization or parsing
  if (result == ONVIF_SUCCESS) {
    result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  }

  // Expect failure
  assert_true(result != ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with invalid namespace
 * @param state Test state (unused)
 *
 * Permissive parser accepts any namespace - namespace validation
 * happens at service handler layer, not parser layer
 */
void test_unit_onvif_gsoap_parse_invalid_namespace(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Setup parsing test with invalid namespace
  int result = setup_parsing_test(&ctx, SOAP_INVALID_NAMESPACE);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse request - permissive parsing accepts any namespace
  // Namespace validation happens at service handler layer
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with missing required parameter
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_missing_required_param(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetStreamUri* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MISSING_REQUIRED_PARAM);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse request - should succeed but ProfileToken will be NULL
  result = onvif_gsoap_parse_get_stream_uri(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify ProfileToken is NULL (missing required parameter)
  assert_null(request->ProfileToken);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing without initialization
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_without_initialization(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context but NOT request parsing
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Try to parse without calling onvif_gsoap_init_request_parsing
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);

  // Should fail - request_state.is_initialized is false
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Context Management Tests (Task 83)
 * ============================================================================ */

/**
 * @brief Test gSOAP context reset with success
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_reset_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context first
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set some state in the context
  ctx.request_state.is_initialized = true;
  ctx.request_state.request_size = 1024;
  ctx.error_context.last_error_code = ONVIF_ERROR_PARSE_FAILED;

  // Reset context
  onvif_gsoap_reset(&ctx);

  // Verify request state is cleared
  assert_false(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, 0);

  // Verify error context is cleared
  assert_int_equal(ctx.error_context.last_error_code, 0);

  // Verify response state is cleared
  assert_int_equal(ctx.response_state.total_bytes_written, 0);
  assert_false(ctx.response_state.is_finalized);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test gSOAP context reset with NULL pointer
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_reset_null(void** state) {
  (void)state;

  // Reset with NULL pointer should not crash and should return safely
  // This test passes if no crash occurs
  onvif_gsoap_reset(NULL);

  // If we reach here, the function handled NULL correctly
  assert_true(1); // Explicit assertion to show test completed successfully
}

/**
 * @brief Test gSOAP context reset after parsing
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_reset_after_parsing(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Initialize request parsing
  const char* test_xml = "<test>data</test>";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify parsing state was set
  assert_true(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, strlen(test_xml));

  // Reset context
  onvif_gsoap_reset(&ctx);

  // Verify parsing state is cleared
  assert_false(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test gSOAP request parsing initialization with success
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_request_parsing_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Initialize request parsing with valid XML
  const char* test_xml = "<soap:Envelope>test data</soap:Envelope>";
  size_t xml_size = strlen(test_xml);

  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, xml_size);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify request state is properly set
  assert_true(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, xml_size);
  assert_true(ctx.request_state.parse_start_time > 0);

  // Verify soap context is configured for parsing
  assert_non_null(ctx.soap.is);
  assert_int_equal(ctx.soap.bufidx, 0);
  assert_int_equal(ctx.soap.buflen, xml_size);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test request parsing initialization with NULL context
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_request_parsing_null_context(void** state) {
  (void)state;

  const char* test_xml = "<test>data</test>";

  // Initialize with NULL context
  int result = onvif_gsoap_init_request_parsing(NULL, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request parsing initialization with NULL XML
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_request_parsing_null_xml(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Initialize with NULL XML
  result = onvif_gsoap_init_request_parsing(&ctx, NULL, 100);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test request parsing initialization with zero size
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_request_parsing_zero_size(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  const char* test_xml = "<test>data</test>";

  // Initialize with zero size
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Error Handling Tests (Task 84)
 * ============================================================================ */

/**
 * @brief Test setting error context
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_set_error_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set error
  const char* test_message = "Test error message";
  const char* test_location = "test_function";
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_PARSE_FAILED, test_location, test_message);

  // Verify error was set
  assert_int_equal(ctx.error_context.last_error_code, ONVIF_ERROR_PARSE_FAILED);
  assert_string_equal(ctx.error_context.error_message, test_message);
  assert_ptr_equal(ctx.error_context.error_location, test_location);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test setting error with NULL message
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_set_error_null_message(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set error with NULL message (should not crash)
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_INVALID, "test_location", NULL);

  // Verify error code was set
  assert_int_equal(ctx.error_context.last_error_code, ONVIF_ERROR_INVALID);

  // Verify message is empty
  assert_int_equal(ctx.error_context.error_message[0], '\0');

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test getting detailed error information
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_get_detailed_error_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set error
  const char* test_message = "Detailed error";
  const char* test_location = "error_function";
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_MEMORY, test_location, test_message);

  // Also set soap error
  ctx.error_context.soap_error_code = SOAP_EOF;

  // Get detailed error
  int error_code = 0;
  const char* location = NULL;
  int soap_error = 0;
  const char* message = onvif_gsoap_get_detailed_error(&ctx, &error_code, &location, &soap_error);

  // Verify all error information is retrieved
  assert_string_equal(message, test_message);
  assert_int_equal(error_code, ONVIF_ERROR_MEMORY);
  assert_ptr_equal(location, test_location);
  assert_int_equal(soap_error, SOAP_EOF);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test getting detailed error with NULL output parameters
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_get_detailed_error_null_outputs(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set error
  const char* test_message = "Test error";
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_INVALID, "test_loc", test_message);

  // Get detailed error with NULL outputs (should not crash)
  const char* message = onvif_gsoap_get_detailed_error(&ctx, NULL, NULL, NULL);

  // Verify message is still returned
  assert_string_equal(message, test_message);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test has_error when error exists
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_has_error_true(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Initially no error
  assert_false(onvif_gsoap_has_error(&ctx));

  // Set error
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_PARSE_FAILED, "test", "error");

  // Now has error
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test has_error when no error exists
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_has_error_false(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // No error set
  assert_false(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test get_error with message
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_get_error_with_message(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set error
  const char* test_message = "Error message";
  onvif_gsoap_set_error(&ctx, ONVIF_ERROR_INVALID, "test", test_message);

  // Get error
  const char* message = onvif_gsoap_get_error(&ctx);

  // Verify message is returned
  assert_non_null(message);
  assert_string_equal(message, test_message);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test get_error without message
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_get_error_no_message(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // No error set - get_error should return NULL
  const char* message = onvif_gsoap_get_error(&ctx);
  assert_null(message);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Parsing Workflow Tests (Task 85)
 * ============================================================================ */

/**
 * @brief Test validate_and_begin_parse with success
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_validate_and_begin_parse_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  void* output_ptr = &ctx; // Non-NULL output pointer

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Initialize request parsing
  const char* test_xml = "<test>data</test>";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate and begin parse
  result = onvif_gsoap_validate_and_begin_parse(&ctx, output_ptr, "TestOperation", "test_func");
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify operation name was set
  assert_string_equal(ctx.request_state.operation_name, "TestOperation");

  // Verify parse start time was updated
  assert_true(ctx.request_state.parse_start_time > 0);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test validate_and_begin_parse with NULL context
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_validate_and_begin_parse_null_context(void** state) {
  (void)state;

  void* output_ptr = (void*)0x1234; // Non-NULL pointer

  // Validate with NULL context
  int result = onvif_gsoap_validate_and_begin_parse(NULL, output_ptr, "Op", "func");
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test validate_and_begin_parse with NULL output pointer
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_validate_and_begin_parse_null_output(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate with NULL output pointer
  result = onvif_gsoap_validate_and_begin_parse(&ctx, NULL, "Op", "func");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test validate_and_begin_parse when parsing not initialized
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_validate_and_begin_parse_not_initialized(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  void* output_ptr = &ctx;

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context but NOT request parsing
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate and begin parse without initializing request parsing
  result = onvif_gsoap_validate_and_begin_parse(&ctx, output_ptr, "Op", "func");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test finalize_parse with success
 * @param state Test state (unused)
 *
 * Note: This test uses a complete parsing workflow to test finalize_parse.
 * The function internally calls soap finalization functions which require
 * a properly initialized and parsed SOAP envelope.
 */
void test_unit_onvif_gsoap_finalize_parse_success(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Setup parsing test with valid GetProfiles request
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_GET_PROFILES);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse the request - this internally calls finalize_parse
  struct _trt__GetProfiles* request = NULL;
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify parse end time was set (this is set by finalize_parse)
  assert_true(ctx.request_state.parse_end_time > 0);

  // Verify parsing was successful
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test finalize_parse with NULL context
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_finalize_parse_null_context(void** state) {
  (void)state;

  // Finalize with NULL context
  int result = onvif_gsoap_finalize_parse(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
