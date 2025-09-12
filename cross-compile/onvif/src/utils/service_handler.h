/**
 * @file service_handler.h
 * @brief Common service request handling utilities for ONVIF services
 * 
 * This module provides shared service request handling functions that are used
 * across all ONVIF service implementations to reduce code duplication.
 * Includes both legacy and refactored patterns for backward compatibility.
 */

#ifndef ONVIF_SERVICE_HANDLER_H
#define ONVIF_SERVICE_HANDLER_H

#include "common/onvif_types.h"
#include "response_helpers.h"
#include "unified_soap_generator.h"
#include "xml_builder.h"
#include "centralized_config.h"
#include "common_error_handling.h"
#include <stddef.h>

#define MAX_ACTIONS 32

/**
 * @brief Action statistics
 */
typedef struct {
  onvif_action_type_t action_type;
  int call_count;
  int error_count;
  double avg_response_time;
} action_stats_t;

/**
 * @brief Service statistics
 */
typedef struct {
  int total_requests;
  int total_errors;
  int total_success;
  action_stats_t action_stats[MAX_ACTIONS];
  size_t action_stats_count;
} service_stats_t;

/**
 * @brief Service handler configuration
 */
typedef struct {
  onvif_service_type_t service_type;
  const char *service_name;
  centralized_config_t *config;
  int enable_validation;
  int enable_logging;
} service_handler_config_t;

/**
 * @brief Service action handler function type
 */
typedef int (*service_action_handler_t)(const service_handler_config_t *config,
                                       const onvif_request_t *request,
                                       onvif_response_t *response,
                                       xml_builder_t *xml_builder);

/**
 * @brief Service action definition
 */
typedef struct {
  onvif_action_type_t action_type;
  const char *action_name;
  service_action_handler_t handler;
  int requires_validation;
} service_action_def_t;

/**
 * @brief Refactored service handler
 */
typedef struct {
  service_handler_config_t config;
  service_action_def_t *actions;
  size_t action_count;
  xml_builder_t xml_builder;
  char xml_buffer[ONVIF_XML_BUFFER_SIZE];
} service_handler_t;

/**
 * @brief Service handler function type (legacy)
 * @param action The ONVIF action to handle
 * @param request The incoming request
 * @param response The response to populate
 * @return Response body length on success, negative error code on failure
 */
typedef int (*onvif_service_handler_t)(onvif_action_type_t action, 
                                      const onvif_request_t *request, 
                                      onvif_response_t *response);

/**
 * @brief Common service request handler with standard error handling
 * @param action The ONVIF action to handle
 * @param request The incoming request
 * @param response The response to populate
 * @param handler The service-specific handler function
 * @return Response body length on success, negative error code on failure
 */
int onvif_handle_service_request(onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response,
                                 onvif_service_handler_t handler);

/**
 * @brief Initialize response with standard defaults
 * @param response Response structure to initialize
 * @return 0 on success, negative error code on failure
 */
int onvif_init_service_response(onvif_response_t *response);

/**
 * @brief Handle unsupported action with standard error response
 * @param response Response structure to populate
 * @return 0 on success, negative error code on failure
 */
int onvif_handle_unsupported_action(onvif_response_t *response);

/**
 * @brief Handle missing required parameters with standard error response
 * @param response Response structure to populate
 * @param param_name Name of the missing parameter
 * @return 0 on success, negative error code on failure
 */
int onvif_handle_missing_parameter(onvif_response_t *response, const char *param_name);

/**
 * @brief Handle service error with standard error response
 * @param response Response structure to populate
 * @param error_message Description of the error
 * @return 0 on success, negative error code on failure
 */
int onvif_handle_service_error(onvif_response_t *response, const char *error_message);

/* Service handler functions */

/**
 * @brief Initialize refactored service handler
 * @param handler Handler to initialize
 * @param config Service configuration
 * @param actions Array of action definitions
 * @param action_count Number of actions
 * @return 0 on success, negative error code on failure
 */
int service_handler_init(service_handler_t *handler,
                                   const service_handler_config_t *config,
                                   const service_action_def_t *actions,
                                   size_t action_count);

/**
 * @brief Handle ONVIF request using refactored patterns
 * @param handler Service handler
 * @param action Action type
 * @param request Request structure
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int service_handler_handle_request(service_handler_t *handler,
                                             onvif_action_type_t action,
                                             const onvif_request_t *request,
                                             onvif_response_t *response);

/**
 * @brief Clean up refactored service handler
 * @param handler Handler to clean up
 */
void service_handler_cleanup(service_handler_t *handler);

/**
 * @brief Validate request parameters
 * @param handler Service handler
 * @param request Request structure
 * @param required_params Array of required parameter names
 * @param param_count Number of required parameters
 * @return 0 on success, negative error code on failure
 */
int service_handler_validate_request(service_handler_t *handler,
                                               const onvif_request_t *request,
                                               const char **required_params,
                                               size_t param_count);

/**
 * @brief Generate success response using XML builder
 * @param handler Service handler
 * @param action_name Action name
 * @param xml_content XML content
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int service_handler_generate_success(service_handler_t *handler,
                                               const char *action_name,
                                               const char *xml_content,
                                               onvif_response_t *response);

/**
 * @brief Generate error response using common error handling
 * @param handler Service handler
 * @param action_name Action name
 * @param error_pattern Error pattern
 * @param error_message Error message
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int service_handler_generate_error(service_handler_t *handler,
                                             const char *action_name,
                                             error_pattern_t error_pattern,
                                             const char *error_message,
                                             onvif_response_t *response);

/**
 * @brief Get service configuration value
 * @param handler Service handler
 * @param section Configuration section
 * @param key Configuration key
 * @param value_ptr Pointer to store value
 * @param value_type Value type
 * @return 0 on success, negative error code on failure
 */
int service_handler_get_config_value(service_handler_t *handler,
                                               config_section_t section,
                                               const char *key,
                                               void *value_ptr,
                                               config_value_type_t value_type);

/**
 * @brief Set service configuration value
 * @param handler Service handler
 * @param section Configuration section
 * @param key Configuration key
 * @param value_ptr Pointer to value
 * @param value_type Value type
 * @return 0 on success, negative error code on failure
 */
int service_handler_set_config_value(service_handler_t *handler,
                                               config_section_t section,
                                               const char *key,
                                               const void *value_ptr,
                                               config_value_type_t value_type);

/**
 * @brief Log service operation
 * @param handler Service handler
 * @param action_name Action name
 * @param message Log message
 * @param level Log level
 */
void service_handler_log(service_handler_t *handler,
                                   const char *action_name,
                                   const char *message,
                                   int level);

/**
 * @brief Create XML builder for service
 * @param handler Service handler
 * @return XML builder pointer
 */
xml_builder_t *service_handler_get_xml_builder(service_handler_t *handler);

/**
 * @brief Reset XML builder for new operation
 * @param handler Service handler
 * @return 0 on success, negative error code on failure
 */
int service_handler_reset_xml_builder(service_handler_t *handler);

/**
 * @brief Get service statistics
 * @param handler Service handler
 * @param stats Statistics structure to populate
 * @return 0 on success, negative error code on failure
 */
int service_handler_get_stats(service_handler_t *handler,
                                        service_stats_t *stats);

/**
 * @brief Register custom action handler
 * @param handler Service handler
 * @param action_def Action definition
 * @return 0 on success, negative error code on failure
 */
int service_handler_register_action(service_handler_t *handler,
                                              const service_action_def_t *action_def);

/**
 * @brief Unregister action handler
 * @param handler Service handler
 * @param action_type Action type to unregister
 * @return 0 on success, negative error code on failure
 */
int service_handler_unregister_action(service_handler_t *handler,
                                                onvif_action_type_t action_type);

#endif /* ONVIF_SERVICE_HANDLER_H */
