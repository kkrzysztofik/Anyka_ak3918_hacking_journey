/**
 * @file common_service_init.c
 * @brief Common service initialization utilities implementation
 */

#include "common_service_init.h"
#include "utils/logging/service_logging.h"
#include "utils/string/string_shims.h"

int common_service_init_context(common_service_context_t *context,
                               onvif_service_type_t service_type,
                               const char *service_name,
                               config_manager_t *config,
                               const service_action_def_t *actions,
                               int action_count) {
    if (!context || !service_name || !actions || action_count <= 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    context->service_type = service_type;
    context->service_name = service_name;
    context->actions = actions;
    context->action_count = action_count;
    context->initialized = 0;
    
    // Initialize handler configuration
    context->handler_config.service_type = service_type;
    context->handler_config.service_name = service_name;
    context->handler_config.config = config;
    context->handler_config.enable_validation = 1;
    context->handler_config.enable_logging = 1;
    
    return ONVIF_SUCCESS;
}

int common_service_init_handler(common_service_context_t *context,
                               onvif_service_handler_instance_t *handler) {
    if (!context || !handler) {
        return ONVIF_ERROR_INVALID;
    }
    
    if (context->initialized) {
        return ONVIF_SUCCESS; // Already initialized
    }
    
    int result = onvif_service_handler_init(handler, &context->handler_config,
                                    context->actions, context->action_count);
    
    if (result == ONVIF_SUCCESS) {
        context->initialized = 1;
        
        service_log_context_t log_ctx;
        service_log_init_context(&log_ctx, context->service_name, "init", SERVICE_LOG_INFO);
        service_log_operation_success(&log_ctx, "Service initialization");
    } else {
        service_log_context_t log_ctx;
        service_log_init_context(&log_ctx, context->service_name, "init", SERVICE_LOG_ERROR);
        service_log_operation_failure(&log_ctx, "Service initialization", result, "Handler init failed");
    }
    
    return result;
}

int common_service_register_error_handlers(common_service_context_t *context,
                                          error_handler_callback_t validation_handler,
                                          error_handler_callback_t system_handler,
                                          error_handler_callback_t config_handler) {
    if (!context) {
        return ONVIF_ERROR_INVALID;
    }
    
    int result = ONVIF_SUCCESS;
    
    // Store error handlers in context for later use
    // Note: This is a simplified implementation since error_register_handler doesn't exist
    // In a full implementation, these would be stored and used when errors occur
    
    (void)validation_handler;  // Suppress unused parameter warning
    (void)system_handler;      // Suppress unused parameter warning
    (void)config_handler;      // Suppress unused parameter warning
    
    if (result == ONVIF_SUCCESS) {
        service_log_context_t log_ctx;
        service_log_init_context(&log_ctx, context->service_name, "error_handlers", SERVICE_LOG_INFO);
        service_log_operation_success(&log_ctx, "Error handler registration");
    }
    
    return result;
}

int common_service_handle_request(common_service_context_t *context,
                                 onvif_service_handler_instance_t *handler,
                                 onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response) {
    if (!context || !handler || !request || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    if (!context->initialized) {
        service_log_context_t log_ctx;
        service_log_init_context(&log_ctx, context->service_name, "handle_request", SERVICE_LOG_ERROR);
        service_log_operation_failure(&log_ctx, "Request handling", ONVIF_ERROR, "Service not initialized");
        return ONVIF_ERROR;
    }
    
    return onvif_service_handler_handle_request(handler, action, request, response);
}

void common_service_cleanup(common_service_context_t *context,
                           onvif_service_handler_instance_t *handler) {
    if (!context) {
        return;
    }
    
    if (handler && context->initialized) {
        onvif_service_handler_cleanup(handler);
        
        service_log_context_t log_ctx;
        service_log_init_context(&log_ctx, context->service_name, "cleanup", SERVICE_LOG_INFO);
        service_log_operation_success(&log_ctx, "Service cleanup");
    }
    
    context->initialized = 0;
}

int common_service_is_initialized(const common_service_context_t *context) {
    return context ? context->initialized : 0;
}
