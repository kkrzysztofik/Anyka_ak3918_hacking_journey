/**
 * @file http_auth.c
 * @brief HTTP Authentication Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains the implementation of HTTP authentication functions
 * for the HTTP server, including Basic authentication support.
 */

#include "networking/http/http_auth.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "core/config/config_runtime.h"
#include "networking/http/http_constants.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"
#include "utils/string/string_shims.h"
#include "utils/validation/input_validation.h"

/* ==================== Constants ==================== */

#define BASIC_AUTH_PREFIX_LEN      6
#define CONTENT_TYPE_BUFFER_SIZE   10
#define RESPONSE_BODY_BUFFER_SIZE  512
#define CHALLENGE_VALUE_EXTRA_SIZE 50

/* ==================== Helper Functions ==================== */

/**
 * Extract and validate a credential field (username or password)
 * @param source Source string to extract from
 * @param source_len Length of source string
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @return HTTP_AUTH_SUCCESS on success, error code on failure
 */
static int extract_and_validate_credential(const char* source, size_t source_len, char* dest,
                                           size_t dest_size) {
  if (!source || !dest || dest_size == 0) {
    return HTTP_AUTH_ERROR_NULL;
  }

  // Check if it fits in buffer (before trimming)
  if (source_len >= dest_size) {
    return HTTP_AUTH_ERROR_INVALID; // Too long
  }

  // Use safe string copy with bounds checking
  strncpy(dest, source, source_len);
  dest[source_len] = '\0';

  // Trim whitespace
  trim_whitespace(dest);

  // Validate length after trimming
  size_t trimmed_len = strnlen(dest, dest_size);
  if (trimmed_len == 0) {
    return HTTP_AUTH_ERROR_INVALID; // Empty after trimming
  }

  // Note: Input validation is done in the calling parse function to avoid duplication

  return HTTP_AUTH_SUCCESS;
}

/* ==================== Authentication Functions ==================== */

/**
 * Initialize HTTP authentication configuration
 */
int http_auth_init(struct http_auth_config* auth_config) {
  if (!auth_config) {
    return HTTP_AUTH_ERROR_NULL;
  }

  memset(auth_config, 0, sizeof(struct http_auth_config));
  auth_config->auth_type = HTTP_AUTH_NONE;
  auth_config->enabled = false;

  // Set default realm with validation
  const char* default_realm = "ONVIF Server";
  if (validate_realm_input(default_realm) != ONVIF_VALIDATION_SUCCESS) {
    platform_log_error("Invalid default realm: %s", default_realm);
    return HTTP_AUTH_ERROR_INVALID;
  }

  strncpy(auth_config->realm, default_realm, sizeof(auth_config->realm) - 1);
  auth_config->realm[sizeof(auth_config->realm) - 1] = '\0';

  return HTTP_AUTH_SUCCESS;
}

/**
 * Cleanup HTTP authentication configuration
 */
void http_auth_cleanup(struct http_auth_config* auth_config) {
  if (!auth_config) {
    return;
  }

  // No dynamic memory to cleanup since we use global config
  memset(auth_config, 0, sizeof(struct http_auth_config));
}

/**
 * Validate HTTP Basic authentication credentials
 *
 * Validates credentials against the runtime user management system only.
 * Legacy config-based authentication has been removed.
 */
int http_auth_validate_basic(const http_request_t* request,
                             const struct http_auth_config* auth_config) {
  if (!request || !auth_config) {
    return HTTP_AUTH_ERROR_NULL;
  }

  if (!auth_config->enabled || auth_config->auth_type != HTTP_AUTH_BASIC) {
    return HTTP_AUTH_SUCCESS; // Authentication not required
  }

  // Check if config_runtime is initialized before attempting to use it
  if (!config_runtime_is_initialized()) {
    platform_log_error(
      "[HTTP_AUTH] config_runtime not initialized - authentication cannot proceed\n");
    return HTTP_AUTH_ERROR_INVALID;
  }

  // Find Authorization header
  char auth_header[HTTP_MAX_AUTH_HEADER_LEN] = {0};
  if (find_header_value(request->headers, request->header_count, "Authorization", auth_header,
                        sizeof(auth_header)) != 0) {
    platform_log_debug("No Authorization header found for client %s\n", request->client_ip);
    return HTTP_AUTH_ERROR_NO_HEADER;
  }

  // Parse Basic credentials
  char username[HTTP_MAX_USERNAME_LEN] = {0};
  char password[HTTP_MAX_PASSWORD_LEN] = {0};
  if (http_auth_parse_basic_credentials(auth_header, username, password) != HTTP_AUTH_SUCCESS) {
    platform_log_debug("Failed to parse Basic auth credentials from %s\n", request->client_ip);
    return HTTP_AUTH_ERROR_PARSE_FAILED;
  }

  // Verify credentials against runtime user management system
  if (http_auth_verify_credentials(username, password) != HTTP_AUTH_SUCCESS) {
    platform_log_error("Authentication failed for user %s from %s\n", username, request->client_ip);
    return HTTP_AUTH_UNAUTHENTICATED;
  }

  platform_log_debug("Authentication successful for user %s from %s\n", username,
                     request->client_ip);
  return HTTP_AUTH_SUCCESS;
}

/**
 * Generate WWW-Authenticate challenge header
 */
int http_auth_generate_challenge(const struct http_auth_config* auth_config, char* challenge,
                                 size_t challenge_size) {
  if (!auth_config || !challenge || challenge_size == 0) {
    return HTTP_AUTH_ERROR_NULL;
  }

  // Validate realm before using it
  if (validate_realm_input(auth_config->realm) != ONVIF_VALIDATION_SUCCESS) {
    platform_log_error("Invalid realm in auth config: %s", auth_config->realm);
    return HTTP_AUTH_ERROR_INVALID;
  }

  int ret =
    snprintf(challenge, challenge_size, "WWW-Authenticate: Basic realm=\"%s\"", auth_config->realm);

  if (ret < 0 || (size_t)ret >= challenge_size) {
    return HTTP_AUTH_ERROR_BUFFER_TOO_SMALL;
  }

  return HTTP_AUTH_SUCCESS;
}

/**
 * Parse Basic authentication credentials from Authorization header
 */
int http_auth_parse_basic_credentials(const char* auth_header, char* username, char* password) {
  if (!auth_header || !username || !password) {
    return HTTP_AUTH_ERROR_NULL;
  }

  // Validate Authorization header first
  if (validate_auth_header_input(auth_header) != ONVIF_VALIDATION_SUCCESS) {
    return HTTP_AUTH_ERROR_INVALID;
  }

  const char* encoded = auth_header + BASIC_AUTH_PREFIX_LEN; // Skip "Basic " prefix
  size_t encoded_len = strnlen(encoded, HTTP_MAX_AUTH_HEADER_LEN - BASIC_AUTH_PREFIX_LEN);

  if (encoded_len == 0) {
    return HTTP_AUTH_ERROR_INVALID;
  }

  // Decode Base64 using secure validation
  char decoded[HTTP_MAX_USERNAME_LEN + HTTP_MAX_PASSWORD_LEN + 2] = {0};
  if (validate_and_decode_base64(encoded, decoded, sizeof(decoded)) != ONVIF_VALIDATION_SUCCESS) {
    return HTTP_AUTH_ERROR_PARSE_FAILED;
  }

  size_t decoded_len = strnlen(decoded, sizeof(decoded));
  if (decoded_len == 0) {
    return HTTP_AUTH_ERROR_PARSE_FAILED;
  }

  // Find colon separator
  char* colon = strchr(decoded, ':');
  if (!colon) {
    return HTTP_AUTH_ERROR_PARSE_FAILED;
  }

  // Extract username
  size_t username_len = colon - decoded;
  int result =
    extract_and_validate_credential(decoded, username_len, username, HTTP_MAX_USERNAME_LEN);
  if (result != HTTP_AUTH_SUCCESS) {
    return result;
  }

  // Extract password
  size_t password_len = decoded_len - username_len - 1;
  result =
    extract_and_validate_credential(colon + 1, password_len, password, HTTP_MAX_PASSWORD_LEN);
  if (result != HTTP_AUTH_SUCCESS) {
    return result;
  }

  return HTTP_AUTH_SUCCESS;
}

/**
 * Verify Basic authentication credentials against runtime user management system
 *
 * This function authenticates users exclusively through the runtime user management
 * system (config_runtime). Legacy config-based authentication has been removed.
 */
int http_auth_verify_credentials(const char* username, const char* password) {
  int result = HTTP_AUTH_ERROR_INVALID;

  if (!username || !password) {
    platform_log_error("[HTTP_AUTH] NULL credentials provided\n");
    return HTTP_AUTH_ERROR_NULL;
  }

  /* Log authentication attempt without exposing credentials (T082) */
  platform_log_info("[HTTP_AUTH] Authentication attempt for user: %s\n", username);

  /* Authenticate against runtime user management system */
  result = config_runtime_authenticate_user(username, password);
  if (result == ONVIF_SUCCESS) {
    platform_log_info("[HTTP_AUTH] Authentication successful for user: %s\n", username);
    return HTTP_AUTH_SUCCESS;
  }

  if (result == ONVIF_ERROR_AUTHENTICATION_FAILED) {
    /* User found but password is wrong */
    platform_log_warning("[HTTP_AUTH] Authentication failed for user: %s (password mismatch)\n",
                         username);
    return HTTP_AUTH_UNAUTHENTICATED;
  }

  if (result == ONVIF_ERROR_NOT_FOUND) {
    /* User not found in system */
    platform_log_warning("[HTTP_AUTH] Authentication failed for user: %s (user not found)\n",
                         username);
    return HTTP_AUTH_UNAUTHENTICATED;
  }

  /* System error */
  platform_log_error("[HTTP_AUTH] System error during authentication for user: %s (error: %d)\n",
                     username, result);
  return HTTP_AUTH_ERROR_INVALID;
}

/**
 * Create HTTP 401 Unauthorized response with WWW-Authenticate header
 */
http_response_t http_auth_create_401_response(const struct http_auth_config* auth_config) {
  http_response_t response = {0};
  response.status_code = HTTP_STATUS_UNAUTHORIZED;

  // Allocate content_type dynamically to avoid segmentation fault in http_response_free
  response.content_type = malloc(CONTENT_TYPE_BUFFER_SIZE);
  if (response.content_type) {
    strcpy(response.content_type, "text/html");
  }

  response.headers = NULL;
  response.header_count = 0;

  // Create response body with realm information
  char body[RESPONSE_BODY_BUFFER_SIZE] = {0};
  if (auth_config && auth_config->realm[0] != '\0') {
    // Validate realm before using it
    if (validate_realm_input(auth_config->realm) == ONVIF_VALIDATION_SUCCESS) {
      int result =
        snprintf(body, sizeof(body),
                 "<html><body><h1>401 Unauthorized</h1><p>Authentication required for realm: "
                 "%s</p></body></html>",
                 auth_config->realm);
      if (result < 0 || (size_t)result >= sizeof(body)) {
        platform_log_error("Failed to format response body with realm");
        strcpy(
          body,
          "<html><body><h1>401 Unauthorized</h1><p>Authentication required.</p></body></html>");
      }
    } else {
      platform_log_warning("Invalid realm in 401 response, using default");
      strcpy(body,
             "<html><body><h1>401 Unauthorized</h1><p>Authentication required.</p></body></html>");
    }
  } else {
    strcpy(body,
           "<html><body><h1>401 Unauthorized</h1><p>Authentication required.</p></body></html>");
  }

  response.body_length = strlen(body);
  response.body = malloc(response.body_length + 1);
  if (response.body) {
    strcpy(response.body, body);
  }

  // Add WWW-Authenticate header
  if (auth_config && auth_config->realm[0] != '\0' &&
      validate_realm_input(auth_config->realm) == ONVIF_VALIDATION_SUCCESS) {
    char challenge_value[HTTP_MAX_REALM_LEN + CHALLENGE_VALUE_EXTRA_SIZE] = {0};
    int result =
      snprintf(challenge_value, sizeof(challenge_value), "Basic realm=\"%s\"", auth_config->realm);
    if (result < 0 || (size_t)result >= sizeof(challenge_value)) {
      platform_log_error("Failed to format challenge value, using default");
      http_response_add_header(&response, "WWW-Authenticate", "Basic realm=\"ONVIF Server\"");
      return response;
    }
    http_response_add_header(&response, "WWW-Authenticate", challenge_value);
  } else {
    http_response_add_header(&response, "WWW-Authenticate", "Basic realm=\"ONVIF Server\"");
  }

  return response;
}
