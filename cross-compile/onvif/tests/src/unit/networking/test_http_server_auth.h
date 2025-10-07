/**
 * @file test_http_server_auth.h
 * @brief Unit tests for HTTP server authentication logic with auth_enabled switch
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_HTTP_SERVER_AUTH_H
#define TEST_HTTP_SERVER_AUTH_H

#include <stddef.h>

#include "cmocka_wrapper.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Test Function Declarations
 * ============================================================================ */

/**
 * @brief Test HTTP server allows requests when authentication is disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_requests(void** state);

/**
 * @brief Test HTTP server allows requests with invalid credentials when auth disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_invalid_credentials(void** state);

/**
 * @brief Test HTTP server allows requests with valid credentials when auth disabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_disabled_allows_valid_credentials(void** state);

/**
 * @brief Test HTTP server rejects requests without auth header when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_rejects_no_header(void** state);

/**
 * @brief Test HTTP server rejects requests with invalid credentials when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_rejects_invalid_credentials(void** state);

/**
 * @brief Test HTTP server accepts requests with valid credentials when auth enabled
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_enabled_accepts_valid_credentials(void** state);

/**
 * @brief Test HTTP server authentication with NULL request parameter
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_request(void** state);

/**
 * @brief Test HTTP server authentication with NULL security context
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_security_context(void** state);

/**
 * @brief Test HTTP server authentication with NULL app config
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_app_config(void** state);

/**
 * @brief Test HTTP server authentication with app config but NULL onvif section
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_null_onvif_config(void** state);

/**
 * @brief Test HTTP server authentication switch behavior
 * @param state Test state (unused)
 */
void test_unit_http_server_auth_switch_behavior(void** state);

/* ============================================================================
 * Test Suite Access
 * ============================================================================ */

/**
 * @brief Get HTTP server auth unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_server_auth_unit_tests(size_t* count);

#ifdef __cplusplus
}
#endif

#endif // TEST_HTTP_SERVER_AUTH_H
