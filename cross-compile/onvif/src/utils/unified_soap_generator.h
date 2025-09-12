/**
 * @file unified_soap_generator.h
 * @brief Unified SOAP response generator for all ONVIF services
 * 
 * This module provides a single, configurable SOAP response generation system
 * that eliminates code duplication across all ONVIF service implementations.
 */

#ifndef ONVIF_UNIFIED_SOAP_GENERATOR_H
#define ONVIF_UNIFIED_SOAP_GENERATOR_H

#include "common/onvif_types.h"
#include "common/onvif_request.h"
#include "utils/constants_clean.h"
#include <stddef.h>

/* ONVIF service types are defined in common/onvif_types.h */

/**
 * @brief SOAP response configuration
 */
typedef struct {
  onvif_service_type_t service_type;
  const char *action_name;
  const char *body_content;
  int status_code;
  const char *content_type;
} soap_response_config_t;

/**
 * @brief Generate a unified SOAP response based on configuration
 * @param response Buffer to write the SOAP response to
 * @param response_size Size of the response buffer
 * @param config Response configuration
 * @return 0 on success, negative error code on failure
 */
int soap_generate_response(char *response, size_t response_size, const soap_response_config_t *config);

/**
 * @brief Generate a SOAP fault response
 * @param response Buffer to write the SOAP fault response to
 * @param response_size Size of the response buffer
 * @param fault_code SOAP fault code (e.g., "soap:Receiver", "soap:Sender")
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int soap_generate_fault(char *response, size_t response_size, const char *fault_code, const char *fault_string);

/**
 * @brief Generate a SOAP success response for any service
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param service_type The ONVIF service type
 * @param action_name The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int soap_generate_success(char *response, size_t response_size, onvif_service_type_t service_type,
                         const char *action_name, const char *body_content);

/**
 * @brief Get the namespace prefix for a service type
 * @param service_type The ONVIF service type
 * @return The namespace prefix (e.g., "tds", "trt", "tptz", "timg")
 */
const char *soap_get_namespace_prefix(onvif_service_type_t service_type);

/**
 * @brief Get the full namespace URI for a service type
 * @param service_type The ONVIF service type
 * @return The full namespace URI
 */
const char *soap_get_namespace_uri(onvif_service_type_t service_type);

/**
 * @brief Generate a complete ONVIF response with proper headers
 * @param response ONVIF response structure to populate
 * @param service_type The ONVIF service type
 * @param action_name The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_generate_complete_response(onvif_response_t *response, onvif_service_type_t service_type,
                                    const char *action_name, const char *body_content);

/**
 * @brief Generate a complete ONVIF fault response
 * @param response ONVIF response structure to populate
 * @param fault_code SOAP fault code
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int onvif_generate_fault_response(onvif_response_t *response, const char *fault_code, const char *fault_string);

#endif /* ONVIF_UNIFIED_SOAP_GENERATOR_H */
