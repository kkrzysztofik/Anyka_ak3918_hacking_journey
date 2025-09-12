/**
 * @file service_handler.h
 * @brief Common service request handling utilities for ONVIF services
 * 
 * This module provides shared service request handling functions that are used
 * across all ONVIF service implementations to reduce code duplication.
 */

#ifndef ONVIF_SERVICE_HANDLER_H
#define ONVIF_SERVICE_HANDLER_H

#include "common/onvif_types.h"
#include "response_helpers.h"
#include <stddef.h>

/**
 * @brief Service handler function type
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

#endif /* ONVIF_SERVICE_HANDLER_H */
