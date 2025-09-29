/**
 * @file test_http_auth.c
 * @brief Refactored HTTP authentication tests demonstrating all common patterns
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "networking/http/http_auth.h"

// Create mock handlers using the new macro system
TEST_HELPER_CREATE_MOCK_HANDLERS(http_auth)

// Declare test counters using the new macro system
TEST_HELPER_DECLARE_COUNTERS(http_auth, 0, 0, 0, 0)

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

static int setup_http_auth_tests(void** state) {
  (void)state;

  // Create standard mock configuration
  mock_config_t mock_config = test_helper_create_standard_mock_config(0, 0);
  test_helper_setup_mocks(&mock_config);

  http_auth_reset_mock_state();
  reset_http_auth_state();
  return 0;
}

static int teardown_http_auth_tests(void** state) {
  (void)state;

  // Create standard mock configuration for cleanup
  mock_config_t mock_config = test_helper_create_standard_mock_config(0, 0);
  test_helper_teardown_mocks(&mock_config);
  return 0;
}

/* ============================================================================
 * NULL Parameter Test Wrappers
 * ============================================================================ */

/**
 * @brief Wrapper for http_auth_validate_basic NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_http_auth_validate_basic_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct http_auth_config config;
  http_request_t request;
  int result;

  // Initialize valid config and request
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);
  test_helper_http_create_request("GET", "/test", &request);

  switch (test_config->param_index) {
  case 0: // NULL request parameter
    result = http_auth_validate_basic(NULL, &config, "admin", "password");
    break;
  case 1: // NULL config parameter
    result = http_auth_validate_basic(&request, NULL, "admin", "password");
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for http_auth_init NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_http_auth_init_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  int result;

  switch (test_config->param_index) {
  case 0: // NULL config parameter
    result = http_auth_init(NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/* ============================================================================
 * Refactored NULL Parameter Tests
 * ============================================================================ */

/**
 * @brief Test HTTP auth validate_basic function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("request parameter", 0, HTTP_AUTH_ERROR_NULL),
    test_helper_create_null_test("config parameter", 1, HTTP_AUTH_ERROR_NULL),
  };

  test_helper_null_parameters(state, "http_auth_validate_basic",
                              test_http_auth_validate_basic_with_null, tests, 2);
}

/**
 * @brief Test HTTP auth init function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_http_auth_init_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("config parameter", 0, HTTP_AUTH_ERROR_INVALID),
  };

  test_helper_null_parameters(state, "http_auth_init", test_http_auth_init_with_null, tests, 1);
}

/* ============================================================================
 * Success Case Tests Using Helper Functions
 * ============================================================================ */

/**
 * @brief Test HTTP auth initialization with valid parameters
 * @param state Test state (unused)
 */
void test_unit_http_auth_init_success(void** state) {
  (void)state;

  struct http_auth_config config;
  memset(&config, 0, sizeof(config));

  int result = http_auth_init(&config);

  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_int_equal(config.enabled, 0);                // Default disabled
  assert_int_equal(config.auth_type, HTTP_AUTH_NONE); // Default type
}

/**
 * @brief Test HTTP auth configuration setup
 * @param state Test state (unused)
 */
void test_unit_http_auth_config_setup(void** state) {
  (void)state;

  struct http_auth_config config;

  // Test helper function for config initialization
  int result = test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);
  assert_int_equal(result, 0);

  assert_int_equal(config.enabled, 1);
  assert_int_equal(config.auth_type, HTTP_AUTH_BASIC);
}

/**
 * @brief Test HTTP Basic Authentication header building
 * @param state Test state (unused)
 */
void test_unit_http_auth_basic_header_building(void** state) {
  (void)state;

  char header_value[256];

  // Test helper function for Basic Auth header
  int result = test_helper_http_build_basic_auth_header("admin", "password", header_value,
                                                        sizeof(header_value));
  assert_int_equal(result, 0);

  // Verify header format
  assert_true(strstr(header_value, "Basic ") != NULL);
  assert_true(strlen(header_value) > 6); // "Basic " + encoded data
}

/**
 * @brief Test HTTP request creation helper
 * @param state Test state (unused)
 */
void test_unit_http_request_creation(void** state) {
  (void)state;

  http_request_t request;

  // Test helper function for request creation
  int result = test_helper_http_create_request("POST", "/onvif/device_service", &request);
  assert_int_equal(result, 0);

  assert_string_equal(request.method, "POST");
  assert_string_equal(request.path, "/onvif/device_service");
}

/**
 * @brief Test HTTP response creation helper
 * @param state Test state (unused)
 */
void test_unit_http_response_creation(void** state) {
  (void)state;

  http_response_t response;

  // Test helper function for response creation
  int result = test_helper_http_create_response(200, &response);
  assert_int_equal(result, 0);

  assert_int_equal(response.status_code, 200);
}

/* ============================================================================
 * Integration Tests Using Mock Framework
 * ============================================================================ */

/**
 * @brief Test HTTP auth with mock handlers
 * @param state Test state (unused)
 */
void test_unit_http_auth_with_mocks(void** state) {
  (void)state;

  // Reset mock state
  http_auth_reset_mock_state();

  // Test mock init handler
  int result = http_auth_mock_init();
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_http_auth_mock_state.init_call_count, 1);

  // Test mock cleanup handler
  http_auth_mock_cleanup();
  assert_int_equal(g_http_auth_mock_state.cleanup_call_count, 1);

  // Test mock operation handler
  http_request_t request;
  http_response_t response;
  test_helper_http_create_request("GET", "/test", &request);
  test_helper_http_create_response(200, &response);

  result = http_auth_mock_operation("VerifyCredentials", &request, &response);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(g_http_auth_mock_state.operation_call_count, 1);
  assert_string_equal(g_http_auth_mock_state.last_operation, "VerifyCredentials");
}

/**
 * @brief Test HTTP auth with failing mock init
 * @param state Test state (unused)
 */
void test_unit_http_auth_mock_init_failure(void** state) {
  (void)state;

  // Reset mock state and set init to fail
  http_auth_reset_mock_state();
  g_http_auth_mock_state.init_result = HTTP_AUTH_ERROR_INVALID;

  // Test failing init handler
  int result = http_auth_mock_init();
  assert_int_equal(result, HTTP_AUTH_ERROR_INVALID);
  assert_int_equal(g_http_auth_mock_state.init_call_count, 1);
}

/* ============================================================================
 * Additional Test Functions Required by Runner
 * ============================================================================ */

/**
 * @brief Test HTTP auth init sets default values
 * @param state Test state (unused)
 */
void test_unit_http_auth_init_sets_defaults(void** state) {
  (void)state;
  test_unit_http_auth_init_success(state);
}

/**
 * @brief Test HTTP auth init with NULL parameter
 * @param state Test state (unused)
 */
void test_unit_http_auth_init_null(void** state) {
  (void)state;
  test_unit_http_auth_init_null_params(state);
}

/**
 * @brief Test HTTP auth verify credentials success
 * @param state Test state (unused)
 */
void test_unit_http_auth_verify_credentials_success(void** state) {
  (void)state;

  // Test successful credential verification
  int result = http_auth_verify_credentials("admin", "password", "admin", "password");
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
}

/**
 * @brief Test HTTP auth verify credentials failure
 * @param state Test state (unused)
 */
void test_unit_http_auth_verify_credentials_failure(void** state) {
  (void)state;

  // Test failed credential verification
  int result = http_auth_verify_credentials("admin", "wrong", "admin", "password");
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);
}

/**
 * @brief Test HTTP auth parse basic credentials success
 * @param state Test state (unused)
 */
void test_unit_http_auth_parse_basic_credentials_success(void** state) {
  (void)state;

  char username[HTTP_MAX_USERNAME_LEN];
  char password[HTTP_MAX_PASSWORD_LEN];

  // Test successful parsing of Basic auth credentials
  int result = http_auth_parse_basic_credentials("Basic YWRtaW46cGFzc3dvcmQ=", username, password);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_string_equal(username, "admin");
  assert_string_equal(password, "password");
}

/**
 * @brief Test HTTP auth parse basic credentials invalid scheme
 * @param state Test state (unused)
 */
void test_unit_http_auth_parse_basic_credentials_invalid_scheme(void** state) {
  (void)state;

  char username[HTTP_MAX_USERNAME_LEN];
  char password[HTTP_MAX_PASSWORD_LEN];

  // Test invalid scheme (not Basic)
  int result = http_auth_parse_basic_credentials("Digest YWRtaW46cGFzc3dvcmQ=", username, password);
  assert_int_equal(result, HTTP_AUTH_ERROR_INVALID);
}

/**
 * @brief Test HTTP auth parse basic credentials decode failure
 * @param state Test state (unused)
 */
void test_unit_http_auth_parse_basic_credentials_decode_failure(void** state) {
  (void)state;

  char username[HTTP_MAX_USERNAME_LEN];
  char password[HTTP_MAX_PASSWORD_LEN];

  // Test invalid base64 encoding
  int result = http_auth_parse_basic_credentials("Basic invalid_base64!", username, password);
  assert_int_equal(result, HTTP_AUTH_ERROR_PARSE_FAILED);
}

/**
 * @brief Test HTTP auth parse basic credentials missing delimiter
 * @param state Test state (unused)
 */
void test_unit_http_auth_parse_basic_credentials_missing_delimiter(void** state) {
  (void)state;

  char username[HTTP_MAX_USERNAME_LEN];
  char password[HTTP_MAX_PASSWORD_LEN];

  // Test missing colon delimiter
  int result = http_auth_parse_basic_credentials("Basic YWRtaW5wYXNzd29yZA==", username, password);
  assert_int_equal(result, HTTP_AUTH_ERROR_PARSE_FAILED);
}

/**
 * @brief Test HTTP auth generate challenge success
 * @param state Test state (unused)
 */
void test_unit_http_auth_generate_challenge_success(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  char challenge[256];
  int result = http_auth_generate_challenge(&config, challenge, sizeof(challenge));
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_true(strstr(challenge, "WWW-Authenticate: Basic realm=") != NULL);
}

/**
 * @brief Test HTTP auth generate challenge invalid
 * @param state Test state (unused)
 */
void test_unit_http_auth_generate_challenge_invalid(void** state) {
  (void)state;

  char challenge[256];
  int result = http_auth_generate_challenge(NULL, challenge, sizeof(challenge));
  assert_int_equal(result, HTTP_AUTH_ERROR_NULL);
}

/**
 * @brief Test HTTP auth validate basic disabled
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_disabled(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 0); // Disabled

  http_request_t request;
  test_helper_http_create_request("GET", "/test", &request);

  int result = http_auth_validate_basic(&request, &config, "admin", "password");
  assert_int_equal(result, HTTP_AUTH_SUCCESS); // Should succeed when disabled
}

/**
 * @brief Test HTTP auth validate basic missing header
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_missing_header(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  http_request_t request;
  test_helper_http_create_request("GET", "/test", &request);
  // No Authorization header added

  int result = http_auth_validate_basic(&request, &config, "admin", "password");
  assert_int_equal(result, HTTP_AUTH_ERROR_NO_HEADER);
}

/**
 * @brief Test HTTP auth validate basic invalid credentials
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_invalid_credentials(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  http_request_t request;
  test_helper_http_create_request("GET", "/test", &request);

  // Add invalid credentials header (manually add to headers)
  request.headers = malloc(sizeof(http_header_t));
  request.headers[0].name = malloc(14);  // "Authorization" + null terminator
  request.headers[0].value = malloc(33); // "Basic d3Jvbmc6d3Jvbmc=" + null terminator
  strcpy(request.headers[0].name, "Authorization");
  strcpy(request.headers[0].value, "Basic d3Jvbmc6d3Jvbmc=");
  request.header_count = 1;

  int result = http_auth_validate_basic(&request, &config, "admin", "password");
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);
}

/**
 * @brief Test HTTP auth validate basic success
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_success(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  http_request_t request;
  test_helper_http_create_request("GET", "/test", &request);

  // Add valid credentials header (manually add to headers)
  request.headers = malloc(sizeof(http_header_t));
  request.headers[0].name = malloc(14);  // "Authorization" + null terminator
  request.headers[0].value = malloc(33); // "Basic YWRtaW46cGFzc3dvcmQ=" + null terminator
  strcpy(request.headers[0].name, "Authorization");
  strcpy(request.headers[0].value, "Basic YWRtaW46cGFzc3dvcmQ=");
  request.header_count = 1;

  int result = http_auth_validate_basic(&request, &config, "admin", "password");
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
}

/**
 * @brief Test HTTP auth validate basic parse failure
 * @param state Test state (unused)
 */
void test_unit_http_auth_validate_basic_parse_failure(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  http_request_t request;
  test_helper_http_create_request("GET", "/test", &request);

  // Add malformed credentials header (manually add to headers)
  request.headers = malloc(sizeof(http_header_t));
  request.headers[0].name = malloc(14);  // "Authorization" + null terminator
  request.headers[0].value = malloc(32); // "Basic invalid!" + null terminator
  strcpy(request.headers[0].name, "Authorization");
  strcpy(request.headers[0].value, "Basic invalid!");
  request.header_count = 1;

  int result = http_auth_validate_basic(&request, &config, "admin", "password");
  assert_int_equal(result, HTTP_AUTH_ERROR_PARSE_FAILED);
}

/**
 * @brief Test HTTP auth create 401 response
 * @param state Test state (unused)
 */
void test_unit_http_auth_create_401_response(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);

  http_response_t response = http_auth_create_401_response(&config);
  assert_int_equal(response.status_code, 401);
  assert_true(response.body != NULL);
  assert_true(strstr(response.body, "401 Unauthorized") != NULL);

  // Cleanup
  http_response_free(&response);
}

/**
 * @brief Test HTTP auth create 401 response invalid realm
 * @param state Test state (unused)
 */
void test_unit_http_auth_create_401_response_invalid_realm(void** state) {
  (void)state;

  struct http_auth_config config;
  test_helper_http_init_auth_config(&config, HTTP_AUTH_BASIC, 1);
  // Set invalid realm
  strcpy(config.realm, "");

  http_response_t response = http_auth_create_401_response(&config);
  assert_int_equal(response.status_code, 401);
  assert_true(response.body != NULL);
  assert_true(strstr(response.body, "401 Unauthorized") != NULL);

  // Cleanup
  http_response_free(&response);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest http_auth_tests[] = {
  // NULL Parameter Tests (Refactored)
  cmocka_unit_test_setup_teardown(test_unit_http_auth_validate_basic_null_params,
                                  setup_http_auth_tests, teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_auth_init_null_params, setup_http_auth_tests,
                                  teardown_http_auth_tests),

  // Success Case Tests Using Helper Functions
  cmocka_unit_test_setup_teardown(test_unit_http_auth_init_success, setup_http_auth_tests,
                                  teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_auth_config_setup, setup_http_auth_tests,
                                  teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_auth_basic_header_building, setup_http_auth_tests,
                                  teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_request_creation, setup_http_auth_tests,
                                  teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_response_creation, setup_http_auth_tests,
                                  teardown_http_auth_tests),

  // Integration Tests Using Mock Framework
  cmocka_unit_test_setup_teardown(test_unit_http_auth_with_mocks, setup_http_auth_tests,
                                  teardown_http_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_http_auth_mock_init_failure, setup_http_auth_tests,
                                  teardown_http_auth_tests),
};
