/**
 * @file onvif_gsoap_core.c
 * @brief Core gSOAP functionality implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements core gSOAP context management, initialization,
 * cleanup, and error handling functions.
 */

#include "protocol/gsoap/onvif_gsoap_core.h"

#include <stdio.h>
#include <string.h>

#include "platform/platform.h"
#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"
#include "utils/error/error_translation.h"

/* Include gSOAP namespace table */
#include "generated/DeviceBinding.nsmap"

/**
 * @brief Initialize gSOAP context with embedded soap structure
 * @param ctx Context to initialize
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int onvif_gsoap_init(onvif_gsoap_context_t* ctx) {
  platform_log_debug("onvif_gsoap_init: Starting initialization");

  if (!ctx) {
    platform_log_debug("onvif_gsoap_init: NULL context provided");
    return ONVIF_ERROR_INVALID;
  }

  platform_log_debug("onvif_gsoap_init: Clearing context structure");
  /* Clear entire context structure */
  memset(ctx, 0, sizeof(*ctx));

  platform_log_debug("onvif_gsoap_init: Initializing embedded soap context");
  soap_init(&ctx->soap);

  // Set SOAP version to 1.2 for ONVIF compliance
  soap_set_version(&ctx->soap, 2);

  // Parsing is permissive - validates XML syntax only
  // Business logic and namespace validation happens in service handlers
  soap_set_mode(&ctx->soap, SOAP_C_UTFSTRING | SOAP_XML_INDENT);

  ctx->soap.namespaces = namespaces;

  platform_log_debug("onvif_gsoap_init: Initialization completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Cleanup gSOAP context with embedded soap structure
 * @param ctx Context to cleanup
 */
void onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return;
  }

  /* Cleanup embedded soap context (no free needed!) */
  soap_destroy(&ctx->soap);
  soap_end(&ctx->soap);
  soap_done(&ctx->soap);

  /* Clear all state structures */
  memset(ctx, 0, sizeof(*ctx));
}

/**
 * @brief Reset gSOAP context to initial state
 * @param ctx Context to reset
 */
void onvif_gsoap_reset(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return;
  }

  /* Clean up existing state */
  soap_destroy(&ctx->soap);
  soap_end(&ctx->soap);
  ctx->soap.is = NULL;
  ctx->soap.user = NULL;
  ctx->soap.bufidx = 0;
  ctx->soap.buflen = 0;
  ctx->soap.ahead = 0;
  ctx->soap.recvfd = SOAP_INVALID_SOCKET;

  /* Reset state tracking structures */
  memset(&ctx->request_state, 0, sizeof(ctx->request_state));
  memset(&ctx->response_state, 0, sizeof(ctx->response_state));
  memset(&ctx->error_context, 0, sizeof(ctx->error_context));
}

/**
 * @brief Initialize request parsing for the context
 * @param ctx Context to initialize for parsing
 * @param request_xml SOAP request XML data
 * @param xml_size Size of the request XML in bytes
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int onvif_gsoap_init_request_parsing(onvif_gsoap_context_t* ctx, const char* request_xml,
                                     size_t xml_size) {
  platform_log_debug("onvif_gsoap_init_request_parsing: Starting with xml_size=%zu", xml_size);

  if (!ctx || !request_xml || xml_size == 0) {
    platform_log_debug(
      "onvif_gsoap_init_request_parsing: Invalid parameters - ctx=%p, request_xml=%p, xml_size=%zu",
      ctx, request_xml, xml_size);
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or request");
    }
    return ONVIF_ERROR_INVALID;
  }

  platform_log_debug("onvif_gsoap_init_request_parsing: Configuring soap context for parsing");
  /* Configure soap context for parsing from buffer */
  soap_begin(&ctx->soap);

  platform_log_debug("onvif_gsoap_init_request_parsing: Allocating buffer for request XML");
  /* Allocate and copy the request XML to soap-managed memory
   * This ensures the buffer remains valid during parsing */
  char* buffer = (char*)soap_malloc(&ctx->soap, xml_size + 1);
  if (!buffer) {
    platform_log_debug("onvif_gsoap_init_request_parsing: Failed to allocate buffer");
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate buffer for request");
    return ONVIF_ERROR_MEMORY;
  }

  platform_log_debug("onvif_gsoap_init_request_parsing: Copying request XML to buffer");
  memcpy(buffer, request_xml, xml_size);
  buffer[xml_size] = '\0';

  /* Log the incoming XML for debugging */
  platform_log_debug("onvif_gsoap_init_request_parsing: Incoming SOAP request XML:");
  platform_log_debug("%s", buffer);

  platform_log_debug(
    "onvif_gsoap_init_request_parsing: Configuring gSOAP to read from in-memory buffer");
  /* Configure gSOAP to read directly from the in-memory buffer using
   * the Method 1 pattern (soap->is advanced by the runtime). */
  ctx->soap.is = buffer;
  ctx->soap.bufidx = 0;
  ctx->soap.buflen = xml_size;
  ctx->soap.ahead = 0;
  ctx->soap.recvfd = SOAP_INVALID_SOCKET;
  ctx->soap.user = NULL;

  platform_log_debug("onvif_gsoap_init_request_parsing: Updating request state");
  /* Update request state */
  ctx->request_state.is_initialized = true;
  ctx->request_state.request_size = xml_size;
  ctx->request_state.parse_start_time = get_timestamp_us();

  platform_log_debug(
    "onvif_gsoap_init_request_parsing: Request parsing initialization completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Set error context with detailed information
 * @param ctx Context to update
 * @param error_code Error code from error_handling.h
 * @param location Function name where error occurred (__func__)
 * @param message Detailed error message
 */
void onvif_gsoap_set_error(onvif_gsoap_context_t* ctx, int error_code, const char* location,
                           const char* message) {
  if (!ctx) {
    return;
  }

  ctx->error_context.last_error_code = error_code;
  ctx->error_context.error_location = location;
  ctx->error_context.soap_error_code = ctx->soap.error;

  if (message) {
    snprintf(ctx->error_context.error_message, sizeof(ctx->error_context.error_message), "%s",
             message);
  } else {
    ctx->error_context.error_message[0] = '\0';
  }
}

/**
 * @brief Get detailed error information
 * @param ctx Context to query
 * @param error_code Output: error code (can be NULL)
 * @param location Output: function where error occurred (can be NULL)
 * @param soap_error Output: gSOAP error code (can be NULL)
 * @return Error message string
 */
const char* onvif_gsoap_get_detailed_error(onvif_gsoap_context_t* ctx, int* error_code,
                                           const char** location, int* soap_error) {
  if (!ctx) {
    return "Invalid context";
  }

  if (error_code) {
    *error_code = ctx->error_context.last_error_code;
  }
  if (location) {
    *location = ctx->error_context.error_location;
  }
  if (soap_error) {
    *soap_error = ctx->error_context.soap_error_code;
  }

  return ctx->error_context.error_message;
}

/**
 * @brief Check if context has error
 * @param ctx Context to query
 * @return true if context has error, false otherwise
 */
bool onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return true;
  }

  return ctx->error_context.last_error_code != 0 || ctx->soap.error != SOAP_OK;
}

/**
 * @brief Get error message
 * @param ctx Context to query
 * @return Error message string, or NULL if no error
 */
const char* onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return "Invalid context";
  }

  if (ctx->error_context.error_message[0] != '\0') {
    return ctx->error_context.error_message;
  }

  if (ctx->soap.error != SOAP_OK) {
    /* Cast away const for gSOAP API - function doesn't modify the soap context */
    return soap_fault_string((struct soap*)&ctx->soap);
  }

  return NULL;
}

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
 */
int onvif_gsoap_validate_and_begin_parse(onvif_gsoap_context_t* ctx, void* out_ptr,
                                         const char* operation_name, const char* func_name) {
  platform_log_debug("%s: Starting %s parsing", func_name, operation_name);

  /* 1. Validate parameters */
  if (!ctx || !out_ptr) {
    platform_log_debug("%s: Invalid parameters - ctx=%p, out=%p", func_name, ctx, out_ptr);
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, func_name,
                            "Invalid parameters: NULL context or output pointer");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* 2. Check request parsing is initialized */
  if (!ctx->request_state.is_initialized) {
    platform_log_debug("%s: Request parsing not initialized", func_name);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, func_name, "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  platform_log_debug("%s: Request parsing is initialized, proceeding with parsing", func_name);

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = operation_name;
  ctx->request_state.parse_start_time = get_timestamp_us();

  platform_log_debug("%s: Validation completed, operation tracking started", func_name);
  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SOAP envelope structure
 * @param ctx Context with initialized request parsing
 * @param func_name Function name for error reporting (__func__)
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int onvif_gsoap_parse_soap_envelope(onvif_gsoap_context_t* ctx, const char* func_name) {
  if (!ctx) {
    return ONVIF_ERROR_INVALID;
  }

  platform_log_debug("%s: Starting SOAP envelope parsing sequence", func_name);
  platform_log_debug("%s: gSOAP context state - soap.is=%p, soap.bufidx=%lu, soap.buflen=%lu",
                     func_name, ctx->soap.is, (unsigned long)ctx->soap.bufidx,
                     (unsigned long)ctx->soap.buflen);

  int soap_result = SOAP_OK;

  /* Begin receiving */
  platform_log_debug("%s: Calling soap_begin_recv", func_name);
  soap_result = soap_begin_recv(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("%s: soap_begin_recv failed: %d", func_name, soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, func_name, "Failed to begin SOAP receive");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* Parse SOAP envelope */
  platform_log_debug("%s: Calling soap_envelope_begin_in", func_name);
  soap_result = soap_envelope_begin_in(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("%s: soap_envelope_begin_in failed: %d", func_name, soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, func_name,
                          "Failed to begin SOAP envelope parsing");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* Skip SOAP header if present */
  platform_log_debug("%s: Calling soap_recv_header", func_name);
  soap_result = soap_recv_header(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("%s: soap_recv_header failed: %d", func_name, soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, func_name, "Failed to receive SOAP header");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* Parse SOAP body */
  platform_log_debug("%s: Calling soap_body_begin_in", func_name);
  soap_result = soap_body_begin_in(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("%s: soap_body_begin_in failed: %d", func_name, soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, func_name,
                          "Failed to begin SOAP body parsing");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  platform_log_debug("%s: SOAP envelope parsing sequence completed successfully", func_name);
  return ONVIF_SUCCESS;
}

/**
 * @brief Finalize SOAP envelope parsing and complete operation timing
 * @param ctx Context to finalize
 */
int onvif_gsoap_finalize_parse(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return ONVIF_ERROR_INVALID;
  }

  platform_log_debug("onvif_gsoap_finalize_parse: Completing SOAP parsing sequence");

  /* Complete the parsing sequence - check return values for errors */
  int soap_result = soap_body_end_in(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("onvif_gsoap_finalize_parse: soap_body_end_in failed: %d", soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to finalize SOAP body parsing");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  soap_result = soap_envelope_end_in(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("onvif_gsoap_finalize_parse: soap_envelope_end_in failed: %d", soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to finalize SOAP envelope parsing");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  soap_result = soap_end_recv(&ctx->soap);
  if (soap_result != SOAP_OK) {
    platform_log_debug("onvif_gsoap_finalize_parse: soap_end_recv failed: %d", soap_result);
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to finalize SOAP receive");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  platform_log_debug("onvif_gsoap_finalize_parse: Parsing finalized successfully");
  return ONVIF_SUCCESS;
}
