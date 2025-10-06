/**
 * @file test_suites.c
 * @brief Centralized test suite registry
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_suites.h"

#include "utils/test_gsoap_utils.h"

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
   .setup = gsoap_core_suite_setup,
   .teardown = gsoap_core_suite_teardown},

  {.name = "gsoap-response",
   .full_name = "gSOAP Response Generation",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_gsoap_response_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "gsoap-edge-cases",
   .full_name = "gSOAP Edge Cases",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_gsoap_edge_unit_tests,
   .setup = gsoap_core_suite_setup,
   .teardown = gsoap_core_suite_teardown},

  // Service tests
  {.name = "service-dispatcher",
   .full_name = "Service Dispatcher",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_service_dispatcher_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "service-handler",
   .full_name = "Service Handler",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_service_handler_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  // PTZ unit tests (unified: service + callbacks + adapter)
  {.name = "ptz-unit",
   .full_name = "PTZ Unit Tests",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_ptz_unit_tests,
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
 * Unified PTZ Test Suite
 * ============================================================================ */

// Unified PTZ test suite getter is defined in:
// tests/src/unit/services/ptz/test_ptz_unit_suite.c
