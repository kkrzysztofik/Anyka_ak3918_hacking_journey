/**
 * @file common_service_init.h
 * @brief Common service initialization utilities for ONVIF services
 * 
 * This module provides unified service initialization patterns that eliminate
 * code duplication across all ONVIF service implementations.
 */

#ifndef ONVIF_COMMON_SERVICE_INIT_H
#define ONVIF_COMMON_SERVICE_INIT_H

#include "services/common/onvif_types.h"
#include "protocol/response/onvif_service_handler.h"
#include "utils/error/error_handling.h"

/**
 * @brief Error handler callback function type
 */
typedef int (*error_handler_callback_t)(const error_context_t *context, const error_result_t *result);

/**
 * @brief Service initialization context
 */
typedef struct {
    onvif_service_type_t service_type;
    const char *service_name;
    service_handler_config_t handler_config;
    const service_action_def_t *actions;
    int action_count;
    int initialized;
} common_service_context_t;

/**
 * @brief Initialize common service context
 * @param context Context to initialize
 * @param service_type Type of the service
 * @param service_name Name of the service
 * @param config Configuration manager
 * @param actions Service actions
 * @param action_count Number of actions
 * @return 0 on success, -1 on error
 */
int common_service_init_context(common_service_context_t *context,
                               onvif_service_type_t service_type,
                               const char *service_name,
                               config_manager_t *config,
                               const service_action_def_t *actions,
                               int action_count);

/**
 * @brief Initialize service handler
 * @param context Service context
 * @param handler Service handler to initialize
 * @return 0 on success, -1 on error
 */
int common_service_init_handler(common_service_context_t *context,
                               onvif_service_handler_instance_t *handler);

/**
 * @brief Register service error handlers
 * @param context Service context
 * @param validation_handler Validation error handler
 * @param system_handler System error handler
 * @param config_handler Configuration error handler
 * @return 0 on success, -1 on error
 */
int common_service_register_error_handlers(common_service_context_t *context,
                                          error_handler_callback_t validation_handler,
                                          error_handler_callback_t system_handler,
                                          error_handler_callback_t config_handler);

/**
 * @brief Handle service request
 * @param context Service context
 * @param handler Service handler
 * @param action Action type
 * @param request Request
 * @param response Response
 * @return 0 on success, -1 on error
 */
int common_service_handle_request(common_service_context_t *context,
                                 onvif_service_handler_instance_t *handler,
                                 onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response);

/**
 * @brief Cleanup service
 * @param context Service context
 * @param handler Service handler
 */
void common_service_cleanup(common_service_context_t *context,
                           onvif_service_handler_instance_t *handler);

/**
 * @brief Check if service is initialized
 * @param context Service context
 * @return 1 if initialized, 0 if not
 */
int common_service_is_initialized(const common_service_context_t *context);

#endif /* ONVIF_COMMON_SERVICE_INIT_H */
