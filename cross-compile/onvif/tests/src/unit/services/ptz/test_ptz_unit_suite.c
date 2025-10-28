/**
 * @file test_ptz_unit_suite.c
 * @brief Unified PTZ unit test suite (service + callbacks + adapter)
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides a unified test suite that combines all PTZ-related unit tests:
 * - PTZ Service tests (core functionality)
 * - PTZ Callbacks tests (registration and dispatch)
 * - PTZ Adapter tests (hardware abstraction layer)
 */

#include <string.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * External Test Array and Function Declarations
 * ============================================================================ */

// External test arrays
extern const struct CMUnitTest ptz_adapter_unit_tests[];
extern const struct CMUnitTest ptz_tests[];

// External getter functions
const struct CMUnitTest* get_ptz_service_unit_tests(size_t* count);
const struct CMUnitTest* get_ptz_callbacks_unit_tests(size_t* count);

/* ============================================================================
 * Unified PTZ Test Suite Getter
 * ============================================================================ */

/**
 * @brief Get unified PTZ unit tests (service + callbacks + adapter)
 * @param count Output parameter for total test count
 * @return Pointer to unified test array
 *
 * This function combines all PTZ unit tests into a single suite:
 * - PTZ Service tests (8 tests) - core PTZ operations
 * - PTZ Callbacks tests (14 tests) - service registration and dispatch
 * - PTZ Adapter tests (21 tests) - hardware abstraction layer
 *
 * Total: 43 tests
 *
 * @note The unified array is static and sized for growth (64 slots)
 */
const struct CMUnitTest* get_ptz_unit_tests(size_t* count) {
  static struct CMUnitTest unified_ptz_tests[64]; // Sized for growth
  size_t offset = 0;

  // Get PTZ service tests
  size_t service_count = 0;
  const struct CMUnitTest* service_tests = get_ptz_service_unit_tests(&service_count);
  memcpy(&unified_ptz_tests[offset], service_tests, service_count * sizeof(struct CMUnitTest));
  offset += service_count;

  // Get PTZ callbacks tests
  size_t callbacks_count = 0;
  const struct CMUnitTest* callbacks_tests = get_ptz_callbacks_unit_tests(&callbacks_count);
  memcpy(&unified_ptz_tests[offset], callbacks_tests, callbacks_count * sizeof(struct CMUnitTest));
  offset += callbacks_count;

  // Get PTZ adapter tests (21 tests from external array)
  memcpy(&unified_ptz_tests[offset], ptz_adapter_unit_tests, 21 * sizeof(struct CMUnitTest));
  offset += 21;

  *count = offset;
  return unified_ptz_tests;
}
