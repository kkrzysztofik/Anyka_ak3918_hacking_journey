/**
 * @file test_suites.c
 * @brief Centralized test suite registry
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_suites.h"

#include <stddef.h>

#include "unit/core/config/test_config_path_resolution.h"
#include "unit/core/config/test_config_runtime.h"
#include "unit/core/config/test_config_storage.h"
#include "unit/core/config/test_user_persistence.h"
#include "unit/networking/test_http_server_auth.h"
#include "unit/utils/test_hash_utils.h"
#include "utils/test_gsoap_utils.h"

// Forward declarations for T086 (Snapshot integration tests)
extern const struct CMUnitTest* get_snapshot_integration_tests(size_t* count);

// Forward declarations for T087 (Network layer integration tests)
extern const struct CMUnitTest* get_network_integration_tests(size_t* count);

// Forward declarations for config path resolution tests
extern const struct CMUnitTest* get_config_path_resolution_unit_tests(size_t* count);

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

  {.name = "hash-utils",
   .full_name = "Hash Utilities (SHA256, Password Hashing)",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_hash_utils_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "config-runtime",
   .full_name = "Configuration Runtime Manager",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_config_runtime_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "user-persistence",
   .full_name = "User Credentials Persistence",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_user_persistence_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "config-storage",
   .full_name = "Configuration Storage Layer",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_config_storage_unit_tests,
   .setup = NULL,
   .teardown = NULL},

  {.name = "config-path-resolution",
   .full_name = "Configuration Path Resolution",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_config_path_resolution_unit_tests,
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

  {.name = "http-server-auth",
   .full_name = "HTTP Server Authentication",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_http_server_auth_unit_tests,
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

  {.name = "device-service",
   .full_name = "Device Service Unit Tests",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_device_service_unit_tests,
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

  {.name = "imaging-service",
   .full_name = "Imaging Service Operations",
   .category = TEST_CATEGORY_UNIT,
   .get_tests = get_imaging_service_unit_tests,
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
   .setup = NULL,
   .teardown = NULL},

  // Imaging integration tests
  {.name = "imaging-integration",
   .full_name = "Imaging Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_imaging_integration_tests,
   .setup = setup_imaging_integration,
   .teardown = teardown_imaging_integration},

  // Snapshot service integration tests (T086)
  {.name = "snapshot-integration",
   .full_name = "Snapshot Service Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_snapshot_integration_tests,
   .setup = NULL,
   .teardown = NULL},

  // SOAP error tests
  {.name = "soap-errors",
   .full_name = "SOAP Error Handling",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_soap_error_integration_tests,
   .setup = NULL,
   .teardown = NULL},

  // HTTP authentication integration tests
  {.name = "http-auth-integration",
   .full_name = "HTTP Authentication Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_http_auth_integration_tests,
   .setup = NULL,
   .teardown = NULL},

  // Network layer integration tests (T087)
  {.name = "network-integration",
   .full_name = "Network Layer Integration",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_network_integration_tests,
   .setup = NULL,
   .teardown = NULL},

  // ============================================================================
  // Polish Phase Test Suites (T104-T106)
  // ============================================================================

  // Configuration performance tests (T104)
  {.name = "config-performance",
   .full_name = "Configuration System Performance Benchmarking",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_config_performance_integration_tests,
   .setup = NULL,
   .teardown = NULL},

  // Configuration security tests (T105)
  {.name = "config-security",
   .full_name = "Configuration System Security Hardening",
   .category = TEST_CATEGORY_INTEGRATION,
   .get_tests = get_config_security_integration_tests,
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
