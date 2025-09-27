/**
 * @file http_server.c
 * @brief HTTP server implementation for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#include "networking/http/http_server.h"

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
#include "utils/memory/memory_manager.h"
#include "utils/security/security_hardening.h"
#include "utils/validation/common_validation.h"
#include "utils/validation/input_validation.h"

#include <arpa/inet.h>
#include <asm/socket.h>
#include <bits/types.h>
#include <errno.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

/* ============================================================================
 * Global Variables
 * ============================================================================
 */

/** @brief Global HTTP server state */
server_state_t g_http_server = {0, -1, 0, {0}, {0}, 0, 0}; // NOLINT

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
    platform_log_debug("gSOAP extracted operation name: %s\n", operation_name);
    return operation_name;
  }

  platform_log_warning("gSOAP failed to extract operation name, error: %d\n", result);
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
    return ONVIF_ERROR;
  }

  // Ensure auth config is initialized
  if (!g_http_auth_config.enabled) {
    // Initialize auth config on first use
    if (http_auth_init(&g_http_auth_config) != HTTP_AUTH_SUCCESS) {
      platform_log_error("Failed to initialize HTTP auth configuration\n");
      return ONVIF_ERROR;
    }

    // Enable Basic authentication
    g_http_auth_config.enabled = 1;
    g_http_auth_config.auth_type = HTTP_AUTH_BASIC;
  }

  // Validate credentials against configuration
  if (!g_http_app_config) {
    platform_log_error("Application configuration not available for authentication\n");
    return ONVIF_ERROR;
  }

  // Use http_auth module for validation
  int auth_result =
    http_auth_validate_basic(request, &g_http_auth_config, g_http_app_config->onvif.username,
                             g_http_app_config->onvif.password);

  if (auth_result != HTTP_AUTH_SUCCESS) {
    // Map http_auth error codes to appropriate logging and security actions
    switch (auth_result) {
    case HTTP_AUTH_ERROR_NO_HEADER:
      platform_log_warning("No Authorization header in request from %s\n", security_ctx->client_ip);
      break;
    case HTTP_AUTH_ERROR_PARSE_FAILED:
      platform_log_warning("Failed to parse Authorization header from %s\n",
                           security_ctx->client_ip);
      break;
    case HTTP_AUTH_UNAUTHENTICATED:
      platform_log_warning("Invalid credentials from %s\n", security_ctx->client_ip);
      // Log authentication failure for brute force detection
      security_log_security_event("AUTHENTICATION_FAILURE", security_ctx->client_ip, 3);
      // Update rate limiting to track authentication failures
      security_update_rate_limit(security_ctx->client_ip, security_ctx);
      break;
    default:
      platform_log_warning("Authentication error from %s: %d\n", security_ctx->client_ip,
                           auth_result);
      break;
    }
    return ONVIF_ERROR;
  }

  platform_log_info("Authentication successful from %s\n", security_ctx->client_ip);
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
    return onvif_imaging_handle_request(operation_name, request, response);

  case ONVIF_SERVICE_SNAPSHOT:
    return onvif_snapshot_handle_request(operation_name, request, response);

  default:
    platform_log_error("Unknown service type: %d\n", service_type);
    return ONVIF_ERROR;
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
    platform_log_error("Failed to format chunk header\n");
    return ONVIF_ERROR;
  }

  ssize_t sent = send(client_fd, chunk_header, header_len, 0);
  if (sent < 0 || (size_t)sent != (size_t)header_len) {
    platform_log_error("Failed to send chunk header: %s\n", strerror(errno));
    return ONVIF_ERROR;
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
    platform_log_error("Failed to send chunk data: %s\n", strerror(errno));
    return ONVIF_ERROR;
  }

  // Send trailing CRLF
  sent = send(client_fd, "\r\n", 2, 0);
  if (sent < 0 || sent != 2) {
    platform_log_error("Failed to send chunk trailing CRLF: %s\n", strerror(errno));
    return ONVIF_ERROR;
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
    platform_log_error("Failed to send final chunk: %s\n", strerror(errno));
    return ONVIF_ERROR;
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
    platform_log_error("Failed to format chunked HTTP response header\n");
    return ONVIF_ERROR;
  }

  // Add custom headers
  for (size_t i = 0; i < response->header_count; i++) {
    int header_result =
      snprintf(header + strlen(header), header_size - strlen(header), "%s: %s\r\n",
               response->headers[i].name, response->headers[i].value);

    if (header_result < 0 || (size_t)header_result >= (header_size - strlen(header))) {
      platform_log_error("Failed to format custom header\n");
      return ONVIF_ERROR;
    }
  }

  // Add final CRLF to end headers
  if (strlen(header) + 2 >= header_size) {
    platform_log_error("Response header too large\n");
    return ONVIF_ERROR;
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
      platform_log_error("Failed to send chunk at offset %zu\n", offset);
      return ONVIF_ERROR;
    }

    offset += chunk_size;
    remaining -= chunk_size;

    platform_log_debug("Sent chunk: %zu bytes, remaining: %zu bytes\n", chunk_size, remaining);
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
    platform_log_error("Invalid parameters for chunked response\n");
    return ONVIF_ERROR;
  }

  // Build HTTP header with chunked transfer encoding
  char header[HTTP_RESPONSE_BUFFER_SIZE];
  if (build_chunked_header(response, header, sizeof(header)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Send header
  ssize_t sent = send(client_fd, header, strlen(header), 0);
  if (sent < 0) {
    platform_log_error("Failed to send chunked header: %s\n", strerror(errno));
    return ONVIF_ERROR;
  }

  // Send response body in chunks
  if (send_response_body_chunks(client_fd, response) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Send final chunk to indicate end of response
  if (send_final_chunk(client_fd) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  platform_log_debug("Chunked response completed: %zu total bytes\n", response->body_length);
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
  if (!request) {
    return ONVIF_ERROR_NULL;
  }

  // Validate HTTP method
  validation_result_t method_validation =
    validate_string("HTTP method", request->method, 1, sizeof(request->method) - 1, 0);
  if (!validation_is_valid(&method_validation)) {
    platform_log_error("Invalid HTTP method from %s: %s\n", client_ip,
                       validation_get_error_message(&method_validation));
    return ONVIF_ERROR_INVALID;
  }

  // Validate HTTP path
  validation_result_t path_validation =
    validate_string("HTTP path", request->path, 1, sizeof(request->path) - 1, 0);
  if (!validation_is_valid(&path_validation)) {
    platform_log_error("Invalid HTTP path from %s: %s\n", client_ip,
                       validation_get_error_message(&path_validation));
    return ONVIF_ERROR_INVALID;
  }

  // Validate HTTP version
  validation_result_t version_validation =
    validate_string("HTTP version", request->version, 1, sizeof(request->version) - 1, 0);
  if (!validation_is_valid(&version_validation)) {
    platform_log_error("Invalid HTTP version from %s: %s\n", client_ip,
                       validation_get_error_message(&version_validation));
    return ONVIF_ERROR_INVALID;
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
    return ONVIF_ERROR_NULL;
  }

  // Perform comprehensive security validation
  if (security_validate_request(request, security_ctx) != ONVIF_SUCCESS) {
    platform_log_error("Request security validation failed for client %s\n",
                       security_ctx->client_ip);
    return ONVIF_ERROR;
  }

  // Check HTTP Basic Authentication with attack detection
  if (http_validate_authentication(request, security_ctx) != ONVIF_SUCCESS) {
    platform_log_error("ONVIF Authentication failed for request from %s\n",
                       security_ctx->client_ip);
    // Log authentication failure as potential brute force attempt
    security_log_security_event("AUTH_FAILURE", security_ctx->client_ip, 2);
    return ONVIF_ERROR;
  }

  // Validate request body for security threats
  if (security_validate_request_body(request, security_ctx) != ONVIF_SUCCESS) {
    platform_log_error("Request body security validation failed for client %s\n",
                       security_ctx->client_ip);
    return ONVIF_ERROR;
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
    platform_log_error("Failed to copy client IP safely\n");
    return ONVIF_ERROR;
  }
  security_ctx.last_request_time = security_get_current_time();
  security_ctx.security_level = SECURITY_LEVEL_BASIC;

  // Validate HTTP request using common validation utilities
  if (validate_http_request(request) != ONVIF_VALIDATION_SUCCESS) {
    platform_log_error("HTTP request validation failed for client %s\n", security_ctx.client_ip);
    return ONVIF_ERROR;
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
      platform_log_error("Invalid XML content in request body from %s\n", security_ctx.client_ip);
      return ONVIF_ERROR;
    }
  }

  // Extract operation name from SOAP body
  const char* operation_name = extract_operation_name(request->body);
  if (!operation_name) {
    platform_log_error("Could not extract operation name from request body\n");
    return ONVIF_ERROR;
  }

  platform_log_info("Handling ONVIF request: service=%d, operation=%s\n", service_type,
                    operation_name);

  // Route to appropriate service handler
  int result = handle_onvif_request_by_operation(service_type, operation_name, request, response);

  // Add security headers to successful responses
  if (result == ONVIF_SUCCESS && response) {
    if (security_add_security_headers(response, &security_ctx) != ONVIF_SUCCESS) {
      platform_log_warning("Failed to add security headers to response\n");
      // Don't fail the request, just log the warning
    }
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
  if (g_http_server.running) {
    platform_log_warning("HTTP server already initialized\n");
    return ONVIF_SUCCESS;
  }

  // Initialize buffer pool
  if (buffer_pool_init(&g_http_server.buffer_pool) != 0) {
    platform_log_error("Failed to initialize buffer pool\n");
    return ONVIF_ERROR;
  }

  // Create socket
  g_http_server.socket = socket(AF_INET, SOCK_STREAM, 0);
  if (g_http_server.socket < 0) {
    platform_log_error("Failed to create socket: %s\n", strerror(errno));
    return ONVIF_ERROR;
  }

  // Set socket options
  int opt = 1;
  if (setsockopt(g_http_server.socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
    platform_log_error("Failed to set socket options: %s\n", strerror(errno));
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
    platform_log_error("Failed to bind socket: %s\n", strerror(errno));
    close(g_http_server.socket);
    g_http_server.socket = -1;
    return ONVIF_ERROR;
  }

  // Listen for connections
  if (listen(g_http_server.socket, HTTP_SOCKET_BACKLOG_SIZE) < 0) {
    platform_log_error("Failed to listen on socket: %s\n", strerror(errno));
    close(g_http_server.socket);
    g_http_server.socket = -1;
    return ONVIF_ERROR;
  }

  g_http_server.running = 1;
  g_http_server.running = 1;

  platform_log_info("HTTP server initialized on port %d\n", port);
  return ONVIF_SUCCESS;
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

  // Get client IP address
  struct sockaddr_in client_addr;
  socklen_t client_len = sizeof(client_addr);
  char client_ip_str[HTTP_CLIENT_IP_BUFFER_SIZE] = "unknown";

  if (getpeername(client_fd, (struct sockaddr*)&client_addr, &client_len) == 0) {
    inet_ntop(AF_INET, &client_addr.sin_addr, client_ip_str, HTTP_CLIENT_IP_BUFFER_SIZE);
  }

  // Get buffer from pool
  char* buffer = buffer_pool_get(&g_http_server.buffer_pool);
  if (!buffer) {
    platform_log_error("No available buffers in pool\n");
    return ONVIF_ERROR;
  }

  // Read request
  ssize_t bytes_read = read(client_fd, buffer, BUFFER_SIZE - 1);
  if (bytes_read <= 0) {
    platform_log_error("Failed to read from client socket\n");
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return ONVIF_ERROR;
  }

  buffer[bytes_read] = '\0';

  // Parse HTTP request
  http_request_t request = {0};
  http_response_t response = {0};

  // Set client IP in request using safe string function
  if (memory_safe_strncpy(request.client_ip, sizeof(request.client_ip), client_ip_str,
                          strlen(client_ip_str)) < 0) {
    platform_log_error("Failed to copy client IP safely\n");
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return ONVIF_ERROR;
  }

  int need_more_data = 0;
  if (parse_http_request_state_machine(buffer, bytes_read, &request, &need_more_data) != 0) {
    platform_log_error("Failed to parse HTTP request\n");
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return ONVIF_ERROR;
  }

  // Handle ONVIF request
  int result = handle_onvif_request(&request, &response);

  // Send response with chunked streaming for large responses
  if (result == ONVIF_SUCCESS) {
    // Check if we should use chunked transfer encoding
    if (response.body_length > CHUNKED_TRANSFER_THRESHOLD) {
      // Create a connection structure for chunked streaming
      connection_t conn = {0};
      conn.fd = client_fd;

      platform_log_debug("Using chunked transfer for large response: %zu bytes\n",
                         response.body_length);

      result = send_chunked_response(client_fd, &response, &conn);
      if (result != ONVIF_SUCCESS) {
        platform_log_error("Failed to send chunked response\n");
      }
    } else {
      // Use regular HTTP response for smaller responses
      platform_log_debug("Using regular HTTP response: %zu bytes\n", response.body_length);

      result = send_http_response(client_fd, &response);
      if (result != 0) {
        platform_log_error("Failed to send HTTP response\n");
      }
    }
  }

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

  platform_log_info("Starting HTTP server on port %d\n", port);

  while (1) {
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);

    int client_fd = accept(g_http_server.socket, (struct sockaddr*)&client_addr, &client_len);
    if (client_fd < 0) {
      platform_log_error("Failed to accept connection: %s\n", strerror(errno));
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
  g_http_server.running = 0;

  platform_log_info("HTTP server stopped\n");
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
  platform_log_debug("process_connection called with connection %p\n", conn);
}

/**
 * @brief Cleanup HTTP server resources
 * @return ONVIF_SUCCESS if cleanup succeeds, error code if it fails
 */
int http_server_cleanup(void) {
  return http_server_stop();
}
