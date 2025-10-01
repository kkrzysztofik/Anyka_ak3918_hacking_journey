/**
 * @file onvif_gsoap_response.h
 * @brief Generic ONVIF gSOAP response generation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_GSOAP_RESPONSE_H
#define ONVIF_GSOAP_RESPONSE_H

#include <stddef.h>

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT

/* Forward declaration to avoid circular dependencies */
typedef struct onvif_gsoap_context_s onvif_gsoap_context_t;

/**
 * @brief Callback function type for response generation
 * @param soap gSOAP context
 * @param user_data User-provided data for response generation
 * @return 0 on success, error code on failure
 */
typedef int (*onvif_response_callback_t)(struct soap* soap, void* user_data);

/**
 * @brief Validates gSOAP context for response generation
 * @param soap gSOAP context to validate
 * @return 0 on success, ONVIF_ERROR_INVALID if context is invalid
 * @note Ensures fault structure is available for error handling
 */
int onvif_gsoap_validate_context(struct soap* soap);


/**
 * @brief Begins response serialization
 * @param ctx gSOAP context
 * @param response_data Response data structure (unused, reserved for future use)
 * @return 0 on success, error code on failure
 * @note Starts timing and begins SOAP send operation
 */
int onvif_gsoap_serialize_response(onvif_gsoap_context_t* ctx, void* response_data);

/**
 * @brief Finalizes response serialization
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 * @note Ends SOAP send and updates statistics (bytes written, timing)
 */
int onvif_gsoap_finalize_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generates complete SOAP response using callback
 * @param ctx gSOAP context
 * @param callback Callback function to generate response content
 * @param user_data User data passed to callback
 * @return 0 on success, error code on failure
 * @note Handles complete SOAP envelope generation with proper error handling
 */
int onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                onvif_response_callback_t callback,
                                                void* user_data);

/**
 * @brief Validates generated response
 * @param ctx gSOAP context
 * @return 0 if response is valid, ONVIF_ERROR_INVALID otherwise
 * @note Checks for SOAP errors and ensures response was generated
 */
int onvif_gsoap_validate_response(const onvif_gsoap_context_t* ctx);

/**
 * @brief Extracts ONVIF operation name from SOAP request
 * @param request_data Raw SOAP request data
 * @param request_size Size of request data in bytes
 * @param operation_name Output buffer for operation name
 * @param operation_name_size Size of operation name buffer
 * @return ONVIF_XML_SUCCESS on success, error code on failure
 * @note Parses SOAP envelope to extract operation element name
 */
int onvif_gsoap_extract_operation_name(const char* request_data, size_t request_size,
                                       char* operation_name, size_t operation_name_size);

/**
 * @brief Generates SOAP fault response
 * @param ctx gSOAP context (can be NULL, will create temporary context)
 * @param fault_code SOAP fault code (e.g., "soap:Server", NULL uses default)
 * @param fault_string Fault string describing the error
 * @param fault_actor Optional fault actor (can be NULL)
 * @param fault_detail Optional fault detail (can be NULL)
 * @param output_buffer Optional output buffer to receive generated XML
 * @param buffer_size Size of output buffer
 * @return Length of generated response on success, error code on failure
 * @note If ctx is NULL, creates and cleans up temporary context
 */
int onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, const char* fault_code,
                                        const char* fault_string, const char* fault_actor,
                                        const char* fault_detail, char* output_buffer,
                                        size_t buffer_size);

#endif // ONVIF_GSOAP_RESPONSE_H

/**
 * @brief Get response data from context
 * @param ctx gSOAP context
 * @return Response data string, or NULL if no response
 */
const char* onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx);

/**
 * @brief Get response data length from context
 * @param ctx gSOAP context
 * @return Response data length in bytes
 */
size_t onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx);

/**
 * @brief Check if context has error  
 * @param ctx gSOAP context
 * @return true if error exists, false otherwise
 */
bool onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx);

