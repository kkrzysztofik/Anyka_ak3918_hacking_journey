/**
 * @file test_suites.h
 * @brief Test suite registry infrastructure for dynamic test execution
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_SUITES_H
#define TEST_SUITES_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/**
 * @brief Test category enumeration
 */
typedef enum {
  TEST_CATEGORY_UNIT,        /**< Unit tests */
  TEST_CATEGORY_INTEGRATION, /**< Integration tests */
} test_category_t;

/**
 * @brief Test suite descriptor structure
 *
 * Describes a test suite with its name, category, tests, and optional setup/teardown functions.
 */
typedef struct {
  const char* name;     /**< Suite name (e.g., "ptz", "media", "networking") */
  const char* full_name; /**< Full descriptive name for display */
  test_category_t category; /**< Test category (unit or integration) */
  const struct CMUnitTest* (*get_tests)(
    size_t* count); /**< Function to retrieve test array */
  int (*setup)(void** state);   /**< Optional suite setup function */
  int (*teardown)(void** state); /**< Optional suite teardown function */
} test_suite_t;

/**
 * @brief Global test suite registry
 *
 * Array of all registered test suites. Defined in test_suites.c
 */
extern const test_suite_t g_test_suites[];

/**
 * @brief Number of registered test suites
 *
 * Total count of suites in g_test_suites array. Defined in test_suites.c
 */
extern const size_t g_test_suite_count;

// ============================================================================
// Suite Export Function Declarations
// ============================================================================

// Unit test suite exports
const struct CMUnitTest* get_basic_unit_tests(size_t* count);
const struct CMUnitTest* get_memory_utils_unit_tests(size_t* count);
const struct CMUnitTest* get_logging_utils_unit_tests(size_t* count);
const struct CMUnitTest* get_http_auth_unit_tests(size_t* count);
const struct CMUnitTest* get_http_metrics_unit_tests(size_t* count);
const struct CMUnitTest* get_gsoap_protocol_unit_tests(size_t* count);
const struct CMUnitTest* get_service_dispatcher_unit_tests(size_t* count);
const struct CMUnitTest* get_device_service_unit_tests(size_t* count);
const struct CMUnitTest* get_ptz_service_unit_tests(size_t* count);
const struct CMUnitTest* get_ptz_callbacks_unit_tests(size_t* count);
const struct CMUnitTest* get_media_utils_unit_tests(size_t* count);
const struct CMUnitTest* get_media_callbacks_unit_tests(size_t* count);
const struct CMUnitTest* get_imaging_callbacks_unit_tests(size_t* count);

// Integration test suite exports
const struct CMUnitTest* get_ptz_integration_tests(size_t* count);
const struct CMUnitTest* get_media_integration_tests(size_t* count);
const struct CMUnitTest* get_device_integration_tests(size_t* count);
const struct CMUnitTest* get_imaging_integration_tests(size_t* count);
const struct CMUnitTest* get_soap_error_integration_tests(size_t* count);

// Setup/teardown function declarations for integration tests
int ptz_service_setup(void** state);
int ptz_service_teardown(void** state);
int ptz_service_reset(void** state);
int media_service_setup(void** state);
int media_service_teardown(void** state);
int device_service_setup(void** state);
int device_service_teardown(void** state);
int setup_imaging_integration(void** state);
int teardown_imaging_integration(void** state);

#endif // TEST_SUITES_H
