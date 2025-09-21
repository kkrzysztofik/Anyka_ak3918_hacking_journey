/**
 * @file onvif_service_handler.c
 * @brief Unified service request handling utilities implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module consolidates all service request handling functionality to
 * eliminate duplication and provide a single, consistent API for service
 * operations.
 */

#include "onvif_service_handler.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "platform/platform.h"
#include "protocol/soap/onvif_soap.h"
#include "protocol/xml/unified_xml.h"
#include "services/common/onvif_request.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

/* ============================================================================
 * Service Handler Management
 * ============================================================================
 */

int onvif_service_handler_init(onvif_service_handler_instance_t *handler,
                               const service_handler_config_t *config,
                               const service_action_def_t *actions,
                               size_t action_count) {
  if (!handler || !config || !actions || action_count == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize handler structure
  memset(handler, 0, sizeof(*handler));

  // Copy configuration
  handler->config = *config;

  // Allocate and copy actions
  handler->actions = ONVIF_MALLOC(action_count * sizeof(service_action_def_t));
  if (!handler->actions) {
    return ONVIF_ERROR;
  }

  memcpy(handler->actions, actions,
         action_count * sizeof(service_action_def_t));
  handler->action_count = action_count;

  // Initialize XML builder
  if (onvif_xml_builder_init(&handler->xml_builder, handler->xml_buffer,
                             sizeof(handler->xml_buffer),
                             NULL) != ONVIF_SUCCESS) {
    ONVIF_FREE(handler->actions);
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_handle_request(
    onvif_service_handler_instance_t *handler, onvif_action_type_t action,
    const onvif_request_t *request, onvif_response_t *response) {
  if (!handler || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Find action handler
  service_action_def_t *action_def = NULL;
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action) {
      action_def = &handler->actions[i];
      break;
    }
  }

  if (!action_def) {
    return onvif_handle_unsupported_action(response);
  }

  // Update statistics
  handler->stats.total_requests++;

  // Reset XML builder for new operation
  if (onvif_service_handler_reset_xml_builder(handler) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Call action handler
  clock_t start_time = clock();
  int result = action_def->handler(&handler->config, request, response,
                                   &handler->xml_builder);
  clock_t end_time = clock();

  // Update action statistics
  for (size_t i = 0; i < handler->stats.action_stats_count; i++) {
    if (handler->stats.action_stats[i].action_type == action) {
      handler->stats.action_stats[i].call_count++;
      if (result != ONVIF_SUCCESS) {
        handler->stats.action_stats[i].error_count++;
        handler->stats.total_errors++;
      } else {
        handler->stats.total_success++;
      }

      // Update average response time
      double response_time = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;
      handler->stats.action_stats[i].avg_response_time =
          (handler->stats.action_stats[i].avg_response_time *
               (handler->stats.action_stats[i].call_count - 1) +
           response_time) /
          handler->stats.action_stats[i].call_count;
      break;
    }
  }

  return result;
}

void onvif_service_handler_cleanup(onvif_service_handler_instance_t *handler) {
  if (handler) {
    if (handler->actions) {
      ONVIF_FREE(handler->actions);
      handler->actions = NULL;
    }

    onvif_xml_builder_cleanup(&handler->xml_builder);
    handler->action_count = 0;
  }
}

int onvif_service_handler_validate_request(
    onvif_service_handler_instance_t *handler, const onvif_request_t *request,
    const char **required_params, size_t param_count) {
  if (!handler || !request || !required_params) {
    return ONVIF_ERROR_INVALID;
  }

  // Basic validation - check if request body exists
  if (!request->body || request->body_length == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check required parameters
  for (size_t i = 0; i < param_count; i++) {
    if (!required_params[i]) {
      continue;
    }

    // Simple check for parameter presence in request body
    if (strstr(request->body, required_params[i]) == NULL) {
      return ONVIF_ERROR_INVALID;
    }
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_generate_success(
    onvif_service_handler_instance_t *handler, const char *action,
    const char *body_content, onvif_response_t *response) {
  if (!handler || !action || !body_content || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Generate SOAP response
  int result = onvif_generate_complete_response(
      response, handler->config.service_type, action, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Log success
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action, "Request processed successfully",
                              0);
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_generate_error(
    onvif_service_handler_instance_t *handler, const char *action_name,
    error_pattern_t error_pattern, const char *error_message,
    onvif_response_t *response) {
  if (!handler || !action_name || !error_message || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Generate error response
  int result = onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                             error_message);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Log error
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action_name, error_message, 1);
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_get_config_value(
    onvif_service_handler_instance_t *handler, config_section_t section,
    const char *key, void *value_ptr, config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // TODO: Implement configuration value retrieval
  // This would integrate with the config manager
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int onvif_service_handler_set_config_value(
    onvif_service_handler_instance_t *handler, config_section_t section,
    const char *key, const void *value_ptr, config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // TODO: Implement configuration value setting
  // This would integrate with the config manager
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

void onvif_service_handler_log(onvif_service_handler_instance_t *handler,
                               const char *action_name, const char *message,
                               int level) {
  if (!handler || !action_name || !message) {
    return;
  }

  // Log using platform logging
  const char *level_str = (level == 0) ? "INFO" : "ERROR";
  platform_log_info("[%s] %s: %s", handler->config.service_name, action_name,
                    message);
}

onvif_xml_builder_t *onvif_service_handler_get_xml_builder(
    onvif_service_handler_instance_t *handler) {
  return handler ? &handler->xml_builder : NULL;
}

int onvif_service_handler_reset_xml_builder(
    onvif_service_handler_instance_t *handler) {
  if (!handler) {
    return ONVIF_ERROR_INVALID;
  }

  // Reset XML builder
  onvif_xml_builder_cleanup(&handler->xml_builder);
  return onvif_xml_builder_init(&handler->xml_builder, handler->xml_buffer,
                                sizeof(handler->xml_buffer), NULL);
}

int onvif_service_handler_get_stats(onvif_service_handler_instance_t *handler,
                                    service_stats_t *stats) {
  if (!handler || !stats) {
    return ONVIF_ERROR_INVALID;
  }

  *stats = handler->stats;
  return ONVIF_SUCCESS;
}

int onvif_service_handler_register_action(
    onvif_service_handler_instance_t *handler,
    const service_action_def_t *action_def) {
  if (!handler || !action_def) {
    return ONVIF_ERROR_INVALID;
  }

  // Check if action already exists
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action_def->action_type) {
      return ONVIF_ERROR;  // Action already exists
    }
  }

  // Reallocate actions array
  size_t new_count = handler->action_count + 1;
  service_action_def_t *new_actions =
      ONVIF_REALLOC(handler->actions, new_count * sizeof(service_action_def_t));
  if (!new_actions) {
    return ONVIF_ERROR;
  }

  handler->actions = new_actions;
  handler->actions[handler->action_count] = *action_def;
  handler->action_count = new_count;

  return ONVIF_SUCCESS;
}

int onvif_service_handler_unregister_action(
    onvif_service_handler_instance_t *handler,
    onvif_action_type_t action_type) {
  if (!handler) {
    return ONVIF_ERROR_INVALID;
  }

  // Find and remove action
  for (size_t i = 0; i < handler->action_count; i++) {
    if (handler->actions[i].action_type == action_type) {
      // Shift remaining actions
      for (size_t j = i; j < handler->action_count - 1; j++) {
        handler->actions[j] = handler->actions[j + 1];
      }

      handler->action_count--;
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR;  // Action not found
}

/* ============================================================================
 * Legacy Service Handler Functions
 * ============================================================================
 */

int onvif_handle_service_request(onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response,
                                 onvif_service_handler_t handler) {
  if (!request || !response || !handler) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response
  if (onvif_init_service_response(response) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Call service handler
  int result = handler(action, request, response);

  // Clean up on error
  if (result < 0) {
    onvif_response_cleanup(response);
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

  return onvif_generate_fault_response(response, SOAP_FAULT_SENDER,
                                       "Unsupported action");
}

int onvif_handle_missing_parameter(onvif_response_t *response,
                                   const char *param_name) {
  if (!response || !param_name) {
    return ONVIF_ERROR_INVALID;
  }

  char error_message[256];
  int ret = snprintf(error_message, sizeof(error_message),
                     "Missing required parameter: %s", param_name);
  (void)ret;  // Explicitly ignore return value to silence clang-tidy

  return onvif_generate_fault_response(response, SOAP_FAULT_SENDER,
                                       error_message);
}

int onvif_handle_service_error(onvif_response_t *response,
                               const char *error_message) {
  if (!response || !error_message) {
    return ONVIF_ERROR_INVALID;
  }

  return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                       error_message);
}
