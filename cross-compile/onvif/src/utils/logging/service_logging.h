/**
 * @file service_logging.h
 * @brief Service-specific logging utilities for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides service-specific logging functions that integrate
 * with the unified error handling system and provide consistent logging
 * across all ONVIF services.
 */

#ifndef ONVIF_SERVICE_LOGGING_H
#define ONVIF_SERVICE_LOGGING_H

/**
 * @brief Service logging levels
 */
typedef enum {
  SERVICE_LOG_ERROR = 0,
  SERVICE_LOG_WARNING = 1,
  SERVICE_LOG_INFO = 2,
  SERVICE_LOG_DEBUG = 3
} service_log_level_t;

/**
 * @brief Service logging context
 */
typedef struct {
  const char* service_name;
  const char* action_name;
  service_log_level_t level;
} service_log_context_t;

/**
 * @brief Initialize service logging context
 * @param context Context to initialize
 * @param service_name Name of the service
 * @param action_name Name of the action (can be NULL)
 * @param level Log level
 */
void service_log_init_context(service_log_context_t* context, const char* service_name,
                              const char* action_name, service_log_level_t level);

/**
 * @brief Log service operation success
 * @param context Logging context
 * @param operation Description of the operation
 */
void service_log_operation_success(const service_log_context_t* context, const char* operation);

/**
 * @brief Log service operation failure
 * @param context Logging context
 * @param operation Description of the operation
 * @param error_code Error code
 * @param error_message Error message
 */
void service_log_operation_failure(const service_log_context_t* context, const char* operation,
                                   int error_code, const char* error_message);

/**
 * @brief Log service validation error
 * @param context Logging context
 * @param field_name Name of the field that failed validation
 * @param value Value that failed validation
 */
void service_log_validation_error(const service_log_context_t* context, const char* field_name,
                                  const char* value);

/**
 * @brief Log service configuration error
 * @param context Logging context
 * @param config_key Configuration key
 * @param error_message Error message
 */
void service_log_config_error(const service_log_context_t* context, const char* config_key,
                              const char* error_message);

/**
 * @brief Log service platform operation failure
 * @param context Logging context
 * @param platform_operation Platform operation that failed
 * @param error_code Platform error code
 */
void service_log_platform_error(const service_log_context_t* context,
                                const char* platform_operation, int error_code);

/**
 * @brief Log service not implemented
 * @param context Logging context
 * @param feature Feature that's not implemented
 */
void service_log_not_implemented(const service_log_context_t* context, const char* feature);

/**
 * @brief Log service timeout
 * @param context Logging context
 * @param operation Operation that timed out
 * @param timeout_ms Timeout in milliseconds
 */
void service_log_timeout(const service_log_context_t* context, const char* operation,
                         int timeout_ms);

/**
 * @brief Log service warning
 * @param context Logging context
 * @param format Warning message format
 * @param ... Format arguments
 */
void service_log_warning(const service_log_context_t* context, const char* format, ...);

/**
 * @brief Log service info
 * @param context Logging context
 * @param format Info message format
 * @param ... Format arguments
 */
void service_log_info(const service_log_context_t* context, const char* format, ...);

/**
 * @brief Log service debug
 * @param context Logging context
 * @param format Debug message format
 * @param ... Format arguments
 */
void service_log_debug(const service_log_context_t* context, const char* format, ...);

#endif /* ONVIF_SERVICE_LOGGING_H */
