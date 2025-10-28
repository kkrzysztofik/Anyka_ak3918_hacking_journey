/**
 * @file onvif_service_handler.h
 * @brief ONVIF service request handling utilities
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SERVICE_HANDLER_H
#define ONVIF_SERVICE_HANDLER_H

#include <stddef.h>

#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"

#define MAX_ACTIONS 32

/**
 * @brief Service request handler function type
 */
typedef int (*onvif_service_handler_t)(const char* action_name, const http_request_t* request, http_response_t* response);

/**
 * @brief Action statistics
 */
typedef struct {
  const char* action_name;
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
  const char* service_name;
  config_manager_t* config;
  int enable_validation;
  int enable_logging;
} service_handler_config_t;

/**
 * @brief Service action handler function type
 */
typedef int (*service_action_handler_t)(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                        onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Service action definition
 */
typedef struct {
  const char* action_name;
  service_action_handler_t handler;
  int requires_validation;
} service_action_def_t;

/**
 * @brief ONVIF service handler instance
 */
typedef struct {
  service_handler_config_t config;
  service_action_def_t* actions;
  size_t action_count;
  onvif_gsoap_context_t* gsoap_ctx;
  service_stats_t stats;
} onvif_service_handler_instance_t;

/* ============================================================================
 * Service Handler Management
 * ============================================================================
 */

/**
 * @brief Initialize ONVIF service handler
 * @param handler Handler to initialize
 * @param config Service configuration
 * @param actions Array of action definitions
 * @param action_count Number of actions
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_init(onvif_service_handler_instance_t* handler, const service_handler_config_t* config, const service_action_def_t* actions,
                               size_t action_count);

/**
 * @brief Handle ONVIF request using unified patterns
 * @param handler Service handler
 * @param action Action type
 * @param request Request structure
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_handle_request(onvif_service_handler_instance_t* handler, const char* action_name, const http_request_t* request,
                                         http_response_t* response);

/**
 * @brief Clean up unified service handler
 * @param handler Handler to clean up
 */
void onvif_service_handler_cleanup(onvif_service_handler_instance_t* handler);

/**
 * @brief Validate request parameters
 * @param handler Service handler
 * @param request Request structure
 * @param required_params Array of required parameter names
 * @param param_count Number of required parameters
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_validate_request(onvif_service_handler_instance_t* handler, const http_request_t* request, const char** required_params,
                                           size_t param_count);

/**
 * @brief Generate success response using XML builder
 * @param handler Service handler
 * @param action Action name
 * @param body_content XML body content
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_generate_success(onvif_service_handler_instance_t* handler, const char* action, const char* body_content,
                                           http_response_t* response);

/**
 * @brief Generate error response using common error handling
 * @param handler Service handler
 * @param action_name Action name
 * @param error_pattern Error pattern
 * @param error_message Error message
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_generate_error(onvif_service_handler_instance_t* handler, const char* action_name, error_pattern_t error_pattern,
                                         const char* error_message, http_response_t* response);

/**
 * @brief Get service configuration value
 * @param handler Service handler
 * @param section Configuration section
 * @param key Configuration key
 * @param value_ptr Pointer to store value
 * @param value_type Value type
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_get_config_value(onvif_service_handler_instance_t* handler, config_section_t section, const char* key, void* value_ptr,
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
int onvif_service_handler_set_config_value(onvif_service_handler_instance_t* handler, config_section_t section, const char* key,
                                           const void* value_ptr, config_value_type_t value_type);

/**
 * @brief Log service operation
 * @param handler Service handler
 * @param action_name Action name
 * @param message Log message
 * @param level Log level
 */
void onvif_service_handler_log(onvif_service_handler_instance_t* handler, const char* action_name, const char* message);

/**
 * @brief Get gSOAP context for service
 * @param handler Service handler
 * @return gSOAP context pointer
 */
onvif_gsoap_context_t* onvif_service_handler_get_gsoap_context(onvif_service_handler_instance_t* handler);

/**
 * @brief Reset XML builder for new operation
 * @param handler Service handler
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_reset_xml_builder(onvif_service_handler_instance_t* handler);

/**
 * @brief Get service statistics
 * @param handler Service handler
 * @param stats Statistics structure to populate
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_get_stats(onvif_service_handler_instance_t* handler, service_stats_t* stats);

/**
 * @brief Register custom action handler
 * @param handler Service handler
 * @param action_def Action definition
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_register_action(onvif_service_handler_instance_t* handler, const service_action_def_t* action_def);

/**
 * @brief Unregister action handler
 * @param handler Service handler
 * @param action_type Action type to unregister
 * @return 0 on success, negative error code on failure
 */
int onvif_service_handler_unregister_action(onvif_service_handler_instance_t* handler, const char* action_name);

#endif /* ONVIF_SERVICE_HANDLER_H */
