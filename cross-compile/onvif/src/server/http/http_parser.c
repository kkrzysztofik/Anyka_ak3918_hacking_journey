/**
 * @file http_parser.c
 * @brief HTTP request parsing module implementation.
 */

#include "http_parser.h"
#include "platform/platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <strings.h>
#include <sys/socket.h>

/* Fallback implementations for missing functions */
#ifndef strnlen
static size_t strnlen(const char *s, size_t maxlen) {
    size_t len = 0;
    while (len < maxlen && s[len] != '\0') {
        len++;
    }
    return len;
}
#endif

/* Fallback implementation for missing strcasestr function */
#ifndef strcasestr
static char *strcasestr(const char *haystack, const char *needle) {
    size_t needle_len = strlen(needle);
    size_t haystack_len = strlen(haystack);
    
    if (needle_len > haystack_len) {
        return NULL;
    }
    
    for (size_t i = 0; i <= haystack_len - needle_len; i++) {
        if (strncasecmp(haystack + i, needle, needle_len) == 0) {
            return (char *)(haystack + i);
        }
    }
    return NULL;
}
#endif

#define MAX_METHOD_LEN 15
#define MAX_PATH_LEN 255
#define MAX_VERSION_LEN 15
#define MAX_CONTENT_LENGTH 262144  // 256KB max content length

/**
 * @brief Safe string copy with bounds checking and null termination
 * @param dest Destination buffer
 * @param src Source string
 * @param dest_size Size of destination buffer
 * @return 0 on success, -1 on error
 */
int safe_strncpy(char *dest, const char *src, size_t dest_size) {
    if (!dest || !src || dest_size == 0) return -1;
    
    size_t src_len = strnlen(src, dest_size - 1);
    if (src_len >= dest_size) return -1;
    
    memcpy(dest, src, src_len);
    dest[src_len] = '\0';
    return 0;
}

/**
 * @brief Safe string concatenation with bounds checking
 * @param dest Destination buffer
 * @param src Source string
 * @param dest_size Size of destination buffer
 * @return 0 on success, -1 on error
 */
int safe_strncat(char *dest, const char *src, size_t dest_size) {
    if (!dest || !src || dest_size == 0) return -1;
    
    size_t dest_len = strnlen(dest, dest_size - 1);
    size_t src_len = strnlen(src, dest_size - dest_len - 1);
    
    if (dest_len + src_len >= dest_size) return -1;
    
    memcpy(dest + dest_len, src, src_len);
    dest[dest_len + src_len] = '\0';
    return 0;
}

/**
 * @brief Validate content length value
 * @param content_length Content length to validate
 * @return 0 if valid, -1 if invalid
 */
int validate_content_length(size_t content_length) {
    if (content_length > MAX_CONTENT_LENGTH) {
        platform_log_error("Content-Length too large: %zu (max: %d)\n", 
                          content_length, MAX_CONTENT_LENGTH);
        return -1;
    }
    return 0;
}

/**
 * @brief Parse HTTP request line (method, path, version)
 * @param request Raw request buffer
 * @param req Parsed request structure
 * @return 0 on success, -1 on error
 */
int parse_http_request_line(const char *request, http_request_t *req) {
    if (!request || !req) return -1;
    
    if (sscanf(request, "%15s %255s %15s", req->method, req->path, req->version) != 3) {
        platform_log_error("Failed to parse HTTP request line\n");
        return -1;
    }
    
    return 0;
}

/**
 * @brief Parse Content-Length header
 * @param request Raw request buffer
 * @return Content length on success, 0 if not found, -1 on error
 */
ssize_t parse_content_length(const char *request) {
    if (!request) return -1;
    
    const char *cl = strcasestr(request, "Content-Length:");
    if (!cl) return 0;
    
    cl += 15; // Skip "Content-Length:"
    while (*cl == ' ' || *cl == '\t') cl++;
    
    long parsed_length = atol(cl);
    if (parsed_length < 0) {
        platform_log_error("Invalid negative Content-Length: %ld\n", parsed_length);
        return -1;
    }
    
    size_t content_length = (size_t)parsed_length;
    if (validate_content_length(content_length) != 0) {
        return -1;
    }
    
    return (ssize_t)content_length;
}

/**
 * @brief Parse HTTP request using state machine
 * @param buffer Request buffer
 * @param buffer_used Amount of data in buffer
 * @param request Output parsed request structure
 * @param need_more_data Output flag indicating if more data needed
 * @return 0 on success, -1 on error
 */
int parse_http_request_state_machine(char *buffer, size_t buffer_used, 
                                   http_request_t *request, int *need_more_data) {
    if (!buffer || !request || !need_more_data) return -1;
    
    size_t pos = 0;
    http_parse_state_t state = HTTP_STATE_METHOD;
    char *line_start = buffer;
    char *header_end = NULL;
    size_t header_length = 0;
    
    while (pos < buffer_used) {
        char c = buffer[pos];
        
        switch (state) {
            case HTTP_STATE_METHOD:
                if (c == ' ') {
                    buffer[pos] = '\0';
                    if (pos >= MAX_METHOD_LEN) {
                        platform_log_error("Method too long\n");
                        return -1;
                    }
                    strcpy(request->method, line_start);
                    line_start = buffer + pos + 1;
                    state = HTTP_STATE_PATH;
                } else if (pos >= MAX_METHOD_LEN) {
                    platform_log_error("Method too long\n");
                    return -1;
                }
                break;
                
            case HTTP_STATE_PATH:
                if (c == ' ') {
                    buffer[pos] = '\0';
                    if (pos - (line_start - buffer) >= MAX_PATH_LEN) {
                        platform_log_error("Path too long\n");
                        return -1;
                    }
                    strcpy(request->path, line_start);
                    line_start = buffer + pos + 1;
                    state = HTTP_STATE_VERSION;
                } else if (pos - (line_start - buffer) >= MAX_PATH_LEN) {
                    platform_log_error("Path too long\n");
                    return -1;
                }
                break;
                
            case HTTP_STATE_VERSION:
                if (c == '\r' && pos + 1 < buffer_used && buffer[pos + 1] == '\n') {
                    buffer[pos] = '\0';
                    if (pos - (line_start - buffer) >= MAX_VERSION_LEN) {
                        platform_log_error("Version too long\n");
                        return -1;
                    }
                    strcpy(request->version, line_start);
                    pos++; // Skip \n
                    state = HTTP_STATE_HEADERS;
                    line_start = buffer + pos + 1;
                } else if (pos - (line_start - buffer) >= MAX_VERSION_LEN) {
                    platform_log_error("Version too long\n");
                    return -1;
                }
                break;
                
            case HTTP_STATE_HEADERS:
                if (c == '\r' && pos + 1 < buffer_used && buffer[pos + 1] == '\n') {
                    if (pos == line_start - buffer) {
                        // Empty line - end of headers
                        header_end = buffer + pos;
                        state = HTTP_STATE_BODY;
                        header_length = pos + 2;
                        break;
                    }
                    pos++; // Skip \n
                    line_start = buffer + pos + 1;
                }
                break;
                
            case HTTP_STATE_BODY:
                // Body parsing is handled separately
                state = HTTP_STATE_COMPLETE;
                break;
                
            default:
                break;
        }
        
        pos++;
    }
    
    if (state == HTTP_STATE_COMPLETE) {
        // Parse Content-Length header
        if (header_end) {
            char *cl = strcasestr(buffer, "Content-Length:");
            if (cl && cl < header_end) {
                cl += 15;
                while (*cl == ' ' || *cl == '\t') cl++;
                long parsed_length = atol(cl);
                if (parsed_length < 0 || parsed_length > MAX_CONTENT_LENGTH) {
                    platform_log_error("Invalid Content-Length: %ld\n", parsed_length);
                    return -1;
                }
                request->content_length = (size_t)parsed_length;
            }
        }
        
        // Set up request structure
        request->headers = buffer;
        request->body = buffer + header_length;
        request->body_length = buffer_used - header_length;
        request->total_length = buffer_used;
        
        // Check if we have complete body
        if (request->body_length >= request->content_length) {
            *need_more_data = 0;
            return 0; // Complete request
        } else {
            *need_more_data = 1;
            return 0; // Need more data
        }
    }
    
    *need_more_data = 1;
    return 0; // Need more data
}

/**
 * @brief Send HTTP response to client
 * @param client Client socket
 * @param response Response structure
 * @return 0 on success, -1 on error
 */
int send_http_response(int client, const http_response_t *response) {
    if (!response) return -1;
    
    char header[512];
    const char *status_text = (response->status_code == 200) ? "OK" : 
                             (response->status_code == 404) ? "Not Found" : "Internal Server Error";
    
    snprintf(header, sizeof(header),
        "HTTP/1.1 %d %s\r\n"
        "Content-Type: %s\r\n"
        "Content-Length: %zu\r\n"
        "Connection: close\r\n\r\n",
        response->status_code, status_text,
        response->content_type ? response->content_type : "text/plain",
        response->body_length);
    
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
            platform_log_error("Failed to send response body: %s\n", strerror(errno));
            return -1;
        }
    }
    
    return 0;
}

/**
 * @brief Create HTTP 200 OK response
 * @param body Response body
 * @param body_length Body length
 * @param content_type Content type
 * @return Response structure (static, don't free)
 */
http_response_t create_http_200_response(const char *body, size_t body_length, const char *content_type) {
    static http_response_t response;
    response.status_code = 200;
    response.content_type = (char*)(content_type ? content_type : "application/soap+xml; charset=utf-8");
    response.body = (char*)body;
    response.body_length = body_length;
    return response;
}

/**
 * @brief Create HTTP 404 Not Found response
 * @return Response structure (static, don't free)
 */
http_response_t create_http_404_response(void) {
    static http_response_t response;
    static const char *not_found_body = "404 Not Found";
    response.status_code = 404;
    response.content_type = (char*)"text/plain";
    response.body = (char*)not_found_body;
    response.body_length = strlen(not_found_body);
    return response;
}
