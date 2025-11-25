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
#include "core/config/config_runtime.h"
#include "networking/http/http_constants.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Clock conversion constant
#define HTTP_STATUS_CLOCKS_PER_MS 1000

// Buffer size constants
#define CONFIG_STRING_BUFFER_SIZE 256 /* Default buffer for config strings */
#define ERROR_MESSAGE_BUFFER_SIZE 256 /* Error message buffer size */

/* ============================================================================
 * Service Handler Management
 * ============================================================================
 */

int onvif_service_handler_init(onvif_service_handler_instance_t* handler, const service_handler_config_t* config, const service_action_def_t* actions,
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

  memcpy(handler->actions, actions, action_count * sizeof(service_action_def_t));
  handler->action_count = action_count;

  // Initialize gSOAP context
  handler->gsoap_ctx = ONVIF_MALLOC(sizeof(onvif_gsoap_context_t));
  if (!handler->gsoap_ctx) {
    ONVIF_FREE(handler->actions);
    return ONVIF_ERROR;
  }

  if (onvif_gsoap_init(handler->gsoap_ctx) != ONVIF_SUCCESS) {
    ONVIF_FREE(handler->gsoap_ctx);
    ONVIF_FREE(handler->actions);
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_handle_request(onvif_service_handler_instance_t* handler, const char* action_name, const http_request_t* request,
                                         http_response_t* response) {
  if (!handler || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response status code to unset value (0) - will be set by handler or default logic
  response->status_code = 0;

  // Find action handler by action name
  service_action_def_t* action_def = NULL;
  for (size_t i = 0; i < handler->action_count; i++) {
    if (strcmp(handler->actions[i].action_name, action_name) == 0) {
      action_def = &handler->actions[i];
      break;
    }
  }

  if (!action_def) {
    // Set error response for unsupported action
    response->status_code = HTTP_STATUS_BAD_REQUEST;
    response->body = (char*)"Unsupported action";
    response->body_length = strlen("Unsupported action");
    response->content_type = memory_safe_strdup("application/soap+xml");
    if (!response->content_type) {
      return ONVIF_ERROR_MEMORY;
    }
    return ONVIF_SUCCESS;
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
  platform_log_info("Service Handler: Processing time: %ld ms\n", (end_time - start_time) / (CLOCKS_PER_SEC / HTTP_STATUS_CLOCKS_PER_MS));

  // Set appropriate status code based on handler result and response
  if (result == ONVIF_SUCCESS) {
    // Handler succeeded - set to 200 if not already set by handler
    if (response->status_code == 0) {
      response->status_code = HTTP_STATUS_OK;
    }
  } else {
    // Handler failed - set to 500 if handler didn't set a specific error code
    if (response->status_code == 0) {
      response->status_code = HTTP_STATUS_INTERNAL_SERVER_ERROR;
    }
    // If handler set a specific error code (400, 404, etc.), respect it
  }

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
        (handler->stats.action_stats[i].avg_response_time * (handler->stats.action_stats[i].call_count - 1) + response_time) /
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

int onvif_service_handler_validate_request(onvif_service_handler_instance_t* handler, const http_request_t* request, const char** required_params,
                                           size_t param_count) {
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

int onvif_service_handler_generate_success(onvif_service_handler_instance_t* handler, const char* action, const char* body_content,
                                           http_response_t* response) {
  if (!handler || !action || !body_content || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Set basic response data (specific gSOAP response generation is handled by
  // service handlers)
  response->status_code = HTTP_STATUS_OK;
  response->body = (char*)body_content;
  response->body_length = strlen(body_content);
  response->content_type = memory_safe_strdup("application/soap+xml");
  if (!response->content_type) {
    return ONVIF_ERROR_MEMORY;
  }

  // Log success
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action, "Request processed successfully");
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_generate_error(onvif_service_handler_instance_t* handler, const char* action_name, error_pattern_t error_pattern,
                                         const char* error_message, http_response_t* response) {
  (void)error_pattern; // Reserved for future error pattern handling
  if (!handler || !action_name || !error_message || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Set error response data
  response->status_code = HTTP_STATUS_INTERNAL_SERVER_ERROR;
  response->body = (char*)error_message;
  response->body_length = strlen(error_message);
  response->content_type = memory_safe_strdup("application/soap+xml");
  if (!response->content_type) {
    return ONVIF_ERROR_MEMORY;
  }

  // Log error
  if (handler->config.enable_logging) {
    onvif_service_handler_log(handler, action_name, error_message);
  }

  return ONVIF_SUCCESS;
}

int onvif_service_handler_get_config_value(onvif_service_handler_instance_t* handler, config_section_t section, const char* key, void* value_ptr,
                                           config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // Dispatch to appropriate config_runtime function based on value type
  switch (value_type) {
  case CONFIG_TYPE_INT:
    return config_runtime_get_int(section, key, (int*)value_ptr);

  case CONFIG_TYPE_BOOL:
    return config_runtime_get_bool(section, key, (int*)value_ptr);

  case CONFIG_TYPE_FLOAT:
    return config_runtime_get_float(section, key, (float*)value_ptr);

  case CONFIG_TYPE_STRING:
    // String requires buffer size - use a reasonable default
    return config_runtime_get_string(section, key, (char*)value_ptr, CONFIG_STRING_BUFFER_SIZE);

  default:
    return ONVIF_ERROR_INVALID;
  }
}

int onvif_service_handler_set_config_value(onvif_service_handler_instance_t* handler, config_section_t section, const char* key,
                                           const void* value_ptr, config_value_type_t value_type) {
  if (!handler || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // Dispatch to appropriate config_runtime function based on value type
  switch (value_type) {
  case CONFIG_TYPE_INT:
    return config_runtime_set_int(section, key, *(const int*)value_ptr);

  case CONFIG_TYPE_BOOL:
    return config_runtime_set_bool(section, key, *(const int*)value_ptr);

  case CONFIG_TYPE_FLOAT:
    return config_runtime_set_float(section, key, *(const float*)value_ptr);

  case CONFIG_TYPE_STRING:
    return config_runtime_set_string(section, key, (const char*)value_ptr);

  default:
    return ONVIF_ERROR_INVALID;
  }
}

void onvif_service_handler_log(onvif_service_handler_instance_t* handler, const char* action_name, const char* message) {
  if (!handler || !action_name || !message) {
    return;
  }

  // Log using platform logging
  platform_log_info("[%s] %s: %s", handler->config.service_name, action_name, message);
}

onvif_gsoap_context_t* onvif_service_handler_get_gsoap_context(onvif_service_handler_instance_t* handler) {
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

int onvif_service_handler_get_stats(onvif_service_handler_instance_t* handler, service_stats_t* stats) {
  if (!handler || !stats) {
    return ONVIF_ERROR_INVALID;
  }

  *stats = handler->stats;
  return ONVIF_SUCCESS;
}

int onvif_service_handler_register_action(onvif_service_handler_instance_t* handler, const service_action_def_t* action_def) {
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
  service_action_def_t* new_actions = ONVIF_REALLOC(handler->actions, new_count * sizeof(service_action_def_t));
  if (!new_actions) {
    return ONVIF_ERROR;
  }

  handler->actions = new_actions;
  handler->actions[handler->action_count] = *action_def;
  handler->action_count = new_count;

  return ONVIF_SUCCESS;
}

int onvif_service_handler_unregister_action(onvif_service_handler_instance_t* handler, const char* action_name) {
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
