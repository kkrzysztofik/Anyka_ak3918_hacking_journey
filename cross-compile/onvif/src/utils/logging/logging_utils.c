/**
 * @file logging_utils.c
 * @brief Common logging utilities implementation.
 */

#include "logging_utils.h"
#include "platform/platform.h"
#include <stdio.h>
#include <string.h>

void log_service_init_success(const char *service_name) {
    if (service_name) {
        platform_log_notice("ONVIF %s service initialized successfully\n", service_name);
    }
}

void log_service_init_failure(const char *service_name, const char *error_msg) {
    if (service_name && error_msg) {
        platform_log_error("Failed to initialize ONVIF %s service: %s\n", service_name, error_msg);
    } else if (service_name) {
        platform_log_error("Failed to initialize ONVIF %s service\n", service_name);
    }
}

void log_service_cleanup(const char *service_name) {
    if (service_name) {
        platform_log_notice("ONVIF %s service cleaned up\n", service_name);
    }
}

void log_invalid_parameters(const char *function_name) {
    if (function_name) {
        platform_log_error("Invalid parameters in %s\n", function_name);
    } else {
        platform_log_error("Invalid parameters\n");
    }
}

void log_service_not_initialized(const char *service_name) {
    if (service_name) {
        platform_log_error("%s service not initialized\n", service_name);
    } else {
        platform_log_error("Service not initialized\n");
    }
}

void log_operation_success(const char *operation) {
    if (operation) {
        platform_log_notice("%s operation completed successfully\n", operation);
    }
}

void log_operation_failure(const char *operation, const char *error_msg) {
    if (operation && error_msg) {
        platform_log_error("%s operation failed: %s\n", operation, error_msg);
    } else if (operation) {
        platform_log_error("%s operation failed\n", operation);
    }
}

void log_config_updated(const char *config_type) {
    if (config_type) {
        platform_log_notice("%s configuration updated successfully\n", config_type);
    }
}

void log_platform_operation_failure(const char *operation, const char *error_msg) {
    if (operation && error_msg) {
        platform_log_error("Failed to %s: %s\n", operation, error_msg);
    } else if (operation) {
        platform_log_error("Failed to %s\n", operation);
    }
}
