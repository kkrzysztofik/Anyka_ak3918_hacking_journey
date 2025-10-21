/**
 * @file http_auth.h
 * @brief HTTP Authentication Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all authentication-related function declarations
 * for the HTTP server, including Basic authentication support.
 */

#ifndef HTTP_AUTH_H
#define HTTP_AUTH_H

#include <bits/types.h>
#include <stdbool.h>

#include "http_parser.h"

/* HTTP Authentication constants */
#define HTTP_MAX_USERNAME_LEN    64
#define HTTP_MAX_PASSWORD_LEN    64
#define HTTP_MAX_REALM_LEN       128
#define HTTP_MAX_AUTH_HEADER_LEN 1024

/* HTTP Authentication return codes */
#define HTTP_AUTH_SUCCESS                0
#define HTTP_AUTH_ERROR_NULL             -1
#define HTTP_AUTH_ERROR_INVALID          -2
#define HTTP_AUTH_ERROR_NO_HEADER        -3
#define HTTP_AUTH_ERROR_PARSE_FAILED     -4
#define HTTP_AUTH_UNAUTHENTICATED        -5
#define HTTP_AUTH_ERROR_BUFFER_TOO_SMALL -6

/* HTTP Authentication types */
typedef enum { HTTP_AUTH_NONE = 0, HTTP_AUTH_BASIC = 1 } http_auth_type_t;

/* HTTP Authentication configuration */
struct http_auth_config {
  http_auth_type_t auth_type;
  char realm[HTTP_MAX_REALM_LEN];
  bool enabled;
};

/* Authentication functions */
/**
 * @brief Initialize HTTP authentication configuration
 * @param auth_config Authentication configuration to initialize
 * @return 0 on success, -1 on error
 */
int http_auth_init(struct http_auth_config* auth_config);

/**
 * @brief Cleanup HTTP authentication configuration
 * @param auth_config Authentication configuration to cleanup
 */
void http_auth_cleanup(struct http_auth_config* auth_config);

/**
 * @brief Validate HTTP Basic authentication credentials
 *
 * Validates credentials against the runtime user management system only.
 * Legacy config-based authentication has been removed.
 *
 * @param request HTTP request structure
 * @param auth_config Authentication configuration
 * @return HTTP_AUTH_SUCCESS on success, error code on failure
 */
int http_auth_validate_basic(const http_request_t* request, const struct http_auth_config* auth_config);

/**
 * @brief Generate WWW-Authenticate challenge header
 * @param auth_config Authentication configuration
 * @param challenge Buffer to store the challenge header
 * @param challenge_size Size of the challenge buffer
 * @return 0 on success, -1 on error
 */
int http_auth_generate_challenge(const struct http_auth_config* auth_config, char* challenge, size_t challenge_size);

/**
 * @brief Parse Basic authentication credentials from Authorization header
 * @param auth_header Authorization header value
 * @param username Buffer to store username
 * @param password Buffer to store password
 * @return 0 on success, -1 on error
 */
int http_auth_parse_basic_credentials(const char* auth_header, char* username, char* password);

/**
 * @brief Verify Basic authentication credentials against runtime user management
 *
 * Authenticates users exclusively through the runtime user management system.
 * Legacy config-based authentication has been removed.
 *
 * @param username Username to verify
 * @param password Password to verify
 * @return HTTP_AUTH_SUCCESS on success, HTTP_AUTH_UNAUTHENTICATED on invalid credentials,
 *         HTTP_AUTH_ERROR_NULL on null parameters, HTTP_AUTH_ERROR_INVALID on system error
 */
int http_auth_verify_credentials(const char* username, const char* password);

/**
 * @brief Create HTTP 401 Unauthorized response with WWW-Authenticate header
 * @param auth_config Authentication configuration
 * @return HTTP response structure with 401 status and challenge header
 */
http_response_t http_auth_create_401_response(const struct http_auth_config* auth_config);

#endif /* HTTP_AUTH_H */
