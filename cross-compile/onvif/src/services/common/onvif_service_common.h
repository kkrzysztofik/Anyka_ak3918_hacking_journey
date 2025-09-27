/**
 * @file onvif_service_common.h
 * @brief Common ONVIF service types and callback definitions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SERVICE_COMMON_H
#define ONVIF_SERVICE_COMMON_H

#include "core/config/config.h"
#include "generated/soapH.h"
#include "networking/http/http_parser.h"
#include "onvif_types.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"

/* ============================================================================
 * Common Callback Types and Structures
 * ============================================================================
 */

/**
 * @brief Parameter validation callback function type
 */
typedef int (*onvif_validation_callback_t)(const service_handler_config_t* config,
                                           const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx,
                                           service_log_context_t* log_ctx,
                                           error_context_t* error_ctx);

/**
 * @brief Business logic execution callback function type
 */
typedef int (*onvif_business_logic_callback_t)(const service_handler_config_t* config,
                                               const http_request_t* request,
                                               http_response_t* response,
                                               onvif_gsoap_context_t* gsoap_ctx,
                                               service_log_context_t* log_ctx,
                                               error_context_t* error_ctx, void* callback_data);

/**
 * @brief Post-processing callback function type
 */
typedef int (*onvif_post_process_callback_t)(http_response_t* response,
                                             service_log_context_t* log_ctx);

/**
 * @brief Enhanced ONVIF handler callbacks structure
 */
typedef struct {
  onvif_validation_callback_t validate_parameters;
  onvif_business_logic_callback_t execute_business_logic;
  onvif_post_process_callback_t post_process_response;
} onvif_handler_callbacks_t;

/**
 * @brief ONVIF service operation definition
 */
typedef struct {
  const char* service_name;
  const char* operation_name;
  const char* operation_context;
  onvif_handler_callbacks_t callbacks;
} onvif_service_operation_t;

/* ============================================================================
 * Common Utility Function Declarations
 * ============================================================================
 */

/**
 * @brief Standard parameter validation callback
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @return ONVIF_SUCCESS if validation succeeds, error code if it fails
 */
int onvif_util_validate_standard_parameters(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx);

/**
 * @brief Standard post-processing callback
 * @param response ONVIF response
 * @param log_ctx Service logging context
 * @return ONVIF_SUCCESS if post-processing succeeds, error code if it fails
 */
int onvif_util_standard_post_process(http_response_t* response, service_log_context_t* log_ctx);

/**
 * @brief Generic ONVIF service handler with enhanced callback pattern
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param operation Service operation definition
 * @param soap_callback SOAP callback function
 * @param callback_data Callback data
 * @return ONVIF_SUCCESS if handling succeeds, error code if it fails
 */
int onvif_util_handle_service_request(const service_handler_config_t* config,
                                      const http_request_t* request, http_response_t* response,
                                      onvif_gsoap_context_t* gsoap_ctx,
                                      const onvif_service_operation_t* operation,
                                      int (*soap_callback)(struct soap* soap, void* user_data),
                                      void* callback_data);

/**
 * @brief Get configuration string with fallback and error handling
 * @param handler Service handler instance
 * @param section Configuration section
 * @param key Configuration key
 * @param value Output value buffer
 * @param value_size Size of value buffer
 * @param default_value Default value if config not found
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param response ONVIF response
 * @param config_name Configuration name for logging
 * @return ONVIF_SUCCESS if successful, error code if it fails
 */
int onvif_util_get_config_string_with_fallback(
  onvif_service_handler_instance_t* handler, config_section_t section, const char* key, char* value,
  size_t value_size, const char* default_value, service_log_context_t* log_ctx,
  error_context_t* error_ctx, http_response_t* response, const char* config_name);

/**
 * @brief Get configuration integer with fallback and error handling
 * @param handler Service handler instance
 * @param section Configuration section
 * @param key Configuration key
 * @param value Output value pointer
 * @param default_value Default value if config not found
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param response ONVIF response
 * @param config_name Configuration name for logging
 * @return ONVIF_SUCCESS if successful, error code if it fails
 */
int onvif_util_get_config_int_with_fallback(onvif_service_handler_instance_t* handler,
                                            config_section_t section, const char* key, int* value,
                                            int default_value, service_log_context_t* log_ctx,
                                            error_context_t* error_ctx, http_response_t* response,
                                            const char* config_name);

/**
 * @brief Convert service type to string
 * @param service ONVIF service type
 * @return String representation of service type
 */
const char* onvif_service_type_to_string(onvif_service_type_t service);

/**
 * @brief Convert action type to string
 * @param action ONVIF action type
 * @return String representation of action type
 */
/* onvif_action_type_to_string removed - using string names directly */

#endif /* ONVIF_SERVICE_COMMON_H */
