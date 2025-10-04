/**
 * @file test_gsoap_response_suite.c
 * @brief Test suite registration for gSOAP response generation tests
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>

#include "cmocka_wrapper.h"
#include "unit/protocol/test_onvif_gsoap_response_generation.h"

// ============================================================================
// Test Suite Getter Function
// ============================================================================

/**
 * @brief Get the gSOAP response generation test suite
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_gsoap_response_unit_tests(size_t* count) {
  // Use pre-defined test count constant
  // Note: sizeof() doesn't work with extern arrays, so we use a constant
  *count = RESPONSE_GENERATION_TEST_COUNT;
  return response_generation_tests;
}
