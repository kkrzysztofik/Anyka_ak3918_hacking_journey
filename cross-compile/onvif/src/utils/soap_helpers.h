/**
 * @file soap_helpers.h
 * @brief Common SOAP XML generation utilities for ONVIF services
 * 
 * This module provides shared SOAP XML generation functions that are used
 * across all ONVIF service implementations to reduce code duplication.
 */

#ifndef ONVIF_SOAP_HELPERS_H
#define ONVIF_SOAP_HELPERS_H

#include <stddef.h>

/**
 * @brief Generate a SOAP fault response
 * @param response Buffer to write the SOAP fault response to
 * @param response_size Size of the response buffer
 * @param fault_code SOAP fault code (e.g., "soap:Receiver", "soap:Sender")
 * @param fault_string Human-readable fault description
 */
void soap_fault_response(char *response, size_t response_size, const char *fault_code, const char *fault_string);

/**
 * @brief Generate a SOAP success response with custom namespace
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param action The action name (e.g., "GetCapabilities", "GetProfiles")
 * @param namespace The service namespace (e.g., "tds", "trt", "tptz", "timg")
 * @param body_content The XML content for the response body
 */
void soap_success_response_with_namespace(char *response, size_t response_size, 
                                         const char *action, const char *namespace, 
                                         const char *body_content);

/**
 * @brief Generate a SOAP success response for Device service
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param action The action name
 * @param body_content The XML content for the response body
 */
void soap_device_success_response(char *response, size_t response_size, 
                                 const char *action, const char *body_content);

/**
 * @brief Generate a SOAP success response for Media service
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param action The action name
 * @param body_content The XML content for the response body
 */
void soap_media_success_response(char *response, size_t response_size, 
                                const char *action, const char *body_content);

/**
 * @brief Generate a SOAP success response for PTZ service
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param action The action name
 * @param body_content The XML content for the response body
 */
void soap_ptz_success_response(char *response, size_t response_size, 
                              const char *action, const char *body_content);

/**
 * @brief Generate a SOAP success response for Imaging service
 * @param response Buffer to write the SOAP success response to
 * @param response_size Size of the response buffer
 * @param action The action name
 * @param body_content The XML content for the response body
 */
void soap_imaging_success_response(char *response, size_t response_size, 
                                  const char *action, const char *body_content);

#endif /* ONVIF_SOAP_HELPERS_H */
