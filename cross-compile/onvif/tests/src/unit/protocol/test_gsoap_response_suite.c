/**
 * @file test_gsoap_response_suite.c
 * @brief Test suite registration for gSOAP response generation tests
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>

#include "cmocka_wrapper.h"
#include "unit/protocol/test_onvif_gsoap_response_generation.h"

// External test array from test_onvif_gsoap_response_generation.c
extern const struct CMUnitTest response_generation_tests[];

// ============================================================================
// Test Suite Getter Function
// ============================================================================

/**
 * @brief Get the gSOAP response generation test suite
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_gsoap_response_unit_tests(size_t* count) {
  // Count tests in the array (14 tests total)
  *count = 14;
  return response_generation_tests;
}
