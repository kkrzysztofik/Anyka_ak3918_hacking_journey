/**
 * @file security_hardening.h
 * @brief Security hardening measures for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SECURITY_HARDENING_H
#define ONVIF_SECURITY_HARDENING_H

#include <stddef.h>
#include <time.h>

#include "networking/http/http_parser.h"

/* Forward declaration */
typedef struct connection connection_t;

/* Security configuration constants */
#define MAX_REQUESTS_PER_MINUTE 100
#define MAX_INPUT_LENGTH        4096
#define MAX_XML_DEPTH           32
#define MAX_XML_ATTRIBUTES      64

/* Security levels */
typedef enum {
  SECURITY_LEVEL_NONE = 0,
  SECURITY_LEVEL_BASIC = 1,
  SECURITY_LEVEL_ENHANCED = 2,
  SECURITY_LEVEL_MAXIMUM = 3
} security_level_t;

/* Security context structure */
typedef struct {
  char client_ip[46]; /* IPv6 address max length */
  time_t last_request_time;
  int request_count;
  security_level_t security_level;
} security_context_t;

/* Rate limiting structure */
typedef struct {
  char client_ip[46];
  time_t window_start;
  int request_count;
  int is_blocked;
} rate_limit_entry_t;

/* Security validation result */
typedef struct {
  int is_valid;
  int security_level;
  const char* error_message;
  const char* recommended_action;
} security_validation_result_t;

/* Function prototypes */

/* Input validation and sanitization */
int security_validate_http_headers(const http_request_t* request, security_context_t* context);
int security_validate_xml_structure(const char* xml, size_t length, security_context_t* context);
int security_sanitize_input(const char* input, char* output, size_t output_size,
                            security_context_t* context);
int security_validate_file_path(const char* path, security_context_t* context);

/* Rate limiting and DoS protection */
int security_check_rate_limit(const char* client_ip, security_context_t* context);
int security_update_rate_limit(const char* client_ip, security_context_t* context);
int security_is_client_blocked(const char* client_ip, security_context_t* context);
void security_reset_rate_limits(void);

/* Attack detection and prevention */
int security_detect_sql_injection(const char* input);
int security_detect_xss_attack(const char* input);
int security_detect_path_traversal(const char* input);
int security_detect_xml_bomb(const char* xml, size_t length);
int security_detect_xxe_attack(const char* xml, size_t length);

/* Security headers and responses */
int security_add_security_headers(http_response_t* response, security_context_t* context);
int security_generate_security_headers(char* headers, size_t headers_size,
                                       security_context_t* context);

/* Configuration and initialization */
int security_init(security_level_t level);
void security_cleanup(void);
int security_set_rate_limit(int max_requests, int window_seconds);

/* Logging and monitoring */
void security_log_attack(const char* attack_type, const char* client_ip, const char* details);
void security_log_security_event(const char* event_type, const char* client_ip, int severity);

/* Utility functions */
const char* security_get_client_ip(const connection_t* conn);
time_t security_get_current_time(void);
int security_is_valid_ip(const char* ip_address);
int security_is_private_ip(const char* ip_address);

/* Comprehensive request validation functions */
int security_validate_request(const http_request_t* request, security_context_t* context);
int security_validate_request_body(const http_request_t* request, security_context_t* context);

/* Security validation macros */
#define SECURITY_VALIDATE_INPUT(input, max_len)                                                    \
  security_validate_input_length((input), (max_len), __FILE__, __LINE__)

#define SECURITY_VALIDATE_XML(xml, len) security_validate_xml_structure((xml), (len), NULL)

#define SECURITY_LOG_ATTACK(type, ip, details) security_log_attack((type), (ip), (details))

#endif /* ONVIF_SECURITY_HARDENING_H */
