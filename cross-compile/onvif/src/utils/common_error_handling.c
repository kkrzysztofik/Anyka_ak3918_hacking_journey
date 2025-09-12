/**
 * @file common_error_handling.c
 * @brief Common error handling patterns implementation
 */

#include "common_error_handling.h"
#include "utils/error_handling.h"
#include "platform/platform.h"
#include <stdio.h>
#include <string.h>
#include <stdarg.h>

#define MAX_ERROR_HANDLERS 16

typedef struct {
  error_pattern_t pattern;
  error_handler_callback_t callback;
} error_handler_entry_t;

static error_handler_entry_t error_handlers[MAX_ERROR_HANDLERS];
static size_t error_handler_count = 0;

// Error pattern definitions
typedef struct {
  error_pattern_t pattern;
  const char *message;
  const char *soap_fault_code;
  const char *soap_fault_string;
} error_pattern_def_t;

static const error_pattern_def_t error_patterns[] = {
  {ERROR_PATTERN_VALIDATION_FAILED, "Validation failed", SOAP_FAULT_SENDER, "Validation failed"},
  {ERROR_PATTERN_NOT_FOUND, "Resource not found", SOAP_FAULT_SENDER, "Resource not found"},
  {ERROR_PATTERN_NOT_SUPPORTED, "Operation not supported", SOAP_FAULT_SENDER, "Operation not supported"},
  {ERROR_PATTERN_INTERNAL_ERROR, "Internal server error", SOAP_FAULT_RECEIVER, "Internal server error"},
  {ERROR_PATTERN_INVALID_PARAMETER, "Invalid parameter", SOAP_FAULT_SENDER, "Invalid parameter"},
  {ERROR_PATTERN_MISSING_PARAMETER, "Missing required parameter", SOAP_FAULT_SENDER, "Missing required parameter"},
  {ERROR_PATTERN_AUTHENTICATION_FAILED, "Authentication failed", SOAP_FAULT_SENDER, "Authentication failed"},
  {ERROR_PATTERN_AUTHORIZATION_FAILED, "Authorization failed", SOAP_FAULT_SENDER, "Authorization failed"}
};

static const error_pattern_def_t *find_error_pattern(error_pattern_t pattern) {
  for (size_t i = 0; i < sizeof(error_patterns) / sizeof(error_patterns[0]); i++) {
    if (error_patterns[i].pattern == pattern) {
      return &error_patterns[i];
    }
  }
  return NULL;
}

static error_handler_callback_t find_error_handler(error_pattern_t pattern) {
  for (size_t i = 0; i < error_handler_count; i++) {
    if (error_handlers[i].pattern == pattern) {
      return error_handlers[i].callback;
    }
  }
  return NULL;
}

int error_context_init(error_context_t *context, const char *service_name,
                      const char *action_name, const char *error_context) {
  if (!context || !service_name || !action_name) {
    return ONVIF_ERROR_INVALID;
  }
  
  memset(context, 0, sizeof(error_context_t));
  context->service_name = service_name;
  context->action_name = action_name;
  context->error_context = error_context;
  context->log_level = 1; // Default to warning level
  
  return ONVIF_SUCCESS;
}

int error_create_result_from_pattern(error_pattern_t pattern, const char *custom_message,
                                   error_result_t *result) {
  if (!result) {
    return ONVIF_ERROR_INVALID;
  }
  
  const error_pattern_def_t *def = find_error_pattern(pattern);
  if (!def) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  memset(result, 0, sizeof(error_result_t));
  result->error_code = (int)pattern;
  result->error_message = custom_message ? custom_message : def->message;
  result->soap_fault_code = def->soap_fault_code;
  result->soap_fault_string = def->soap_fault_string;
  
  return ONVIF_SUCCESS;
}

int error_handle_pattern(const error_context_t *context, error_pattern_t pattern,
                        const char *custom_message, onvif_response_t *response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Check for custom error handler
  error_handler_callback_t handler = find_error_handler(pattern);
  if (handler) {
    error_result_t result;
    if (error_create_result_from_pattern(pattern, custom_message, &result) == ONVIF_SUCCESS) {
      return handler(context, &result, response);
    }
  }
  
  // Use default error handling
  error_result_t result;
  if (error_create_result_from_pattern(pattern, custom_message, &result) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Log error if appropriate
  if (error_should_log(context, &result)) {
    error_log_with_context(context, &result, NULL);
  }
  
  // Generate SOAP fault response
  return onvif_generate_fault_response(response, result.soap_fault_code, result.soap_fault_string);
}

int error_handle_validation(const error_context_t *context, int validation_result,
                           const char *field_name, onvif_response_t *response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char custom_message[256];
  snprintf(custom_message, sizeof(custom_message), "Validation failed for field '%s' (code: %d)", 
           field_name ? field_name : "unknown", validation_result);
  
  return error_handle_pattern(context, ERROR_PATTERN_VALIDATION_FAILED, custom_message, response);
}

int error_handle_parameter(const error_context_t *context, const char *parameter_name,
                          const char *error_type, onvif_response_t *response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char custom_message[256];
  snprintf(custom_message, sizeof(custom_message), "Parameter error: %s for parameter '%s'", 
           error_type ? error_type : "unknown error", parameter_name ? parameter_name : "unknown");
  
  error_pattern_t pattern = ERROR_PATTERN_INVALID_PARAMETER;
  if (error_type && strstr(error_type, "missing") != NULL) {
    pattern = ERROR_PATTERN_MISSING_PARAMETER;
  }
  
  return error_handle_pattern(context, pattern, custom_message, response);
}

int error_handle_service(const error_context_t *context, int error_code,
                        const char *error_message, onvif_response_t *response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char custom_message[256];
  snprintf(custom_message, sizeof(custom_message), "Service error %d: %s", 
           error_code, error_message ? error_message : "Unknown service error");
  
  return error_handle_pattern(context, ERROR_PATTERN_INTERNAL_ERROR, custom_message, response);
}

int error_handle_system(const error_context_t *context, int system_error,
                       const char *operation, onvif_response_t *response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char custom_message[256];
  snprintf(custom_message, sizeof(custom_message), "System error %d during %s", 
           system_error, operation ? operation : "unknown operation");
  
  return error_handle_pattern(context, ERROR_PATTERN_INTERNAL_ERROR, custom_message, response);
}

void error_log_with_context(const error_context_t *context, const error_result_t *result,
                           const char *additional_info) {
  if (!context || !result) {
    return;
  }
  
  char summary[512];
  if (error_create_summary(context, result, summary, sizeof(summary)) == ONVIF_SUCCESS) {
    platform_log_error("ERROR: %s", summary);
    if (additional_info) {
      platform_log_error("Additional info: %s", additional_info);
    }
  }
}

int error_register_handler(error_pattern_t pattern, error_handler_callback_t callback) {
  if (!callback || error_handler_count >= MAX_ERROR_HANDLERS) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Check if handler already exists
  for (size_t i = 0; i < error_handler_count; i++) {
    if (error_handlers[i].pattern == pattern) {
      error_handlers[i].callback = callback;
      return ONVIF_SUCCESS;
    }
  }
  
  // Add new handler
  error_handlers[error_handler_count].pattern = pattern;
  error_handlers[error_handler_count].callback = callback;
  error_handler_count++;
  
  return ONVIF_SUCCESS;
}

int error_unregister_handler(error_pattern_t pattern) {
  for (size_t i = 0; i < error_handler_count; i++) {
    if (error_handlers[i].pattern == pattern) {
      // Move remaining handlers up
      for (size_t j = i; j < error_handler_count - 1; j++) {
        error_handlers[j] = error_handlers[j + 1];
      }
      error_handler_count--;
      return ONVIF_SUCCESS;
    }
  }
  
  return ONVIF_ERROR_NOT_FOUND;
}

const char *error_get_message_for_pattern(error_pattern_t pattern) {
  const error_pattern_def_t *def = find_error_pattern(pattern);
  return def ? def->message : "Unknown error pattern";
}

const char *error_get_soap_fault_code_for_pattern(error_pattern_t pattern) {
  const error_pattern_def_t *def = find_error_pattern(pattern);
  return def ? def->soap_fault_code : SOAP_FAULT_RECEIVER;
}

int error_should_log(const error_context_t *context, const error_result_t *result) {
  if (!context || !result) {
    return 0;
  }
  
  // Log all errors by default, but this could be made configurable
  return 1;
}

int error_handle_multiple(const error_context_t *context, const error_pattern_t *errors,
                         size_t error_count, onvif_response_t *response) {
  if (!context || !errors || !response || error_count == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Handle the first error
  int result = error_handle_pattern(context, errors[0], NULL, response);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  
  // Log additional errors
  for (size_t i = 1; i < error_count; i++) {
    error_result_t error_result;
    if (error_create_result_from_pattern(errors[i], NULL, &error_result) == ONVIF_SUCCESS) {
      error_log_with_context(context, &error_result, "Additional error in sequence");
    }
  }
  
  return ONVIF_SUCCESS;
}

int error_create_summary(const error_context_t *context, const error_result_t *result,
                        char *summary, size_t summary_size) {
  if (!context || !result || !summary) {
    return ONVIF_ERROR_INVALID;
  }
  
  int ret = snprintf(summary, summary_size,
    "[%s::%s] %s (Code: %d, SOAP: %s)",
    context->service_name,
    context->action_name,
    result->error_message,
    result->error_code,
    result->soap_fault_code);
  
  if (context->error_context) {
    snprintf(summary + strlen(summary), summary_size - strlen(summary),
      " [Context: %s]", context->error_context);
  }
  
  return (ret < 0 || (size_t)ret >= summary_size) ? ONVIF_ERROR : ONVIF_SUCCESS;
}
