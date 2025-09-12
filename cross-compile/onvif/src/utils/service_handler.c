/**
 * @file service_handler.c
 * @brief Common service request handling utilities for ONVIF services
 * 
 * This module provides both legacy and refactored service handling patterns
 * for backward compatibility and improved code organization.
 */

#include "service_handler.h"
#include "utils/error_handling.h"
#include "utils/logging_utils.h"
#include "utils/safe_string.h"
#include "utils/common_error_handling.h"
#include "platform/platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>


/* Legacy service handler functions (still needed by other services) */

int onvif_handle_service_request(onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response,
                                 onvif_service_handler_t handler) {
  if (!request || !response || !handler) {
    log_invalid_parameters("onvif_handle_service_request");
    return ONVIF_ERROR_INVALID;
  }
  
  // Initialize response with standard defaults
  if (onvif_init_service_response(response) != 0) {
    return ONVIF_ERROR;
  }
  
  // Call the service-specific handler
  int result = handler(action, request, response);
  
  // If handler failed, generate appropriate error response
  if (result < 0) {
    onvif_handle_service_error(response, "Service handler failed");
    return response->body_length;
  }
  
  return result;
}

int onvif_init_service_response(onvif_response_t *response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_init(response, ONVIF_RESPONSE_BUFFER_SIZE);
}

int onvif_handle_unsupported_action(onvif_response_t *response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_soap_fault(response, "soap:Receiver", "Unsupported action");
}

int onvif_handle_missing_parameter(onvif_response_t *response, const char *param_name) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char error_message[256];
  snprintf(error_message, sizeof(error_message), "Missing required parameter: %s", 
           param_name ? param_name : "unknown");
  
  return onvif_response_soap_fault(response, "soap:Sender", error_message);
}

int onvif_handle_service_error(onvif_response_t *response, const char *error_message) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_soap_fault(response, "soap:Receiver", 
                                   error_message ? error_message : "Service error");
}

/* Service handler implementation */

static service_stats_t g_service_stats = {0};

int service_handler_init(service_handler_t *handler,
                        const service_handler_config_t *config,
                        const service_action_def_t *actions,
                        size_t action_count) {
  if (!handler || !config || !actions || action_count == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  memset(handler, 0, sizeof(service_handler_t));
  
  // Copy configuration
  handler->config = *config;
  
  // Allocate and copy actions
  handler->actions = malloc(action_count * sizeof(service_action_def_t));
  if (!handler->actions) {
    return ONVIF_ERROR;
  }
  
  memcpy(handler->actions, actions, action_count * sizeof(service_action_def_t));
  handler->action_count = action_count;
  
  // Initialize XML builder
  if (xml_builder_init(&handler->xml_builder, handler->xml_buffer, sizeof(handler->xml_buffer)) != ONVIF_SUCCESS) {
    free(handler->actions);
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

void service_handler_cleanup(service_handler_t *handler) {
  if (handler) {
    if (handler->actions) {
      free(handler->actions);
      handler->actions = NULL;
    }
    xml_builder_cleanup(&handler->xml_builder);
    memset(handler, 0, sizeof(service_handler_t));
  }
}

int service_handler_validate_request(service_handler_t *handler,
                                               const onvif_request_t *request,
                                               const char **required_params,
                                               size_t param_count) {
  if (!handler || !request || !required_params) {
    return ONVIF_ERROR_INVALID;
  }
  
  if (!handler->config.enable_validation) {
    return ONVIF_SUCCESS;
  }
  
  error_context_t error_context;
  error_context_init(&error_context, handler->config.service_name, "validate_request", NULL);
  
  for (size_t i = 0; i < param_count; i++) {
    if (!required_params[i]) continue;
    
    // Check if parameter exists in request body
    char search_pattern[128];
    snprintf(search_pattern, sizeof(search_pattern), "<%s>", required_params[i]);
    
    if (strstr(request->body, search_pattern) == NULL) {
      // Parameter not found
      onvif_response_t dummy_response;
      memset(&dummy_response, 0, sizeof(dummy_response));
      
      error_handle_parameter(&error_context, required_params[i], "missing", &dummy_response);
      return ONVIF_ERROR_INVALID;
    }
  }
  
  return ONVIF_SUCCESS;
}

int service_handler_generate_success(service_handler_t *handler,
                                               const char *action_name,
                                               const char *xml_content,
                                               onvif_response_t *response) {
  if (!handler || !action_name || !xml_content || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Generate success response using unified SOAP generator
  int result = onvif_generate_complete_response(response, handler->config.service_type,
                                               action_name, xml_content);
  
  if (result == ONVIF_SUCCESS) {
    g_service_stats.total_success++;
  }
  
  return result;
}

int service_handler_generate_error(service_handler_t *handler,
                                             const char *action_name,
                                             error_pattern_t error_pattern,
                                             const char *error_message,
                                             onvif_response_t *response) {
  if (!handler || !action_name || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  error_context_t error_context;
  error_context_init(&error_context, handler->config.service_name, action_name, NULL);
  
  int result = error_handle_pattern(&error_context, error_pattern, error_message, response);
  
  if (result == ONVIF_SUCCESS) {
    g_service_stats.total_errors++;
  }
  
  return result;
}

int service_handler_handle_request(service_handler_t *handler,
                                             onvif_action_type_t action,
                                             const onvif_request_t *request,
                                             onvif_response_t *response) {
  if (!handler || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }
  
  g_service_stats.total_requests++;
  
  // Find action handler
  service_action_def_t *action_def = NULL;
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action) {
      action_def = &handler->actions[i];
      break;
    }
  }
  
  if (!action_def) {
    return service_handler_generate_error(handler, "unknown_action",
                                                   ERROR_PATTERN_NOT_SUPPORTED,
                                                   "Action not supported", response);
  }
  
  // Update action statistics
  for (size_t i = 0; i < g_service_stats.action_stats_count; i++) {
    if (g_service_stats.action_stats[i].action_type == action) {
      g_service_stats.action_stats[i].call_count++;
      break;
    }
  }
  
  // Validate request if required
  if (action_def->requires_validation) {
    // This would need to be customized per action
    // For now, just do basic validation
    if (strlen(request->body) == 0) {
      return service_handler_generate_error(handler, action_def->action_name,
                                                     ERROR_PATTERN_INVALID_PARAMETER,
                                                     "Empty request body", response);
    }
  }
  
  // Reset XML builder for new operation
  if (service_handler_reset_xml_builder(handler) != ONVIF_SUCCESS) {
    return service_handler_generate_error(handler, action_def->action_name,
                                                   ERROR_PATTERN_INTERNAL_ERROR,
                                                   "Failed to reset XML builder", response);
  }
  
  // Call action handler
  int result = action_def->handler(&handler->config, request, response, &handler->xml_builder);
  
  if (result != ONVIF_SUCCESS) {
    // Update error statistics
    for (size_t i = 0; i < g_service_stats.action_stats_count; i++) {
      if (g_service_stats.action_stats[i].action_type == action) {
        g_service_stats.action_stats[i].error_count++;
        break;
      }
    }
  }
  
  return result;
}

int service_handler_get_config_value(service_handler_t *handler,
                                               config_section_t section,
                                               const char *key,
                                               void *value_ptr,
                                               config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }
  
  if (!handler->config.config) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  return centralized_config_get_value(handler->config.config, section, key, value_ptr, value_type);
}

int service_handler_set_config_value(service_handler_t *handler,
                                               config_section_t section,
                                               const char *key,
                                               const void *value_ptr,
                                               config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }
  
  if (!handler->config.config) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  return centralized_config_set_value(handler->config.config, section, key, value_ptr, value_type);
}

void service_handler_log(service_handler_t *handler,
                                   const char *action_name,
                                   const char *message,
                                   int level) {
  if (!handler || !action_name || !message) {
    return;
  }
  
  if (!handler->config.enable_logging) {
    return;
  }
  
  switch (level) {
    case 0: // Error
      platform_log_error("[%s::%s] %s", handler->config.service_name, action_name, message);
      break;
    case 1: // Warning
      platform_log_warning("[%s::%s] %s", handler->config.service_name, action_name, message);
      break;
    case 2: // Info
      platform_log_info("[%s::%s] %s", handler->config.service_name, action_name, message);
      break;
    case 3: // Debug
      platform_log_debug("[%s::%s] %s", handler->config.service_name, action_name, message);
      break;
    default:
      platform_log_notice("[%s::%s] %s", handler->config.service_name, action_name, message);
      break;
  }
}

xml_builder_t *service_handler_get_xml_builder(service_handler_t *handler) {
  return handler ? &handler->xml_builder : NULL;
}

int service_handler_reset_xml_builder(service_handler_t *handler) {
  if (!handler) {
    return ONVIF_ERROR_INVALID;
  }
  
  xml_builder_cleanup(&handler->xml_builder);
  return xml_builder_init(&handler->xml_builder, handler->xml_buffer, sizeof(handler->xml_buffer));
}

int service_handler_get_stats(service_handler_t *handler,
                                        service_stats_t *stats) {
  if (!handler || !stats) {
    return ONVIF_ERROR_INVALID;
  }
  
  *stats = g_service_stats;
  return ONVIF_SUCCESS;
}

int service_handler_register_action(service_handler_t *handler,
                                              const service_action_def_t *action_def) {
  if (!handler || !action_def) {
    return ONVIF_ERROR_INVALID;
  }
  
  if (handler->action_count >= MAX_ACTIONS) {
    return ONVIF_ERROR;
  }
  
  // Check if action already exists
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action_def->action_type) {
      // Update existing action
      handler->actions[i] = *action_def;
      return ONVIF_SUCCESS;
    }
  }
  
  // Add new action
  handler->actions[handler->action_count] = *action_def;
  handler->action_count++;
  
  return ONVIF_SUCCESS;
}

int service_handler_unregister_action(service_handler_t *handler,
                                                onvif_action_type_t action_type) {
  if (!handler) {
    return ONVIF_ERROR_INVALID;
  }
  
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action_type) {
      // Move remaining actions up
      for (size_t j = i; j < handler->action_count - 1; j++) {
        handler->actions[j] = handler->actions[j + 1];
      }
      handler->action_count--;
      return ONVIF_SUCCESS;
    }
  }
  
  return ONVIF_ERROR_NOT_FOUND;
}
