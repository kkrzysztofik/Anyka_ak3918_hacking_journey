/**
 * @file http_server_mock.c
 * @brief Mock implementation of HTTP server functions for testing
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <string.h>
#include <strings.h>

#include "core/config/config.h"
#include "networking/http/http_auth.h"
#include "networking/http/http_parser.h"
#include "networking/http/http_server.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Real Function Declarations (for __real_ calls)
 * ============================================================================ */

// Declare real functions that can be called when g_use_real_functions is true
int __real_http_server_start(int port, const struct application_config* config);
int __real_http_server_stop(void);
void __real_process_connection(void* conn);
int __real_http_validate_authentication(const http_request_t* request, security_context_t* security_ctx);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void http_server_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * Global Variables and State
 * ============================================================================ */

/* Global variable for testing - use extern to avoid multiple definition */
extern const struct application_config* g_http_app_config;

/* Mock HTTP server state */
static int g_http_server_mock_state = 0;

/* ============================================================================
 * CMocka Wrapped HTTP Server Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped version of http_server_start
 * @param port TCP port to bind
 * @param config Application configuration
 * @return 0 on success, negative on failure
 */
int __wrap_http_server_start(int port, const struct application_config* config) {
  if (g_use_real_functions) {
    return __real_http_server_start(port, config);
  }

  (void)port;
  g_http_app_config = config;
  g_http_server_mock_state = 1;
  return 0;
}

/**
 * @brief CMocka wrapped version of http_server_stop
 * @return 0 on success, negative on failure
 */
int __wrap_http_server_stop(void) {
  if (g_use_real_functions) {
    return __real_http_server_stop();
  }

  g_http_server_mock_state = 0;
  g_http_app_config = NULL;
  return 0;
}

/**
 * @brief CMocka wrapped version of process_connection
 * @param conn Connection to process
 */
void __wrap_process_connection(void* conn) {
  if (g_use_real_functions) {
    __real_process_connection(conn);
    return;
  }

  (void)conn;
  // Mock implementation - do nothing
}

/**
 * @brief CMocka wrapped version of http_validate_authentication
 * @param request HTTP request structure
 * @param security_ctx Security context for logging
 * @return ONVIF_SUCCESS if authentication succeeds, ONVIF_ERROR if it fails
 */
int __wrap_http_validate_authentication(const http_request_t* request,
                                        security_context_t* security_ctx) {
  if (g_use_real_functions) {
    return __real_http_validate_authentication(request, security_ctx);
  }

  if (!request || !security_ctx) {
    return ONVIF_ERROR_NULL;
  }

  // Check if authentication is enabled in configuration
  if (!g_http_app_config || !g_http_app_config->onvif.auth_enabled) {
    // Authentication is disabled, allow request to proceed
    return ONVIF_SUCCESS;
  }

  // Authentication is enabled, check for Authorization header
  bool has_auth_header = false;
  for (size_t i = 0; i < request->header_count; i++) {
    if (strcasecmp(request->headers[i].name, "Authorization") == 0) {
      has_auth_header = true;
      break;
    }
  }

  if (!has_auth_header) {
    return ONVIF_ERROR;
  }

  // For simplicity in the mock, we'll just check if the header exists
  // In a real implementation, this would validate the credentials
  return ONVIF_SUCCESS;
}

// HTTP metrics functions are defined in test_http_metrics_simple.c
// to avoid multiple definition conflicts
