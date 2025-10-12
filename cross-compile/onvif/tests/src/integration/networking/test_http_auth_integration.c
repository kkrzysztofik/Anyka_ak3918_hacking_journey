/**
 * @file test_http_auth_integration.c
 * @brief Integration tests for HTTP authentication with runtime user management
 * @author kkrzysztofik
 * @date 2025
 *
 * These tests verify the integration between:
 * - HTTP authentication layer (http_auth.c)
 * - Runtime user management (config_runtime.c)
 * - Configuration system (config.h)
 * - Security utilities (hash_utils.c)
 */

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "networking/http/http_auth.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"
#include "utils/security/hash_utils.h"

/* Test constants */
#define TEST_USERNAME "testuser"
#define TEST_PASSWORD "testpass123"
#define TEST_USERNAME_2 "admin"
#define TEST_PASSWORD_2 "admin456"
#define TEST_REALM "ONVIF Camera"
#define TEST_INVALID_USER "nonexistent"
#define TEST_INVALID_PASS "wrongpass"

/* Test state structure */
struct http_auth_integration_state {
  struct http_auth_config auth_config;
  struct application_config app_config;
  config_manager_t config_mgr;
  int init_result;
};

/**
 * @brief Setup function for HTTP auth integration tests
 */
static int http_auth_integration_setup(void** state)
{
  struct http_auth_integration_state* test_state =
      calloc(1, sizeof(struct http_auth_integration_state));
  if (!test_state) {
    return -1;
  }

  /* Initialize configuration structures */
  memset(&test_state->config_mgr, 0, sizeof(config_manager_t));
  memset(&test_state->app_config, 0, sizeof(struct application_config));

  /* Initialize runtime configuration manager */
  test_state->init_result = config_runtime_init(&test_state->app_config);
  if (test_state->init_result != ONVIF_SUCCESS) {
    free(test_state);
    return -1;
  }

  /* Initialize HTTP authentication */
  test_state->init_result = http_auth_init(&test_state->auth_config);
  if (test_state->init_result != HTTP_AUTH_SUCCESS) {
    config_runtime_cleanup();
    free(test_state);
    return -1;
  }

  /* Configure auth settings */
  test_state->auth_config.auth_type = HTTP_AUTH_BASIC;
  test_state->auth_config.enabled = true;
  strncpy(test_state->auth_config.realm, TEST_REALM, HTTP_MAX_REALM_LEN - 1);

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for HTTP auth integration tests
 */
static int http_auth_integration_teardown(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;

  if (test_state) {
    http_auth_cleanup(&test_state->auth_config);
    config_runtime_cleanup();
    free(test_state);
  }

  return 0;
}

/**
 * @brief Test successful authentication with runtime user
 */
static void test_integration_http_auth_runtime_user_success(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Add user to runtime configuration */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify credentials through HTTP auth layer */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test failed authentication with incorrect password
 */
static void test_integration_http_auth_runtime_user_wrong_password(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Add user to runtime configuration */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify with wrong password */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_INVALID_PASS);
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test authentication with non-existent user
 */
static void test_integration_http_auth_runtime_user_not_found(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Try to authenticate non-existent user */
  result = http_auth_verify_credentials(TEST_INVALID_USER, TEST_INVALID_PASS);
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);
}

/**
 * @brief Test authentication with multiple users
 */
static void test_integration_http_auth_multiple_users(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Add first user */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Add second user */
  result = config_runtime_add_user(TEST_USERNAME_2, TEST_PASSWORD_2);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Authenticate first user */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Authenticate second user */
  result = http_auth_verify_credentials(TEST_USERNAME_2, TEST_PASSWORD_2);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Verify first user can't use second user's password */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD_2);
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
  result = config_runtime_remove_user(TEST_USERNAME_2);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test user password update and authentication
 */
static void test_integration_http_auth_password_update(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;
  const char* new_password = "newpass789";

  /* Add user */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify original password works */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Update password */
  result = config_runtime_update_user_password(TEST_USERNAME, new_password);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify old password no longer works */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);

  /* Verify new password works */
  result = http_auth_verify_credentials(TEST_USERNAME, new_password);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test user removal invalidates authentication
 */
static void test_integration_http_auth_user_removal(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Add user */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify authentication works */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Remove user */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify authentication now fails */
  result = http_auth_verify_credentials(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_UNAUTHENTICATED);
}

/**
 * @brief Test user enumeration integration
 */
static void test_integration_http_auth_user_enumeration(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;
  char usernames[MAX_USERS][MAX_USERNAME_LENGTH + 1];
  int user_count = 0;

  /* Add multiple users */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);
  result = config_runtime_add_user(TEST_USERNAME_2, TEST_PASSWORD_2);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Enumerate users */
  result = config_runtime_enumerate_users(usernames, MAX_USERS, &user_count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(user_count, 2);

  /* Verify both usernames are present (order may vary) */
  int found_user1 = 0, found_user2 = 0;
  for (int i = 0; i < user_count; i++) {
    if (strcmp(usernames[i], TEST_USERNAME) == 0) {
      found_user1 = 1;
    } else if (strcmp(usernames[i], TEST_USERNAME_2) == 0) {
      found_user2 = 1;
    }
  }
  assert_int_equal(found_user1, 1);
  assert_int_equal(found_user2, 1);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
  result = config_runtime_remove_user(TEST_USERNAME_2);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify enumeration after removal */
  result = config_runtime_enumerate_users(usernames, MAX_USERS, &user_count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(user_count, 0);
}

/**
 * @brief Test null parameter handling
 */
static void test_integration_http_auth_null_parameters(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;

  /* Null username */
  result = http_auth_verify_credentials(NULL, TEST_PASSWORD);
  assert_int_equal(result, HTTP_AUTH_ERROR_NULL);

  /* Null password */
  result = http_auth_verify_credentials(TEST_USERNAME, NULL);
  assert_int_equal(result, HTTP_AUTH_ERROR_NULL);

  /* Both null */
  result = http_auth_verify_credentials(NULL, NULL);
  assert_int_equal(result, HTTP_AUTH_ERROR_NULL);
}

/**
 * @brief Test Basic auth header parsing and verification integration
 */
static void test_integration_http_auth_basic_header_parsing(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  int result;
  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};

  /* Create Basic auth header: "Basic dGVzdHVzZXI6dGVzdHBhc3MxMjM=" (testuser:testpass123) */
  const char* auth_header = "Basic dGVzdHVzZXI6dGVzdHBhc3MxMjM=";

  /* Add user to runtime */
  result = config_runtime_add_user(TEST_USERNAME, TEST_PASSWORD);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Parse Basic auth credentials */
  result = http_auth_parse_basic_credentials(auth_header, username, password);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_string_equal(username, TEST_USERNAME);
  assert_string_equal(password, TEST_PASSWORD);

  /* Verify parsed credentials */
  result = http_auth_verify_credentials(username, password);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);

  /* Cleanup */
  result = config_runtime_remove_user(TEST_USERNAME);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test 401 response generation
 */
static void test_integration_http_auth_401_response(void** state)
{
  struct http_auth_integration_state* test_state =
      (struct http_auth_integration_state*)*state;
  http_response_t response;

  /* Generate 401 response */
  response = http_auth_create_401_response(&test_state->auth_config);

  /* Verify response properties */
  assert_int_equal(response.status_code, 401);
  assert_non_null(response.body);
  assert_true(response.body_length > 0);

  /* Verify WWW-Authenticate header is present */
  int found_auth_header = 0;
  if (response.headers != NULL && response.header_count > 0) {
    for (size_t i = 0; i < response.header_count; i++) {
      if (response.headers[i].name && strstr(response.headers[i].name, "WWW-Authenticate") != NULL) {
        found_auth_header = 1;
        assert_true(response.headers[i].value != NULL);
        assert_true(strstr(response.headers[i].value, "Basic") != NULL);
        assert_true(strstr(response.headers[i].value, TEST_REALM) != NULL);
        break;
      }
    }
  }
  assert_int_equal(found_auth_header, 1);

  /* Cleanup using proper response free function */
  http_response_free(&response);
}

/* Test suite */
const struct CMUnitTest http_auth_integration_tests[] = {
    cmocka_unit_test_setup_teardown(test_integration_http_auth_runtime_user_success,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_runtime_user_wrong_password,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_runtime_user_not_found,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_multiple_users,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_password_update,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_user_removal,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_user_enumeration,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_null_parameters,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_basic_header_parsing,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
    cmocka_unit_test_setup_teardown(test_integration_http_auth_401_response,
                                    http_auth_integration_setup,
                                    http_auth_integration_teardown),
};
