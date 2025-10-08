/**
 * @file onvif_gsoap_response.c
 * @brief Generic ONVIF gSOAP response generation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "protocol/gsoap/onvif_gsoap_response.h"

#include <errno.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "utils/error/error_handling.h"

// Include gSOAP generated files
#include "generated/soapH.h"
#include "generated/soapStub.h"

/* ============================================================================
 * Constants
 * ============================================================================
 */

#define ONVIF_GSOAP_ERROR_MSG_SIZE 256

// Time conversion constants
#define SECONDS_TO_MICROSECONDS     1000000ULL
#define NANOSECONDS_TO_MICROSECONDS 1000ULL

/* ============================================================================
 * Internal Type Definitions
 * ============================================================================
 */

#define FAULT_CODE_MAX_LEN   64
#define FAULT_STRING_MAX_LEN 256
#define FAULT_ACTOR_MAX_LEN  128
#define FAULT_DETAIL_MAX_LEN 512

/**
 * @brief Callback data structure for fault response generation
 */
typedef struct {
  char fault_code[FAULT_CODE_MAX_LEN];
  char fault_string[FAULT_STRING_MAX_LEN];
  char fault_actor[FAULT_ACTOR_MAX_LEN];
  char fault_detail[FAULT_DETAIL_MAX_LEN];
} fault_callback_data_t;

/* ============================================================================
 * Global Variables
 * ============================================================================
 */

static char g_onvif_gsoap_error_msg[ONVIF_GSOAP_ERROR_MSG_SIZE] = {0};

/* ============================================================================
 * Internal Helper Functions
 * ============================================================================
 */

/**
 * @brief Get current timestamp in microseconds
 * @return Current timestamp in microseconds
 */
static uint64_t get_timestamp_us(void) {
  struct timespec timespec_val;
  clock_gettime(CLOCK_MONOTONIC, &timespec_val);
  return (uint64_t)timespec_val.tv_sec * SECONDS_TO_MICROSECONDS +
         timespec_val.tv_nsec / NANOSECONDS_TO_MICROSECONDS;
}

/**
 * @brief Callback function for SOAP fault response generation
 * @param soap gSOAP context
 * @param user_data Pointer to fault_callback_data_t
 * @return 0 on success, negative error code on failure
 */
static int fault_response_callback(struct soap* soap, void* user_data) {
  fault_callback_data_t* data = (fault_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  // Create fault structure using generated gSOAP functions
  struct SOAP_ENV__Fault* fault = soap_new_SOAP_ENV__Fault(soap, 1);
  if (!fault) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Initialize fault structure with defaults
  soap_default_SOAP_ENV__Fault(soap, fault);

  // SOAP 1.2: Create Code structure with Value
  fault->SOAP_ENV__Code = soap_new_SOAP_ENV__Code(soap, 1);
  if (!fault->SOAP_ENV__Code) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }
  soap_default_SOAP_ENV__Code(soap, fault->SOAP_ENV__Code);
  fault->SOAP_ENV__Code->SOAP_ENV__Value = soap_strdup(soap, data->fault_code);

  // SOAP 1.2: Create Reason structure with Text
  fault->SOAP_ENV__Reason = soap_new_SOAP_ENV__Reason(soap, 1);
  if (!fault->SOAP_ENV__Reason) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }
  soap_default_SOAP_ENV__Reason(soap, fault->SOAP_ENV__Reason);
  fault->SOAP_ENV__Reason->SOAP_ENV__Text = soap_strdup(soap, data->fault_string);

  // SOAP 1.2: Set optional Role (actor) if provided
  if (data->fault_actor[0] != '\0') {
    fault->SOAP_ENV__Role = soap_strdup(soap, data->fault_actor);
  }

  // SOAP 1.2: Set optional Detail if provided
  if (data->fault_detail[0] != '\0') {
    fault->SOAP_ENV__Detail = soap_new_SOAP_ENV__Detail(soap, 1);
    if (fault->SOAP_ENV__Detail) {
      fault->SOAP_ENV__Detail->__any = soap_strdup(soap, data->fault_detail);
    }
  }

  // Serialize fault within SOAP body using generated gSOAP function
  if (soap_put_SOAP_ENV__Fault(soap, fault, "SOAP-ENV:Fault", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Public API Functions
 * ============================================================================
 */

int onvif_gsoap_validate_context(struct soap* soap) {
  if (!soap) {
    return ONVIF_ERROR_INVALID;
  }

  // Ensure fault structure is available for error handling
  if (!soap->fault) {
    soap->fault = soap_new_SOAP_ENV__Fault(soap, 1);
    if (!soap->fault) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }
  }

  return ONVIF_SUCCESS;
}

static void set_soap_error(struct soap* soap, const char* format, ...) {
  va_list args;
  va_start(args, format);
  (void)vsnprintf(g_onvif_gsoap_error_msg, sizeof(g_onvif_gsoap_error_msg), format, args);
  va_end(args);

  if (soap) {
    soap->error = SOAP_FAULT;
    // Ensure fault structure exists before accessing it
    if (onvif_gsoap_validate_context(soap) == ONVIF_SUCCESS && soap->fault) {
      soap->fault->faultstring = soap_strdup(soap, g_onvif_gsoap_error_msg);
    }
  }
  platform_log_error("ONVIF gSOAP Error: %s", g_onvif_gsoap_error_msg);
}

int onvif_gsoap_serialize_response(onvif_gsoap_context_t* ctx, void* response_data) {
  if (!ctx || !response_data) {
    set_soap_error(ctx ? &ctx->soap : NULL, "Invalid parameters for serialize response");
    return ONVIF_ERROR_INVALID;
  }

  // Start timing
  ctx->response_state.generation_start_time = get_timestamp_us();

  // Begin SOAP response
  if (soap_begin_send(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to begin SOAP send");
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  platform_log_debug("ONVIF gSOAP: Started response serialization");
  return ONVIF_SUCCESS;
}

int onvif_gsoap_finalize_response(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    set_soap_error(ctx ? &ctx->soap : NULL, "NULL context pointer");
    return ONVIF_ERROR_INVALID;
  }

  // End SOAP response
  if (soap_end_send(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to end SOAP send");
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  // Update statistics
  ctx->response_state.generation_end_time = get_timestamp_us();
  ctx->response_state.total_bytes_written = ctx->soap.length;

  platform_log_debug("ONVIF gSOAP: Finalized response (%zu bytes, %llu us)",
                     ctx->response_state.total_bytes_written,
                     (unsigned long long)(ctx->response_state.generation_end_time -
                                          ctx->response_state.generation_start_time));
  return ONVIF_SUCCESS;
}

int onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                onvif_response_callback_t callback,
                                                void* user_data) {
  if (!ctx || !callback) {
    if (ctx) {
      set_soap_error(&ctx->soap, "Invalid parameters for response generation");
    }
    return ONVIF_ERROR_INVALID;
  }

  // Ensure gSOAP context is properly initialized for fault handling
  if (onvif_gsoap_validate_context(&ctx->soap) != ONVIF_SUCCESS) {
    platform_log_error("ONVIF gSOAP: Failed to initialize fault handling context");
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Set up gSOAP for string output - this is the correct way to get XML as a
  // string
  char* output_string = NULL;
  ctx->soap.os = (void*)&output_string;

  // Begin SOAP send with string output mode
  if (soap_begin_send(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to begin SOAP send");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  // Use gSOAP's proper envelope functions for complete SOAP envelope generation
  if (soap_envelope_begin_out(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to begin SOAP envelope");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  if (soap_body_begin_out(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to begin SOAP body");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  // Call the endpoint-specific callback to generate the response content
  int callback_result = callback(&ctx->soap, user_data);
  if (callback_result != ONVIF_SUCCESS) {
    set_soap_error(&ctx->soap, "Callback failed to generate response content");
    ctx->soap.os = NULL;
    return callback_result;
  }

  if (soap_body_end_out(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to end SOAP body");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  if (soap_envelope_end_out(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to end SOAP envelope");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  if (soap_end_send(&ctx->soap) != SOAP_OK) {
    set_soap_error(&ctx->soap, "Failed to end SOAP send");
    ctx->soap.os = NULL;
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  // Clear the output stream pointer
  ctx->soap.os = NULL;

  // Copy the generated string to our buffer
  if (output_string) {
    size_t response_len = strlen(output_string);
    platform_log_debug("ONVIF gSOAP: output_string length=%zu, content=%s", response_len,
                       output_string);
    if (response_len < sizeof(ctx->soap.buf)) {
      strncpy(ctx->soap.buf, output_string, sizeof(ctx->soap.buf) - 1);
      ctx->soap.buf[sizeof(ctx->soap.buf) - 1] = '\0';
      ctx->soap.length = response_len;
      platform_log_debug("ONVIF gSOAP: Copied to buffer, length set to %zu", ctx->soap.length);
    } else {
      set_soap_error(&ctx->soap, "Response too large for buffer");
      return ONVIF_ERROR_SERIALIZATION_FAILED;
    }
  } else {
    platform_log_error("ONVIF gSOAP: output_string is NULL after callback");
    set_soap_error(&ctx->soap, "No output string generated");
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  // Debug: Check buffer after generation
  platform_log_debug("ONVIF gSOAP: Buffer after generation: ptr=%p, length=%zu", ctx->soap.buf,
                     ctx->soap.length);

  platform_log_debug("ONVIF gSOAP: Generated response with callback");
  return ONVIF_SUCCESS;
}

int onvif_gsoap_validate_response(const onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return ONVIF_ERROR_INVALID;
  }

  if (ctx->soap.error != SOAP_OK) {
    return ONVIF_ERROR_INVALID;
  }

  // Return error if no response has been generated
  if (ctx->soap.length == 0) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int onvif_gsoap_extract_operation_name(const char* request_data, size_t request_size,
                                       char* operation_name, size_t operation_name_size) {
  if (!request_data || request_size == 0 || !operation_name || operation_name_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context for proper XML parsing
  struct soap soap_ctx;
  soap_init(&soap_ctx);
  soap_set_mode(&soap_ctx, SOAP_C_UTFSTRING | SOAP_XML_STRICT);

  int result = ONVIF_ERROR_PARSE_FAILED;

  // Set up input buffer for gSOAP parsing
  soap_ctx.is = (char*)request_data;
  soap_ctx.bufidx = 0;
  soap_ctx.buflen = request_size;
  soap_ctx.ahead = 0;

  // Use gSOAP's XML parser to parse the SOAP envelope
  if (soap_begin_recv(&soap_ctx) == SOAP_OK) {
    // Parse SOAP envelope
    if (soap_envelope_begin_in(&soap_ctx) == SOAP_OK) {
      // Skip SOAP header if present
      if (soap_recv_header(&soap_ctx) == SOAP_OK) {
        // Parse SOAP body start
        if (soap_body_begin_in(&soap_ctx) == SOAP_OK) {
          // Get the next XML element tag - this is our operation
          if (soap_element_begin_in(&soap_ctx, NULL, 0, NULL) == SOAP_OK) {
            // The soap_ctx.tag now contains the operation element name
            const char* tag = soap_ctx.tag;
            if (tag) {
              // Extract operation name, handling namespace prefixes
              const char* operation = tag;
              const char* colon = strchr(tag, ':');
              if (colon) {
                operation = colon + 1; // Skip namespace prefix
              }

              size_t op_len = strlen(operation);
              if (op_len > 0 && op_len < operation_name_size) {
                strncpy(operation_name, operation, operation_name_size - 1);
                operation_name[operation_name_size - 1] = '\0';
                result = ONVIF_SUCCESS;
              }
            }
          }
        }
      }
    }
  }

  // Check for gSOAP parsing errors
  if (result != ONVIF_SUCCESS && soap_ctx.error != SOAP_OK) {
    // All parsing errors map to the same result for simplicity
    result = ONVIF_ERROR_PARSE_FAILED;
  }

  soap_end(&soap_ctx);
  soap_done(&soap_ctx);

  return result;
}

int onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, const char* fault_code,
                                        const char* fault_string, const char* fault_actor,
                                        const char* fault_detail, char* output_buffer,
                                        size_t buffer_size) {
  // Validate required parameters
  if (!fault_string) {
    return ONVIF_ERROR_INVALID;
  }

  // Use default SOAP 1.2 fault code if not provided
  const char* default_fault_code = "SOAP-ENV:Receiver";
  const char* actual_fault_code = fault_code ? fault_code : default_fault_code;

  // Determine if we need to create a temporary context
  bool temp_ctx = (ctx == NULL);
  onvif_gsoap_context_t* actual_ctx = ctx;

  // Create temporary context if needed
  if (temp_ctx) {
    actual_ctx = (onvif_gsoap_context_t*)malloc(sizeof(onvif_gsoap_context_t));
    if (!actual_ctx) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }

    if (onvif_gsoap_init(actual_ctx) != ONVIF_SUCCESS) {
      free(actual_ctx);
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }
  }

  // Validate context
  if (!actual_ctx) {
    if (temp_ctx && actual_ctx) {
      free(actual_ctx);
    }
    return ONVIF_ERROR_INVALID;
  }

  // Prepare callback data
  fault_callback_data_t callback_data;
  memset(&callback_data, 0, sizeof(callback_data));

  // Set fault code and string
  strncpy(callback_data.fault_code, actual_fault_code, sizeof(callback_data.fault_code) - 1);
  callback_data.fault_code[sizeof(callback_data.fault_code) - 1] = '\0';

  strncpy(callback_data.fault_string, fault_string, sizeof(callback_data.fault_string) - 1);
  callback_data.fault_string[sizeof(callback_data.fault_string) - 1] = '\0';

  // Set optional fault actor
  if (fault_actor) {
    strncpy(callback_data.fault_actor, fault_actor, sizeof(callback_data.fault_actor) - 1);
    callback_data.fault_actor[sizeof(callback_data.fault_actor) - 1] = '\0';
  }

  // Set optional fault detail
  if (fault_detail) {
    strncpy(callback_data.fault_detail, fault_detail, sizeof(callback_data.fault_detail) - 1);
    callback_data.fault_detail[sizeof(callback_data.fault_detail) - 1] = '\0';
  }

  // Generate fault response
  int result = onvif_gsoap_generate_response_with_callback(actual_ctx, fault_response_callback,
                                                           &callback_data);
  if (result != ONVIF_SUCCESS) {
    if (temp_ctx) {
      onvif_gsoap_cleanup(actual_ctx);
      free(actual_ctx);
    }
    return result;
  }

  // If output buffer is provided, copy the generated XML
  if (output_buffer && buffer_size > 0) {
    const char* response_data = onvif_gsoap_get_response_data(actual_ctx);
    size_t response_length = onvif_gsoap_get_response_length(actual_ctx);

    if (response_data && response_length > 0) {
      if (response_length < buffer_size) {
        strncpy(output_buffer, response_data, buffer_size - 1);
        output_buffer[buffer_size - 1] = '\0';
        result = (int)response_length;
      } else {
        result = ONVIF_ERROR_MEMORY;
      }
    } else {
      result = ONVIF_ERROR_IO;
    }
  }

  // Clean up temporary context if created
  if (temp_ctx) {
    onvif_gsoap_cleanup(actual_ctx);
    free(actual_ctx);
  }

  if (result == ONVIF_SUCCESS) {
    platform_log_debug("ONVIF gSOAP: Generated fault response: %s - %s", callback_data.fault_code,
                       callback_data.fault_string);
  }

  return result;
}

/* ============================================================================
 * Response Utility Functions
 * ============================================================================
 */

/**
 * @brief Get response data from context
 * @param ctx gSOAP context
 * @return Response data string, or NULL if no response
 */
const char* onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return NULL;
  }

  /* Return NULL if no response has been generated (length is 0) */
  if (ctx->soap.length == 0) {
    return NULL;
  }

  return ctx->soap.buf;
}

/**
 * @brief Get response data length from context
 * @param ctx gSOAP context
 * @return Response data length in bytes
 */
size_t onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return 0;
  }

  return ctx->soap.length;
}

// onvif_gsoap_has_error is defined in onvif_gsoap_core.c
