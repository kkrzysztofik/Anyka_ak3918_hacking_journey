/**
 * @file test_network_integration.c
 * @brief Integration tests for ONVIF Networking layer configuration
 * @author Claude Code (T087 - Unified Configuration System)
 * @date 2025-10-15
 *
 * PURPOSE:
 * Validates that the Networking layer (HTTP server and network settings) properly
 * integrates with the unified configuration system.
 *
 * CURRENT INTEGRATION SCOPE:
 * - device_ip (CONFIG_SECTION_NETWORK) - Device IP address for XAddr generation
 * - http_port (CONFIG_SECTION_ONVIF) - HTTP server port configuration
 * - rtsp_port (CONFIG_SECTION_NETWORK) - RTSP service port
 * - ws_discovery_port (CONFIG_SECTION_NETWORK) - WS-Discovery port
 * - auth_enabled (CONFIG_SECTION_ONVIF) - Authentication setting
 * - http_verbose (CONFIG_SECTION_LOGGING) - HTTP verbose logging setting
 * - worker_threads (CONFIG_SECTION_SERVER) - HTTP server worker thread count
 * - max_connections (CONFIG_SECTION_SERVER) - Maximum concurrent connections
 * - connection_timeout (CONFIG_SECTION_SERVER) - Connection timeout (seconds)
 */

#include <stdbool.h>
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
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Test mocks
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/http_server_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/network_mock.h"
#include "mocks/smart_response_mock.h"

// Configuration storage
#include "core/config/config_storage.h"

// Test state structure
typedef struct {
  struct application_config* app_config;
  config_manager_t* config;
  int config_initialized_by_this_test; // Flag to track if we initialized the config system
} network_test_state_t;

/**
 * @brief Setup function for Networking layer integration tests
 * @param state Test state pointer
 * @return 0 on success, -1 on failure
 */
int network_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Allocate test state structure
  network_test_state_t* test_state = calloc(1, sizeof(network_test_state_t));
  assert_non_null(test_state);

  // Heap-allocate application config structure (required for config_runtime_init)
  test_state->app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(test_state->app_config);

  // Initialize direct struct members (no allocation needed)
  memset(&test_state->app_config->network, 0, sizeof(struct network_settings));
  memset(&test_state->app_config->logging, 0, sizeof(struct logging_settings));
  memset(&test_state->app_config->server, 0, sizeof(struct server_settings));

  // Allocate config manager used by config_storage_load early so it is valid
  // when config_runtime_init/config_storage_load call into it.
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
    config_result = test_helper_get_test_resource_path("configs/network_test_config.ini", config_path, sizeof(config_path));
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
    config_result = test_helper_get_test_resource_path("configs/network_test_config.ini", config_path, sizeof(config_path));
    assert_int_equal(0, config_result);
    config_result = config_storage_load(config_path, test_state->config);
    assert_int_equal(ONVIF_SUCCESS, config_result);
  }

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for Networking layer integration tests
 * @param state Test state pointer
 * @return 0 on success
 */
int network_service_teardown(void** state) {
  network_test_state_t* test_state = (network_test_state_t*)*state;

  // Free config first, before leak checking
  if (test_state && test_state->config) {
    free(test_state->config);
    test_state->config = NULL;
  }

  memory_manager_cleanup();

  // Cleanup runtime configuration system
  // Note: Only cleanup if we were the ones who initialized it
  if (test_state && test_state->config_initialized_by_this_test) {
    config_runtime_cleanup();
  }

  // Free network settings BEFORE freeing app_config
  if (test_state && test_state->app_config) {
    // All struct members are direct, no free needed

    // Free heap-allocated app_config
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
 * @brief Test ONVIF service network configuration integration
 *
 * Validates that HTTP port and authentication settings are properly
 * configured and accessible through the config_runtime API.
 */
void test_integration_network_onvif_config(void** state) {
  network_test_state_t* test_state = (network_test_state_t*)*state;

  // Verify configuration system is initialized
  assert_non_null(test_state);
  assert_non_null(test_state->config);
  assert_non_null(test_state->app_config);

  // Retrieve HTTP port from runtime config
  int http_port = 0;
  int result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(8080, http_port); // Default HTTP port

  // Retrieve authentication enabled flag
  int auth_enabled = -1;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "auth_enabled", &auth_enabled);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(0, auth_enabled); // Default: auth disabled
}
/**
 * @brief Test network service ports configuration
 *
 * Validates that RTSP, snapshot, and WS-Discovery ports are properly
 * configured and accessible through the config_runtime API.
 */
void test_integration_network_service_ports(void** state) {
  (void)state; // Unused variable

  // Retrieve RTSP port
  int rtsp_port = 0;
  int result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "rtsp_port", &rtsp_port);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(554, rtsp_port); // Default RTSP port

  // Retrieve snapshot port
  int snapshot_port = 0;
  result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "snapshot_port", &snapshot_port);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(8080, snapshot_port); // Default snapshot port

  // Retrieve WS-Discovery port
  int ws_discovery_port = 0;
  result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "ws_discovery_port", &ws_discovery_port);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(3702, ws_discovery_port); // Default WS-Discovery port
}
/**
 * @brief Test HTTP server configuration
 *
 * Validates that HTTP server settings (worker threads, max connections, timeouts)
 * are properly configured and accessible through the config_runtime API.
 */
void test_integration_network_http_server_config(void** state) {
  (void)state; // Unused variable

  // Retrieve worker threads count
  int worker_threads = 0;
  int result = config_runtime_get_int(CONFIG_SECTION_SERVER, "worker_threads", &worker_threads);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(4, worker_threads); // Default worker threads

  // Retrieve maximum connections
  int max_connections = 0;
  result = config_runtime_get_int(CONFIG_SECTION_SERVER, "max_connections", &max_connections);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(100, max_connections); // Default max connections

  // Retrieve connection timeout
  int connection_timeout = 0;
  result = config_runtime_get_int(CONFIG_SECTION_SERVER, "connection_timeout", &connection_timeout);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(30, connection_timeout); // Default connection timeout
}
/**
 * @brief Test HTTP logging configuration
 *
 * Validates that HTTP verbose logging setting is properly configured
 * and can be retrieved through the config_runtime API.
 */
void test_integration_network_logging_config(void** state) {
  (void)state; // Unused variable

  // Retrieve HTTP verbose logging flag
  int http_verbose = -1;
  int result = config_runtime_get_int(CONFIG_SECTION_LOGGING, "http_verbose", &http_verbose);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(0, http_verbose); // Default: HTTP verbose logging disabled
}
/**
 * @brief Test network configuration runtime updates
 *
 * Validates that network configuration can be updated at runtime and that
 * changes are properly reflected when re-queried.
 */
void test_integration_network_runtime_updates(void** state) {
  (void)state; // Unused variable

  // Update HTTP port to non-standard value
  int result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 9000);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the update was applied
  int http_port = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(9000, http_port);

  // Update worker threads
  result = config_runtime_set_int(CONFIG_SECTION_SERVER, "worker_threads", 8);
  assert_int_equal(ONVIF_SUCCESS, result);

  int worker_threads = 0;
  result = config_runtime_get_int(CONFIG_SECTION_SERVER, "worker_threads", &worker_threads);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(8, worker_threads);

  // Update HTTP verbose logging flag
  result = config_runtime_set_int(CONFIG_SECTION_LOGGING, "http_verbose", 1);
  assert_int_equal(ONVIF_SUCCESS, result);

  int http_verbose = -1;
  result = config_runtime_get_int(CONFIG_SECTION_LOGGING, "http_verbose", &http_verbose);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1, http_verbose);
}

// Test suite definition
const struct CMUnitTest network_integration_tests[] = {
  // ONVIF service configuration test
  cmocka_unit_test_setup_teardown(test_integration_network_onvif_config, network_service_setup, network_service_teardown),
  // Network service ports test
  cmocka_unit_test_setup_teardown(test_integration_network_service_ports, network_service_setup, network_service_teardown),
  // HTTP server configuration test
  cmocka_unit_test_setup_teardown(test_integration_network_http_server_config, network_service_setup, network_service_teardown),
  // HTTP logging configuration test
  cmocka_unit_test_setup_teardown(test_integration_network_logging_config, network_service_setup, network_service_teardown),
  // Runtime updates test
  cmocka_unit_test_setup_teardown(test_integration_network_runtime_updates, network_service_setup, network_service_teardown),
};
