/**
 * @file test_suites.c
 * @brief Centralized test suite registry
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_suites.h"

/**
 * @brief Global test suite registry
 *
 * All test suites are registered here. New test suites should be added to this array.
 */
const test_suite_t g_test_suites[] = {
  // ============================================================================
  // Unit Test Suites
  // ============================================================================

  // Utility tests
  {.name = "memory-utils",
   .full_name = "Memory Management Utilities",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_memory_utils_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "logging-utils",
   .full_name = "Logging Utilities",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_logging_utils_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // Networking tests
  {.name = "http-auth",
   .full_name = "HTTP Authentication",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_http_auth_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "http-metrics",
   .full_name = "HTTP Metrics",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_http_metrics_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // Protocol tests
  {.name = "gsoap-protocol",
   .full_name = "gSOAP Protocol",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_gsoap_protocol_unit_tests,
   .setup = gsoap_protocol_suite_setup,
   .teardown = gsoap_protocol_suite_teardown},

  // Service tests
  {.name = "service-dispatcher",
   .full_name = "Service Dispatcher",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_service_dispatcher_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // PTZ service tests
  {.name = "ptz-service",
   .full_name = "PTZ Service",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_ptz_service_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "ptz-callbacks",
   .full_name = "PTZ Callbacks",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_ptz_callbacks_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "ptz-adapter",
   .full_name = "PTZ Adapter",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_ptz_adapter_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // Media service tests
  {.name = "media-utils",
   .full_name = "Media Utilities",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_media_utils_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "media-callbacks",
   .full_name = "Media Callbacks",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_media_callbacks_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // Imaging service tests
  {.name = "imaging-callbacks",
   .full_name = "Imaging Callbacks",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_imaging_callbacks_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // ============================================================================
  // Integration Test Suites
  // ============================================================================

  // PTZ integration tests
  {.name = "ptz-integration",
   .full_name = "PTZ Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_ptz_integration_tests,
   .setup = ptz_service_setup,
   .teardown = ptz_service_teardown},

  // Media integration tests
  {.name = "media-integration",
   .full_name = "Media Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_media_integration_tests,
   .setup = media_service_setup,
   .teardown = media_service_teardown},

  // Device integration tests
  {.name = "device-integration",
   .full_name = "Device Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_device_integration_tests,
   .setup = device_service_setup,
   .teardown = device_service_teardown},

  // Imaging integration tests
  {.name = "imaging-integration",
   .full_name = "Imaging Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_imaging_integration_tests,
   .setup = setup_imaging_integration,
   .teardown = teardown_imaging_integration},

  // SOAP error tests
  {.name = "soap-errors",
   .full_name = "SOAP Error Handling",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_soap_error_integration_tests,
   .setup = NULL,
   .teardown = NULL},
};

/**
 * @brief Number of registered test suites
 */
const size_t g_test_suite_count = sizeof(g_test_suites) / sizeof(g_test_suites[0]);

/* ============================================================================
 * PTZ Adapter Test Suite Getter
 * ============================================================================ */

// External test array from ptz_adapter_tests.c
extern const struct CMUnitTest ptz_adapter_unit_tests[];

const struct CMUnitTest* get_ptz_adapter_unit_tests(size_t* count) {
  // Count tests in the array (21 tests)
  *count = 21;
  return ptz_adapter_unit_tests;
}
