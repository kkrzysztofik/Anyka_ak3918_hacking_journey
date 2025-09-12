/**
 * @file response_helpers.h
 * @brief Common response handling utilities for ONVIF services
 * 
 * This module provides shared response handling functions that are used
 * across all ONVIF service implementations to reduce code duplication.
 */

#ifndef ONVIF_RESPONSE_HELPERS_H
#define ONVIF_RESPONSE_HELPERS_H

#include "common/onvif_types.h"
#include "common/onvif_request.h"
#include "utils/constants_clean.h"
#include <stddef.h>

/**
 * @brief Initialize a response structure with common defaults
 * @param response Response structure to initialize
 * @param buffer_size Size of the response buffer to allocate
 * @return 0 on success, negative error code on failure
 */
int onvif_response_init(onvif_response_t *response, size_t buffer_size);

/**
 * @brief Clean up a response structure and free allocated memory
 * @param response Response structure to clean up
 */
void onvif_response_cleanup(onvif_response_t *response);

/**
 * @brief Set response body content with automatic length calculation
 * @param response Response structure to update
 * @param body_content The content to set as the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_set_body(onvif_response_t *response, const char *body_content);

/**
 * @brief Set response body content using printf-style formatting
 * @param response Response structure to update
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return 0 on success, negative error code on failure
 */
int onvif_response_set_body_printf(onvif_response_t *response, const char *format, ...);

/**
 * @brief Generate a standard SOAP fault response
 * @param response Response structure to populate
 * @param fault_code SOAP fault code
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int onvif_response_soap_fault(onvif_response_t *response, const char *fault_code, const char *fault_string);

/**
 * @brief Generate a standard SOAP success response for Device service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_device_success(onvif_response_t *response, const char *action, const char *body_content);

/**
 * @brief Generate a standard SOAP success response for Media service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_media_success(onvif_response_t *response, const char *action, const char *body_content);

/**
 * @brief Generate a standard SOAP success response for PTZ service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_ptz_success(onvif_response_t *response, const char *action, const char *body_content);

/**
 * @brief Generate a standard SOAP success response for Imaging service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_imaging_success(onvif_response_t *response, const char *action, const char *body_content);

#endif /* ONVIF_RESPONSE_HELPERS_H */
