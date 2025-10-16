/**
 * @file test_config_performance_suite.c
 * @brief Configuration system performance test suite wrapper
 *
 * Wraps the performance tests to integrate with the common test launcher.
 * Provides the standard CMUnitTest array interface for dynamic test execution.
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-16
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * External Performance Test Suite
 * ============================================================================ */

// Forward declaration - implementation in test_config_performance.c
extern const struct CMUnitTest g_config_performance_tests[];
extern size_t g_config_performance_test_count;

/* ============================================================================
 * Suite Getter Function
 * ============================================================================ */

/**
 * @brief Get the configuration performance test suite
 *
 * Returns the array of performance tests for the unified test launcher.
 *
 * @param[out] count Pointer to store the number of tests
 * @return Pointer to array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_performance_integration_tests(size_t* count) {
    if (count) {
        *count = g_config_performance_test_count;
    }
    return g_config_performance_tests;
}
