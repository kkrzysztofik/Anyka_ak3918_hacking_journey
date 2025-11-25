/**
 * @file error_handling.c
 * @brief Simplified unified error handling system implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "error_handling.h"

#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "utils/error/error_translation.h"
#include "utils/memory/memory_manager.h"

/* ============================================================================
 * Constants and Types
 * ============================================================================ */

/* Error message buffer sizes */
#define ERROR_MESSAGE_BUFFER_SIZE 256  /* Standard error message buffer */
#define ERROR_DETAIL_BUFFER_SIZE  512  /* Detailed error message buffer */
#define ERROR_CONTEXT_BUFFER_SIZE 1024 /* Extended context buffer */

/* SOAP Fault Codes - using constants from common/onvif_constants.h */

typedef struct {
  error_pattern_t pattern;
  const char* message;
  const char* soap_fault_code;
  const char* soap_fault_string;
} error_pattern_def_t;

static const error_pattern_def_t error_patterns[] = {
  {ERROR_PATTERN_VALIDATION_FAILED, "Validation failed", SOAP_FAULT_SENDER, "Validation failed"},
  {ERROR_PATTERN_NOT_FOUND, "Resource not found", SOAP_FAULT_SENDER, "Resource not found"},
  {ERROR_PATTERN_NOT_SUPPORTED, "Operation not supported", SOAP_FAULT_SENDER, "Operation not supported"},
  {ERROR_PATTERN_INTERNAL_ERROR, "Internal server error", SOAP_FAULT_RECEIVER, "Internal server error"},
  {ERROR_PATTERN_INVALID_PARAMETER, "Invalid parameter", SOAP_FAULT_SENDER, "Invalid parameter"},
  {ERROR_PATTERN_MISSING_PARAMETER, "Missing required parameter", SOAP_FAULT_SENDER, "Missing required parameter"},
  {ERROR_PATTERN_AUTHENTICATION_FAILED, "Authentication failed", SOAP_FAULT_SENDER, "Authentication failed"},
  {ERROR_PATTERN_AUTHORIZATION_FAILED, "Authorization failed", SOAP_FAULT_SENDER, "Authorization failed"}};

/* ============================================================================
 * INTERNAL HELPERS - Error Pattern Lookup
 * ============================================================================ */

static const error_pattern_def_t* find_error_pattern(error_pattern_t pattern) {
  for (size_t i = 0; i < sizeof(error_patterns) / sizeof(error_patterns[0]); i++) {
    if (error_patterns[i].pattern == pattern) {
      return &error_patterns[i];
    }
  }
  return NULL;
}

/* ============================================================================
 * PUBLIC API - Error Context Management
 * ============================================================================ */

int error_context_init(error_context_t* context,
                       const char* service_name,    // NOLINT
                       const char* action_name,     // NOLINT
                       const char* error_context) { // NOLINT
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

int error_create_result_from_pattern(error_pattern_t pattern, const char* custom_message, error_result_t* result) {
  if (!result) {
    return ONVIF_ERROR_INVALID;
  }

  const error_pattern_def_t* def = find_error_pattern(pattern);
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

/* ============================================================================
 * PUBLIC API - Error Handling Functions
 * ============================================================================ */

int error_handle_pattern(const error_context_t* context, error_pattern_t pattern, const char* custom_message, http_response_t* response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Create error result
  error_result_t result;
  if (error_create_result_from_pattern(pattern, custom_message, &result) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Log error
  error_log_with_context(context, &result, NULL);

  // Generate SOAP Fault XML envelope
  const size_t soap_buffer_size = 4096;
  char* soap_fault_xml = (char*)ONVIF_MALLOC(soap_buffer_size);
  if (!soap_fault_xml) {
    return ONVIF_ERROR_MEMORY;
  }

  // Use custom message if provided, otherwise use default fault string
  const char* fault_message = custom_message ? custom_message : result.soap_fault_string;

  int xml_length = onvif_gsoap_generate_fault_response(NULL,                   // ctx - will create temporary context
                                                       result.soap_fault_code, // fault_code (e.g., "soap:Sender" or "soap:Receiver")
                                                       fault_message,          // fault_string (descriptive error message)
                                                       NULL,                   // fault_actor (optional)
                                                       NULL,                   // fault_detail (optional)
                                                       soap_fault_xml,         // output_buffer
                                                       soap_buffer_size        // buffer_size
  );

  if (xml_length < 0) {
    ONVIF_FREE(soap_fault_xml);
    return ONVIF_ERROR;
  }

  // Set HTTP response for SOAP fault
  // SOAP faults should return HTTP 200 with fault in body (per SOAP 1.2 spec)
  response->status_code = HTTP_STATUS_OK;
  response->content_type = memory_safe_strdup("application/soap+xml; charset=utf-8");
  if (!response->content_type) {
    ONVIF_FREE(soap_fault_xml);
    return ONVIF_ERROR_MEMORY;
  }
  response->body = soap_fault_xml;
  response->body_length = (size_t)xml_length;

  // Return special error code to indicate SOAP fault was generated
  return ONVIF_ERROR_SOAP_FAULT;
}

int error_handle_validation(const error_context_t* context, int validation_result, const char* field_name, http_response_t* response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }

  char custom_message[ERROR_MESSAGE_BUFFER_SIZE];
  (void)snprintf(custom_message, sizeof(custom_message), "Validation failed for field '%s' (code: %d)", field_name ? field_name : "unknown",
                 validation_result);

  return error_handle_pattern(context, ERROR_PATTERN_VALIDATION_FAILED, custom_message, response);
}

int error_handle_parameter(const error_context_t* context, const char* parameter_name, const char* error_type, http_response_t* response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }

  char custom_message[ERROR_MESSAGE_BUFFER_SIZE];
  (void)snprintf(custom_message, sizeof(custom_message), "Parameter error: %s for parameter '%s'", error_type ? error_type : "unknown error",
                 parameter_name ? parameter_name : "unknown");

  error_pattern_t pattern = ERROR_PATTERN_INVALID_PARAMETER;
  if (error_type && strstr(error_type, "missing") != NULL) {
    pattern = ERROR_PATTERN_MISSING_PARAMETER;
  }

  return error_handle_pattern(context, pattern, custom_message, response);
}

int error_handle_service(const error_context_t* context, int error_code, const char* error_message, http_response_t* response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }

  char custom_message[ERROR_MESSAGE_BUFFER_SIZE];
  (void)snprintf(custom_message, sizeof(custom_message), "Service error %d: %s", error_code, error_message ? error_message : "Unknown service error");

  return error_handle_pattern(context, ERROR_PATTERN_INTERNAL_ERROR, custom_message, response);
}

int error_handle_system(const error_context_t* context, int error_code, const char* operation, http_response_t* response) {
  if (!context || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Map error code to human-readable message
  const char* error_description = "Unknown error";
  switch (error_code) {
  case ONVIF_ERROR_NOT_FOUND:
    error_description = "Resource not found";
    break;
  case ONVIF_ERROR_NOT_SUPPORTED:
    error_description = "Operation not supported";
    break;
  case ONVIF_ERROR_DUPLICATE:
    error_description = "Resource already exists";
    break;
  case ONVIF_ERROR_INVALID:
    error_description = "Invalid parameter";
    break;
  case ONVIF_ERROR_MEMORY:
    error_description = "Memory allocation failed";
    break;
  case ONVIF_ERROR_NOT_IMPLEMENTED:
    error_description = "Feature not implemented";
    break;
  case ONVIF_ERROR:
  default:
    error_description = "Internal error";
    break;
  }

  char custom_message[ERROR_MESSAGE_BUFFER_SIZE];
  if (operation) {
    (void)snprintf(custom_message, sizeof(custom_message), "%s during %s", error_description, operation);
  } else {
    (void)snprintf(custom_message, sizeof(custom_message), "%s", error_description);
  }

  return error_handle_pattern(context, ERROR_PATTERN_INTERNAL_ERROR, custom_message, response);
}

/* ============================================================================
 * PUBLIC API - Logging Functions
 * ============================================================================ */

void onvif_log_error_context(const error_context_t* ctx) {
  if (!ctx) {
    return;
  }

  (void)platform_log_error("ERROR [%d (%s)] in %s() at %s:%d\n", ctx->error_code, onvif_error_to_string(ctx->error_code), ctx->function, ctx->file,
                           ctx->line);

  if (ctx->message[0] != '\0') {
    (void)platform_log_error("  Message: %s\n", ctx->message);
  }

  if (ctx->context[0] != '\0') {
    (void)platform_log_error("  Context: %s\n", ctx->context);
  }

  if (ctx->service_name && ctx->action_name) {
    (void)platform_log_error("  Service: %s::%s\n", ctx->service_name, ctx->action_name);
  }
}

void onvif_log_error_with_context(int error_code,
                                  const char* function, // NOLINT
                                  const char* file,
                                  int line, // NOLINT
                                  const char* message,
                                  ...) { // NOLINT
  error_context_t ctx;
  ERROR_CONTEXT_INIT(&ctx, error_code, function, file, line);

  if (message) {
    va_list args;
    va_start(args, message);
    (void)vsnprintf(ctx.message, sizeof(ctx.message), message, args);
    va_end(args);
  }

  onvif_log_error_context(&ctx);
}

/* ============================================================================
 * PUBLIC API - Utility Functions
 * ============================================================================ */

const char* error_get_message_for_pattern(error_pattern_t pattern) {
  const error_pattern_def_t* def = find_error_pattern(pattern);
  return def ? def->message : "Unknown error pattern";
}

const char* error_get_soap_fault_code_for_pattern(error_pattern_t pattern) {
  const error_pattern_def_t* def = find_error_pattern(pattern);
  return def ? def->soap_fault_code : SOAP_FAULT_RECEIVER;
}

int onvif_get_error_context_string(const error_context_t* ctx, char* buffer, size_t buffer_size) {
  if (!ctx || !buffer || buffer_size == 0) {
    return -1;
  }

  int len = snprintf(buffer, buffer_size, "ERROR [%d (%s)] in %s() at %s:%d", ctx->error_code, onvif_error_to_string(ctx->error_code), ctx->function,
                     ctx->file, ctx->line);

  if (len < 0 || (size_t)len >= buffer_size) {
    return -1;
  }

  if (ctx->message[0] != '\0') {
    int msg_len = snprintf(buffer + len, buffer_size - len, " - %s", ctx->message);
    if (msg_len < 0 || (size_t)(len + msg_len) >= buffer_size) {
      return -1;
    }
    len += msg_len;
  }

  if (ctx->context[0] != '\0') {
    int ctx_len = snprintf(buffer + len, buffer_size - len, " [%s]", ctx->context);
    if (ctx_len < 0 || (size_t)(len + ctx_len) >= buffer_size) {
      return -1;
    }
    len += ctx_len;
  }

  return len;
}

int error_should_log(const error_context_t* context, const error_result_t* result) {
  if (!context || !result) {
    return 0;
  }

  // Log all errors by default, but this could be made configurable
  return 1;
}

int error_create_summary(const error_context_t* context, const error_result_t* result, char* summary, size_t summary_size) {
  if (!context || !result || !summary) {
    return ONVIF_ERROR_INVALID;
  }

  int ret = snprintf(summary, summary_size, "[%s::%s] %s (Code: %d (%s), SOAP: %s)", context->service_name ? context->service_name : "Unknown",
                     context->action_name ? context->action_name : "Unknown", result->error_message, result->error_code,
                     onvif_error_to_string(result->error_code), result->soap_fault_code);

  if (context->error_context) {
    (void)snprintf(summary + strlen(summary), summary_size - strlen(summary), " [Context: %s]", context->error_context);
  }

  return (ret < 0 || (size_t)ret >= summary_size) ? ONVIF_ERROR : ONVIF_SUCCESS;
}

void error_log_with_context(const error_context_t* context, const error_result_t* result, const char* additional_info) {
  if (!context || !result) {
    return;
  }

  char summary[ERROR_DETAIL_BUFFER_SIZE];
  if (error_create_summary(context, result, summary, sizeof(summary)) == ONVIF_SUCCESS) {
    (void)platform_log_error("ERROR: %s", summary);
    if (additional_info) {
      (void)platform_log_error("Additional info: %s", additional_info);
    }
  }
}

/* ============================================================================
 * PUBLIC API - Standardized Error Handling
 * ============================================================================ */

int onvif_standardized_validation(const char* field_name, int validation_result, const char* error_context) {
  if (validation_result == ONVIF_VALIDATION_SUCCESS) {
    return ONVIF_VALIDATION_SUCCESS;
  }

  if (error_context) {
    platform_log_error("Validation failed for field '%s': %s\n", field_name, error_context);
  } else {
    platform_log_error("Validation failed for field '%s'\n", field_name);
  }

  return ONVIF_VALIDATION_FAILED;
}

int onvif_standardized_operation(const char* operation_name, int operation_result, const char* error_context) {
  if (operation_result == ONVIF_SUCCESS) {
    return ONVIF_SUCCESS;
  }

  if (error_context) {
    platform_log_error("Operation failed: %s - %s (error code: %d (%s))\n", operation_name, error_context, operation_result,
                       onvif_error_to_string(operation_result));
  } else {
    platform_log_error("Operation failed: %s (error code: %d (%s))\n", operation_name, operation_result, onvif_error_to_string(operation_result));
  }

  return ONVIF_ERROR;
}

void onvif_standardized_log_error(const char* function, const char* file, int line, const char* message, ...) {
  if (!function || !file || !message) {
    return;
  }

  va_list args;
  va_start(args, message);

  // Log with standardized format
  char full_message[ERROR_CONTEXT_BUFFER_SIZE];
  (void)vsnprintf(full_message, sizeof(full_message), message, args);
  (void)platform_log_error("[%s:%d] %s: %s\n", file, line, function, full_message);

  va_end(args);
}
