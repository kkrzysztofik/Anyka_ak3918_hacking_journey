/**
 * @file onvif_soap.h
 * @brief Unified SOAP response generator for all ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SOAP_H
#define ONVIF_SOAP_H

#include <stddef.h>

#include "common/onvif_request.h"
#include "common/onvif_types.h"

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
 * @brief Response buffer with automatic cleanup
 */
typedef struct {
  onvif_response_t *response; /**< Pointer to response structure */
  int owned;                  /**< Whether this structure owns the response */
} response_buffer_t;

/* ============================================================================
 * SOAP Response Generation
 * ============================================================================
 */

/**
 * @brief Generate a unified SOAP response based on configuration
 * @param response Buffer to write the SOAP response to
 * @param response_size Size of the response buffer
 * @param config Response configuration
 * @return 0 on success, negative error code on failure
 */
int soap_generate_response(char *response, size_t response_size,
                           const soap_response_config_t *config);

/**
 * @brief Generate a SOAP fault response
 * @param response Buffer to write the SOAP fault response to
 * @param response_size Size of the response buffer
 * @param fault_code SOAP fault code (e.g., "soap:Receiver", "soap:Sender")
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int soap_generate_fault(char *response, size_t response_size,
                        const char *fault_code, const char *fault_string);

/**
 * @brief Generate a SOAP success response for any service
 * @param response Buffer to write the SOAP success response to
 * @param service_type The ONVIF service type
 * @param response_size Size of the response buffer
 * @param action_name The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int soap_generate_success(char *response, onvif_service_type_t service_type,
                          size_t response_size, const char *action_name,
                          const char *body_content);

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
int onvif_generate_complete_response(onvif_response_t *response,
                                     onvif_service_type_t service_type,
                                     const char *action_name,
                                     const char *body_content);

/**
 * @brief Generate a complete ONVIF fault response
 * @param response ONVIF response structure to populate
 * @param fault_code SOAP fault code
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int onvif_generate_fault_response(onvif_response_t *response,
                                  const char *fault_code,
                                  const char *fault_string);

/* ============================================================================
 * Response Buffer Management
 * ============================================================================
 */

/**
 * @brief Initialize response buffer with automatic cleanup
 * @param buffer Buffer structure to initialize
 * @param response Response structure to manage (can be NULL)
 * @return 0 on success, -1 on error
 */
int response_buffer_init(response_buffer_t *buffer, onvif_response_t *response);

/**
 * @brief Create new response buffer with allocated response
 * @param buffer Buffer structure to initialize
 * @return 0 on success, -1 on error
 */
int response_buffer_create(response_buffer_t *buffer);

/**
 * @brief Cleanup response buffer and free all resources
 * @param buffer Buffer structure to cleanup
 */
void response_buffer_cleanup(response_buffer_t *buffer);

/**
 * @brief Get response pointer from buffer
 * @param buffer Buffer structure
 * @return Response pointer or NULL if not initialized
 */
onvif_response_t *response_buffer_get(response_buffer_t *buffer);

/**
 * @brief Set response body with automatic memory management
 * @param buffer Buffer structure
 * @param body Body content to set
 * @param length Length of body content
 * @return 0 on success, -1 on error
 */
int response_buffer_set_body(response_buffer_t *buffer, const char *body,
                             size_t length);

/**
 * @brief Set response body using printf-style formatting
 * @param buffer Buffer structure
 * @param format Printf format string
 * @param ... Format arguments
 * @return 0 on success, -1 on error
 */
int response_buffer_set_body_printf(response_buffer_t *buffer,
                                    const char *format, ...);

/**
 * @brief Macro for automatic cleanup using RAII pattern
 */
#define RESPONSE_BUFFER_AUTO(buffer)     \
  response_buffer_t buffer;              \
  response_buffer_init(&(buffer), NULL); \
  __attribute__((cleanup(response_buffer_cleanup)))

/* ============================================================================
 * Response Helpers
 * ============================================================================
 */

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
int onvif_response_set_body(onvif_response_t *response,
                            const char *body_content);

/**
 * @brief Set response body content using printf-style formatting
 * @param response Response structure to update
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return 0 on success, negative error code on failure
 */
int onvif_response_set_body_printf(onvif_response_t *response,
                                   const char *format, ...);

/**
 * @brief Generate a standard SOAP fault response
 * @param response Response structure to populate
 * @param fault_code SOAP fault code
 * @param fault_string Human-readable fault description
 * @return 0 on success, negative error code on failure
 */
int onvif_response_soap_fault(onvif_response_t *response,
                              const char *fault_code, const char *fault_string);

/**
 * @brief Generate a standard SOAP success response for Device service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_device_success(onvif_response_t *response,
                                  const char *action, const char *body_content);

/**
 * @brief Generate a standard SOAP success response for Media service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_media_success(onvif_response_t *response, const char *action,
                                 const char *body_content);

/**
 * @brief Generate a standard SOAP success response for PTZ service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_ptz_success(onvif_response_t *response, const char *action,
                               const char *body_content);

/**
 * @brief Generate a standard SOAP success response for Imaging service
 * @param response Response structure to populate
 * @param action The action name
 * @param body_content The XML content for the response body
 * @return 0 on success, negative error code on failure
 */
int onvif_response_imaging_success(onvif_response_t *response,
                                   const char *action,
                                   const char *body_content);

#endif /* ONVIF_SOAP_H */
