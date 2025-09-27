/**
 * @file http_parser.c
 * @brief HTTP request parsing module implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements HTTP request parsing with comprehensive debug logging
 * for all HTTP headers. When debug logging is enabled, each individual header
 * will be logged with its file descriptor context for detailed analysis.
 */

#include "http_parser.h"

#include "common/onvif_constants.h"
#include "platform/platform.h"
#include "utils/string/string_shims.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

/* Fallback implementations for missing functions */

#define MAX_METHOD_LEN     15
#define MAX_PATH_LEN       255
#define MAX_VERSION_LEN    15
#define MAX_CONTENT_LENGTH 262144 // 256KB max content length

/* HTTP Parsing Constants */
#define HTTP_CONTENT_LENGTH_BUFFER_SIZE 32
#define HTTP_STRTOL_BASE                10
#define HTTP_RESPONSE_BUFFER_SIZE       512
#define HTTP_STATUS_OK                  200
#define HTTP_STATUS_BAD_REQUEST         400
#define HTTP_STATUS_NOT_FOUND           404
#define HTTP_MAX_HEADER_LINE_LENGTH     8192

/* Safe string functions are now provided by utils/safe_string.h */

/* validate_content_length is now provided by
 * utils/validation/input_validation.h */

/* Helper function declarations */
static int parse_single_header_line(const char* line_start, size_t line_len, http_header_t* header,
                                    size_t header_index);
static int validate_header_name_value(const char* name, size_t name_len, const char* value,
                                      size_t value_len);
static int parse_http_method(const char* buffer, size_t* pos, size_t buffer_used,
                             http_request_t* request, const char** line_start);
static int parse_http_path(const char* buffer, size_t* pos, size_t buffer_used,
                           http_request_t* request, const char** line_start);
static int parse_http_version(const char* buffer, size_t* pos, size_t buffer_used,
                              http_request_t* request, const char** line_start);
static int parse_http_headers_state(const char* buffer, size_t* pos, size_t buffer_used,
                                    http_request_t* request, const char** line_start,
                                    size_t* header_length);
static int parse_http_body(const char* buffer, size_t buffer_used, http_request_t* request,
                           size_t header_length, int* need_more_data);
static size_t count_http_headers(const char* headers, size_t headers_size);

/* ============================================================================
 * HELPER FUNCTIONS (Static Functions - Definitions at Top)
 * ============================================================================
 */

/**
 * @brief Parse a single header line and populate header structure
 * @param line_start Start of the header line
 * @param line_len Length of the header line
 * @param header Header structure to populate
 * @param header_index Index of the header in the array
 * @return 0 on success, -1 on error
 */
static int parse_single_header_line(const char* line_start, size_t line_len, http_header_t* header,
                                    size_t header_index) {
  if (!line_start || !header || line_len == 0) {
    return -1;
  }

  // Find colon separator
  const char* colon = (const char*)memchr(line_start, ':', line_len);
  if (!colon) {
    platform_log_error("Invalid header format (no colon): %.*s\n", (int)line_len, line_start);
    return -1;
  }

  size_t name_len = colon - line_start;
  if (name_len == 0) {
    platform_log_error("Empty header name\n");
    return -1;
  }

  // Find value start (skip colon and whitespace)
  const char* value_start = colon + 1;
  while (value_start < line_start + line_len && (*value_start == ' ' || *value_start == '\t')) {
    value_start++;
  }

  size_t value_len = line_len - (value_start - line_start);
  if (value_len == 0) {
    platform_log_error("Empty header value\n");
    return -1;
  }

  // Validate header name and value
  if (validate_header_name_value(line_start, name_len, value_start, value_len) != 0) {
    return -1;
  }

  // Zero-copy implementation: insert null terminators and assign pointers
  char* name_end = (char*)line_start + name_len;
  char* value_end = (char*)line_start + (value_start - line_start) + value_len;

  *name_end = '\0';
  *value_end = '\0';

  header[header_index].name = (char*)line_start;
  header[header_index].value = (char*)value_start;

  return 0;
}

/**
 * @brief Validate header name and value format
 * @param name Header name
 * @param name_len Length of header name
 * @param value Header value
 * @param value_len Length of header value
 * @return 0 on success, -1 on error
 */
static int validate_header_name_value(const char* name, size_t name_len, const char* value,
                                      size_t value_len) {
  if (!name || !value || name_len == 0 || value_len == 0) {
    return -1;
  }

  // Check for valid header name characters
  for (size_t i = 0; i < name_len; i++) {
    char char_value = name[i];
    if (!((char_value >= 'A' && char_value <= 'Z') || (char_value >= 'a' && char_value <= 'z') ||
          (char_value >= '0' && char_value <= '9') || char_value == '-' || char_value == '_')) {
      platform_log_error("Invalid character in header name: %c\n", char_value);
      return -1;
    }
  }

  // Check for reasonable length limits
  if (name_len > HTTP_MAX_HEADER_LINE_LENGTH || value_len > HTTP_MAX_HEADER_LINE_LENGTH) {
    platform_log_error("Header name or value too long\n");
    return -1;
  }

  return 0;
}

/**
 * @brief Parse HTTP method from request line
 * @param buffer Request buffer
 * @param pos Current position in buffer
 * @param buffer_used Amount of data in buffer
 * @param request Request structure to populate
 * @param line_start Start of current line
 * @return 0 on success, -1 on error
 */
static int parse_http_method(const char* buffer, size_t* pos, size_t buffer_used,
                             http_request_t* request, const char** line_start) {
  if (!buffer || !pos || !request || !line_start) {
    return -1;
  }

  // Find end of method (space or end of buffer)
  size_t method_end = *pos;
  while (method_end < buffer_used && buffer[method_end] != ' ') {
    method_end++;
  }

  if (method_end == *pos || method_end - *pos >= sizeof(request->method)) {
    platform_log_error("Invalid or too long HTTP method\n");
    return -1;
  }

  // Copy method with bounds checking
  strncpy(request->method, *line_start, method_end - *pos);
  request->method[method_end - *pos] = '\0';

  *pos = method_end + 1; // Skip space
  return 0;
}

/**
 * @brief Parse HTTP path from request line
 * @param buffer Request buffer
 * @param pos Current position in buffer
 * @param buffer_used Amount of data in buffer
 * @param request Request structure to populate
 * @param line_start Start of current line
 * @return 0 on success, -1 on error
 */
static int parse_http_path(const char* buffer, size_t* pos, size_t buffer_used,
                           http_request_t* request, const char** line_start) {
  if (!buffer || !pos || !request || !line_start) {
    return -1;
  }

  // Find end of path (space or end of buffer)
  size_t path_end = *pos;
  while (path_end < buffer_used && buffer[path_end] != ' ') {
    path_end++;
  }

  if (path_end == *pos || path_end - *pos >= sizeof(request->path)) {
    platform_log_error("Invalid or too long HTTP path\n");
    return -1;
  }

  // Copy path with bounds checking
  strncpy(request->path, *line_start + (*pos - (*line_start - buffer)), path_end - *pos);
  request->path[path_end - *pos] = '\0';

  *pos = path_end + 1; // Skip space
  return 0;
}

/**
 * @brief Parse HTTP version from request line
 * @param buffer Request buffer
 * @param pos Current position in buffer
 * @param buffer_used Amount of data in buffer
 * @param request Request structure to populate
 * @param line_start Start of current line
 * @return 0 on success, -1 on error
 */
static int parse_http_version(const char* buffer, size_t* pos, size_t buffer_used,
                              http_request_t* request, const char** line_start) {
  if (!buffer || !pos || !request || !line_start) {
    return -1;
  }

  // Find end of version (CRLF or end of buffer)
  size_t version_end = *pos;
  while (version_end < buffer_used && buffer[version_end] != '\r') {
    version_end++;
  }

  if (version_end == *pos || version_end - *pos >= sizeof(request->version)) {
    platform_log_error("Invalid or too long HTTP version\n");
    return -1;
  }

  // Copy version with bounds checking
  strncpy(request->version, *line_start + (*pos - (*line_start - buffer)), version_end - *pos);
  request->version[version_end - *pos] = '\0';

  *pos = version_end;
  return 0;
}

/**
 * @brief Parse HTTP headers state
 * @param buffer Request buffer
 * @param pos Current position in buffer
 * @param buffer_used Amount of data in buffer
 * @param request Request structure to populate
 * @param line_start Start of current line
 * @param header_length Length of headers section
 * @return 0 on success, -1 on error
 */
static int parse_http_headers_state(const char* buffer, size_t* pos, size_t buffer_used,
                                    http_request_t* request, const char** line_start,
                                    size_t* header_length) {
  if (!buffer || !pos || !request || !line_start || !header_length) {
    return -1;
  }

  // Parse headers
  int result = parse_http_headers(buffer + *pos, buffer_used - *pos, &request->headers,
                                  &request->header_count);
  if (result != 0) {
    platform_log_error("Failed to parse HTTP headers\n");
    return -1;
  }

  *header_length = buffer_used - *pos;
  *pos = buffer_used; // Move to end of headers
  return 0;
}

/**
 * @brief Parse HTTP body
 * @param buffer Request buffer
 * @param buffer_used Amount of data in buffer
 * @param request Request structure to populate
 * @param header_length Length of headers section
 * @param need_more_data Flag indicating if more data is needed
 * @return 0 on success, -1 on error
 */
static int parse_http_body(const char* buffer, size_t buffer_used, http_request_t* request,
                           size_t header_length, int* need_more_data) {
  if (!buffer || !request || !need_more_data) {
    return -1;
  }

  // Check if we have Content-Length header
  char content_length_str[HTTP_CONTENT_LENGTH_BUFFER_SIZE];
  if (find_header_value(request->headers, request->header_count, "Content-Length",
                        content_length_str, sizeof(content_length_str)) == 0) {
    char* endptr = NULL;
    long parsed_length = strtol(content_length_str, &endptr, HTTP_STRTOL_BASE);
    if (endptr == content_length_str || *endptr != '\0' || parsed_length < 0 ||
        parsed_length > MAX_CONTENT_LENGTH) {
      platform_log_error("Invalid Content-Length: %s\n", content_length_str);
      return -1;
    }

    request->content_length = (size_t)parsed_length;
    request->body_length = request->content_length;

    // Check if we have enough data for the body
    size_t body_start = header_length;
    if (buffer_used < body_start + request->content_length) {
      *need_more_data = 1;
      return 0;
    }

    request->body = (char*)buffer + body_start;
    request->body_length = request->content_length;
  } else {
    // No Content-Length header, assume no body
    request->body = NULL;
    request->body_length = 0;
    request->content_length = 0;
  }

  *need_more_data = 0;
  return 0;
}

/**
 * @brief Count the number of HTTP headers in raw header data
 * @param headers Raw header data
 * @param headers_size Size of header data
 * @return Number of headers found
 */
static size_t count_http_headers(const char* headers, size_t headers_size) {
  if (!headers || headers_size == 0) {
    return 0;
  }

  const char* current = headers;
  const char* line_start = headers;
  size_t count = 0;

  while (current < headers + headers_size) {
    if (*current == '\r' && current + 1 < headers + headers_size && *(current + 1) == '\n') {
      size_t line_len = current - line_start;
      if (line_len > 0 && strchr(line_start, ':') != NULL) {
        count++;
      }
      current += 2; // Skip \r\n
      line_start = current;
    } else {
      current++;
    }
  }

  return count;
}

/* ============================================================================
 * CORE PARSING FUNCTIONS (Main Functionality)
 * ============================================================================
 */

/**
 * @brief Parse HTTP request line (method, path, version)
 * @param request Raw request buffer
 * @param req Parsed request structure
 * @return 0 on success, -1 on error
 */
int parse_http_request_line(const char* request, http_request_t* req) {
  if (!request || !req) {
    return -1;
  }

  // Find end of line
  const char* line_end = strstr(request, "\r\n");
  if (!line_end) {
    platform_log_error("Invalid request line format\n");
    return -1;
  }

  size_t line_len = line_end - request;
  if (line_len == 0) {
    platform_log_error("Empty request line\n");
    return -1;
  }

  // Find method end
  const char* method_end = strchr(request, ' ');
  if (!method_end || method_end >= line_end) {
    platform_log_error("Invalid method in request line\n");
    return -1;
  }

  // Find path end
  const char* path_start = method_end + 1;
  const char* path_end = strchr(path_start, ' ');
  if (!path_end || path_end >= line_end) {
    platform_log_error("Invalid path in request line\n");
    return -1;
  }

  // Extract method
  size_t method_len = method_end - request;
  if (method_len >= sizeof(req->method)) {
    platform_log_error("Method too long\n");
    return -1;
  }
  strncpy(req->method, request, method_len);
  req->method[method_len] = '\0';

  // Extract path
  size_t path_len = path_end - path_start;
  if (path_len >= sizeof(req->path)) {
    platform_log_error("Path too long\n");
    return -1;
  }
  strncpy(req->path, path_start, path_len);
  req->path[path_len] = '\0';

  // Extract version
  const char* version_start = path_end + 1;
  size_t version_len = line_end - version_start;
  if (version_len >= sizeof(req->version)) {
    platform_log_error("Version too long\n");
    return -1;
  }
  strncpy(req->version, version_start, version_len);
  req->version[version_len] = '\0';

  return 0;
}

/**
 * @brief Parse HTTP headers from raw header data
 * @param headers Raw header data
 * @param headers_size Size of header data
 * @param parsed_headers Array to store parsed headers
 * @param header_count Number of headers found
 * @return 0 on success, -1 on error
 */
int parse_http_headers(const char* headers, size_t headers_size, http_header_t** parsed_headers,
                       size_t* header_count) {
  if (!headers || !parsed_headers || !header_count) {
    return -1;
  }

  // Count headers first
  size_t count = count_http_headers(headers, headers_size);
  if (count == 0) {
    return 0; // No headers found
  }

  // Allocate header array
  *parsed_headers = malloc(count * sizeof(http_header_t));
  if (!*parsed_headers) {
    return -1;
  }

  // Parse headers
  const char* current = headers;
  const char* line_start = headers;
  size_t header_index = 0;

  while (current < headers + headers_size && header_index < count) {
    if (*current == '\r' && current + 1 < headers + headers_size && *(current + 1) == '\n') {
      size_t line_len = current - line_start;
      if (line_len > 0) {
        // Use helper function to parse single header line
        if (parse_single_header_line(line_start, line_len, *parsed_headers, header_index) == 0) {
          header_index++;
        }
      }
      current += 2; // Skip \r\n
      line_start = current;
    } else {
      current++;
    }
  }

  *header_count = header_index;
  return 0;
}

/**
 * @brief Parse HTTP request using state machine approach
 * @param buffer Request buffer
 * @param buffer_used Amount of data in buffer
 * @param request Parsed request structure
 * @param need_more_data Flag indicating if more data is needed
 * @return 0 on success, -1 on error
 */
int parse_http_request_state_machine(char* buffer, size_t buffer_used, http_request_t* request,
                                     int* need_more_data) {
  if (!buffer || !request || !need_more_data) {
    return -1;
  }

  static int state = 0; // 0: reading request line, 1: reading headers, 2: reading body
  static size_t pos = 0;
  static char* line_start = NULL;
  static size_t header_length = 0;

  // Reset state if starting fresh
  if (pos == 0) {
    state = 0;
    line_start = buffer;
  }

  *need_more_data = 0;

  while (pos < buffer_used) {
    switch (state) {
    case 0: { // Reading request line
      // Find end of request line
      char* line_end = strstr(buffer + pos, "\r\n");
      if (!line_end) {
        *need_more_data = 1;
        return 0;
      }

      // Parse request line using helper functions
      const char* current_line_start = buffer + pos;
      size_t current_pos = pos;

      if (parse_http_method(buffer, &current_pos, buffer_used, request, &current_line_start) != 0 ||
          parse_http_path(buffer, &current_pos, buffer_used, request, &current_line_start) != 0 ||
          parse_http_version(buffer, &current_pos, buffer_used, request, &current_line_start) !=
            0) {
        platform_log_error("Failed to parse request line\n");
        return -1;
      }

      pos = line_end - buffer + 2; // Skip \r\n
      state = 1;
      line_start = buffer + pos;
      break;
    }

    case 1: { // Reading headers
      // Find end of headers (double \r\n)
      char* header_end = strstr(buffer + pos, "\r\n\r\n");
      if (!header_end) {
        *need_more_data = 1;
        return 0;
      }

      // Parse headers using helper function
      const char* const_line_start = line_start;
      if (parse_http_headers_state(buffer, &pos, buffer_used, request, &const_line_start,
                                   &header_length) != 0) {
        platform_log_error("Failed to parse headers\n");
        return -1;
      }

      state = 2;
      break;
    }

    case 2: { // Reading body
      if (parse_http_body(buffer, buffer_used, request, header_length, need_more_data) != 0) {
        platform_log_error("Failed to parse body\n");
        return -1;
      }

      if (*need_more_data) {
        return 0;
      }

      // Request parsing complete
      state = 0;
      pos = 0;
      return 0;
    }

    default:
      platform_log_error("Invalid state: %d\n", state);
      return -1;
    }
  }

  return 0;
}

/* ============================================================================
 * UTILITY FUNCTIONS (Supporting Functions)
 * ============================================================================
 */

/**
 * @brief Find header value by name
 * @param headers Array of headers
 * @param header_count Number of headers
 * @param header_name Name of header to find
 * @param value Buffer to store value
 * @param value_size Size of value buffer
 * @return 0 if found, -1 if not found
 */
int find_header_value(const http_header_t* headers, size_t header_count, const char* header_name,
                      char* value, size_t value_size) {
  if (!headers || !header_name || !value || value_size == 0) {
    return -1;
  }

  for (size_t i = 0; i < header_count; i++) {
    if (headers[i].name && strcasecmp(headers[i].name, header_name) == 0) {
      if (headers[i].value) {
        strncpy(value, headers[i].value, value_size - 1);
        value[value_size - 1] = '\0';
        return 0;
      }
    }
  }

  return -1;
}

/**
 * @brief Free HTTP headers array
 * @param headers Headers array to free
 * @param header_count Number of headers
 */
void free_http_headers(http_header_t* headers, size_t header_count) { // NOLINT
  if (!headers) {
    return;
  }
  // Zero-copy implementation: no need to free individual strings
  // as they point directly into the connection buffer
  // Only free the headers array itself
  free(headers);
}

/* ============================================================================
 * RESPONSE FUNCTIONS (HTTP Response Handling)
 * ============================================================================
 */

/**
 * @brief Send HTTP response to client
 * @param client Client socket file descriptor
 * @param response Response structure
 * @return 0 on success, -1 on error
 */
int send_http_response(int client, const http_response_t* response) {
  if (!response) {
    return -1;
  }

  char header[HTTP_RESPONSE_BUFFER_SIZE];
  const char* status_text = (response->status_code == HTTP_STATUS_OK)             ? "OK"
                            : (response->status_code == HTTP_STATUS_NOT_FOUND)    ? "Not Found"
                            : (response->status_code == HTTP_STATUS_UNAUTHORIZED) ? "Unauthorized"
                            : (response->status_code == HTTP_STATUS_BAD_REQUEST)
                              ? "Bad Request"
                              : "Internal Server Error";

  // Start building the response header
  int result =
    snprintf(header, sizeof(header),
             "HTTP/1.1 %d %s\r\n"
             "Content-Type: %s\r\n"
             "Content-Length: %zu\r\n"
             "Connection: close\r\n",
             response->status_code, status_text,
             response->content_type ? response->content_type : "text/plain", response->body_length);

  if (result < 0 || (size_t)result >= sizeof(header)) {
    platform_log_error("Failed to format HTTP response header\n");
    return -1;
  }

  // Add custom headers
  for (size_t i = 0; i < response->header_count; i++) {
    int header_result =
      snprintf(header + strlen(header), sizeof(header) - strlen(header), "%s: %s\r\n",
               response->headers[i].name, response->headers[i].value);

    if (header_result < 0 || (size_t)header_result >= (sizeof(header) - strlen(header))) {
      platform_log_error("Failed to format custom header\n");
      return -1;
    }
  }

  // Add final CRLF
  if (strlen(header) + 2 >= sizeof(header)) {
    platform_log_error("Response header too large\n");
    return -1;
  }
  strcat(header, "\r\n");

  // Send header
  ssize_t sent = send(client, header, strlen(header), 0);
  if (sent < 0) {
    platform_log_error("Failed to send header: %s\n", strerror(errno));
    return -1;
  }

  // Send body if present
  if (response->body && response->body_length > 0) {
    sent = send(client, response->body, response->body_length, 0);
    if (sent < 0) {
      platform_log_error("Failed to send body: %s\n", strerror(errno));
      return -1;
    }
  }

  return 0;
}

/**
 * @brief Create HTTP 200 OK response
 * @param body Response body
 * @param body_length Length of response body
 * @param content_type Content type
 * @return Response structure (static, don't free)
 */
http_response_t create_http_200_response(const char* body, size_t body_length,
                                         const char* content_type) {
  static http_response_t response;
  response.status_code = HTTP_STATUS_OK;
  response.content_type =
    (char*)(content_type ? content_type : "application/soap+xml; charset=utf-8");
  response.body = (char*)body;
  response.body_length = body_length;
  response.headers = NULL;
  response.header_count = 0;
  return response;
}

/**
 * @brief Create HTTP 404 Not Found response
 * @return Response structure (static, don't free)
 */
http_response_t create_http_404_response(void) {
  static http_response_t response;
  static const char* not_found_body = "404 Not Found";
  response.status_code = HTTP_STATUS_NOT_FOUND;
  response.content_type = (char*)"text/plain";
  response.body = (char*)not_found_body;
  response.body_length = strlen(not_found_body);
  response.headers = NULL;
  response.header_count = 0;
  return response;
}

/**
 * @brief Create HTTP 400 Bad Request response
 * @return Response structure (static, don't free)
 */
http_response_t create_http_400_response(void) {
  static http_response_t response;
  static const char* bad_request_body = "400 Bad Request";
  response.status_code = HTTP_STATUS_BAD_REQUEST;
  response.content_type = (char*)"text/plain";
  response.body = (char*)bad_request_body;
  response.body_length = strlen(bad_request_body);
  response.headers = NULL;
  response.header_count = 0;
  return response;
}

/**
 * @brief Add a header to HTTP response
 * @param response Response structure to add header to
 * @param name Header name
 * @param value Header value
 * @return 0 on success, -1 on error
 */
int http_response_add_header(http_response_t* response, const char* name, const char* value) {
  if (!response || !name || !value) {
    return -1;
  }

  // Allocate new header
  http_header_t* new_headers =
    realloc(response->headers, (response->header_count + 1) * sizeof(http_header_t));
  if (!new_headers) {
    return -1;
  }

  response->headers = new_headers;

  // Allocate and copy header name
  response->headers[response->header_count].name = malloc(strlen(name) + 1);
  if (!response->headers[response->header_count].name) {
    return -1;
  }
  strcpy(response->headers[response->header_count].name, name);

  // Allocate and copy header value
  response->headers[response->header_count].value = malloc(strlen(value) + 1);
  if (!response->headers[response->header_count].value) {
    free(response->headers[response->header_count].name);
    return -1;
  }
  strcpy(response->headers[response->header_count].value, value);

  response->header_count++;
  return 0;
}

/**
 * @brief Free HTTP response structure and all its resources
 * @param response Response structure to free
 */
void http_response_free(http_response_t* response) {
  if (!response) {
    return;
  }

  // Free headers
  if (response->headers) {
    for (size_t i = 0; i < response->header_count; i++) {
      free(response->headers[i].name);
      free(response->headers[i].value);
    }
    free(response->headers);
    response->headers = NULL;
    response->header_count = 0;
  }

  // Free body
  if (response->body) {
    free(response->body);
    response->body = NULL;
  }

  // Free content type
  if (response->content_type) {
    free(response->content_type);
    response->content_type = NULL;
  }
}
