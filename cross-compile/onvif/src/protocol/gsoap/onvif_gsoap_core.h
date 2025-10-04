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
/* gSOAP context defined below */

/* ============================================================================
 * gSOAP Context
 * Structure
 * ============================================================================
 */

/**
 * @brief Enhanced ONVIF gSOAP context with embedded soap context
 *
 * This structure improves
 * upon the original design by:
 * - Using embedded soap context (no allocation needed)
 * -
 * Tracking request parsing and response generation state
 * - Providing detailed error context for
 * debugging
 * - Automatic performance metric collection
 */
typedef struct onvif_gsoap_context_s {
  /* Embedded gSOAP context (no pointer indirection) */
  struct soap soap;

  /* Request parsing state tracking */
  struct {
    const char* operation_name; /* Parsed operation name (for logging) */
    bool is_initialized;        /* Request parsing initialized */
    size_t request_size;        /* Original request size in bytes */
    uint64_t parse_start_time;  /* Parse start timestamp (microseconds) */
    uint64_t parse_end_time;    /* Parse end timestamp (microseconds) */
  } request_state;

  /* Response generation state tracking */
  struct {
    size_t total_bytes_written;     /* Total response bytes written */
    uint64_t generation_start_time; /* Generation start timestamp */
    uint64_t generation_end_time;   /* Generation end timestamp */
    bool is_finalized;              /* Response finalization complete */
  } response_state;

  /* Enhanced error context for debugging */
  struct {
    int last_error_code;        /* Last error code from error_handling.h */
    char error_message[256];    /* Detailed error message */
    const char* error_location; /* Function where error occurred */
    int soap_error_code;        /* gSOAP-specific error code */
  } error_context;

  /* Optional user data */
  void* user_data;
} onvif_gsoap_context_t;

/* ============================================================================
 * gSOAP Core
 * Functions
 * ============================================================================
 */

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

/* ============================================================================
 * Request Parsing Helper Functions
 * ============================================================================
 */

/**
 * @brief Validate context and begin parse operation
 * @param ctx Context to validate
 * @param out_ptr Output pointer to validate
 * @param operation_name Operation name for logging and tracking
 * @param func_name Function name for error reporting (__func__)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Combines parameter validation, request state check, and timing start
 */
int onvif_gsoap_validate_and_begin_parse(onvif_gsoap_context_t* ctx, void* out_ptr,
                                         const char* operation_name, const char* func_name);

/**
 * @brief Parse SOAP envelope structure
 * @param ctx Context with initialized request parsing
 * @param func_name Function name for error reporting (__func__)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Executes: soap_begin_recv → soap_envelope_begin_in → soap_recv_header →
 * soap_body_begin_in
 * @note Handles errors at each step with detailed logging
 */
int onvif_gsoap_parse_soap_envelope(onvif_gsoap_context_t* ctx, const char* func_name);

/**
 * @brief Finalize SOAP envelope parsing and complete operation timing
 * @param ctx Context to finalize
 * @note Executes: soap_body_end_in → soap_envelope_end_in → soap_end_recv
 * @note Records parse_end_time for performance tracking
 */
void onvif_gsoap_finalize_parse(onvif_gsoap_context_t* ctx);

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
