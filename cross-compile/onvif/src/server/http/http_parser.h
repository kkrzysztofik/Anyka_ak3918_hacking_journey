/**
 * @file http_parser.h
 * @brief HTTP request parsing module with state machine implementation.
 * 
 * This module provides efficient HTTP request parsing using a state machine
 * approach for better performance and maintainability.
 */

#ifndef HTTP_PARSER_H
#define HTTP_PARSER_H

#include <stdint.h>
#include <stddef.h>
#include <sys/types.h>

/* HTTP parsing states */
typedef enum {
    HTTP_STATE_METHOD = 0,
    HTTP_STATE_PATH,
    HTTP_STATE_VERSION,
    HTTP_STATE_HEADERS,
    HTTP_STATE_BODY,
    HTTP_STATE_COMPLETE
} http_parse_state_t;

/* HTTP Request Structure */
typedef struct {
    char method[16];
    char path[256];
    char version[16];
    char *headers;
    char *body;
    size_t body_length;
    size_t content_length;
    size_t total_length;
} http_request_t;

/* HTTP Response Structure */
typedef struct {
    int status_code;
    char *content_type;
    char *body;
    size_t body_length;
} http_response_t;

/* HTTP parsing functions */
int parse_http_request_state_machine(char *buffer, size_t buffer_used, 
                                   http_request_t *request, int *need_more_data);
int parse_http_request_line(const char *request, http_request_t *req);
ssize_t parse_content_length(const char *request);
int validate_content_length(size_t content_length);

/* HTTP response functions */
int send_http_response(int client, const http_response_t *response);
http_response_t create_http_200_response(const char *body, size_t body_length, const char *content_type);
http_response_t create_http_404_response(void);

/* Utility functions */
int safe_strncpy(char *dest, const char *src, size_t dest_size);
int safe_strncat(char *dest, const char *src, size_t dest_size);

#endif /* HTTP_PARSER_H */
