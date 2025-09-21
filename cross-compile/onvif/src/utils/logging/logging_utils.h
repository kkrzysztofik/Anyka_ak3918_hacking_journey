/**
 * @file logging_utils.h
 * @brief Common logging utilities for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_LOGGING_UTILS_H
#define ONVIF_LOGGING_UTILS_H

#include <stddef.h>

/**
 * @brief Log service initialization success
 * @param service_name Name of the service being initialized
 */
void log_service_init_success(const char *service_name);

/**
 * @brief Log service initialization failure
 * @param service_name Name of the service being initialized
 * @param error_msg Error message describing the failure
 */
void log_service_init_failure(const char *service_name, const char *error_msg);

/**
 * @brief Log service cleanup
 * @param service_name Name of the service being cleaned up
 */
void log_service_cleanup(const char *service_name);

/**
 * @brief Log invalid parameters error
 * @param function_name Name of the function where error occurred
 */
void log_invalid_parameters(const char *function_name);

/**
 * @brief Log service not initialized error
 * @param service_name Name of the service that's not initialized
 */
void log_service_not_initialized(const char *service_name);

/**
 * @brief Log operation success
 * @param operation Description of the operation
 */
void log_operation_success(const char *operation);

/**
 * @brief Log operation failure
 * @param operation Description of the operation
 * @param error_msg Error message describing the failure
 */
void log_operation_failure(const char *operation, const char *error_msg);

/**
 * @brief Log configuration update
 * @param config_type Type of configuration updated
 */
void log_config_updated(const char *config_type);

/**
 * @brief Log platform operation failure
 * @param operation Description of the platform operation
 * @param error_msg Error message describing the failure
 */
void log_platform_operation_failure(const char *operation,
                                    const char *error_msg);

#endif /* ONVIF_LOGGING_UTILS_H */
