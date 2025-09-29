/**
 * @file http_server.c
 * @brief HTTP server implementation for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#include "networking/http/http_server.h"

#include <arpa/inet.h>
#include <asm/socket.h>
#include <bits/types.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/common/buffer_pool.h"
#include "networking/common/connection_manager.h"
#include "networking/http/http_auth.h"
#include "networking/http/http_constants.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "services/common/onvif_types.h"
#include "services/device/onvif_device.h"
#include "services/imaging/onvif_imaging.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"
#include "services/snapshot/onvif_snapshot.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/security/security_hardening.h"
#include "utils/validation/common_validation.h"
#include "utils/validation/input_validation.h"

/* ============================================================================
 * Global Variables
 * ============================================================================
 */

/** @brief Global HTTP server state */
server_state_t g_http_server; // NOLINT

/** @brief Global application configuration for HTTP server */
static const struct application_config* g_http_app_config = NULL; // NOLINT

/** @brief Global HTTP authentication configuration */
static struct http_auth_config g_http_auth_config = {0}; // NOLINT

/* ============================================================================
 * Chunked Transfer Encoding Constants
 * ============================================================================
 */

/** @brief Chunk size threshold for enabling chunked transfer encoding */
#define CHUNKED_TRANSFER_THRESHOLD 32768

/** @brief Maximum chunk size for streaming (8KB chunks for efficiency) */
#define MAX_CHUNK_SIZE 8192

/** @brief HTTP response header buffer size */
#define HTTP_RESPONSE_BUFFER_SIZE 512

/** @brief Chunk header buffer size for chunked transfer encoding */
#define CHUNK_HEADER_BUFFER_SIZE 16

/** @brief Final chunk message size (including CRLF) */
#define FINAL_CHUNK_SIZE 5

/** @brief Maximum HTTP header line length */
#define HTTP_MAX_HEADER_LINE_LENGTH 8192

/** @brief HTTP status code constants */
#define HTTP_STATUS_OK                    200
#define HTTP_STATUS_INTERNAL_SERVER_ERROR 500
#define HTTP_STATUS_SUCCESS_MIN           200
#define HTTP_STATUS_SUCCESS_MAX           299
#define HTTP_STATUS_CLIENT_ERROR_MIN      400
#define HTTP_STATUS_CLIENT_ERROR_MAX      499
#define HTTP_STATUS_SERVER_ERROR_MIN      500

/* ============================================================================
 * HTTP Logging Helpers
 * ============================================================================
 */

/**
 * @brief Create HTTP logging context for request processing
 * @param context Context to initialize
 * @param client_ip Client IP address (can be NULL)
 * @param operation HTTP operation
 * @param level Log level
 */
static void http_log_init_context(service_log_context_t* context, const char* client_ip,
                                  const char* operation, service_log_level_t level) {
  if (!context) {
    return;
  }

  // Create enhanced action name that includes client IP for better context
  static char enhanced_action[HTTP_ENHANCED_ACTION_MAX_LEN];
  if (client_ip && strlen(client_ip) > 0) {
    (void)snprintf(enhanced_action, sizeof(enhanced_action), "%s [%s]",
                   operation ? operation : "unknown", client_ip);
    service_log_init_context(context, "HTTP", enhanced_action, level);
  } else {
    service_log_init_context(context, "HTTP", operation, level);
  }
}

/**
 * @brief Create HTTP logging context for server operations
 * @param context Context to initialize
 * @param operation Server operation
 * @param level Log level
 */
static void http_server_log_init_context(service_log_context_t* context, const char* operation,
                                         service_log_level_t level) {
  service_log_init_context(context, "HTTP_SERVER", operation, level);
}

/* ============================================================================
 * Centralized Error Handling Helpers
 * ============================================================================
 */

/**
 * @brief Initialize error context with HTTP service information for complex error handling
 * @param error_ctx Error context to initialize
 * @param function_name Function name where error occurred
 * @param error_code Error code
 * @param message Error message format string
 * @param ... Additional arguments for message formatting
 */
static void http_error_context_init(error_context_t* error_ctx, const char* function_name,
                                    int error_code, const char* message, ...) {
  if (!error_ctx || !function_name) {
    return;
  }

  /* Initialize using shared helper to ensure consistency with other services */
  (void)error_context_init(error_ctx, "HTTP", function_name, "HTTP server error");
  error_ctx->error_code = error_code;
  /* Fill debug fields set by the macro previously */
  error_ctx->function = function_name;
  error_ctx->file = __FILE__;
  error_ctx->line = __LINE__;

  if (message) {
    va_list args;
    va_start(args, message);
    ERROR_CONTEXT_SET_MESSAGE(error_ctx, message, args);
    va_end(args);
  }
}

/**
 * @brief Log HTTP error with context and return error code (simple error handling)
 * @param function_name Function name where error occurred
 * @param error_code Error code to return
 * @param message Error message format string
 * @param ... Additional arguments for message formatting
 * @return The error code passed as parameter
 */
static int http_error_log_and_return(const char* function_name, int error_code, const char* message,
                                     ...) {
  error_context_t error_ctx;

  /* Use http_error_context_init for consistent error context setup */
  http_error_context_init(&error_ctx, function_name, error_code, message);

  onvif_log_error_context(&error_ctx);
  return error_code;
}

/* ============================================================================
 * Service Type Detection
 * ============================================================================
 */

/**
 * @brief Determine ONVIF service type from HTTP path
 * @param path HTTP request path
 * @return ONVIF service type
 */
static onvif_service_type_t get_service_type(const char* path) {
  if (!path) {
    return ONVIF_SERVICE_DEVICE;
  }

  if (strstr(path, "/device_service") || strstr(path, "/device")) {
    return ONVIF_SERVICE_DEVICE;
  }
  if (strstr(path, "/media_service") || strstr(path, "/media")) {
    return ONVIF_SERVICE_MEDIA;
  }
  if (strstr(path, "/ptz_service") || strstr(path, "/ptz")) {
    return ONVIF_SERVICE_PTZ;
  }
  if (strstr(path, "/imaging_service") || strstr(path, "/imaging")) {
    return ONVIF_SERVICE_IMAGING;
  }
  if (strstr(path, "/snapshot_service") || strstr(path, "/snapshot")) {
    return ONVIF_SERVICE_SNAPSHOT;
  }

  return ONVIF_SERVICE_DEVICE;
}

/* ============================================================================
 * Operation Name Extraction using gSOAP
 * ============================================================================
 */

/**
 * @brief Extract operation name from SOAP request body using gSOAP
 * @param body SOAP request body content
 * @return Operation name string, or NULL if not found
 */
static const char* extract_operation_name(const char* body) {
  size_t body_length = strlen(body);

  if (!body || body_length == 0) {
    return NULL;
  }

  static char operation_name[HTTP_OPERATION_NAME_BUFFER_SIZE];

  // Use gSOAP-based extraction for robust SOAP parsing
  int result =
    onvif_gsoap_extract_operation_name(body, body_length, operation_name, sizeof(operation_name));

  if (result == ONVIF_XML_SUCCESS) {
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, NULL, "gsoap_extract", SERVICE_LOG_DEBUG);
    service_log_debug(&log_ctx, "gSOAP extracted operation name: %s", operation_name);
    return operation_name;
  }

  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, NULL, "gsoap_extract", SERVICE_LOG_WARNING);
  service_log_warning(&log_ctx, "gSOAP failed to extract operation name, error: %d", result);
  return NULL;
}

/* ============================================================================
 * HTTP Authentication
 * ============================================================================
 */

/**
 * @brief Validate HTTP Basic Authentication using http_auth module
 * @param request HTTP request structure
 * @param security_ctx Security context for logging
 * @return ONVIF_SUCCESS if authentication succeeds, ONVIF_ERROR if it fails
 */
static int http_validate_authentication(const http_request_t* request,
                                        security_context_t* security_ctx) {
  if (!request || !security_ctx) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_NULL,
                                     "HTTP authentication validation failed: null parameters");
  }

  // Ensure auth config is initialized
  if (!g_http_auth_config.enabled) {
    // Initialize auth config on first use
    if (http_auth_init(&g_http_auth_config) != HTTP_AUTH_SUCCESS) {
      return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                       "Failed to initialize HTTP auth configuration");
    }

    // Enable Basic authentication
    g_http_auth_config.enabled = 1;
    g_http_auth_config.auth_type = HTTP_AUTH_BASIC;
  }

  // Validate credentials against configuration
  if (!g_http_app_config) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Application configuration not available for authentication");
  }

  // Use http_auth module for validation
  int auth_result =
    http_auth_validate_basic(request, &g_http_auth_config, g_http_app_config->onvif.username,
                             g_http_app_config->onvif.password);

  if (auth_result != HTTP_AUTH_SUCCESS) {
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, security_ctx->client_ip, "auth_validation",
                          SERVICE_LOG_WARNING);

    // Map http_auth error codes to appropriate logging and security actions
    switch (auth_result) {
    case HTTP_AUTH_ERROR_NO_HEADER:
      service_log_warning(&log_ctx, "No Authorization header in request");
      break;
    case HTTP_AUTH_ERROR_PARSE_FAILED:
      service_log_warning(&log_ctx, "Failed to parse Authorization header");
      break;
    case HTTP_AUTH_UNAUTHENTICATED:
      service_log_warning(&log_ctx, "Invalid credentials provided");
      // Log authentication failure for brute force detection
      security_log_security_event("AUTHENTICATION_FAILURE", security_ctx->client_ip, 3);
      // Update rate limiting to track authentication failures
      security_update_rate_limit(security_ctx->client_ip, security_ctx);
      break;
    default:
      service_log_warning(&log_ctx, "Authentication error: %d", auth_result);
      break;
    }
    return ONVIF_ERROR;
  }

  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, security_ctx->client_ip, "auth_validation", SERVICE_LOG_INFO);
  service_log_operation_success(&log_ctx, "HTTP authentication");
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * ONVIF Request Handling
 * ============================================================================
 */

/**
 * @brief Handle ONVIF service request by operation name
 * @param service_type ONVIF service type
 * @param operation_name Operation name string
 * @param request HTTP request structure
 * @param response HTTP response structure
 * @return ONVIF_SUCCESS if handling succeeds, error code if it fails
 */
static int handle_onvif_request_by_operation(onvif_service_type_t service_type,
                                             const char* operation_name,
                                             const http_request_t* request,
                                             http_response_t* response) {
  if (!operation_name || !request || !response) {
    return ONVIF_ERROR;
  }

  // Route to appropriate service handler based on service type
  switch (service_type) {
  case ONVIF_SERVICE_DEVICE:
    return onvif_device_handle_operation(operation_name, request, response);

  case ONVIF_SERVICE_MEDIA:
    return onvif_media_handle_request(operation_name, request, response);

  case ONVIF_SERVICE_PTZ:
    return onvif_ptz_handle_request(operation_name, request, response);

  case ONVIF_SERVICE_IMAGING:
    return onvif_imaging_handle_operation(operation_name, request, response);

  case ONVIF_SERVICE_SNAPSHOT:
    return onvif_snapshot_handle_request(operation_name, request, response);

  default:
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_NOT_SUPPORTED,
                                     "Unknown ONVIF service type: %d", service_type);
  }
}

/* ============================================================================
 * Chunked Transfer Encoding Implementation
 * ============================================================================
 */

/**
 * @brief Send chunk header with size in hexadecimal format
 * @param chunk_size Size of the chunk in bytes
 * @param client_fd Client socket file descriptor
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int send_chunk_header(size_t chunk_size, int client_fd) { // NOLINT
  char chunk_header[CHUNK_HEADER_BUFFER_SIZE];
  int header_len = snprintf(chunk_header, sizeof(chunk_header), "%zx\r\n", chunk_size);

  if (header_len < 0 || (size_t)header_len >= sizeof(chunk_header)) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Failed to format chunk header for size %zu", chunk_size);
  }

  ssize_t sent = send(client_fd, chunk_header, header_len, 0);
  if (sent < 0 || (size_t)sent != (size_t)header_len) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_IO,
                                     "Failed to send chunk header: %s", strerror(errno));
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Send a data chunk with proper chunked encoding
 * @param client_fd Client socket file descriptor
 * @param data Data to send in the chunk
 * @param size Size of the data
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int send_chunk(int client_fd, const char* data, size_t size) {
  // Send chunk size header
  if (send_chunk_header(size, client_fd) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Send chunk data
  ssize_t sent = send(client_fd, data, size, 0);
  if (sent < 0 || (size_t)sent != size) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_IO, "Failed to send chunk data: %s",
                                     strerror(errno));
  }

  // Send trailing CRLF
  sent = send(client_fd, "\r\n", 2, 0);
  if (sent < 0 || sent != 2) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_IO,
                                     "Failed to send chunk trailing CRLF: %s", strerror(errno));
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Send final chunk to indicate end of chunked response
 * @param client_fd Client socket file descriptor
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int send_final_chunk(int client_fd) {
  // Send zero-length chunk header and final CRLF
  const char* final_chunk = "0\r\n\r\n";
  ssize_t sent = send(client_fd, final_chunk, FINAL_CHUNK_SIZE, 0);
  if (sent < 0 || sent != FINAL_CHUNK_SIZE) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_IO, "Failed to send final chunk: %s",
                                     strerror(errno));
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Get HTTP status text for status code
 * @param status_code HTTP status code
 * @return Status text string
 */
static const char* get_http_status_text(int status_code) {
  switch (status_code) {
  case HTTP_STATUS_OK:
    return "OK";
  case HTTP_STATUS_NOT_FOUND:
    return "Not Found";
  case HTTP_STATUS_UNAUTHORIZED:
    return "Unauthorized";
  case HTTP_STATUS_BAD_REQUEST:
    return "Bad Request";
  default:
    return "Internal Server Error";
  }
}

/**
 * @brief Generate standardized HTTP error response using error handling utilities
 * @param error_ctx Error context containing error details
 * @param http_response HTTP response structure to populate
 * @param client_ip Client IP address for logging context
 * @return ONVIF_SUCCESS if error response generated successfully, error code if it fails
 */
static int generate_http_error_response(const error_context_t* error_ctx,
                                        http_response_t* http_response, const char* client_ip) {
  if (!error_ctx || !http_response) {
    return ONVIF_ERROR_INVALID;
  }

  // Map ONVIF error codes to HTTP status codes
  int http_status_code = HTTP_STATUS_INTERNAL_SERVER_ERROR;
  error_pattern_t pattern = ERROR_PATTERN_INTERNAL_ERROR;

  // Handle special case where HTTP_AUTH_UNAUTHENTICATED conflicts with ONVIF_ERROR_IO
  if (error_ctx->error_code == HTTP_AUTH_UNAUTHENTICATED) {
    http_status_code = HTTP_STATUS_UNAUTHORIZED;
    pattern = ERROR_PATTERN_AUTHENTICATION_FAILED;
  } else {
    switch (error_ctx->error_code) {
    case ONVIF_ERROR_NULL:
    case ONVIF_ERROR_INVALID:
    case ONVIF_ERROR_INVALID_PARAMETER:
      http_status_code = HTTP_STATUS_BAD_REQUEST;
      pattern = ERROR_PATTERN_INVALID_PARAMETER;
      break;
    case ONVIF_ERROR_NOT_FOUND:
      http_status_code = HTTP_STATUS_NOT_FOUND;
      pattern = ERROR_PATTERN_NOT_FOUND;
      break;
    case ONVIF_ERROR_NOT_SUPPORTED:
      http_status_code = HTTP_STATUS_NOT_FOUND;
      pattern = ERROR_PATTERN_NOT_SUPPORTED;
      break;
    case ONVIF_ERROR_AUTHENTICATION_FAILED:
      http_status_code = HTTP_STATUS_UNAUTHORIZED;
      pattern = ERROR_PATTERN_AUTHENTICATION_FAILED;
      break;
    case ONVIF_ERROR_AUTHORIZATION_FAILED:
      http_status_code = HTTP_STATUS_FORBIDDEN;
      pattern = ERROR_PATTERN_AUTHORIZATION_FAILED;
      break;
    case ONVIF_ERROR_MEMORY:
    case ONVIF_ERROR_MEMORY_ALLOCATION:
      http_status_code = HTTP_STATUS_INSUFFICIENT_STORAGE;
      pattern = ERROR_PATTERN_INTERNAL_ERROR;
      break;
    case ONVIF_ERROR_IO:
    case ONVIF_ERROR_NETWORK:
      http_status_code = HTTP_STATUS_SERVICE_UNAVAILABLE;
      pattern = ERROR_PATTERN_INTERNAL_ERROR;
      break;
    case ONVIF_ERROR_TIMEOUT:
      http_status_code = HTTP_STATUS_REQUEST_TIMEOUT;
      pattern = ERROR_PATTERN_INTERNAL_ERROR;
      break;
    default:
      http_status_code = HTTP_STATUS_INTERNAL_SERVER_ERROR;
      pattern = ERROR_PATTERN_INTERNAL_ERROR;
      break;
    }
  }

  // Set HTTP response status code
  http_response->status_code = http_status_code;

  // Create error result for SOAP fault generation
  error_result_t error_result;
  if (error_create_result_from_pattern(pattern, error_ctx->message, &error_result) !=
      ONVIF_SUCCESS) {
    // Fallback to simple error message
    error_result.error_code = error_ctx->error_code;
    error_result.error_message = error_ctx->message;
    error_result.soap_fault_code = SOAP_FAULT_RECEIVER;
    error_result.soap_fault_string = "Internal server error";
  }

  // Generate SOAP fault response body using gSOAP
  {
    // Use gSOAP to generate proper SOAP fault XML
    const char* fault_code =
      error_result.soap_fault_code ? error_result.soap_fault_code : "soap:Server";
    const char* fault_string =
      error_result.soap_fault_string ? error_result.soap_fault_string : "Internal server error";

    // Allocate buffer for the fault XML
    const size_t max_fault_size = 2048;
    char* error_xml = (char*)malloc(max_fault_size);
    if (!error_xml) {
      return ONVIF_ERROR_MEMORY;
    }

    int written = onvif_gsoap_generate_fault_response(NULL, fault_code, fault_string, NULL, NULL,
                                                      error_xml, max_fault_size);

    if (written < 0) {
      free(error_xml);
      return ONVIF_ERROR;
    }

    http_response->body = error_xml;
    http_response->body_length = (size_t)written;
  }

  // Set content type for SOAP error response
  if (http_response->content_type) {
    free(http_response->content_type);
  }
  http_response->content_type = memory_safe_strdup("application/soap+xml; charset=utf-8");
  if (!http_response->content_type) {
    return ONVIF_ERROR_MEMORY;
  }

  // Log the error with full context
  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, client_ip, "error_response", SERVICE_LOG_ERROR);
  service_log_operation_failure(&log_ctx, "HTTP error response", error_ctx->error_code,
                                error_ctx->message);

  return ONVIF_SUCCESS;
}

/**
 * @brief Build HTTP chunked response header
 * @param response HTTP response structure
 * @param header Buffer to store the header
 * @param header_size Size of the header buffer
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int build_chunked_header(const http_response_t* response, char* header, size_t header_size) {
  if (!response || !header) {
    return ONVIF_ERROR;
  }

  const char* status_text = get_http_status_text(response->status_code);
  const char* content_type =
    response->content_type ? response->content_type : "application/soap+xml; charset=utf-8";

  int result = snprintf(header, header_size,
                        "HTTP/1.1 %d %s\r\n"
                        "Content-Type: %s\r\n"
                        "Transfer-Encoding: chunked\r\n"
                        "Connection: close\r\n",
                        response->status_code, status_text, content_type);

  if (result < 0 || (size_t)result >= header_size) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Failed to format chunked HTTP response header");
  }

  // Add custom headers
  for (size_t i = 0; i < response->header_count; i++) {
    int header_result =
      snprintf(header + strlen(header), header_size - strlen(header), "%s: %s\r\n",
               response->headers[i].name, response->headers[i].value);

    if (header_result < 0 || (size_t)header_result >= (header_size - strlen(header))) {
      return http_error_log_and_return(
        __FUNCTION__, ONVIF_ERROR, "Failed to format custom header: %s", response->headers[i].name);
    }
  }

  // Add final CRLF to end headers
  if (strlen(header) + 2 >= header_size) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Response header too large: %zu bytes", strlen(header));
  }
  strcat(header, "\r\n");

  return ONVIF_SUCCESS;
}

/**
 * @brief Send response body in chunks
 * @param client_fd Client socket file descriptor
 * @param response HTTP response structure
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int send_response_body_chunks(int client_fd, const http_response_t* response) {
  if (!response->body || response->body_length == 0) {
    return ONVIF_SUCCESS;
  }

  size_t remaining = response->body_length;
  size_t offset = 0;

  while (remaining > 0) {
    size_t chunk_size = (remaining > MAX_CHUNK_SIZE) ? MAX_CHUNK_SIZE : remaining;

    if (send_chunk(client_fd, response->body + offset, chunk_size) != ONVIF_SUCCESS) {
      service_log_context_t log_ctx;
      http_log_init_context(&log_ctx, NULL, "chunk_send", SERVICE_LOG_ERROR);
      service_log_operation_failure(&log_ctx, "chunk transmission", -1, "Failed to send chunk");
      return ONVIF_ERROR;
    }

    offset += chunk_size;
    remaining -= chunk_size;

    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, NULL, "chunk_progress", SERVICE_LOG_DEBUG);
    service_log_debug(&log_ctx, "Sent chunk: %zu bytes, remaining: %zu bytes", chunk_size,
                      remaining);
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Send HTTP response using chunked transfer encoding for large responses
 * @param client_fd Client socket file descriptor
 * @param response HTTP response structure
 * @param conn Connection structure with buffer for streaming
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int send_chunked_response(int client_fd, const http_response_t* response,
                                 connection_t* conn) {
  if (!response || !conn) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_NULL,
                                     "Invalid parameters for chunked response");
  }

  // Build HTTP header with chunked transfer encoding
  char header[HTTP_RESPONSE_BUFFER_SIZE];
  if (build_chunked_header(response, header, sizeof(header)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Send header
  ssize_t sent = send(client_fd, header, strlen(header), 0);
  if (sent < 0) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_IO,
                                     "Failed to send chunked header: %s", strerror(errno));
  }

  // Send response body in chunks
  if (send_response_body_chunks(client_fd, response) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Send final chunk to indicate end of response
  if (send_final_chunk(client_fd) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, NULL, "chunked_complete", SERVICE_LOG_DEBUG);
  service_log_debug(&log_ctx, "Chunked response completed: %zu total bytes", response->body_length);
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Request Validation Functions
 * ============================================================================
 */

/**
 * @brief Validate HTTP request components (method, path, version)
 * @param request HTTP request structure
 * @param client_ip Client IP address for logging
 * @return ONVIF_SUCCESS if validation succeeds, error code if it fails
 */
static int validate_request_components(const http_request_t* request, const char* client_ip) {
  (void)client_ip; /* client_ip currently unused in validation but kept for future logging */
  if (!request) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_NULL,
                                     "HTTP request validation failed: null request");
  }

  // Validate HTTP method
  validation_result_t method_validation =
    validate_string("HTTP method", request->method, 1, sizeof(request->method) - 1, 0);
  if (!validation_is_valid(&method_validation)) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_INVALID, "Invalid HTTP method: %s",
                                     request->method);
  }

  // Validate HTTP path
  validation_result_t path_validation =
    validate_string("HTTP path", request->path, 1, sizeof(request->path) - 1, 0);
  if (!validation_is_valid(&path_validation)) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_INVALID, "Invalid HTTP path: %s",
                                     request->path);
  }

  // Validate HTTP version
  validation_result_t version_validation =
    validate_string("HTTP version", request->version, 1, sizeof(request->version) - 1, 0);
  if (!validation_is_valid(&version_validation)) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_INVALID, "Invalid HTTP version: %s",
                                     request->version);
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Perform comprehensive security validation
 * @param request HTTP request structure
 * @param security_ctx Security context
 * @return ONVIF_SUCCESS if validation succeeds, error code if it fails
 */
static int perform_security_validation(const http_request_t* request,
                                       security_context_t* security_ctx) {
  if (!request || !security_ctx) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR_NULL,
                                     "Security validation failed: null parameters");
  }

  // Perform comprehensive security validation
  if (security_validate_request(request, security_ctx) != ONVIF_SUCCESS) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Request security validation failed");
  }

  // Check HTTP Basic Authentication with attack detection
  if (http_validate_authentication(request, security_ctx) != ONVIF_SUCCESS) {
    // Log authentication failure as potential brute force attempt
    security_log_security_event("AUTH_FAILURE", security_ctx->client_ip, 2);
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR, "ONVIF Authentication failed");
  }

  // Validate request body for security threats
  if (security_validate_request_body(request, security_ctx) != ONVIF_SUCCESS) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Request body contains potential security threats");
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Main Request Handler
 * ============================================================================
 */

/**
 * @brief Handle ONVIF request with authentication and routing
 * @param request HTTP request structure
 * @param response HTTP response structure
 * @return ONVIF_SUCCESS if handling succeeds, error code if it fails
 */
static int handle_onvif_request(const http_request_t* request, http_response_t* response) {
  if (!request || !response) {
    return ONVIF_ERROR;
  }

  // Initialize security context
  security_context_t security_ctx = {{0}, 0, 0, 0};
  if (memory_safe_strncpy(security_ctx.client_ip, sizeof(security_ctx.client_ip),
                          request->client_ip, strlen(request->client_ip)) < 0) {
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, NULL, "client_ip_copy", SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "client IP copy", -1,
                                  "Failed to copy client IP safely");
    return ONVIF_ERROR;
  }
  security_ctx.last_request_time = security_get_current_time();
  security_ctx.security_level = SECURITY_LEVEL_BASIC;

  // Validate HTTP request using common validation utilities
  if (validate_http_request(request) != ONVIF_VALIDATION_SUCCESS) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR, "HTTP request validation failed");
  }

  // Validate request components (method, path, version)
  if (validate_request_components(request, security_ctx.client_ip) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Perform comprehensive security validation
  if (perform_security_validation(request, &security_ctx) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Determine service type from path
  onvif_service_type_t service_type = get_service_type(request->path);

  // Validate request body content if present
  if (request->body && request->body_length > 0) {
    if (validate_xml_content(request->body, request->body_length) != ONVIF_VALIDATION_SUCCESS) {
      return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                       "Invalid XML content in request body (length: %zu)",
                                       request->body_length);
    }
  }

  // Extract operation name from SOAP body
  const char* operation_name = extract_operation_name(request->body);
  if (!operation_name) {
    return http_error_log_and_return(__FUNCTION__, ONVIF_ERROR,
                                     "Failed to extract operation name from SOAP body");
  }

  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, security_ctx.client_ip, "request_handling", SERVICE_LOG_INFO);
  service_log_info(&log_ctx, "Handling ONVIF request: service=%d, operation=%s", service_type,
                   operation_name);

  // Route to appropriate service handler
  int result = handle_onvif_request_by_operation(service_type, operation_name, request, response);

  // Add security headers to successful responses
  if (result == ONVIF_SUCCESS && response) {
    if (security_add_security_headers(response, &security_ctx) != ONVIF_SUCCESS) {
      service_log_context_t log_ctx;
      http_log_init_context(&log_ctx, security_ctx.client_ip, "security_headers",
                            SERVICE_LOG_WARNING);
      service_log_warning(&log_ctx, "Failed to add security headers to response");
      // Don't fail the request, just log the warning
    }
  } else if (result != ONVIF_SUCCESS) {
    // Generate standardized error response for service handler failure
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, result, "ONVIF service handler failed");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s, Service: %d, Operation: %s",
                              security_ctx.client_ip, service_type, operation_name);

    // Generate error response using utilities
    if (error_handle_service(&error_ctx, result, "ONVIF service handler", response) !=
        ONVIF_SUCCESS) {
      service_log_context_t log_ctx;
      http_log_init_context(&log_ctx, security_ctx.client_ip, "error_handling", SERVICE_LOG_ERROR);
      service_log_warning(&log_ctx,
                          "Failed to generate error response for service handler failure");
    }

    service_log_context_t error_log_ctx;
    http_log_init_context(&error_log_ctx, security_ctx.client_ip, "service_handler",
                          SERVICE_LOG_ERROR);
    service_log_operation_failure(&error_log_ctx, "ONVIF service handler", result,
                                  "Service handler returned error");
  }

  return result;
}

/* ============================================================================
 * HTTP Server Implementation
 * ============================================================================
 */

/**
 * @brief Initialize HTTP server
 * @param port Port number to listen on
 * @return ONVIF_SUCCESS if initialization succeeds, error code if it fails
 */
int http_server_init(int port) {
  // Initialize server state
  memset(&g_http_server, 0, sizeof(server_state_t));
  g_http_server.socket = -1;

  if (g_http_server.running) {
    service_log_context_t log_ctx;
    http_server_log_init_context(&log_ctx, "server_init", SERVICE_LOG_WARNING);
    service_log_warning(&log_ctx, "HTTP server already initialized");
    return ONVIF_SUCCESS;
  }

  // Initialize buffer pool
  if (buffer_pool_init(&g_http_server.buffer_pool) != 0) {
    service_log_context_t log_ctx;
    http_server_log_init_context(&log_ctx, "buffer_pool_init", SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "buffer pool initialization", -1,
                                  "Failed to initialize buffer pool");
    return ONVIF_ERROR;
  }

  // Initialize performance metrics
  if (http_metrics_init() != ONVIF_SUCCESS) {
    service_log_context_t log_ctx;
    http_server_log_init_context(&log_ctx, "metrics_init", SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "metrics initialization", -1,
                                  "Failed to initialize performance metrics");
    buffer_pool_cleanup(&g_http_server.buffer_pool);
    return ONVIF_ERROR;
  }

  // Create socket
  g_http_server.socket = socket(AF_INET, SOCK_STREAM, 0);
  if (g_http_server.socket < 0) {
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, ONVIF_ERROR, "Socket creation failed");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Error: %s (errno: %d)", strerror(errno), errno);
    onvif_log_error_context(&error_ctx);
    return ONVIF_ERROR;
  }

  // Set socket options
  int opt = 1;
  if (setsockopt(g_http_server.socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, ONVIF_ERROR, "Socket options setting failed");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Error: %s (errno: %d)", strerror(errno), errno);
    onvif_log_error_context(&error_ctx);
    close(g_http_server.socket);
    g_http_server.socket = -1;
    return ONVIF_ERROR;
  }

  // Bind socket
  struct sockaddr_in server_addr = {0};
  server_addr.sin_family = AF_INET;
  server_addr.sin_addr.s_addr = INADDR_ANY;
  server_addr.sin_port = htons(port);

  if (bind(g_http_server.socket, (struct sockaddr*)&server_addr, sizeof(server_addr)) < 0) {
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, ONVIF_ERROR, "Socket binding failed");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Port: %d, Error: %s (errno: %d)", port, strerror(errno),
                              errno);
    onvif_log_error_context(&error_ctx);
    close(g_http_server.socket);
    g_http_server.socket = -1;
    return ONVIF_ERROR;
  }

  // Listen for connections
  if (listen(g_http_server.socket, HTTP_SOCKET_BACKLOG_SIZE) < 0) {
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, ONVIF_ERROR, "Socket listening failed");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Backlog: %d, Error: %s (errno: %d)",
                              HTTP_SOCKET_BACKLOG_SIZE, strerror(errno), errno);
    onvif_log_error_context(&error_ctx);
    close(g_http_server.socket);
    g_http_server.socket = -1;
    return ONVIF_ERROR;
  }

  g_http_server.running = 1;

  service_log_context_t log_ctx;
  http_server_log_init_context(&log_ctx, "server_init", SERVICE_LOG_INFO);
  service_log_operation_success(&log_ctx, "HTTP server initialization");
  service_log_info(&log_ctx, "HTTP server initialized on port %d", port);
  return ONVIF_SUCCESS;
}

/**
 * @brief Get client IP address from socket
 * @param client_fd Client socket file descriptor
 * @param client_ip_str Buffer to store client IP string
 * @param buffer_size Size of the client IP buffer
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int get_client_ip_address(int client_fd, char* client_ip_str, size_t buffer_size) {
  struct sockaddr_in client_addr;
  socklen_t client_len = sizeof(client_addr);

  if (getpeername(client_fd, (struct sockaddr*)&client_addr, &client_len) == 0) {
    inet_ntop(AF_INET, &client_addr.sin_addr, client_ip_str, buffer_size);
  } else {
    strncpy(client_ip_str, "unknown", buffer_size - 1);
    client_ip_str[buffer_size - 1] = '\0';
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Read HTTP request from client socket
 * @param client_fd Client socket file descriptor
 * @param buffer Buffer to store request data
 * @param client_ip_str Client IP string for logging
 * @return Number of bytes read on success, -1 on failure
 */
static ssize_t read_http_request(int client_fd, char* buffer, const char* client_ip_str) {
  ssize_t bytes_read = read(client_fd, buffer, BUFFER_SIZE - 1);
  if (bytes_read <= 0) {
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, client_ip_str, "socket_read", SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "socket read", errno,
                                  "Failed to read from client socket");
    return -1;
  }

  buffer[bytes_read] = '\0';
  return bytes_read;
}

/**
 * @brief Parse HTTP request and validate
 * @param buffer Request buffer
 * @param bytes_read Number of bytes read
 * @param request HTTP request structure to populate
 * @param client_ip_str Client IP string for logging
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int parse_and_validate_request(char* buffer, ssize_t bytes_read, http_request_t* request,
                                      const char* client_ip_str) {
  // Set client IP in request using safe string function
  if (memory_safe_strncpy(request->client_ip, sizeof(request->client_ip), client_ip_str,
                          strlen(client_ip_str)) < 0) {
    error_context_t error_ctx;
    http_error_context_init(&error_ctx, __FUNCTION__, ONVIF_ERROR,
                            "Failed to copy client IP safely");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s", client_ip_str);
    onvif_log_error_context(&error_ctx);
    return ONVIF_ERROR;
  }

  int need_more_data = 0;
  if (parse_http_request_state_machine(buffer, bytes_read, request, &need_more_data) != 0) {
    error_context_t error_ctx;
    error_context_init(&error_ctx, "HTTP", __FUNCTION__, "HTTP server error");
    ERROR_CONTEXT_SET_MESSAGE(&error_ctx, "Failed to parse HTTP request");
    ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s, Bytes read: %d", client_ip_str,
                              bytes_read);
    onvif_log_error_context(&error_ctx);
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Send HTTP response (chunked or regular)
 * @param client_fd Client socket file descriptor
 * @param response HTTP response to send
 * @param client_ip_str Client IP string for logging
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int send_http_response_with_fallback(int client_fd, http_response_t* response,
                                            const char* client_ip_str) {
  int result = ONVIF_SUCCESS;

  // Check if we should use chunked transfer encoding
  if (response->body_length > CHUNKED_TRANSFER_THRESHOLD) {
    // Create a connection structure for chunked streaming
    connection_t conn = {0};
    conn.fd = client_fd;

    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, client_ip_str, "chunked_transfer", SERVICE_LOG_DEBUG);
    service_log_debug(&log_ctx, "Using chunked transfer for large response: %zu bytes",
                      response->body_length);

    result = send_chunked_response(client_fd, response, &conn);
    if (result != ONVIF_SUCCESS) {
      // Generate standardized error response for chunked transfer failure
      error_context_t error_ctx;
      error_context_init(&error_ctx, "HTTP", __FUNCTION__, "HTTP server error");
      error_ctx.error_code = ONVIF_ERROR_IO;
      ERROR_CONTEXT_SET_MESSAGE(&error_ctx, "Failed to send chunked response");
      ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s, Response size: %zu", client_ip_str,
                                response->body_length);

      // Create error response
      http_response_t error_response = {0};
      if (generate_http_error_response(&error_ctx, &error_response, client_ip_str) ==
          ONVIF_SUCCESS) {
        send_http_response(client_fd, &error_response);
        http_response_free(&error_response);
      }

      service_log_context_t error_log_ctx;
      http_log_init_context(&error_log_ctx, client_ip_str, "chunked_send", SERVICE_LOG_ERROR);
      service_log_operation_failure(&error_log_ctx, "chunked response transmission", result,
                                    "Failed to send chunked response");
    }
  } else {
    // Use regular HTTP response for smaller responses
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, client_ip_str, "regular_response", SERVICE_LOG_DEBUG);
    service_log_debug(&log_ctx, "Using regular HTTP response: %zu bytes", response->body_length);

    result = send_http_response(client_fd, response);
    if (result != 0) {
      // Generate standardized error response for regular response failure
      error_context_t error_ctx;
      error_context_init(&error_ctx, "HTTP", __FUNCTION__, "HTTP server error");
      error_ctx.error_code = ONVIF_ERROR_IO;
      ERROR_CONTEXT_SET_MESSAGE(&error_ctx, "Failed to send HTTP response");
      ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s, Response size: %zu", client_ip_str,
                                response->body_length);

      // Create error response
      http_response_t error_response = {0};
      if (generate_http_error_response(&error_ctx, &error_response, client_ip_str) ==
          ONVIF_SUCCESS) {
        send_http_response(client_fd, &error_response);
        http_response_free(&error_response);
      }

      service_log_context_t log_ctx;
      http_log_init_context(&log_ctx, client_ip_str, "response_send", SERVICE_LOG_ERROR);
      service_log_operation_failure(&log_ctx, "HTTP response transmission", result,
                                    "Failed to send HTTP response");
    }
  }

  return result;
}

/**
 * @brief Handle request processing error
 * @param client_fd Client socket file descriptor
 * @param client_ip_str Client IP string for logging
 * @param result Error result code
 */
static void handle_request_error(int client_fd, const char* client_ip_str, int result) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "HTTP", __FUNCTION__, "HTTP server error");
  error_ctx.error_code = result;
  ERROR_CONTEXT_SET_MESSAGE(&error_ctx, "HTTP request processing failed");
  ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Client IP: %s, Result: %d", client_ip_str, result);

  // Create error response
  http_response_t error_response = {0};
  if (generate_http_error_response(&error_ctx, &error_response, client_ip_str) == ONVIF_SUCCESS) {
    send_http_response(client_fd, &error_response);
    http_response_free(&error_response);
  }

  service_log_context_t error_log_ctx;
  http_log_init_context(&error_log_ctx, client_ip_str, "request_processing", SERVICE_LOG_ERROR);
  service_log_operation_failure(&error_log_ctx, "HTTP request processing", result,
                                "Failed to process HTTP request");
}

/**
 * @brief Process HTTP request
 * @param client_fd Client socket file descriptor
 * @return ONVIF_SUCCESS if processing succeeds, error code if it fails
 */
int http_server_process_request(int client_fd) {
  if (client_fd < 0) {
    return ONVIF_ERROR;
  }

  // Record request start time for latency measurement
  uint64_t request_start_time = platform_get_time_ms();

  // Get client IP address
  char client_ip_str[HTTP_CLIENT_IP_BUFFER_SIZE] = "unknown";
  if (get_client_ip_address(client_fd, client_ip_str, sizeof(client_ip_str)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Get buffer from pool
  char* buffer = buffer_pool_get(&g_http_server.buffer_pool);
  if (!buffer) {
    service_log_context_t log_ctx;
    http_log_init_context(&log_ctx, client_ip_str, "buffer_pool", SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "buffer pool acquisition", -1,
                                  "No available buffers in pool");
    return ONVIF_ERROR;
  }

  // Read request
  ssize_t bytes_read = read_http_request(client_fd, buffer, client_ip_str);
  if (bytes_read < 0) {
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return ONVIF_ERROR;
  }

  // Parse HTTP request
  http_request_t request = {0};
  http_response_t response = {0};

  if (parse_and_validate_request(buffer, bytes_read, &request, client_ip_str) != ONVIF_SUCCESS) {
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return ONVIF_ERROR;
  }

  // Handle ONVIF request
  int result = handle_onvif_request(&request, &response);

  // Send response
  if (result == ONVIF_SUCCESS) {
    result = send_http_response_with_fallback(client_fd, &response, client_ip_str);
  } else {
    handle_request_error(client_fd, client_ip_str, result);
  }

  // Record performance metrics
  uint64_t request_end_time = platform_get_time_ms();
  uint64_t latency_ms = request_end_time - request_start_time;
  size_t response_size = (result == ONVIF_SUCCESS) ? response.body_length : 0;
  int status_code = (result == ONVIF_SUCCESS) ? HTTP_STATUS_OK : HTTP_STATUS_INTERNAL_SERVER_ERROR;

  http_metrics_record_request(latency_ms, response_size, status_code);

  // Cleanup response
  http_response_free(&response);

  // Cleanup
  buffer_pool_return(&g_http_server.buffer_pool, buffer);
  close(client_fd);

  return result;
}

/**
 * @brief Start HTTP server main loop
 * @param port Port number to listen on
 * @param config Application configuration (can be NULL for defaults)
 * @return ONVIF_SUCCESS if server starts successfully, error code if it fails
 */
int http_server_start(int port, const struct application_config* config) {
  // Store the configuration for later use
  g_http_app_config = config;

  // Initialize server if not already done
  if (!g_http_server.running) {
    int result = http_server_init(port);
    if (result != ONVIF_SUCCESS) {
      return result;
    }
  }

  service_log_context_t log_ctx;
  http_server_log_init_context(&log_ctx, "server_start", SERVICE_LOG_INFO);
  service_log_info(&log_ctx, "Starting HTTP server on port %d", port);

  while (1) {
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);

    int client_fd = accept(g_http_server.socket, (struct sockaddr*)&client_addr, &client_len);
    if (client_fd < 0) {
      error_context_t error_ctx;
      error_context_init(&error_ctx, "HTTP", __FUNCTION__, "HTTP server error");
      error_ctx.error_code = ONVIF_ERROR;
      ERROR_CONTEXT_SET_MESSAGE(&error_ctx, "Connection acceptance failed");
      ERROR_CONTEXT_SET_CONTEXT(&error_ctx, "Error: %s (errno: %d)", strerror(errno), errno);
      onvif_log_error_context(&error_ctx);
      continue;
    }

    // Store client IP for logging
    char client_ip_str[HTTP_CLIENT_IP_BUFFER_SIZE];
    inet_ntop(AF_INET, &client_addr.sin_addr, client_ip_str, HTTP_CLIENT_IP_BUFFER_SIZE);

    // Process request in a simple synchronous manner
    // In a production system, this would be handled by a thread pool
    http_server_process_request(client_fd);
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Stop HTTP server
 * @return ONVIF_SUCCESS if server stops successfully, error code if it fails
 */
int http_server_stop(void) {
  if (!g_http_server.running) {
    return ONVIF_SUCCESS;
  }

  if (g_http_server.socket >= 0) {
    close(g_http_server.socket);
    g_http_server.socket = -1;
  }

  buffer_pool_cleanup(&g_http_server.buffer_pool);

  // Cleanup performance metrics
  http_metrics_cleanup();

  g_http_server.running = 0;

  service_log_context_t log_ctx;
  http_server_log_init_context(&log_ctx, "server_stop", SERVICE_LOG_INFO);
  service_log_operation_success(&log_ctx, "HTTP server shutdown");
  service_log_info(&log_ctx, "HTTP server stopped");
  return ONVIF_SUCCESS;
}

/**
 * @brief Process a single connection (used by thread pool)
 * @param conn Connection to process
 */
void process_connection(void* conn) {
  // This is a placeholder implementation
  // In a real implementation, this would process the connection
  // For now, we'll just log that it was called
  service_log_context_t log_ctx;
  http_log_init_context(&log_ctx, NULL, "process_connection", SERVICE_LOG_DEBUG);
  service_log_debug(&log_ctx, "process_connection called with connection %p", conn);
}

/* ============================================================================
 * HTTP Performance Metrics Implementation
 * ============================================================================
 */

/**
 * @brief Initialize HTTP performance metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_init(void) {
  // Initialize counters
  g_http_server.metrics.total_requests = 0;
  g_http_server.metrics.successful_requests = 0;
  g_http_server.metrics.client_errors = 0;
  g_http_server.metrics.server_errors = 0;
  g_http_server.metrics.total_response_bytes = 0;
  g_http_server.metrics.total_latency_ms = 0;
  g_http_server.metrics.min_latency_ms = UINT64_MAX;
  g_http_server.metrics.max_latency_ms = 0;
  g_http_server.metrics.current_connections = 0;

  // Initialize mutex
  if (pthread_mutex_init(&g_http_server.metrics.metrics_mutex, NULL) != 0) {
    return ONVIF_ERROR;
  }

  // Set start time
  g_http_server.metrics.metrics_start_time = platform_get_time_ms();

  return ONVIF_SUCCESS;
}

/**
 * @brief Cleanup HTTP performance metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_cleanup(void) {
  // Destroy mutex
  if (pthread_mutex_destroy(&g_http_server.metrics.metrics_mutex) != 0) {
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Get current HTTP performance metrics
 * @param metrics Output structure to fill with current metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_get_current(http_performance_metrics_t* metrics) {
  if (!metrics) {
    return ONVIF_ERROR_NULL;
  }

  // Copy values with mutex protection
  pthread_mutex_lock(&g_http_server.metrics.metrics_mutex);

  metrics->total_requests = g_http_server.metrics.total_requests;
  metrics->successful_requests = g_http_server.metrics.successful_requests;
  metrics->client_errors = g_http_server.metrics.client_errors;
  metrics->server_errors = g_http_server.metrics.server_errors;
  metrics->total_response_bytes = g_http_server.metrics.total_response_bytes;
  metrics->total_latency_ms = g_http_server.metrics.total_latency_ms;
  metrics->min_latency_ms = g_http_server.metrics.min_latency_ms;
  metrics->max_latency_ms = g_http_server.metrics.max_latency_ms;
  metrics->current_connections = g_http_server.metrics.current_connections;
  metrics->metrics_start_time = g_http_server.metrics.metrics_start_time;

  pthread_mutex_unlock(&g_http_server.metrics.metrics_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Record HTTP request metrics
 * @param latency_ms Request latency in milliseconds
 * @param response_size Response size in bytes
 * @param status_code HTTP status code
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_record_request(uint64_t latency_ms, size_t response_size,
                                int status_code) { // NOLINT
  // Update all metrics with mutex protection for thread safety
  pthread_mutex_lock(&g_http_server.metrics.metrics_mutex);

  // Increment total requests
  g_http_server.metrics.total_requests++;

  // Add response bytes
  g_http_server.metrics.total_response_bytes += response_size;

  // Add latency
  g_http_server.metrics.total_latency_ms += latency_ms;

  // Update min/max latency
  if (latency_ms < g_http_server.metrics.min_latency_ms) {
    g_http_server.metrics.min_latency_ms = latency_ms;
  }

  if (latency_ms > g_http_server.metrics.max_latency_ms) {
    g_http_server.metrics.max_latency_ms = latency_ms;
  }

  // Categorize by status code
  if (status_code >= HTTP_STATUS_SUCCESS_MIN && status_code <= HTTP_STATUS_SUCCESS_MAX) {
    g_http_server.metrics.successful_requests++;
  } else if (status_code >= HTTP_STATUS_CLIENT_ERROR_MIN &&
             status_code <= HTTP_STATUS_CLIENT_ERROR_MAX) {
    g_http_server.metrics.client_errors++;
  } else if (status_code >= HTTP_STATUS_SERVER_ERROR_MIN) {
    g_http_server.metrics.server_errors++;
  }

  pthread_mutex_unlock(&g_http_server.metrics.metrics_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Update current connection count
 * @param delta Change in connection count (+1 for new, -1 for closed)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_update_connections(int delta) {
  pthread_mutex_lock(&g_http_server.metrics.metrics_mutex);
  g_http_server.metrics.current_connections += delta;
  pthread_mutex_unlock(&g_http_server.metrics.metrics_mutex);
  return ONVIF_SUCCESS;
}

/**
 * @brief Cleanup HTTP server resources
 * @return ONVIF_SUCCESS if cleanup succeeds, error code if it fails
 */
int http_server_cleanup(void) {
  return http_server_stop();
}
