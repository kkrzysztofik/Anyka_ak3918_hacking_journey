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

#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"

/* Include gSOAP namespace table */
#include "generated/DeviceBinding.nsmap"

/**
 * @brief Initialize gSOAP context with embedded soap structure
 * @param ctx Context to initialize
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int onvif_gsoap_init(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return ONVIF_ERROR_INVALID;
  }

  /* Clear entire context structure */
  memset(ctx, 0, sizeof(*ctx));

  /* Initialize embedded soap context (no allocation!) */
  soap_init(&ctx->soap);
  soap_set_mode(&ctx->soap, SOAP_C_UTFSTRING);
  ctx->soap.namespaces = namespaces;

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

  /* Reset state tracking structures */
  memset(&ctx->request_state, 0, sizeof(ctx->request_state));
  memset(&ctx->response_state, 0, sizeof(ctx->response_state));
  memset(&ctx->error_context, 0, sizeof(ctx->error_context));
}

/**
 * @brief Structure to track buffer reading state
 */
typedef struct {
  const char* buffer;     /* Source buffer */
  size_t size;            /* Total buffer size */
  size_t position;        /* Current read position */
} buffer_read_state_t;

/**
 * @brief Receive callback for reading from memory buffer
 * @param soap gSOAP runtime context
 * @param buf Buffer to read into
 * @param len Maximum bytes to read
 * @return Number of bytes read, or 0 for EOF, or -1 for error
 */
static size_t frecv_buffer(struct soap* soap, char* buf, size_t len) {
  buffer_read_state_t* state = (buffer_read_state_t*)soap->user;
  if (!state || !buf) {
    return 0;
  }

  /* Calculate how many bytes are remaining in the buffer */
  size_t remaining = state->size - state->position;
  if (remaining == 0) {
    return 0; /* EOF */
  }

  /* Read up to len bytes from the buffer */
  size_t to_read = (len < remaining) ? len : remaining;
  memcpy(buf, state->buffer + state->position, to_read);
  state->position += to_read;

  return to_read;
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
  if (!ctx || !request_xml || xml_size == 0) {
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or request");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* Configure soap context for parsing from buffer */
  soap_begin(&ctx->soap);

  /* Allocate and copy the request XML to soap-managed memory
   * This ensures the buffer remains valid during parsing */
  char* buffer = (char*)soap_malloc(&ctx->soap, xml_size + 1);
  if (!buffer) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate buffer for request");
    return ONVIF_ERROR_MEMORY;
  }

  memcpy(buffer, request_xml, xml_size);
  buffer[xml_size] = '\0';

  /* Allocate buffer read state structure */
  buffer_read_state_t* read_state = (buffer_read_state_t*)soap_malloc(
    &ctx->soap, sizeof(buffer_read_state_t));
  if (!read_state) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate read state");
    return ONVIF_ERROR_MEMORY;
  }

  read_state->buffer = buffer;
  read_state->size = xml_size;
  read_state->position = 0;

  /* Configure gSOAP to read from the buffer using frecv callback
   * This is the standard way to parse from memory in gSOAP */
  ctx->soap.user = read_state;          /* Store read state in user field */
  ctx->soap.frecv = frecv_buffer;       /* Set custom receive callback */
  ctx->soap.recvfd = -1;                /* No file descriptor */

  /* NOTE: We do NOT call soap_begin_recv() here because the soap_read_* macros
   * call it themselves. Calling it twice would reset the parsing state. */

  /* Update request state */
  ctx->request_state.is_initialized = true;
  ctx->request_state.request_size = xml_size;
  ctx->request_state.parse_start_time = get_timestamp_us();

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
