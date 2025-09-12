/**
 * @file common_error_handling.h
 * @brief Common error handling patterns for ONVIF services
 * 
 * This module provides unified error handling patterns that eliminate
 * code duplication across all ONVIF service implementations.
 */

#ifndef ONVIF_COMMON_ERROR_HANDLING_H
#define ONVIF_COMMON_ERROR_HANDLING_H

#include "common/onvif_types.h"
#include "common/onvif_request.h"
#include "unified_soap_generator.h"
#include <stddef.h>

/**
 * @brief Error handling context
 */
typedef struct {
  const char *service_name;
  const char *action_name;
  const char *error_context;
  int log_level;
} error_context_t;

/**
 * @brief Error handling result
 */
typedef struct {
  int error_code;
  const char *error_message;
  const char *soap_fault_code;
  const char *soap_fault_string;
} error_result_t;

/**
 * @brief Error handling callback function type
 */
typedef int (*error_handler_callback_t)(const error_context_t *context, 
                                       const error_result_t *result,
                                       onvif_response_t *response);

/**
 * @brief Common error handling patterns
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

/**
 * @brief Initialize error handling context
 * @param context Error context to initialize
 * @param service_name Name of the service
 * @param action_name Name of the action
 * @param error_context Additional context information
 * @return 0 on success, negative error code on failure
 */
int error_context_init(error_context_t *context, const char *service_name,
                      const char *action_name, const char *error_context);

/**
 * @brief Handle common error patterns
 * @param context Error context
 * @param pattern Error pattern to handle
 * @param custom_message Custom error message (optional)
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_pattern(const error_context_t *context, error_pattern_t pattern,
                        const char *custom_message, onvif_response_t *response);

/**
 * @brief Handle validation errors
 * @param context Error context
 * @param validation_result Validation result code
 * @param field_name Name of the field that failed validation
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_validation(const error_context_t *context, int validation_result,
                           const char *field_name, onvif_response_t *response);

/**
 * @brief Handle parameter errors
 * @param context Error context
 * @param parameter_name Name of the parameter
 * @param error_type Type of parameter error
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_parameter(const error_context_t *context, const char *parameter_name,
                          const char *error_type, onvif_response_t *response);

/**
 * @brief Handle service-specific errors
 * @param context Error context
 * @param error_code Service-specific error code
 * @param error_message Error message
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_service(const error_context_t *context, int error_code,
                        const char *error_message, onvif_response_t *response);

/**
 * @brief Handle system errors (memory, file I/O, etc.)
 * @param context Error context
 * @param system_error System error code
 * @param operation Operation that failed
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_system(const error_context_t *context, int system_error,
                       const char *operation, onvif_response_t *response);

/**
 * @brief Create error result from pattern
 * @param pattern Error pattern
 * @param custom_message Custom error message
 * @param result Error result structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_create_result_from_pattern(error_pattern_t pattern, const char *custom_message,
                                   error_result_t *result);

/**
 * @brief Log error with context
 * @param context Error context
 * @param result Error result
 * @param additional_info Additional information to log
 */
void error_log_with_context(const error_context_t *context, const error_result_t *result,
                           const char *additional_info);

/**
 * @brief Register custom error handler
 * @param pattern Error pattern to handle
 * @param callback Callback function
 * @return 0 on success, negative error code on failure
 */
int error_register_handler(error_pattern_t pattern, error_handler_callback_t callback);

/**
 * @brief Unregister error handler
 * @param pattern Error pattern
 * @return 0 on success, negative error code on failure
 */
int error_unregister_handler(error_pattern_t pattern);

/**
 * @brief Get error message for pattern
 * @param pattern Error pattern
 * @return Error message string
 */
const char *error_get_message_for_pattern(error_pattern_t pattern);

/**
 * @brief Get SOAP fault code for pattern
 * @param pattern Error pattern
 * @return SOAP fault code string
 */
const char *error_get_soap_fault_code_for_pattern(error_pattern_t pattern);

/**
 * @brief Check if error should be logged
 * @param context Error context
 * @param result Error result
 * @return 1 if should log, 0 if not
 */
int error_should_log(const error_context_t *context, const error_result_t *result);

/**
 * @brief Handle multiple errors in sequence
 * @param context Error context
 * @param errors Array of error patterns
 * @param error_count Number of errors
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int error_handle_multiple(const error_context_t *context, const error_pattern_t *errors,
                         size_t error_count, onvif_response_t *response);

/**
 * @brief Create error summary for logging
 * @param context Error context
 * @param result Error result
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return 0 on success, negative error code on failure
 */
int error_create_summary(const error_context_t *context, const error_result_t *result,
                        char *summary, size_t summary_size);

#endif /* ONVIF_COMMON_ERROR_HANDLING_H */
