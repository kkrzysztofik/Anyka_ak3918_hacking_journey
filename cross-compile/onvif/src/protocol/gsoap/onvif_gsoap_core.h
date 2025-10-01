/**
 * @file onvif_gsoap_core.h
 * @brief Core gSOAP functionality for ONVIF protocol layer
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides core gSOAP context management, initialization,
 * cleanup, and error handling functions used by all ONVIF service modules.
 */

#ifndef ONVIF_GSOAP_CORE_H
#define ONVIF_GSOAP_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT

/* Include the main header for the context structure definition */
#include "protocol/gsoap/onvif_gsoap.h"

/**
 * @brief Initialize gSOAP context with embedded soap structure
 * @param ctx Context to initialize
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note No dynamic allocation - uses embedded soap context
 */
int onvif_gsoap_init(onvif_gsoap_context_t* ctx);

/**
 * @brief Cleanup gSOAP context with embedded soap structure
 * @param ctx Context to cleanup
 * @note No deallocation needed - uses embedded soap context
 */
void onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx);

/**
 * @brief Reset gSOAP context to initial state
 * @param ctx Context to reset
 */
void onvif_gsoap_reset(onvif_gsoap_context_t* ctx);

/**
 * @brief Initialize request parsing for the context
 * @param ctx Context to initialize for parsing
 * @param request_xml SOAP request XML data
 * @param xml_size Size of the request XML in bytes
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Sets up soap input stream and marks request_state as initialized
 */
int onvif_gsoap_init_request_parsing(onvif_gsoap_context_t* ctx, const char* request_xml,
                                      size_t xml_size);

/**
 * @brief Set error context with detailed information
 * @param ctx Context to update
 * @param error_code Error code from error_handling.h
 * @param location Function name where error occurred (__func__)
 * @param message Detailed error message
 */
void onvif_gsoap_set_error(onvif_gsoap_context_t* ctx, int error_code, const char* location,
                            const char* message);

/**
 * @brief Get detailed error information
 * @param ctx Context to query
 * @param error_code Output: error code (can be NULL)
 * @param location Output: function where error occurred (can be NULL)
 * @param soap_error Output: gSOAP error code (can be NULL)
 * @return Error message string
 */
const char* onvif_gsoap_get_detailed_error(onvif_gsoap_context_t* ctx, int* error_code,
                                            const char** location, int* soap_error);

/**
 * @brief Check if context has error
 * @param ctx Context to query
 * @return true if context has error, false otherwise
 */
bool onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx);

/**
 * @brief Get error message
 * @param ctx Context to query
 * @return Error message string, or NULL if no error
 */
const char* onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx);

#endif /* ONVIF_GSOAP_CORE_H */
