/**
 * @file test_gsoap_edge_suite.c
 * @brief Test suite registration for gSOAP edge case tests
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

/* Forward declarations for edge case tests */

/* Memory allocation edge cases */
extern void test_unit_gsoap_edge_init_zero_context(void** state);
extern void test_unit_gsoap_edge_large_xml_allocation(void** state);
extern void test_unit_gsoap_edge_repeated_alloc_dealloc(void** state);
extern void test_unit_gsoap_edge_zero_size_allocation(void** state);
extern void test_unit_gsoap_edge_multiple_reset_cycles(void** state);

/* Invalid XML edge cases */
extern void test_unit_gsoap_edge_empty_xml(void** state);
extern void test_unit_gsoap_edge_malformed_xml_syntax(void** state);
extern void test_unit_gsoap_edge_incomplete_soap_envelope(void** state);
extern void test_unit_gsoap_edge_missing_soap_body(void** state);
extern void test_unit_gsoap_edge_extremely_long_strings(void** state);
extern void test_unit_gsoap_edge_whitespace_only_xml(void** state);

/* Parameter validation edge cases */
extern void test_unit_gsoap_edge_null_context_all_functions(void** state);
extern void test_unit_gsoap_edge_null_output_pointers(void** state);
extern void test_unit_gsoap_edge_empty_operation_name(void** state);
extern void test_unit_gsoap_edge_invalid_request_size(void** state);

/* State transition edge cases */
extern void test_unit_gsoap_edge_double_init(void** state);
extern void test_unit_gsoap_edge_parse_before_init(void** state);
extern void test_unit_gsoap_edge_cleanup_without_init(void** state);
extern void test_unit_gsoap_edge_interleaved_operations(void** state);
extern void test_unit_gsoap_edge_error_recovery(void** state);
extern void test_unit_gsoap_edge_rapid_state_transitions(void** state);

/**
 * @brief Get gSOAP edge case test suite
 * @param count Output parameter for test count
 * @return Array of cmocka unit tests
 */
const struct CMUnitTest* get_gsoap_edge_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
      /* Memory allocation edge cases */
      cmocka_unit_test(test_unit_gsoap_edge_init_zero_context),
      cmocka_unit_test(test_unit_gsoap_edge_repeated_alloc_dealloc),
      cmocka_unit_test(test_unit_gsoap_edge_zero_size_allocation),
      cmocka_unit_test(test_unit_gsoap_edge_multiple_reset_cycles),

      /* Invalid XML edge cases - only safe wrapper validation tests */
      cmocka_unit_test(test_unit_gsoap_edge_empty_xml),

      /* Parameter validation edge cases */
      cmocka_unit_test(test_unit_gsoap_edge_null_context_all_functions),
      cmocka_unit_test(test_unit_gsoap_edge_null_output_pointers),
      cmocka_unit_test(test_unit_gsoap_edge_empty_operation_name),
      cmocka_unit_test(test_unit_gsoap_edge_invalid_request_size),

      /* State transition edge cases */
      cmocka_unit_test(test_unit_gsoap_edge_double_init),
      cmocka_unit_test(test_unit_gsoap_edge_parse_before_init),
      cmocka_unit_test(test_unit_gsoap_edge_cleanup_without_init),
      cmocka_unit_test(test_unit_gsoap_edge_interleaved_operations),
      cmocka_unit_test(test_unit_gsoap_edge_error_recovery),
      cmocka_unit_test(test_unit_gsoap_edge_rapid_state_transitions),
  };

  *count = 16;
  return tests;
}
