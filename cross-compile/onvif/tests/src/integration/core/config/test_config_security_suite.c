/**
 * @file test_config_security_suite.c
 * @brief Configuration system security test suite wrapper
 *
 * Wraps the security tests to integrate with the common test launcher.
 * Provides the standard CMUnitTest array interface for dynamic test execution.
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-16
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * External Security Test Suite
 * ============================================================================ */

// Forward declaration - implementation in test_config_security.c
extern const struct CMUnitTest g_config_security_tests[];
extern size_t g_config_security_test_count;

/* ============================================================================
 * Suite Getter Function
 * ============================================================================ */

/**
 * @brief Get the configuration security test suite
 *
 * Returns the array of security tests for the unified test launcher.
 *
 * @param[out] count Pointer to store the number of tests
 * @return Pointer to array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_security_integration_tests(size_t* count) {
  if (count) {
    *count = g_config_security_test_count;
  }
  return g_config_security_tests;
}
