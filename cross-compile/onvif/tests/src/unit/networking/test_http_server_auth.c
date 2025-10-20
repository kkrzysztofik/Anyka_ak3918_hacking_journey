/**
 * @file test_http_server_auth.c
 * @brief Unit tests for HTTP server authentication logic with auth_enabled switch
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "mocks/config_mock.h"
#include "mocks/http_server_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "networking/http/http_auth.h"
#include "networking/http/http_server.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Create mock handlers using the new macro system
TEST_HELPER_CREATE_MOCK_HANDLERS(http_server_auth)

// Declare test counters using the new macro system
TEST_HELPER_DECLARE_COUNTERS(http_server_auth, 0, 0, 0, 0)

/* ============================================================================
 * Forward Declarations
 * ============================================================================ */

static struct application_config* create_test_app_config(int auth_enabled);
static void free_test_app_config(struct application_config* app_config);

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

// Test-specific state structure
struct http_server_auth_test_state {
  struct application_config* runtime_config;
};

static int setup_http_server_auth_tests(void** state) {
  // Initialize memory manager for proper memory leak detection
  memory_manager_init();

  // Create test state for this specific test
  struct http_server_auth_test_state* test_state =
    test_malloc(sizeof(struct http_server_auth_test_state));
  if (!test_state) {
    return -1;
  }

  // Create application config for runtime init
  test_state->runtime_config = create_test_app_config(1);
  if (!test_state->runtime_config) {
    test_free(test_state);
    return -1;
  }

  // Enable real config functions for integration testing with real authentication
  config_mock_use_real_function(true);

  // Initialize runtime config system for authentication tests
  int result = config_runtime_init(test_state->runtime_config);
  if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_ALREADY_EXISTS) {
    free_test_app_config(test_state->runtime_config);
    test_free(test_state);
    return -1;
  }

  // Add test user for authentication validation
  result = config_runtime_add_user("admin", "admin");
  if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_ALREADY_EXISTS) {
    config_runtime_cleanup();
    free_test_app_config(test_state->runtime_config);
    test_free(test_state);
    return -1;
  }

  // Enable real HTTP server functions for integration testing
  http_server_mock_use_real_function(true);

  // Initialize service dispatcher mock (pure CMocka pattern)
  mock_service_dispatcher_init();

  http_server_auth_reset_mock_state();
  reset_http_server_auth_state();

  *state = test_state;
  return 0;
}

static int teardown_http_server_auth_tests(void** state) {
  struct http_server_auth_test_state* test_state =
    (struct http_server_auth_test_state*)*state;

  // Reset global HTTP app config to prevent test pollution
  extern const struct application_config* g_http_app_config;
  g_http_app_config = NULL;

  // Reset HTTP auth config to prevent memory leaks
  http_server_reset_auth_config();

  // Cleanup service dispatcher mock (pure CMocka pattern)
  mock_service_dispatcher_cleanup();

  // Cleanup runtime config system (while real functions are still enabled)
  if (test_state) {
    config_runtime_cleanup();
    if (test_state->runtime_config) {
      free_test_app_config(test_state->runtime_config);
    }
    test_free(test_state);
  }

  // Disable real functions to restore mock behavior for other tests
  http_server_mock_use_real_function(false);
  config_mock_use_real_function(false);

  return 0;
}

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

/**
 * @brief Create a test application configuration with auth_enabled set
 * @param auth_enabled Value for auth_enabled field
 * @return Pointer to allocated application_config structure
 */
static struct application_config* create_test_app_config(int auth_enabled) {
  struct application_config* app_config = malloc(sizeof(struct application_config));
  if (!app_config) {
    return NULL;
  }

  memset(app_config, 0, sizeof(struct application_config));

  // Set ONVIF configuration
  app_config->onvif.enabled = 1;
  app_config->onvif.http_port = 8080;
  app_config->onvif.auth_enabled = auth_enabled;
  // Note: username/password fields removed from onvif_settings struct
  // Authentication is now handled through separate auth system

  return app_config;
}

/**
 * @brief Free test application configuration
 * @param app_config Configuration to free
 */
static void free_test_app_config(struct application_config* app_config) {
  if (app_config) {
    free(app_config);
  }
}

/**
 * @brief Create a test HTTP request with authentication header
 * @param has_auth_header Whether to include Authorization header
 * @param valid_credentials Whether to use valid credentials
 * @return HTTP request structure
 */
static http_request_t create_test_http_request(bool has_auth_header, bool valid_credentials) {
  http_request_t request;
  memset(&request, 0, sizeof(request));

  strcpy(request.method, "GET");
  strcpy(request.path, "/onvif/device_service");

  if (has_auth_header) {
    request.header_count = 1;
    request.headers = malloc(sizeof(http_header_t));
    request.headers[0].name = malloc(14);  // "Authorization" + null terminator
    request.headers[0].value = malloc(33); // "Basic ..." + null terminator

    strcpy(request.headers[0].name, "Authorization");

    if (valid_credentials) {
      // Valid credentials: admin:admin -> YWRtaW46YWRtaW4=
      strcpy(request.headers[0].value, "Basic YWRtaW46YWRtaW4=");
    } else {
      // Invalid credentials: wrong:wrong -> d3Jvbmc6d3Jvbmc=
      strcpy(request.headers[0].value, "Basic d3Jvbmc6d3Jvbmc=");
    }
  }

  return request;
}

/**
 * @brief Free HTTP request memory
 * @param request Request to free
 */
static void free_test_http_request(http_request_t* request) {
  if (request && request->headers) {
    for (size_t i = 0; i < request->header_count; i++) {
      free(request->headers[i].name);
      free(request->headers[i].value);
    }
    free(request->headers);
  }
}

/* ============================================================================
 * Authentication Disabled Tests
 * ============================================================================ */

/**
 * @brief Test HTTP server allows requests when authentication is disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_requests(void** state) {
  (void)state;

  // Create configuration with authentication disabled
  struct application_config* app_config = create_test_app_config(0);
  assert_non_null(app_config);

  // Create HTTP request without authentication header
  http_request_t request = create_test_http_request(false, false);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation succeeds (allows request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/**
 * @brief Test HTTP server allows requests with invalid credentials when auth disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_invalid_credentials(void** state) {
  (void)state;

  // Create configuration with authentication disabled
  struct application_config* app_config = create_test_app_config(0);
  assert_non_null(app_config);

  // Create HTTP request with invalid authentication header
  http_request_t request = create_test_http_request(true, false);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation succeeds (allows request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/**
 * @brief Test HTTP server allows requests with valid credentials when auth disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_valid_credentials(void** state) {
  (void)state;

  // Create configuration with authentication disabled
  struct application_config* app_config = create_test_app_config(0);
  assert_non_null(app_config);

  // Create HTTP request with valid authentication header
  http_request_t request = create_test_http_request(true, true);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation succeeds (allows request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/* ============================================================================
 * Authentication Enabled Tests
 * ============================================================================ */

/**
 * @brief Test HTTP server rejects requests without auth header when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_rejects_no_header(void** state) {
  (void)state;

  // Create configuration with authentication enabled
  struct application_config* app_config = create_test_app_config(1);
  assert_non_null(app_config);

  // Create HTTP request without authentication header
  http_request_t request = create_test_http_request(false, false);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation fails (rejects request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_not_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/**
 * @brief Test HTTP server rejects requests with invalid credentials when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_rejects_invalid_credentials(void** state) {
  (void)state;

  // Create configuration with authentication enabled
  struct application_config* app_config = create_test_app_config(1);
  assert_non_null(app_config);

  // Create HTTP request with invalid authentication header
  http_request_t request = create_test_http_request(true, false);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation fails (rejects request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_not_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/**
 * @brief Test HTTP server accepts requests with valid credentials when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_accepts_valid_credentials(void** state) {
  (void)state;

  // Create configuration with authentication enabled
  struct application_config* app_config = create_test_app_config(1);
  assert_non_null(app_config);

  // Create HTTP request with valid authentication header
  http_request_t request = create_test_http_request(true, true);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation succeeds (accepts request)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/* ============================================================================
 * NULL Parameter Tests
 * ============================================================================ */

/**
 * @brief Test HTTP server authentication with NULL request parameter
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_request(void** state) {
  (void)state;

  // Create configuration with authentication enabled
  struct application_config* app_config = create_test_app_config(1);
  assert_non_null(app_config);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation fails with NULL request
  int result = http_validate_authentication(NULL, &security_ctx);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  // Cleanup
  free_test_app_config(app_config);
}

/**
 * @brief Test HTTP server authentication with NULL security context
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_security_context(void** state) {
  (void)state;

  // Create configuration with authentication enabled
  struct application_config* app_config = create_test_app_config(1);
  assert_non_null(app_config);

  // Create HTTP request
  http_request_t request = create_test_http_request(true, true);

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation fails with NULL security context
  int result = http_validate_authentication(&request, NULL);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config);
}

/* ============================================================================
 * Configuration Edge Cases
 * ============================================================================ */

/**
 * @brief Test HTTP server authentication with NULL app config
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_app_config(void** state) {
  (void)state;

  // Create HTTP request
  http_request_t request = create_test_http_request(true, true);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config as NULL
  extern const struct application_config* g_http_app_config;
  g_http_app_config = NULL;

  // Test that authentication validation succeeds when app config is NULL
  // (should default to allowing requests)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
}

/**
 * @brief Test HTTP server authentication with app config but NULL onvif section
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_onvif_config(void** state) {
  (void)state;

  // Create HTTP request
  http_request_t request = create_test_http_request(true, true);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Create a mock app config with NULL onvif section
  struct application_config* app_config = malloc(sizeof(struct application_config));
  memset(app_config, 0, sizeof(struct application_config));
  // Leave onvif section as all zeros (auth_enabled will be 0)

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;
  g_http_app_config = app_config;

  // Test that authentication validation succeeds when onvif config is NULL/zero
  // (should default to allowing requests)
  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free(app_config);
}

/* ============================================================================
 * Integration Tests
 * ============================================================================ */

/**
 * @brief Test HTTP server authentication switch behavior
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_switch_behavior(void** state) {
  (void)state;

  // Create HTTP request
  http_request_t request = create_test_http_request(false, false);

  // Create security context
  security_context_t security_ctx;
  memset(&security_ctx, 0, sizeof(security_ctx));
  strcpy(security_ctx.client_ip, "192.168.1.100");

  // Mock the global HTTP app config
  extern const struct application_config* g_http_app_config;

  // Test with authentication disabled
  struct application_config* app_config_disabled = create_test_app_config(0);
  g_http_app_config = app_config_disabled;

  int result = http_validate_authentication(&request, &security_ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with authentication enabled
  struct application_config* app_config_enabled = create_test_app_config(1);
  g_http_app_config = app_config_enabled;

  result = http_validate_authentication(&request, &security_ctx);
  assert_int_not_equal(result, ONVIF_SUCCESS);

  // Cleanup
  free_test_http_request(&request);
  free_test_app_config(app_config_disabled);
  free_test_app_config(app_config_enabled);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest http_server_auth_tests[] = {
  // Authentication Disabled Tests
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_disabled_allows_requests,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_disabled_allows_invalid_credentials,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_disabled_allows_valid_credentials,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),

  // Authentication Enabled Tests
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_enabled_rejects_no_header,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_enabled_rejects_invalid_credentials,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_enabled_accepts_valid_credentials,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),

  // NULL Parameter Tests
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_null_request,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_null_security_context,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),

  // Configuration Edge Cases
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_null_app_config,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_null_onvif_config,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),

  // Integration Tests
  cmocka_unit_test_setup_teardown(test_unit_http_server_auth_switch_behavior,
                                  setup_http_server_auth_tests, teardown_http_server_auth_tests),
};

/**
 * @brief Get HTTP server auth unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_server_auth_unit_tests(size_t* count) {
  *count = sizeof(http_server_auth_tests) / sizeof(http_server_auth_tests[0]);
  return http_server_auth_tests;
}
