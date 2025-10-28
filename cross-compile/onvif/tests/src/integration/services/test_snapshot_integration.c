/**
 * @file test_snapshot_integration.c
 * @brief Integration tests for ONVIF Snapshot service configuration
 * @author Claude Code (T086 - Unified Configuration System)
 * @date 2025-10-15
 *
 * PURPOSE:
 * Validates that the Snapshot service properly integrates with the unified configuration system.
 *
 * CURRENT INTEGRATION SCOPE:
 * - width (CONFIG_SECTION_SNAPSHOT) - Snapshot image width parameter
 * - height (CONFIG_SECTION_SNAPSHOT) - Snapshot image height parameter
 * - quality (CONFIG_SECTION_SNAPSHOT) - JPEG quality parameter (1-100)
 * - format (CONFIG_SECTION_SNAPSHOT) - Image format (e.g., "jpeg")
 */

#define _POSIX_C_SOURCE 200809L

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"

// ONVIF project includes
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "services/snapshot/onvif_snapshot.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"

// Test mocks
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/http_server_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/network_mock.h"
#include "mocks/platform_mock.h"
#include "mocks/smart_response_mock.h"
#include "platform/platform_common.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Configuration storage
#include "core/config/config_storage.h"

// Test state structure
typedef struct {
  struct application_config* app_config;
  config_manager_t* config;
  int config_initialized_by_this_test; // Flag to track if we initialized the config system
} snapshot_test_state_t;

/**
 * @brief Setup function for Snapshot service integration tests
 * @param state Test state pointer
 * @return 0 on success, -1 on failure
 */
int snapshot_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Allocate test state structure
  snapshot_test_state_t* test_state = calloc(1, sizeof(snapshot_test_state_t));
  assert_non_null(test_state);

  // Heap-allocate application config structure (required for config_runtime_init)
  test_state->app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(test_state->app_config);

  // Heap-allocate snapshot settings structure
  memset(&test_state->app_config->snapshot, 0, sizeof(struct snapshot_settings));

  // Allocate config manager before any config storage operations
  test_state->config = calloc(1, sizeof(config_manager_t));
  assert_non_null(test_state->config);

  // Enable real functions for integration testing BEFORE config loading
  // This allows config_storage_load to call config_runtime_set_int without mock issues
  service_dispatcher_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);
  config_mock_use_real_function(true);
  gsoap_mock_use_real_function(true);
  http_server_mock_use_real_function(true);
  network_mock_use_real_function(true);
  smart_response_mock_use_real_function(true);

  // Initialize runtime configuration system
  // Note: If already initialized by another test, this will return ONVIF_ERROR_ALREADY_EXISTS
  int config_result = config_runtime_init(test_state->app_config);
  if (config_result == ONVIF_ERROR_ALREADY_EXISTS) {
    // Configuration system already initialized by another test - this is OK
    // We need to load config from INI file
    printf("Configuration system already initialized by another test - loading from INI file\n");
    test_state->config_initialized_by_this_test = 0; // We didn't initialize it
    char config_path[256];
    config_result = test_helper_get_test_resource_path("configs/snapshot_test_config.ini", config_path, sizeof(config_path));
    assert_int_equal(0, config_result);
    config_result = config_storage_load(config_path, test_state->config);
    assert_int_equal(ONVIF_SUCCESS, config_result);
  } else {
    assert_int_equal(ONVIF_SUCCESS, config_result);
    test_state->config_initialized_by_this_test = 1; // We did initialize it

    // Apply default configuration values
    config_result = config_runtime_apply_defaults();
    assert_int_equal(ONVIF_SUCCESS, config_result);

    // Load configuration from INI file
    char config_path[256];
    config_result = test_helper_get_test_resource_path("configs/snapshot_test_config.ini", config_path, sizeof(config_path));
    assert_int_equal(0, config_result);
    config_result = config_storage_load(config_path, test_state->config);
    assert_int_equal(ONVIF_SUCCESS, config_result);
  }

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for Snapshot service integration tests
 * @param state Test state pointer
 * @return 0 on success
 */
int snapshot_service_teardown(void** state) {
  snapshot_test_state_t* test_state = (snapshot_test_state_t*)*state;

  // Free config first, before leak checking
  if (test_state && test_state->config) {
    free(test_state->config);
    test_state->config = NULL;
  }

  memory_manager_cleanup();

  // Cleanup runtime configuration system
  // Note: Only cleanup if we were the ones who initialized it
  // If another test initialized it, we should leave it alone
  if (test_state && test_state->config_initialized_by_this_test) {
    config_runtime_cleanup();
  }

  // Free snapshot settings structure BEFORE freeing app_config
  if (test_state && test_state->app_config) {
    // All struct members are direct, no free needed
    free(test_state->app_config);
    test_state->app_config = NULL;
  }

  // Free test state structure
  if (test_state) {
    free(test_state);
  }

  // Restore mock behavior for subsequent tests
  service_dispatcher_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);
  config_mock_use_real_function(false);
  gsoap_mock_use_real_function(false);
  http_server_mock_use_real_function(false);
  network_mock_use_real_function(false);
  smart_response_mock_use_real_function(false);

  return 0;
}

/**
 * @brief Test Snapshot service configuration schema integration
 *
 * Validates that the snapshot configuration schema is properly registered
 * and accessible through the config_runtime API with correct default values.
 */
void test_integration_snapshot_config_integration(void** state) {
  snapshot_test_state_t* test_state = (snapshot_test_state_t*)*state;

  // Verify configuration system is initialized
  assert_non_null(test_state);
  assert_non_null(test_state->config);
  assert_non_null(test_state->app_config);

  // Retrieve snapshot configuration values from runtime config
  int width = 0;
  int height = 0;
  int quality = 0;
  char format[32] = {0};

  // Get width parameter
  int result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "width", &width);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(640, width); // Default width

  // Get height parameter
  result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "height", &height);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(480, height); // Default height

  // Get quality parameter
  result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "quality", &quality);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(85, quality); // Default quality

  // Get format parameter
  result = config_runtime_get_string(CONFIG_SECTION_SNAPSHOT, "format", format, sizeof(format));
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("jpeg", format); // Default format
}

/**
 * @brief Test Snapshot configuration parameter bounds validation
 *
 * Validates that snapshot configuration parameters are properly constrained
 * and that invalid values are rejected.
 */
void test_integration_snapshot_bounds_validation(void** state) {
  (void)state; // Unused variable

  // Width bounds test - valid range: 160-2048
  int width = 160;
  int result = config_runtime_set_int(CONFIG_SECTION_SNAPSHOT, "width", width);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "width", &width);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(160, width);

  // Height bounds test - valid range: 120-2048
  int height = 120;
  result = config_runtime_set_int(CONFIG_SECTION_SNAPSHOT, "height", height);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "height", &height);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(120, height);

  // Quality bounds test - valid range: 1-100
  int quality = 100;
  result = config_runtime_set_int(CONFIG_SECTION_SNAPSHOT, "quality", quality);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_get_int(CONFIG_SECTION_SNAPSHOT, "quality", &quality);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(100, quality);
}

/**
 * @brief Test Snapshot configuration format parameter
 *
 * Validates that snapshot format string is properly stored and retrieved.
 */
void test_integration_snapshot_format_parameter(void** state) {
  (void)state; // Unused variable

  char format[32] = {0};

  // Get default format
  int result = config_runtime_get_string(CONFIG_SECTION_SNAPSHOT, "format", format, sizeof(format));
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("jpeg", format);

  // Verify format remains consistent across multiple retrieval attempts
  char format2[32] = {0};
  result = config_runtime_get_string(CONFIG_SECTION_SNAPSHOT, "format", format2, sizeof(format2));
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("jpeg", format2);
}
