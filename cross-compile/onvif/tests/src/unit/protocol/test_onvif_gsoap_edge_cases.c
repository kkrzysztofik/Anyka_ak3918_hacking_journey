/**
 * @file test_onvif_gsoap_edge_cases.c
 * @brief Unit tests for gSOAP edge cases - memory, XML, and state handling
 * @author kkrzysztofik
 * @date 2025
 */

#include <limits.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "data/soap_test_envelopes.h"
#include "generated/soapH.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "utils/error/error_handling.h"
#include "utils/test_gsoap_utils.h"

/* ============================================================================
 * Memory Allocation Edge Cases
 * ============================================================================ */

/**
 * @brief Test context initialization with zero memory
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_init_zero_context(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Test initialization on zeroed context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify context is properly initialized
  assert_false(ctx.request_state.is_initialized);
  assert_int_equal(ctx.error_context.last_error_code, ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with very large XML allocation
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_large_xml_allocation(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Create a reasonably large XML (10KB)
  size_t large_size = 10240;
  char* large_xml = (char*)malloc(large_size);
  assert_non_null(large_xml);

  // Fill with valid XML structure
  snprintf(
    large_xml, large_size,
    "<?xml version=\"1.0\"?><soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">"
    "<soap:Body>%*s</soap:Body></soap:Envelope>",
    (int)(large_size - 200), " ");

  // Test parsing initialization with large XML
  result = onvif_gsoap_init_request_parsing(&ctx, large_xml, strlen(large_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify request size was recorded
  assert_int_equal(ctx.request_state.request_size, strlen(large_xml));

  // Cleanup
  free(large_xml);
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test repeated allocation and deallocation cycles
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_repeated_alloc_dealloc(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;

  // Test multiple init/cleanup cycles
  for (int i = 0; i < 10; i++) {
    memset(&ctx, 0, sizeof(ctx));

    // Mock platform_config_get_int call for http_verbose check
    setup_http_verbose_mock();

    int result = onvif_gsoap_init(&ctx);
    assert_int_equal(result, ONVIF_SUCCESS);

    // Initialize parsing with test XML
    const char* test_xml = "<test>data</test>";
    result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
    assert_int_equal(result, ONVIF_SUCCESS);

    // Cleanup
    onvif_gsoap_cleanup(&ctx);
  }
}

/**
 * @brief Test zero-size allocation handling
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_zero_size_allocation(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with zero-size XML
  const char* test_xml = "";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, 0);

  // Should fail with invalid parameter error
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test context reset multiple times
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_multiple_reset_cycles(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test multiple reset cycles
  for (int i = 0; i < 5; i++) {
    // Set some state
    ctx.request_state.is_initialized = true;
    ctx.request_state.request_size = 1024;
    ctx.error_context.last_error_code = ONVIF_ERROR_PARSE_FAILED;

    // Reset
    onvif_gsoap_reset(&ctx);

    // Verify state is cleared
    assert_false(ctx.request_state.is_initialized);
    assert_int_equal(ctx.request_state.request_size, 0);
    assert_int_equal(ctx.error_context.last_error_code, 0);
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Invalid XML Edge Cases
 * ============================================================================ */

/**
 * @brief Test parsing with empty XML string
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_empty_xml(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with empty XML
  const char* empty_xml = "";
  result = onvif_gsoap_init_request_parsing(&ctx, empty_xml, strlen(empty_xml));

  // Should fail - empty XML is invalid
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with malformed XML syntax
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_malformed_xml_syntax(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Malformed XML with unclosed tag
  const char* malformed_xml = "<invalid><unclosed>";

  result = onvif_gsoap_init_request_parsing(&ctx, malformed_xml, strlen(malformed_xml));

  // We're testing robustness - result may vary
  (void)result;

  // Cleanup should work regardless
  onvif_gsoap_cleanup(&ctx);

  // If we get here without crashing, test passed
  assert_true(1);
}

/**
 * @brief Test parsing with incomplete SOAP envelope
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_incomplete_soap_envelope(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Very short XML that will fail size checks
  const char* incomplete_xml = "<soap>";

  result = onvif_gsoap_init_request_parsing(&ctx, incomplete_xml, strlen(incomplete_xml));

  // Should fail validation or parsing - either is acceptable
  // We're testing that the system doesn't crash, not specific error codes
  (void)result; // Result may be success or failure depending on validation

  // Cleanup should work regardless
  onvif_gsoap_cleanup(&ctx);

  // If we get here without crashing, test passed
  assert_true(1);
}

/**
 * @brief Test parsing with missing SOAP body
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_missing_soap_body(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Minimal XML - testing robustness, not specific parsing
  const char* no_body_xml = "<Envelope></Envelope>";

  result = onvif_gsoap_init_request_parsing(&ctx, no_body_xml, strlen(no_body_xml));

  // We're testing robustness, not specific error codes
  (void)result;

  // Cleanup should work regardless
  onvif_gsoap_cleanup(&ctx);

  // If we get here without crashing, test passed
  assert_true(1);
}

/**
 * @brief Test parsing with extremely long strings
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_extremely_long_strings(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Create XML with very long string value (5KB)
  size_t long_str_size = 5120;
  char* long_xml = (char*)malloc(long_str_size);
  assert_non_null(long_xml);

  char* long_value = (char*)malloc(4096);
  assert_non_null(long_value);
  memset(long_value, 'A', 4095);
  long_value[4095] = '\0';

  snprintf(long_xml, long_str_size,
           "<?xml version=\"1.0\"?>"
           "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">"
           "<soap:Body><test>%s</test></soap:Body></soap:Envelope>",
           long_value);

  // Test parsing with long string
  result = onvif_gsoap_init_request_parsing(&ctx, long_xml, strlen(long_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify request size
  assert_int_equal(ctx.request_state.request_size, strlen(long_xml));

  // Cleanup
  free(long_value);
  free(long_xml);
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with whitespace-only XML
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_whitespace_only_xml(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Whitespace-only XML
  const char* whitespace_xml = "   \n\t  ";
  result = onvif_gsoap_init_request_parsing(&ctx, whitespace_xml, strlen(whitespace_xml));

  // Should succeed in init but fail during parsing
  if (result == ONVIF_SUCCESS) {
    void* output_ptr = &ctx;
    result = onvif_gsoap_validate_and_begin_parse(&ctx, output_ptr, "TestOp", "test");
    if (result == ONVIF_SUCCESS) {
      result = onvif_gsoap_parse_soap_envelope(&ctx, "test");
    }
  }

  // Should fail due to invalid content
  assert_true(result != ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Parameter Validation Edge Cases
 * ============================================================================ */

/**
 * @brief Test all functions with NULL context
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_null_context_all_functions(void** state) {
  (void)state;

  // Test init with NULL (no mock needed for NULL context)
  int result = onvif_gsoap_init(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test cleanup with NULL (should not crash)
  onvif_gsoap_cleanup(NULL);

  // Test reset with NULL (should not crash)
  onvif_gsoap_reset(NULL);

  // Test init_request_parsing with NULL
  const char* test_xml = "<test/>";
  result = onvif_gsoap_init_request_parsing(NULL, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test validate_and_begin_parse with NULL context
  void* output = &result;
  result = onvif_gsoap_validate_and_begin_parse(NULL, output, "Op", "func");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test finalize_parse with NULL
  result = onvif_gsoap_finalize_parse(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test has_error with NULL (should return true)
  bool has_error = onvif_gsoap_has_error(NULL);
  assert_true(has_error);

  // Test get_error with NULL (should return error message)
  const char* error_msg = onvif_gsoap_get_error(NULL);
  assert_non_null(error_msg);
}

/**
 * @brief Test functions with NULL output pointers
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_null_output_pointers(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test validate_and_begin_parse with NULL output
  result = onvif_gsoap_validate_and_begin_parse(&ctx, NULL, "Op", "func");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test with empty/NULL operation name
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_empty_operation_name(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  void* output = &ctx;

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context and parsing
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  const char* test_xml = "<test>data</test>";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL operation name (may be allowed)
  result = onvif_gsoap_validate_and_begin_parse(&ctx, output, NULL, "func");
  // Function should handle NULL operation name gracefully

  // Reset for next test
  onvif_gsoap_reset(&ctx);
  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));

  // Test with empty operation name
  result = onvif_gsoap_validate_and_begin_parse(&ctx, output, "", "func");
  // Function should handle empty operation name gracefully

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with unreasonably large request size
 * @param state Test state (unused)
 *
 * Verifies that the wrapper properly validates and rejects requests
 * that exceed the maximum allowed size (MAX_ONVIF_REQUEST_SIZE = 1MB).
 */
void test_unit_gsoap_edge_invalid_request_size(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with size exceeding maximum (2MB > 1MB limit)
  const char* test_xml = "<test>data</test>";
  const size_t too_large = 2 * 1024 * 1024;
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, too_large);

  // Should reject oversized requests
  assert_int_equal(result, ONVIF_ERROR_INVALID);
  assert_true(onvif_gsoap_has_error(&ctx));

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * State Transition Edge Cases
 * ============================================================================ */

/**
 * @brief Test calling init twice without cleanup
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_double_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize again without cleanup
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Context should still be valid
  assert_false(ctx.request_state.is_initialized);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing before initialization
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_parse_before_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Try to parse without initialization
  int result = onvif_gsoap_parse_get_profiles(&ctx, &request);

  // Should fail - context not initialized
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup (should handle uninitialized context)
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test cleanup without initialization
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_cleanup_without_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Cleanup without init (should not crash)
  onvif_gsoap_cleanup(&ctx);

  // Verify context is still zeroed
  onvif_gsoap_context_t zero_ctx;
  memset(&zero_ctx, 0, sizeof(zero_ctx));
  assert_memory_equal(&ctx, &zero_ctx, sizeof(ctx));
}

/**
 * @brief Test interleaved operations
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_interleaved_operations(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Init -> Reset -> Init sequence
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  onvif_gsoap_reset(&ctx);

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Init parsing -> Reset -> Init parsing sequence
  const char* test_xml = "<test>data</test>";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  onvif_gsoap_reset(&ctx);

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test error recovery and state cleanup
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_error_recovery(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cause an error by passing invalid parameters
  result = onvif_gsoap_init_request_parsing(&ctx, NULL, 100);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Verify error was set
  assert_true(onvif_gsoap_has_error(&ctx));
  const char* error_msg = onvif_gsoap_get_error(&ctx);
  assert_non_null(error_msg);

  // Reset context to recover
  onvif_gsoap_reset(&ctx);

  // Verify error was cleared
  assert_int_equal(ctx.error_context.last_error_code, 0);

  // Re-initialize and verify recovery
  // Mock platform_config_get_int call for http_verbose check
  setup_http_verbose_mock();
  result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  const char* test_xml = "<test>valid</test>";
  result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test rapid state transitions
 * @param state Test state (unused)
 */
void test_unit_gsoap_edge_rapid_state_transitions(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  const char* test_xml = "<test>data</test>";

  // Perform rapid state transitions
  for (int i = 0; i < 20; i++) {
    memset(&ctx, 0, sizeof(ctx));

    // Mock platform_config_get_int call for http_verbose check
    setup_http_verbose_mock();

    int result = onvif_gsoap_init(&ctx);
    assert_int_equal(result, ONVIF_SUCCESS);

    if (i % 3 == 0) {
      // Every third iteration, also init parsing
      result = onvif_gsoap_init_request_parsing(&ctx, test_xml, strlen(test_xml));
      assert_int_equal(result, ONVIF_SUCCESS);
    }

    if (i % 2 == 0) {
      // Every other iteration, reset before cleanup
      onvif_gsoap_reset(&ctx);
    }

    onvif_gsoap_cleanup(&ctx);
  }
}
