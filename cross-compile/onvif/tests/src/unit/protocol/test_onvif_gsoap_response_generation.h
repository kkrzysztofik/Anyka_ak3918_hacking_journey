/**
 * @file test_onvif_gsoap_response_generation.h
 * @brief Header for gSOAP response generation unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_ONVIF_GSOAP_RESPONSE_GENERATION_H
#define TEST_ONVIF_GSOAP_RESPONSE_GENERATION_H

#include <stddef.h>

#include "cmocka_wrapper.h"

// ============================================================================
// Test Array Declaration
// ============================================================================

/**
 * @brief Array of response generation unit tests
 */
extern const struct CMUnitTest response_generation_tests[];

/**
 * @brief Number of tests in response_generation_tests array
 */
#define RESPONSE_GENERATION_TEST_COUNT 42

// ============================================================================
// Test Suite Getter Function
// ============================================================================

/**
 * @brief Get the gSOAP response generation test suite
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_gsoap_response_unit_tests(size_t* count);

#endif // TEST_ONVIF_GSOAP_RESPONSE_GENERATION_H
