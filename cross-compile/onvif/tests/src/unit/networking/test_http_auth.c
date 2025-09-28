/**
 * @file test_http_auth.c
 * @brief Unit tests for HTTP authentication module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_auth.h"
#include "networking/http/http_parser.h"
#include "utils/error/error_handling.h"
#include "utils/security/base64_utils.h"
#include "utils/security/security_hardening.h"

// Constants for test buffer sizes
#define TEST_CHALLENGE_BUFFER_SIZE 32
#define TEST_CHALLENGE_SMALL_SIZE  64

const char* mock_get_last_header_name(void);
const char* mock_get_last_header_value(void);
void mock_reset_last_header(void);

/* ==================== Test Stubs ==================== */

int find_header_value(const http_header_t* headers, size_t header_count, const char* header_name,
                      char* value, size_t value_size) {
  if (!headers || !header_name || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  for (size_t i = 0; i < header_count; i++) {
    if (!headers[i].name || !headers[i].value) {
      continue;
    }

    if (strcasecmp(headers[i].name, header_name) == 0) {
      size_t required_len = strlen(headers[i].value);
      if (required_len >= value_size) {
        return ONVIF_ERROR_INVALID;
      }

      int written = snprintf(value, value_size, "%s", headers[i].value);
      (void)written; // Suppress unused return value warning
      if (written < 0 || (size_t)written >= value_size) {
        return ONVIF_ERROR_INVALID;
      }
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

/* ==================== Security Stubs ==================== */

int security_detect_sql_injection(const char* input) {
  (void)input;
  return ONVIF_SUCCESS;
}

int security_detect_xss_attack(const char* input) {
  (void)input;
  return ONVIF_SUCCESS;
}

/* ==================== Helper Functions ==================== */

static void build_basic_header(const char* credentials, char* header_value, size_t header_size) {
  assert_non_null(credentials);
  assert_non_null(header_value);

  char encoded[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  int base64_result = onvif_util_base64_encode((const unsigned char*)credentials,
                                               strlen(credentials), encoded, sizeof(encoded));
  assert_int_equal(base64_result, ONVIF_SUCCESS);

  int written = snprintf(header_value, header_size, "Basic %s", encoded);
  (void)written; // Suppress unused return value warning
  assert_true(written > 0);
}

static void initialize_basic_auth_config(struct http_auth_config* config) {
  assert_int_equal(http_auth_init(config), HTTP_AUTH_SUCCESS);
  config->enabled = true;
  config->auth_type = HTTP_AUTH_BASIC;
}

/* ==================== Unit Tests ==================== */

/**
 * @brief Verify that initialization sets default values
 */
void test_http_auth_init_sets_defaults(void** state) {
  (void)state;

  struct http_auth_config config;
  int result = http_auth_init(&config);

  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_int_equal(config.auth_type, HTTP_AUTH_NONE);
  assert_false(config.enabled);
  assert_string_equal(config.realm, "ONVIF Server");
}

/**
 * @brief Verify that initialization handles null pointer
 */
void test_http_auth_init_null(void** state) {
  (void)state;

  assert_int_equal(http_auth_init(NULL), HTTP_AUTH_ERROR_NULL);
}

/**
 * @brief Verify credential verification success path
 */
void test_http_auth_verify_credentials_success(void** state) {
  (void)state;

  assert_int_equal(http_auth_verify_credentials("admin", "secret", "admin", "secret"),
                   HTTP_AUTH_SUCCESS);
}

/**
 * @brief Verify credential verification failure scenarios
 */
void test_http_auth_verify_credentials_failure(void** state) {
  (void)state;

  assert_int_equal(http_auth_verify_credentials("admin", "wrong", "admin", "secret"),
                   HTTP_AUTH_UNAUTHENTICATED);
  assert_int_equal(http_auth_verify_credentials("guest", "secret", "admin", "secret"),
                   HTTP_AUTH_UNAUTHENTICATED);
  assert_int_equal(http_auth_verify_credentials(NULL, "secret", "admin", "secret"),
                   HTTP_AUTH_ERROR_NULL);
}

/**
 * @brief Validate Basic credential parsing when header is correct
 */
void test_http_auth_parse_basic_credentials_success(void** state) {
  (void)state;

  char header_value[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  build_basic_header("admin:secret", header_value, sizeof(header_value));

  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};

  int result = http_auth_parse_basic_credentials(header_value, username, password);
  assert_int_equal(result, HTTP_AUTH_SUCCESS);
  assert_string_equal(username, "admin");
  assert_string_equal(password, "secret");
}

/**
 * @brief Ensure parsing fails for invalid authorization scheme
 */
void test_http_auth_parse_basic_credentials_invalid_scheme(void** state) {
  (void)state;

  const char* header_value = "Bearer token";

  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};

  assert_int_equal(http_auth_parse_basic_credentials(header_value, username, password),
                   HTTP_AUTH_ERROR_INVALID);
}

/**
 * @brief Ensure parsing detects Base64 decoding failures
 */
void test_http_auth_parse_basic_credentials_decode_failure(void** state) {
  (void)state;

  const char* header_value = "Basic invalid@@";

  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};

  assert_int_equal(http_auth_parse_basic_credentials(header_value, username, password),
                   HTTP_AUTH_ERROR_PARSE_FAILED);
}

/**
 * @brief Ensure parsing rejects credentials containing null bytes
 */
void test_http_auth_parse_basic_credentials_missing_delimiter(void** state) {
  (void)state;

  char header_value[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  build_basic_header("adminsecret", header_value, sizeof(header_value));

  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};

  assert_int_equal(http_auth_parse_basic_credentials(header_value, username, password),
                   HTTP_AUTH_ERROR_PARSE_FAILED);
}

/**
 * @brief Validate generation of WWW-Authenticate challenge
 */
void test_http_auth_generate_challenge_success(void** state) {
  (void)state;

  struct http_auth_config config;
  assert_int_equal(http_auth_init(&config), HTTP_AUTH_SUCCESS);
  (void)snprintf(config.realm, sizeof(config.realm), "%s", "Secure Area");

  char challenge[HTTP_MAX_REALM_LEN + TEST_CHALLENGE_BUFFER_SIZE] = {0};
  assert_int_equal(http_auth_generate_challenge(&config, challenge, sizeof(challenge)),
                   HTTP_AUTH_SUCCESS);
  assert_string_equal(challenge, "WWW-Authenticate: Basic realm=\"Secure Area\"");

  http_auth_cleanup(&config);
}

/**
 * @brief Validate challenge generation rejects invalid parameters
 */
void test_http_auth_generate_challenge_invalid(void** state) {
  (void)state;

  struct http_auth_config config;
  assert_int_equal(http_auth_init(&config), HTTP_AUTH_SUCCESS);
  (void)snprintf(config.realm, sizeof(config.realm), "%s", "Invalid\"Realm");

  char challenge[TEST_CHALLENGE_SMALL_SIZE] = {0};
  assert_int_equal(http_auth_generate_challenge(&config, challenge, sizeof(challenge)),
                   HTTP_AUTH_ERROR_INVALID);
  assert_int_equal(http_auth_generate_challenge(NULL, challenge, sizeof(challenge)),
                   HTTP_AUTH_ERROR_NULL);
  assert_int_equal(http_auth_generate_challenge(&config, NULL, sizeof(challenge)),
                   HTTP_AUTH_ERROR_NULL);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP auth returns success when disabled
 */
void test_http_auth_validate_basic_disabled(void** state) {
  (void)state;

  http_request_t request = {0};
  struct http_auth_config config;
  assert_int_equal(http_auth_init(&config), HTTP_AUTH_SUCCESS);

  assert_int_equal(http_auth_validate_basic(&request, &config, "admin", "secret"),
                   HTTP_AUTH_SUCCESS);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP auth detects missing header when enabled
 */
void test_http_auth_validate_basic_missing_header(void** state) {
  (void)state;

  http_request_t request = {0};
  struct http_auth_config config;
  initialize_basic_auth_config(&config);

  assert_int_equal(http_auth_validate_basic(&request, &config, "admin", "secret"),
                   HTTP_AUTH_ERROR_NO_HEADER);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP auth detects invalid credentials
 */
void test_http_auth_validate_basic_invalid_credentials(void** state) {
  (void)state;

  http_request_t request = {0};

  char auth_value[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  build_basic_header("admin:wrong", auth_value, sizeof(auth_value));

  http_header_t headers[1] = {{"Authorization", auth_value}};
  request.headers = headers;
  request.header_count = 1;
  (void)snprintf(request.client_ip, sizeof(request.client_ip), "%s", "127.0.0.1");

  struct http_auth_config config;
  initialize_basic_auth_config(&config);

  assert_int_equal(http_auth_validate_basic(&request, &config, "admin", "secret"),
                   HTTP_AUTH_UNAUTHENTICATED);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP auth success path
 */
void test_http_auth_validate_basic_success(void** state) {
  (void)state;

  http_request_t request = {0};

  char auth_value[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  build_basic_header("admin:secret", auth_value, sizeof(auth_value));

  http_header_t headers[1] = {{"Authorization", auth_value}};
  request.headers = headers;
  request.header_count = 1;
  (void)snprintf(request.client_ip, sizeof(request.client_ip), "%s", "192.168.1.10");

  struct http_auth_config config;
  initialize_basic_auth_config(&config);

  assert_int_equal(http_auth_validate_basic(&request, &config, "admin", "secret"),
                   HTTP_AUTH_SUCCESS);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP auth handles parsing failures
 */
void test_http_auth_validate_basic_parse_failure(void** state) {
  (void)state;

  http_request_t request = {0};

  http_header_t headers[1] = {{"Authorization", "Basic invalid@@"}};
  request.headers = headers;
  request.header_count = 1;
  (void)snprintf(request.client_ip, sizeof(request.client_ip), "%s", "10.0.0.5");

  struct http_auth_config config;
  initialize_basic_auth_config(&config);

  assert_int_equal(http_auth_validate_basic(&request, &config, "admin", "secret"),
                   HTTP_AUTH_ERROR_PARSE_FAILED);

  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP 401 response includes challenge header and HTML body
 */
void test_http_auth_create_401_response(void** state) {
  (void)state;

  struct http_auth_config config;
  assert_int_equal(http_auth_init(&config), HTTP_AUTH_SUCCESS);
  (void)snprintf(config.realm, sizeof(config.realm), "%s", "Protected");

  mock_reset_last_header();

  http_response_t response = http_auth_create_401_response(&config);

  assert_int_equal(response.status_code, 401);
  assert_string_equal(response.content_type, "text/html");
  assert_non_null(response.body);
  assert_true(response.body_length > 0);
  assert_int_equal(response.header_count, 1);
  assert_string_equal(mock_get_last_header_name(), "WWW-Authenticate");
  assert_string_equal(mock_get_last_header_value(), "Basic realm=\"Protected\"");

  http_response_free(&response);
  http_auth_cleanup(&config);
}

/**
 * @brief Validate HTTP 401 fallback when realm is invalid
 */
void test_http_auth_create_401_response_invalid_realm(void** state) {
  (void)state;

  struct http_auth_config config;
  assert_int_equal(http_auth_init(&config), HTTP_AUTH_SUCCESS);
  (void)snprintf(config.realm, sizeof(config.realm), "%s", "Bad\"Realm");

  mock_reset_last_header();

  http_response_t response = http_auth_create_401_response(&config);

  assert_int_equal(response.status_code, 401);
  assert_int_equal(response.header_count, 1);
  assert_string_equal(mock_get_last_header_value(), "Basic realm=\"ONVIF Server\"");

  http_response_free(&response);
  http_auth_cleanup(&config);
}
