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
#include "generated/soapH.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

/* ============================================================================
 * Service Handler Management
 * ============================================================================
 */

int onvif_service_handler_init(onvif_service_handler_instance_t* handler,
                               const service_handler_config_t* config,
                               const service_action_def_t* actions, size_t action_count) {
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

  memcpy(handler->actions, actions, action_count * sizeof(service_action_def_t));
  handler->action_count = action_count;

  // Initialize gSOAP context
  handler->gsoap_ctx = ONVIF_MALLOC(sizeof(onvif_gsoap_context_t));
  if (!handler->gsoap_ctx) {
    ONVIF_FREE(handler->actions);
    return ONVIF_ERROR;
  }

  if (onvif_gsoap_init(handler->gsoap_ctx) != ONVIF_XML_SUCCESS) {
    ONVIF_FREE(handler->gsoap_ctx);
    ONVIF_FREE(handler->actions);
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_handle_request(onvif_service_handler_instance_t* handler,
                                         const char* action_name, const http_request_t* request,
                                         http_response_t* response) {
  if (!handler || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Find action handler by action name
  service_action_def_t* action_def = NULL;
  for (size_t i = 0; i < handler->action_count; i++) {
    if (strcmp(handler->actions[i].action_name, action_name) == 0) {
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
  platform_log_info("Service Handler: Calling action handler for action %s\n", action_name);
  clock_t start_time = clock();
  int result = action_def->handler(&handler->config, request, response, handler->gsoap_ctx);
  clock_t end_time = clock();
  platform_log_info("Service Handler: Action handler completed with result %d\n", result);
  platform_log_info("Service Handler: Processing time: %ld ms\n",
                    (end_time - start_time) / (CLOCKS_PER_SEC / 1000));

  // Update action statistics
  for (size_t i = 0; i < handler->stats.action_stats_count; i++) {
    if (strcmp(handler->stats.action_stats[i].action_name, action_name) == 0) {
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

void onvif_service_handler_cleanup(onvif_service_handler_instance_t* handler) {
  if (handler) {
    if (handler->actions) {
      ONVIF_FREE(handler->actions);
      handler->actions = NULL;
    }

    if (handler->gsoap_ctx) {
      onvif_gsoap_cleanup(handler->gsoap_ctx);
      ONVIF_FREE(handler->gsoap_ctx);
      handler->gsoap_ctx = NULL;
    }
    handler->action_count = 0;
  }
}

int onvif_service_handler_validate_request(onvif_service_handler_instance_t* handler,
                                           const http_request_t* request,
                                           const char** required_params, size_t param_count) {
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

int onvif_service_handler_generate_success(onvif_service_handler_instance_t* handler,
                                           const char* action, const char* body_content,
                                           http_response_t* response) {
  if (!handler || !action || !body_content || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Set basic response data (specific gSOAP response generation is handled by
  // service handlers)
  response->status_code = 200;
  response->body = (char*)body_content;
  response->body_length = strlen(body_content);
  response->content_type = "application/soap+xml";

  // Log success
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action, "Request processed successfully", 0);
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_generate_error(onvif_service_handler_instance_t* handler,
                                         const char* action_name, error_pattern_t error_pattern,
                                         const char* error_message, http_response_t* response) {
  if (!handler || !action_name || !error_message || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Set error response data
  response->status_code = 500;
  response->body = (char*)error_message;
  response->body_length = strlen(error_message);
  response->content_type = "application/soap+xml";

  // Log error
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action_name, error_message, 1);
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_get_config_value(onvif_service_handler_instance_t* handler,
                                           config_section_t section, const char* key,
                                           void* value_ptr, config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // TODO: Implement configuration value retrieval
  // This would integrate with the config manager
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int onvif_service_handler_set_config_value(onvif_service_handler_instance_t* handler,
                                           config_section_t section, const char* key,
                                           const void* value_ptr, config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // TODO: Implement configuration value setting
  // This would integrate with the config manager
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

void onvif_service_handler_log(onvif_service_handler_instance_t* handler, const char* action_name,
                               const char* message, int level) {
  if (!handler || !action_name || !message) {
    return;
  }

  // Log using platform logging
  const char* level_str = (level == 0) ? "INFO" : "ERROR";
  platform_log_info("[%s] %s: %s", handler->config.service_name, action_name, message);
}

onvif_gsoap_context_t* onvif_service_handler_get_gsoap_context(
  onvif_service_handler_instance_t* handler) {
  if (handler) {
    return handler->gsoap_ctx;
  }
  return NULL;
}

int onvif_service_handler_reset_xml_builder(onvif_service_handler_instance_t* handler) {
  if (!handler || !handler->gsoap_ctx) {
    return ONVIF_ERROR_INVALID;
  }

  // Reset gSOAP context to initial state (more efficient than cleanup/init)
  onvif_gsoap_reset(handler->gsoap_ctx);
  return ONVIF_SUCCESS;
}

int onvif_service_handler_get_stats(onvif_service_handler_instance_t* handler,
                                    service_stats_t* stats) {
  if (!handler || !stats) {
    return ONVIF_ERROR_INVALID;
  }

  *stats = handler->stats;
  return ONVIF_SUCCESS;
}

int onvif_service_handler_register_action(onvif_service_handler_instance_t* handler,
                                          const service_action_def_t* action_def) {
  if (!handler || !action_def) {
    return ONVIF_ERROR_INVALID;
  }

  // Check if action already exists
  for (size_t i = 0; i < handler->action_count; i++) {
    if (strcmp(handler->actions[i].action_name, action_def->action_name) == 0) {
      return ONVIF_ERROR; // Action already exists
    }
  }

  // Reallocate actions array
  size_t new_count = handler->action_count + 1;
  service_action_def_t* new_actions =
    ONVIF_REALLOC(handler->actions, new_count * sizeof(service_action_def_t));
  if (!new_actions) {
    return ONVIF_ERROR;
  }

  handler->actions = new_actions;
  handler->actions[handler->action_count] = *action_def;
  handler->action_count = new_count;

  return ONVIF_SUCCESS;
}

int onvif_service_handler_unregister_action(onvif_service_handler_instance_t* handler,
                                            const char* action_name) {
  if (!handler) {
    return ONVIF_ERROR_INVALID;
  }

  // Find and remove action
  for (size_t i = 0; i < handler->action_count; i++) {
    if (strcmp(handler->actions[i].action_name, action_name) == 0) {
      // Shift remaining actions
      for (size_t j = i; j < handler->action_count - 1; j++) {
        handler->actions[j] = handler->actions[j + 1];
      }

      handler->action_count--;
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR; // Action not found
}

/* ============================================================================
 * Legacy Service Handler Functions
 * ============================================================================
 */

int onvif_handle_service_request(const char* action_name, const http_request_t* request,
                                 http_response_t* response, onvif_service_handler_t handler) {
  if (!request || !response || !handler) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response
  if (onvif_init_service_response(response) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Call service handler
  int result = handler(action_name, request, response);

  // Clean up on error
  if (result < 0) {
    // Clear response data on error
    if (response) {
      response->body = NULL;
      response->body_length = 0;
      response->status_code = 0;
      response->content_type = NULL;
    }
  }

  return result;
}

int onvif_init_service_response(http_response_t* response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response structure
  response->status_code = 200;
  response->content_type = "application/soap+xml";
  response->body = NULL; // Will be allocated by smart response builder
  response->body_length = 0;
  return ONVIF_SUCCESS;
}

int onvif_handle_unsupported_action(http_response_t* response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  // Set error response for unsupported action
  response->status_code = 400;
  response->body = (char*)"Unsupported action";
  response->body_length = strlen("Unsupported action");
  response->content_type = "application/soap+xml";
  return ONVIF_SUCCESS;
}

int onvif_handle_missing_parameter(http_response_t* response, const char* param_name) {
  if (!response || !param_name) {
    return ONVIF_ERROR_INVALID;
  }

  char error_message[256];
  int ret =
    snprintf(error_message, sizeof(error_message), "Missing required parameter: %s", param_name);
  (void)ret; // Explicitly ignore return value to silence clang-tidy

  // Set error response for missing parameter
  response->status_code = 400;
  response->body = error_message;
  response->body_length = strlen(error_message);
  response->content_type = "application/soap+xml";
  return ONVIF_SUCCESS;
}

int onvif_handle_service_error(http_response_t* response, const char* error_message) {
  if (!response || !error_message) {
    return ONVIF_ERROR_INVALID;
  }

  // Set error response for service error
  response->status_code = 500;
  response->body = (char*)error_message;
  response->body_length = strlen(error_message);
  response->content_type = "application/soap+xml";
  return ONVIF_SUCCESS;
}
