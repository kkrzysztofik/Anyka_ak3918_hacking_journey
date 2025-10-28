/**
 * @file http_parser.h
 * @brief HTTP request parsing module with state machine implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides efficient HTTP request parsing using a state machine
 * approach for better performance and maintainability. It includes
 * comprehensive debug logging for HTTP headers to aid in troubleshooting and
 * analysis.
 */

#ifndef HTTP_PARSER_H
#define HTTP_PARSER_H

#include <netinet/in.h>
#include <stddef.h>
#include <sys/types.h>

#include "http_constants.h"

/* HTTP parsing states */
typedef enum {
  HTTP_STATE_METHOD = 0,
  HTTP_STATE_PATH,
  HTTP_STATE_VERSION,
  HTTP_STATE_HEADERS,
  HTTP_STATE_BODY,
  HTTP_STATE_COMPLETE
} http_parse_state_t;

/* HTTP Header Structure */
typedef struct {
  char* name;
  char* value;
} http_header_t;

/* HTTP Request Structure */
typedef struct {
  char method[HTTP_METHOD_SIZE];
  char path[HTTP_PATH_SIZE];
  char version[HTTP_VERSION_SIZE];
  char client_ip[INET_ADDRSTRLEN]; /* Client IP address for security and logging
                                    */
  http_header_t* headers;
  size_t header_count;
  char* body;
  size_t body_length;
  size_t content_length;
  size_t total_length;
} http_request_t;

/* HTTP Response Structure */
typedef struct {
  int status_code;
  char* content_type;
  char* body;
  size_t body_length;
  http_header_t* headers;
  size_t header_count;
} http_response_t;

/* HTTP parsing functions */
int parse_http_request_state_machine(char* buffer, size_t buffer_used, http_request_t* request, int* need_more_data);
int parse_http_request_line(const char* request, http_request_t* req);
int parse_http_headers(const char* headers, size_t headers_size, http_header_t** parsed_headers, size_t* header_count);
int find_header_value(const http_header_t* headers, size_t header_count, const char* header_name, char* value, size_t value_size);
void free_http_headers(http_header_t* headers, size_t header_count);
/* validate_content_length is now provided by
 * utils/validation/input_validation.h */

/* HTTP response functions */
int send_http_response(int client, const http_response_t* response);
http_response_t create_http_200_response(const char* body, size_t body_length, const char* content_type);
http_response_t create_http_404_response(void);
http_response_t create_http_400_response(void);
int http_response_add_header(http_response_t* response, const char* name, const char* value);
void http_response_free(http_response_t* response);

/* Utility functions - now using centralized safe_string.h */

#endif /* HTTP_PARSER_H */
