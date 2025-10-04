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
 */
void test_unit_onvif_gsoap_parse_invalid_namespace(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Setup parsing test with invalid namespace
  int result = setup_parsing_test(&ctx, SOAP_INVALID_NAMESPACE);
  // Should fail during parsing
  if (result == ONVIF_SUCCESS) {
    result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  }

  // Expect failure
  assert_true(result != ONVIF_SUCCESS);

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
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
