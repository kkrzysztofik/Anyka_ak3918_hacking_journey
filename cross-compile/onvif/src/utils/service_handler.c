/**
 * @file service_handler.c
 * @brief Common service request handling utilities for ONVIF services
 */

#include "service_handler.h"
#include "utils/error_handling.h"
#include "utils/logging_utils.h"
#include <stdio.h>
#include <stdlib.h>

int onvif_handle_service_request(onvif_action_type_t action,
                                 const onvif_request_t *request,
                                 onvif_response_t *response,
                                 onvif_service_handler_t handler) {
  if (!request || !response || !handler) {
    log_invalid_parameters("onvif_handle_service_request");
    return ONVIF_ERROR_INVALID;
  }
  
  // Initialize response with standard defaults
  if (onvif_init_service_response(response) != 0) {
    return ONVIF_ERROR;
  }
  
  // Call the service-specific handler
  int result = handler(action, request, response);
  
  // If handler failed, generate appropriate error response
  if (result < 0) {
    onvif_handle_service_error(response, "Service handler failed");
    return response->body_length;
  }
  
  return result;
}

int onvif_init_service_response(onvif_response_t *response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_init(response, ONVIF_RESPONSE_BUFFER_SIZE);
}

int onvif_handle_unsupported_action(onvif_response_t *response) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_soap_fault(response, "soap:Receiver", "Unsupported action");
}

int onvif_handle_missing_parameter(onvif_response_t *response, const char *param_name) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  char error_message[256];
  snprintf(error_message, sizeof(error_message), "Missing required parameter: %s", 
           param_name ? param_name : "unknown");
  
  return onvif_response_soap_fault(response, "soap:Sender", error_message);
}

int onvif_handle_service_error(onvif_response_t *response, const char *error_message) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  return onvif_response_soap_fault(response, "soap:Receiver", 
                                   error_message ? error_message : "Service error");
}
