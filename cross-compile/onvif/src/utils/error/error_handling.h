/**
 * @file error_handling.h
 * @brief Simplified unified error handling system for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_ERROR_HANDLING_H
#define ONVIF_ERROR_HANDLING_H

#include <stdarg.h>
#include <stddef.h>

#include "networking/http/http_parser.h"

/* Standardized return codes */
#define ONVIF_SUCCESS               0
#define ONVIF_ERROR                 -1
#define ONVIF_ERROR_NULL            -2
#define ONVIF_ERROR_INVALID         -3
#define ONVIF_ERROR_MEMORY          -4
#define ONVIF_ERROR_IO              -5
#define ONVIF_ERROR_NETWORK         -6
#define ONVIF_ERROR_TIMEOUT         -7
#define ONVIF_ERROR_NOT_FOUND       -8
#define ONVIF_ERROR_ALREADY_EXISTS  -9
#define ONVIF_ERROR_NOT_SUPPORTED   -10
#define ONVIF_ERROR_NOT_IMPLEMENTED -11

/* Additional error codes for gSOAP compatibility */
#define ONVIF_ERROR_INVALID_PARAMETER    -12
#define ONVIF_ERROR_MEMORY_ALLOCATION    -13
#define ONVIF_ERROR_PARSE_FAILED         -14
#define ONVIF_ERROR_SERIALIZATION_FAILED -15

/* Validation return codes */
#define ONVIF_VALIDATION_SUCCESS 1
#define ONVIF_VALIDATION_FAILED  0

/* SOAP Fault Codes - use constants from common/onvif_constants.h */

/**
 * @brief Enhanced error context structure
 */
typedef struct {
  // Service context
  const char* service_name;
  const char* action_name;
  const char* error_context;

  // Debug context
  const char* function;
  const char* file;
  int line;

  // Error details
  int error_code;
  char message[256];
  char context[512];
  int log_level;
} error_context_t;

/**
 * @brief Error handling result
 */
typedef struct {
  int error_code;
  const char* error_message;
  const char* soap_fault_code;
  const char* soap_fault_string;
} error_result_t;

/**
 * @brief Common error patterns
 */
typedef enum {
  ERROR_PATTERN_VALIDATION_FAILED,
  ERROR_PATTERN_NOT_FOUND,
  ERROR_PATTERN_NOT_SUPPORTED,
  ERROR_PATTERN_INTERNAL_ERROR,
  ERROR_PATTERN_INVALID_PARAMETER,
  ERROR_PATTERN_MISSING_PARAMETER,
  ERROR_PATTERN_AUTHENTICATION_FAILED,
  ERROR_PATTERN_AUTHORIZATION_FAILED
} error_pattern_t;

/* Essential error context macros */
#define ERROR_CONTEXT_INIT(ctx, code, func, file, line)                                            \
  do {                                                                                             \
    (ctx)->error_code = (code);                                                                    \
    (ctx)->function = (func);                                                                      \
    (ctx)->file = (file);                                                                          \
    (ctx)->line = (line);                                                                          \
    (ctx)->message[0] = '\0';                                                                      \
    (ctx)->context[0] = '\0';                                                                      \
    (ctx)->service_name = NULL;                                                                    \
    (ctx)->action_name = NULL;                                                                     \
    (ctx)->error_context = NULL;                                                                   \
    (ctx)->log_level = 1;                                                                          \
  } while (0)

#define ERROR_CONTEXT_SET_MESSAGE(ctx, fmt, ...)                                                   \
  do {                                                                                             \
    snprintf((ctx)->message, sizeof((ctx)->message), (fmt), ##__VA_ARGS__);                        \
  } while (0)

#define ERROR_CONTEXT_SET_CONTEXT(ctx, fmt, ...)                                                   \
  do {                                                                                             \
    snprintf((ctx)->context, sizeof((ctx)->context), (fmt), ##__VA_ARGS__);                        \
  } while (0)

/* Essential error checking macros */
#define ONVIF_CHECK_NULL(ptr)                                                                      \
  do {                                                                                             \
    if ((ptr) == NULL) {                                                                           \
      platform_log_error("Null pointer error at %s:%d\n", __FILE__, __LINE__);                     \
      return ONVIF_ERROR_NULL;                                                                     \
    }                                                                                              \
  } while (0)

#define ONVIF_CHECK_RETURN(expr, error_code)                                                       \
  do {                                                                                             \
    int _ret = (expr);                                                                             \
    if (_ret != 0) {                                                                               \
      platform_log_error("Operation failed at %s:%d with code %d\n", __FILE__, __LINE__, _ret);    \
      return (error_code);                                                                         \
    }                                                                                              \
  } while (0)

/* Validation macros */
#define ONVIF_VALIDATE_NULL(ptr, field_name)                                                       \
  do {                                                                                             \
    if ((ptr) == NULL) {                                                                           \
      platform_log_error("Validation failed: %s is NULL at %s:%d\n", (field_name), __FILE__,       \
                         __LINE__);                                                                \
      return ONVIF_VALIDATION_FAILED;                                                              \
    }                                                                                              \
  } while (0)

#define ONVIF_VALIDATE_STRING(str, field_name, min_len, max_len)                                   \
  do {                                                                                             \
    if (!str) {                                                                                    \
      platform_log_error("Validation failed: %s is NULL at %s:%d\n", (field_name), __FILE__,       \
                         __LINE__);                                                                \
      return ONVIF_VALIDATION_FAILED;                                                              \
    }                                                                                              \
    size_t len = strlen(str);                                                                      \
    if (len < (min_len) || len > (max_len)) {                                                      \
      platform_log_error("Validation failed: %s length %zu not in range [%zu, %zu] at "            \
                         "%s:%d\n",                                                                \
                         (field_name), len, (min_len), (max_len), __FILE__, __LINE__);             \
      return ONVIF_VALIDATION_FAILED;                                                              \
    }                                                                                              \
  } while (0)

/* Logging macros */
#define ONVIF_LOG_ERROR(fmt, ...) platform_log_error("[ERROR] " fmt, ##__VA_ARGS__)

#define ONVIF_LOG_WARNING(fmt, ...) platform_log_warning("[WARNING] " fmt, ##__VA_ARGS__)

#define ONVIF_LOG_INFO(fmt, ...) platform_log_info("[INFO] " fmt, ##__VA_ARGS__)

#define ONVIF_LOG_DEBUG(fmt, ...) platform_log_debug("[DEBUG] " fmt, ##__VA_ARGS__)

/* Enhanced error handling macros */
#define ONVIF_ERROR_WITH_CONTEXT(code, message, ...)                                               \
  do {                                                                                             \
    error_context_t ctx;                                                                           \
    ERROR_CONTEXT_INIT(&ctx, (code), __FUNCTION__, __FILE__, __LINE__);                            \
    ERROR_CONTEXT_SET_MESSAGE(&ctx, (message), ##__VA_ARGS__);                                     \
    onvif_log_error_context(&ctx);                                                                 \
    return (code);                                                                                 \
  } while (0)

#define ONVIF_ERROR_IF_NULL(ptr, message, ...)                                                     \
  do {                                                                                             \
    if ((ptr) == NULL) {                                                                           \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_NULL, (message), ##__VA_ARGS__);                        \
    }                                                                                              \
  } while (0)

#define ONVIF_ERROR_IF_INVALID(expr, message, ...)                                                 \
  do {                                                                                             \
    if (!(expr)) {                                                                                 \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_INVALID, (message), ##__VA_ARGS__);                     \
    }                                                                                              \
  } while (0)

/* Function declarations */

/* Basic error context management */
int error_context_init(error_context_t* context, const char* service_name, const char* action_name,
                       const char* error_context);

/* Pattern-based error handling */
int error_handle_pattern(const error_context_t* context, error_pattern_t pattern,
                         const char* custom_message, http_response_t* response);

int error_handle_validation(const error_context_t* context, int validation_result,
                            const char* field_name, http_response_t* response);

int error_handle_parameter(const error_context_t* context, const char* parameter_name,
                           const char* error_type, http_response_t* response);

int error_handle_service(const error_context_t* context, int error_code, const char* error_message,
                         http_response_t* response);

int error_handle_system(const error_context_t* context, int error_code, const char* operation,
                        http_response_t* response);

/* Error result management */
int error_create_result_from_pattern(error_pattern_t pattern, const char* custom_message,
                                     error_result_t* result);

/* Logging and debugging */
void onvif_log_error_context(const error_context_t* ctx);
void onvif_log_error_with_context(int error_code, const char* function, const char* file, int line,
                                  const char* message, ...);
int onvif_get_error_context_string(const error_context_t* ctx, char* buffer, size_t buffer_size);
void error_log_with_context(const error_context_t* context, const error_result_t* result,
                            const char* additional_info);

/* Utility functions */
const char* error_get_message_for_pattern(error_pattern_t pattern);
const char* error_get_soap_fault_code_for_pattern(error_pattern_t pattern);
int error_should_log(const error_context_t* context, const error_result_t* result);
int error_create_summary(const error_context_t* context, const error_result_t* result,
                         char* summary, size_t summary_size);

/* Standardized error handling functions */
int onvif_standardized_validation(const char* field_name, int validation_result,
                                  const char* error_context);
int onvif_standardized_operation(const char* operation_name, int operation_result,
                                 const char* error_context);
void onvif_standardized_log_error(const char* function, const char* file, int line,
                                  const char* message, ...);

#endif /* ONVIF_ERROR_HANDLING_H */
