/**
 * @file onvif_service_common.c
 * @brief Common ONVIF service utility implementations
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_service_common.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "protocol/gsoap/onvif_gsoap_response.h"

/* ============================================================================
 * Common Utility Function Implementations
 * ============================================================================
 */

/**
 * @brief Standard parameter validation callback
 */
int onvif_util_validate_standard_parameters(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx) {
  // Basic validation - check for required parameters
  if (!config || !request || !response || !gsoap_ctx) {
    return ONVIF_ERROR;
  }
  return ONVIF_SUCCESS;
}

/**
 * @brief Standard post-processing callback
 */
int onvif_util_standard_post_process(http_response_t* response, service_log_context_t* log_ctx) {
  // Standard post-processing - ensure response is properly formatted
  if (!response) {
    return ONVIF_ERROR;
  }

  // Set default content type if not set
  if (!response->content_type) {
    response->content_type = "application/soap+xml";
  }

  // Set default status code if not set
  if (response->status_code == 0) {
    response->status_code = 200;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Generic ONVIF service handler with enhanced callback pattern
 */
int onvif_util_handle_service_request(const service_handler_config_t* config,
                                      const http_request_t* request, http_response_t* response,
                                      onvif_gsoap_context_t* gsoap_ctx,
                                      const onvif_service_operation_t* operation,
                                      int (*soap_callback)(struct soap* soap, void* user_data),
                                      void* callback_data) {
  if (!config || !request || !response || !gsoap_ctx || !operation) {
    return ONVIF_ERROR;
  }

  // Initialize contexts
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, operation->service_name, operation->operation_name,
                           SERVICE_LOG_INFO);

  error_context_t error_ctx;
  error_context_init(&error_ctx, operation->service_name, operation->operation_name,
                     operation->operation_context);

  // Validate parameters
  if (operation->callbacks.validate_parameters) {
    int result = operation->callbacks.validate_parameters(config, request, response, gsoap_ctx,
                                                          &log_ctx, &error_ctx);
    if (result != ONVIF_SUCCESS) {
      return result;
    }
  }

  // Execute business logic
  if (operation->callbacks.execute_business_logic) {
    int result = operation->callbacks.execute_business_logic(config, request, response, gsoap_ctx,
                                                             &log_ctx, &error_ctx, callback_data);
    if (result != ONVIF_SUCCESS) {
      return result;
    }
  }

  // Generate SOAP response using callback (only if business logic didn't already set response body)
  if (soap_callback && !response->body) {
    int result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, soap_callback, callback_data);
    if (result != ONVIF_SUCCESS) {
      service_log_operation_failure(&log_ctx, "soap_response_generation", result,
                                    "Failed to generate SOAP response");
      return result;
    }

    // Copy the generated SOAP response to the HTTP response
    if (gsoap_ctx->soap.os) {
      char** output_string = (char**)gsoap_ctx->soap.os;
      if (*output_string) {
        response->body = *output_string;
        response->body_length = strlen(*output_string);
        response->status_code = 200;
      }
    }
  }

  // Post-process response
  if (operation->callbacks.post_process_response) {
    int result = operation->callbacks.post_process_response(response, &log_ctx);
    if (result != ONVIF_SUCCESS) {
      return result;
    }
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Get configuration string with fallback and error handling
 */
int onvif_util_get_config_string_with_fallback(
  onvif_service_handler_instance_t* handler, config_section_t section, const char* key, char* value,
  size_t value_size, const char* default_value, service_log_context_t* log_ctx,
  error_context_t* error_ctx, http_response_t* response, const char* config_name) {
  if (!handler || !key || !value || !default_value) {
    return ONVIF_ERROR;
  }

  // Try to get from config, fallback to default
  const char* config_value = default_value;
  // TODO: Implement actual config lookup
  // For now, use default value

  // Copy to output buffer
  strncpy(value, config_value, value_size - 1);
  value[value_size - 1] = '\0';

  return ONVIF_SUCCESS;
}

/**
 * @brief Get configuration integer with fallback and error handling
 */
int onvif_util_get_config_int_with_fallback(onvif_service_handler_instance_t* handler,
                                            config_section_t section, const char* key, int* value,
                                            int default_value, service_log_context_t* log_ctx,
                                            error_context_t* error_ctx, http_response_t* response,
                                            const char* config_name) {
  if (!handler || !key || !value) {
    return ONVIF_ERROR;
  }

  // Try to get from config, fallback to default
  *value = default_value;
  // TODO: Implement actual config lookup
  // For now, use default value

  return ONVIF_SUCCESS;
}

/**
 * @brief Convert service type to string
 */
const char* onvif_service_type_to_string(onvif_service_type_t service) {
  switch (service) {
  case ONVIF_SERVICE_DEVICE:
    return "Device";
  case ONVIF_SERVICE_MEDIA:
    return "Media";
  case ONVIF_SERVICE_PTZ:
    return "PTZ";
  case ONVIF_SERVICE_IMAGING:
    return "Imaging";
  case ONVIF_SERVICE_SNAPSHOT:
    return "Snapshot";
  default:
    return "Unknown";
  }
}

/**
 * @brief Convert action type to string
 */
/* onvif_action_type_to_string function removed - using string names directly */
