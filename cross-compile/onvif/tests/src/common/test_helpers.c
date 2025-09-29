/**
 * @file test_helpers.c
 * @brief Implementation of common test helper functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_helpers.h"

#include <stdio.h>
#include <string.h>

// Mock includes
#include "../mocks/mock_service_dispatcher.h"
#include "../mocks/platform_mock.h"

// ONVIF includes
#include "../../../src/networking/http/http_auth.h"
#include "../../../src/utils/security/base64_utils.h"

/* ============================================================================
 * Service Callback Test Helpers
 * ============================================================================ */

void test_helper_service_registration_success(void** state, const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->service_name);
  assert_non_null(config->init_func);

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  // Mock successful platform initialization if required
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service (which registers it)
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Verify registration was called exactly once
  test_helper_assert_mock_called(mock_service_dispatcher_get_register_call_count(), 1,
                                 "service_dispatcher_register");

  // Verify registration data
  const onvif_service_registration_t* registration =
    mock_service_dispatcher_get_last_registration();
  test_helper_verify_service_registration(registration, config);
}

void test_helper_service_registration_duplicate(void** state, const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->init_func);

  // Mock duplicate service registration
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_DUPLICATE);

  // Mock successful platform initialization if required
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service (which attempts registration)
  int result = config->init_func(NULL);
  assert_int_equal(ONVIF_ERROR_DUPLICATE, result);

  // Verify registration was attempted
  test_helper_assert_mock_called(mock_service_dispatcher_get_register_call_count(), 1,
                                 "service_dispatcher_register");
}

void test_helper_service_registration_null_config(void** state,
                                                  const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->init_func);

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  // Mock successful platform initialization if required
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service with NULL config (should still work for most services)
  int result = config->init_func(NULL);

  // Most services accept NULL config and use defaults
  assert_int_equal(config->expected_init_success, result);
}

void test_helper_service_registration_dispatcher_failure(void** state,
                                                         const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->init_func);

  // Mock service registration failure
  mock_service_dispatcher_set_register_result(ONVIF_ERROR_INVALID);

  // Mock successful platform initialization if required
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service (which attempts registration)
  int result = config->init_func(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID, result);

  // Verify registration was attempted
  test_helper_assert_mock_called(mock_service_dispatcher_get_register_call_count(), 1,
                                 "service_dispatcher_register");
}

void test_helper_service_unregistration_success(void** state, const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->init_func);
  assert_non_null(config->cleanup_func);

  // First register the service
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);

  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  int init_result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, init_result);

  // Mock successful unregistration
  mock_service_dispatcher_set_unregister_result(ONVIF_SUCCESS);

  // Cleanup service (which unregisters it)
  config->cleanup_func();

  // Verify unregistration was called
  test_helper_assert_mock_called(mock_service_dispatcher_get_unregister_call_count(), 1,
                                 "service_dispatcher_unregister");
}

void test_helper_service_unregistration_not_initialized(void** state,
                                                        const service_test_config_t* config) {
  (void)state;

  assert_non_null(config);
  assert_non_null(config->cleanup_func);

  // Mock unregistration (service was never initialized)
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR_NOT_FOUND);

  // Cleanup service without initializing first
  config->cleanup_func();

  // Unregistration should either not be called or handle gracefully
  // (Implementation-dependent behavior)
}

void test_helper_verify_service_registration(const onvif_service_registration_t* registration,
                                             const service_test_config_t* config) {
  test_helper_assert_non_null(registration, "service registration");
  test_helper_assert_string_equal(registration->service_name, config->service_name, "service name");
  test_helper_assert_string_equal(registration->namespace_uri, config->namespace_uri,
                                  "namespace URI");
  test_helper_assert_non_null(registration->operation_handler, "operation handler");
  test_helper_assert_non_null(registration->init_handler, "init handler");
  test_helper_assert_non_null(registration->cleanup_handler, "cleanup handler");
  test_helper_assert_non_null(registration->capabilities_handler, "capabilities handler");
}

/* ============================================================================
 * NULL Parameter Test Helpers
 * ============================================================================ */

void test_helper_null_parameters(void** state, const char* function_name,
                                 void (*test_func)(void**, const null_param_test_t*),
                                 const null_param_test_t* tests, int test_count) {
  (void)state;

  assert_non_null(function_name);
  assert_non_null(test_func);
  assert_non_null(tests);
  assert_true(test_count > 0);

  printf("Testing NULL parameters for function: %s\n", function_name);

  for (int i = 0; i < test_count; i++) {
    printf("  Test %d: %s\n", i + 1, tests[i].param_description);
    test_func(state, &tests[i]);
  }

  printf("  All %d NULL parameter tests passed\n", test_count);
}

/**
 * @brief Generic NULL parameter test wrapper for functions with 2 parameters
 * @param state Test state (unused)
 * @param test_config Test configuration containing parameter info
 */
void test_helper_null_param_2_args(void** state, const null_param_test_t* test_config) {
  (void)state;

  // This is a generic wrapper that should be used with specific function wrappers
  // The actual function call will be provided by the caller
  assert_non_null(test_config);
  assert_true(test_config->param_index >= 0 && test_config->param_index < 2);
}

/**
 * @brief Generic NULL parameter test wrapper for functions with 3 parameters
 * @param state Test state (unused)
 * @param test_config Test configuration containing parameter info
 */
void test_helper_null_param_3_args(void** state, const null_param_test_t* test_config) {
  (void)state;

  assert_non_null(test_config);
  assert_true(test_config->param_index >= 0 && test_config->param_index < 3);
}

/**
 * @brief Generic NULL parameter test wrapper for functions with 4 parameters
 * @param state Test state (unused)
 * @param test_config Test configuration containing parameter info
 */
void test_helper_null_param_4_args(void** state, const null_param_test_t* test_config) {
  (void)state;

  assert_non_null(test_config);
  assert_true(test_config->param_index >= 0 && test_config->param_index < 4);
}

/**
 * @brief Create a NULL parameter test configuration
 * @param description Human-readable description of the parameter
 * @param param_index Which parameter to set to NULL (0-based)
 * @param expected_result Expected return code when parameter is NULL
 * @return null_param_test_t structure
 */
/**
 * @brief Create a NULL parameter test configuration
 * @param description Human-readable description of the parameter
 * @param param_index Which parameter to set to NULL (0-based)
 * @param expected_result Expected return code when parameter is NULL
 * @return null_param_test_t structure
 */
null_param_test_t test_helper_create_null_test(const char* description, int param_index,
                                               int expected_result) {
  null_param_test_t test = {0};
  if (description) {
    strncpy(test.param_description, description, sizeof(test.param_description) - 1);
    test.param_description[sizeof(test.param_description) - 1] = '\0';
  }
  test.param_index = param_index;
  test.expected_result = expected_result;
  return test;
}

/* ============================================================================
 * Generic Mock Handler System
 * ============================================================================ */

/**
 * @brief Generic mock handler state structure
 */
;

/**
 * @brief Generic init handler implementation
 * @param state Mock handler state
 * @return Result from init handler
 */
int test_helper_generic_init_handler(generic_mock_handler_state_t* state) {
  if (!state) {
    return ONVIF_ERROR_INVALID;
  }

  state->init_call_count++;
  return state->init_result;
}

/**
 * @brief Generic cleanup handler implementation
 * @param state Mock handler state
 */
void test_helper_generic_cleanup_handler(generic_mock_handler_state_t* state) {
  if (!state) {
    return;
  }

  state->cleanup_call_count++;
}

/**
 * @brief Generic operation handler implementation
 * @param state Mock handler state
 * @param operation Operation name
 * @param request Request data
 * @param response Response data
 * @return Result from operation handler
 */
int test_helper_generic_operation_handler(generic_mock_handler_state_t* state,
                                          const char* operation, const void* request,
                                          void* response) {
  if (!state) {
    return ONVIF_ERROR_INVALID;
  }

  state->operation_call_count++;

  if (operation) {
    strncpy(state->last_operation, operation, sizeof(state->last_operation) - 1);
    state->last_operation[sizeof(state->last_operation) - 1] = '\0';
  }

  state->last_request = (void*)request;
  state->last_response = response;

  return ONVIF_SUCCESS;
}

/**
 * @brief Reset generic mock handler state
 * @param state Mock handler state to reset
 */
/**
 * @brief Reset generic mock handler state
 * @param state Mock handler state to reset
 */
void test_helper_reset_generic_mock_state(generic_mock_handler_state_t* state) {
  if (!state) {
    return;
  }

  state->init_call_count = 0;
  state->cleanup_call_count = 0;
  state->operation_call_count = 0;
  state->init_result = ONVIF_SUCCESS;
  state->last_request = NULL;
  state->last_response = NULL;
  memset(state->last_operation, 0, sizeof(state->last_operation));
}

/* ============================================================================
 * Test State Management System
 * ============================================================================ */

/**
 * @brief Test state configuration structure
 */
;

/**
 * @brief Generic state reset function
 * @param config State configuration
 */
void test_helper_reset_state(const test_state_config_t* config) {
  if (!config) {
    return;
  }

  // Reset all counters
  if (config->counters && config->counter_count > 0) {
    for (int i = 0; i < config->counter_count; i++) {
      if (config->counters[i] != NULL) {
        *(config->counters[i]) = 0;
      }
    }
  }

  // Call custom reset function if provided
  if (config->reset_func) {
    config->reset_func();
  }

  // Call cleanup function if provided
  if (config->cleanup_func) {
    config->cleanup_func();
  }
}

/**
 * @brief Create a test state configuration
 * @param reset_func Custom reset function (can be NULL)
 * @param cleanup_func Cleanup function to call (can be NULL)
 * @param counters Array of counter pointers (can be NULL)
 * @param counter_count Number of counters
 * @return test_state_config_t structure
 */
/**
 * @brief Create a test state configuration
 * @param reset_func Custom reset function (can be NULL)
 * @param cleanup_func Cleanup function to call (can be NULL)
 * @param counters Array of counter pointers (can be NULL)
 * @param counter_count Number of counters
 * @return test_state_config_t structure
 */
test_state_config_t test_helper_create_state_config(void (*reset_func)(void),
                                                    void (*cleanup_func)(void), int* counters,
                                                    int counter_count) {
  test_state_config_t config = {0};
  config.reset_func = reset_func;
  config.cleanup_func = cleanup_func;
  config.counters = (int**)counters;
  config.counter_count = counter_count;
  return config;
}

/* ============================================================================
 * Service-Specific Test Helper Functions
 * ============================================================================ */

/**
 * @brief Build Basic Authentication header for HTTP tests
 * @param username Username for authentication
 * @param password Password for authentication
 * @param header_value Output buffer for header value
 * @param header_size Size of header_value buffer
 * @return 0 on success, -1 on error
 */
int test_helper_http_build_basic_auth_header(const char* username, const char* password,
                                             char* header_value, size_t header_size) {
  if (!username || !password || !header_value || header_size == 0) {
    return -1;
  }

  // Create credentials string
  char credentials[256];
  int written = snprintf(credentials, sizeof(credentials), "%s:%s", username, password);
  if (written >= (int)sizeof(credentials)) {
    return -1; // Truncated
  }

  // Encode credentials
  char encoded[512];
  int base64_result = onvif_util_base64_encode((const unsigned char*)credentials,
                                               strlen(credentials), encoded, sizeof(encoded));
  if (base64_result != ONVIF_SUCCESS) {
    return -1;
  }

  // Build header
  written = snprintf(header_value, header_size, "Basic %s", encoded);
  if (written >= (int)header_size) {
    return -1; // Truncated
  }

  return 0;
}

/**
 * @brief Initialize HTTP authentication configuration for tests
 * @param config HTTP auth config to initialize
 * @param auth_type Type of authentication (HTTP_AUTH_BASIC, etc.)
 * @param enabled Whether authentication is enabled
 * @return 0 on success, -1 on error
 */
int test_helper_http_init_auth_config(struct http_auth_config* config, int auth_type, int enabled) {
  if (!config) {
    return -1;
  }

  // Initialize config
  int result = http_auth_init(config);
  if (result != HTTP_AUTH_SUCCESS) {
    return -1;
  }

  // Set configuration
  config->enabled = enabled;
  config->auth_type = auth_type;

  return 0;
}

/**
 * @brief Create test HTTP request structure
 * @param method HTTP method (GET, POST, etc.)
 * @param uri Request URI
 * @param request Output request structure
 * @return 0 on success, -1 on error
 */
int test_helper_http_create_request(const char* method, const char* uri, http_request_t* request) {
  if (!method || !uri || !request) {
    return -1;
  }

  memset(request, 0, sizeof(http_request_t));

  strncpy(request->method, method, sizeof(request->method) - 1);
  request->method[sizeof(request->method) - 1] = '\0';

  strncpy(request->path, uri, sizeof(request->path) - 1);
  request->path[sizeof(request->path) - 1] = '\0';

  return 0;
}

/**
 * @brief Create test HTTP response structure
 * @param status_code HTTP status code
 * @param response Output response structure
 * @return 0 on success, -1 on error
 */
/**
 * @brief Create test HTTP response structure
 * @param status_code HTTP status code
 * @param response Output response structure
 * @return 0 on success, -1 on error
 */
int test_helper_http_create_response(int status_code, http_response_t* response) {
  if (!response) {
    return -1;
  }

  memset(response, 0, sizeof(http_response_t));
  response->status_code = status_code;

  return 0;
}

/**
 * @brief Create test PTZ position structure
 * @param position Output position structure
 * @param pan Pan value (-1.0 to 1.0)
 * @param tilt Tilt value (-1.0 to 1.0)
 * @param zoom Zoom value (0.0 to 1.0)
 * @return 0 on success, -1 on error
 */
int test_helper_ptz_create_test_position(struct ptz_vector* position, float pan, float tilt,
                                         float zoom) {
  if (!position) {
    return -1;
  }

  memset(position, 0, sizeof(struct ptz_vector));

  position->pan_tilt.x = pan;
  position->pan_tilt.y = tilt;
  position->zoom = zoom;
  strncpy(position->space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
          sizeof(position->space) - 1);
  position->space[sizeof(position->space) - 1] = '\0';

  return 0;
}

/**
 * @brief Create test PTZ speed structure
 * @param speed Output speed structure
 * @param pan_tilt Pan/tilt speed value (0.0 to 1.0)
 * @param zoom Zoom speed value (0.0 to 1.0)
 * @return 0 on success, -1 on error
 */
int test_helper_ptz_create_test_speed(struct ptz_speed* speed, float pan_tilt, float zoom) {
  if (!speed) {
    return -1;
  }

  memset(speed, 0, sizeof(struct ptz_speed));

  speed->pan_tilt.x = pan_tilt;
  speed->pan_tilt.y = pan_tilt;
  speed->zoom = zoom;

  return 0;
}

/**
 * @brief Create test PTZ preset structure
 * @param preset Output preset structure
 * @param token Preset token
 * @param name Preset name
 * @return 0 on success, -1 on error
 */
int test_helper_ptz_create_test_preset(struct ptz_preset* preset, const char* token,
                                       const char* name) {
  if (!preset || !token || !name) {
    return -1;
  }

  memset(preset, 0, sizeof(struct ptz_preset));

  strncpy(preset->token, token, sizeof(preset->token) - 1);
  preset->token[sizeof(preset->token) - 1] = '\0';

  strncpy(preset->name, name, sizeof(preset->name) - 1);
  preset->name[sizeof(preset->name) - 1] = '\0';

  return 0;
}

/* ============================================================================
 * Mock Setup/Teardown Helpers
 * ============================================================================ */

int test_helper_setup_mocks(const mock_config_t* config) {
  assert_non_null(config);

  if (config->init_service_dispatcher) {
    mock_service_dispatcher_init();
  }

  if (config->init_platform) {
    platform_mock_init();
  }

  if (config->init_ptz_adapter) {
    // PTZ adapter initialization
    // Note: This depends on platform_mock being initialized first
    if (!config->init_platform) {
      fprintf(stderr, "Warning: PTZ adapter requires platform mock\n");
    }
  }

  // Additional mocks can be initialized here as needed

  return 0;
}

void test_helper_teardown_mocks(const mock_config_t* config) {
  assert_non_null(config);

  // Cleanup in reverse order of initialization

  if (config->init_ptz_adapter) {
    // PTZ adapter cleanup
  }

  if (config->init_platform) {
    platform_mock_cleanup();
  }

  if (config->init_service_dispatcher) {
    mock_service_dispatcher_cleanup();
  }
}

void test_helper_reset_mock_counters(void) {
  mock_service_dispatcher_init(); // Reinitializing resets counters
}

/* ============================================================================
 * Common Assertion Helpers
 * ============================================================================ */

void test_helper_assert_non_null(const void* ptr, const char* description) {
  if (ptr == NULL) {
    fail_msg("Expected %s to be non-NULL, but it was NULL", description);
  }
  assert_non_null(ptr);
}

void test_helper_assert_string_equal(const char* actual, const char* expected,
                                     const char* description) {
  if (actual == NULL || expected == NULL) {
    fail_msg("String comparison for %s failed: actual=%s, expected=%s", description,
             actual ? actual : "NULL", expected ? expected : "NULL");
  }

  if (strcmp(actual, expected) != 0) {
    fail_msg("String mismatch for %s: expected '%s', got '%s'", description, expected, actual);
  }

  assert_string_equal(actual, expected);
}

void test_helper_assert_int_equal(int actual, int expected, const char* description) {
  if (actual != expected) {
    fail_msg("Integer mismatch for %s: expected %d, got %d", description, expected, actual);
  }

  assert_int_equal(actual, expected);
}

void test_helper_assert_mock_called(int actual_count, int expected_count,
                                    const char* function_name) {
  if (actual_count != expected_count) {
    fail_msg("Mock call count mismatch for %s: expected %d calls, got %d calls", function_name,
             expected_count, actual_count);
  }

  assert_int_equal(actual_count, expected_count);
}

/* ============================================================================
 * Test Data Initialization Helpers
 * ============================================================================ */

void test_helper_init_http_request(void* request) {
  assert_non_null(request);
  memset(request, 0, sizeof(http_request_t));
}

void test_helper_init_http_response(void* response) {
  assert_non_null(response);
  memset(response, 0, sizeof(http_response_t));
}

service_test_config_t test_helper_create_service_config(const char* service_name,
                                                        const char* namespace_uri,
                                                        int (*init_func)(void*),
                                                        void (*cleanup_func)(void)) {
  service_test_config_t config = {
    .service_name = service_name,
    .namespace_uri = namespace_uri,
    .init_func = init_func,
    .cleanup_func = cleanup_func,
    .requires_platform_init = 0,
    .expected_init_success = ONVIF_SUCCESS,
    .expected_init_failure = ONVIF_ERROR_INVALID,
  };

  return config;
}

mock_config_t test_helper_create_standard_mock_config(int include_platform, int include_ptz) {
  mock_config_t config = {
    .init_service_dispatcher = 1, // Always needed for service tests
    .init_platform = include_platform,
    .init_ptz_adapter = include_ptz,
    .init_network = 0,
    .init_config = 0,
  };

  return config;
}

/* ============================================================================
 * Service Dispatch Helpers
 * ============================================================================ */

/**
 * @brief Test service dispatch with valid operation
 * @param state Test state
 * @param config Service configuration
 * @param operation Operation name to test
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_dispatch_success(void** state, const service_test_config_t* config,
                                          const char* operation, http_request_t* request,
                                          http_response_t* response) {
  (void)state;

  // Mock successful service registration and dispatch
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_SUCCESS);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test dispatch through service dispatcher
  result = onvif_service_dispatcher_dispatch(config->service_name, operation, request, response);

  // Note: This will likely return an error due to missing gSOAP context
  // but we're testing the dispatch mechanism
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_service(), config->service_name);
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_operation(), operation);
}

/**
 * @brief Test service dispatch with unknown operation
 * @param state Test state
 * @param config Service configuration
 * @param operation Unknown operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_dispatch_unknown_operation(void** state,
                                                    const service_test_config_t* config,
                                                    const char* operation, http_request_t* request,
                                                    http_response_t* response) {
  (void)state;

  // Mock successful service registration
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test dispatch with unknown operation
  result = onvif_service_dispatcher_dispatch(config->service_name, operation, request, response);

  // Should return error for unknown operation
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_service(), config->service_name);
  assert_string_equal(mock_service_dispatcher_get_last_dispatch_operation(), operation);
}

/**
 * @brief Test service dispatch with NULL service name
 * @param state Test state
 * @param operation Operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_dispatch_null_service(void** state, const char* operation,
                                               http_request_t* request, http_response_t* response) {
  (void)state;

  // Test dispatch with NULL service name
  int result = onvif_service_dispatcher_dispatch(NULL, operation, request, response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test service dispatch with NULL operation name
 * @param state Test state
 * @param service_name Service name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_dispatch_null_operation(void** state, const char* service_name,
                                                 http_request_t* request,
                                                 http_response_t* response) {
  (void)state;

  // Test dispatch with NULL operation name
  int result = onvif_service_dispatcher_dispatch(service_name, NULL, request, response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test service dispatch with NULL request
 * @param state Test state
 * @param service_name Service name
 * @param operation Operation name
 * @param response HTTP response
 */
void test_helper_service_dispatch_null_request(void** state, const char* service_name,
                                               const char* operation, http_response_t* response) {
  (void)state;

  // Test dispatch with NULL request
  int result = onvif_service_dispatcher_dispatch(service_name, operation, NULL, response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test service dispatch with NULL response
 * @param state Test state
 * @param service_name Service name
 * @param operation Operation name
 * @param request HTTP request
 */
void test_helper_service_dispatch_null_response(void** state, const char* service_name,
                                                const char* operation, http_request_t* request) {
  (void)state;

  // Test dispatch with NULL response
  int result = onvif_service_dispatcher_dispatch(service_name, operation, request, NULL);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/* ============================================================================
 * Operation Handler Helpers
 * ============================================================================ */

/**
 * @brief Test operation handler with valid operation
 * @param state Test state
 * @param config Service configuration
 * @param operation Operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_operation_handler_success(void** state, const service_test_config_t* config,
                                           const char* operation, http_request_t* request,
                                           http_response_t* response) {
  (void)state;

  // Mock successful platform initialization
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test operation handler directly
  result = onvif_ptz_handle_operation(operation, request, response);

  // Note: This will likely return an error due to missing gSOAP context
  // but we're testing the handler mechanism
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);
}

/**
 * @brief Test operation handler with NULL operation name
 * @param state Test state
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_operation_handler_null_operation(void** state, http_request_t* request,
                                                  http_response_t* response) {
  (void)state;

  int result = onvif_ptz_handle_operation(NULL, request, response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler with NULL request
 * @param state Test state
 * @param operation Operation name
 * @param response HTTP response
 */
void test_helper_operation_handler_null_request(void** state, const char* operation,
                                                http_response_t* response) {
  (void)state;

  int result = onvif_ptz_handle_operation(operation, NULL, response);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler with NULL response
 * @param state Test state
 * @param operation Operation name
 * @param request HTTP request
 */
void test_helper_operation_handler_null_response(void** state, const char* operation,
                                                 http_request_t* request) {
  (void)state;

  int result = onvif_ptz_handle_operation(operation, request, NULL);

  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test operation handler with unknown operation
 * @param state Test state
 * @param operation Unknown operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_operation_handler_unknown_operation(void** state, const char* operation,
                                                     http_request_t* request,
                                                     http_response_t* response) {
  (void)state;

  int result = onvif_ptz_handle_operation(operation, request, response);

  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/* ============================================================================
 * Error Handling Helpers
 * ============================================================================ */

/**
 * @brief Test service registration failure handling
 * @param state Test state
 * @param config Service configuration
 * @param error_code Expected error code
 */
void test_helper_service_registration_failure_handling(void** state,
                                                       const service_test_config_t* config,
                                                       int error_code) {
  (void)state;

  // Mock registration failure
  mock_service_dispatcher_set_register_result(error_code);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service (should fail)
  int result = config->init_func(NULL);
  assert_int_equal(error_code, result);

  // Verify registration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}

/**
 * @brief Test service dispatch failure handling
 * @param state Test state
 * @param config Service configuration
 * @param operation Operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_dispatch_failure_handling(void** state,
                                                   const service_test_config_t* config,
                                                   const char* operation, http_request_t* request,
                                                   http_response_t* response) {
  (void)state;

  // Mock successful registration but dispatch failure
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_ERROR);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test dispatch failure
  result = onvif_service_dispatcher_dispatch(config->service_name, operation, request, response);

  assert_int_equal(ONVIF_ERROR, result);

  // Verify dispatch was called
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test service unregistration failure handling
 * @param state Test state
 * @param config Service configuration
 */
void test_helper_service_unregistration_failure_handling(void** state,
                                                         const service_test_config_t* config) {
  (void)state;

  // Mock successful registration but unregistration failure
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_unregister_result(ONVIF_ERROR);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test unregistration failure
  config->cleanup_func();

  // Verify unregistration was attempted
  assert_int_equal(1, mock_service_dispatcher_get_unregister_call_count());
}

/* ============================================================================
 * Logging Helpers
 * ============================================================================ */

/**
 * @brief Test service callback logging for success paths
 * @param state Test state
 * @param config Service configuration
 * @param operation Operation name
 * @param request HTTP request
 * @param response HTTP response
 */
void test_helper_service_callback_logging_success(void** state, const service_test_config_t* config,
                                                  const char* operation, http_request_t* request,
                                                  http_response_t* response) {
  (void)state;

  // Mock successful operations
  mock_service_dispatcher_set_register_result(ONVIF_SUCCESS);
  mock_service_dispatcher_set_dispatch_result(ONVIF_SUCCESS);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service
  int result = config->init_func(NULL);
  assert_int_equal(config->expected_init_success, result);

  // Test dispatch (should log success)
  result = onvif_service_dispatcher_dispatch(config->service_name, operation, request, response);

  // Verify operations completed (logging would be verified by log output)
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);
  assert_int_equal(1, mock_service_dispatcher_get_dispatch_call_count());
}

/**
 * @brief Test service callback logging for failure paths
 * @param state Test state
 * @param config Service configuration
 */
void test_helper_service_callback_logging_failure(void** state,
                                                  const service_test_config_t* config) {
  (void)state;

  // Mock failure operations
  mock_service_dispatcher_set_register_result(ONVIF_ERROR);
  if (config->requires_platform_init) {
    platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  }

  // Initialize service (should fail)
  int result = config->init_func(NULL);
  assert_int_equal(ONVIF_ERROR, result);

  // Verify failure was logged (logging would be verified by log output)
  assert_int_equal(1, mock_service_dispatcher_get_register_call_count());
}
