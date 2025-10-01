/**
 * @file onvif_gsoap_device.c
 * @brief Device service SOAP request parsing implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements Device service request parsing functions using
 * gSOAP's generated deserialization for proper ONVIF compliance.
 *
 * All parsing functions follow a consistent pattern:
 * 1. Validate input parameters (NULL checks)
 * 2. Verify request parsing is initialized
 * 3. Set operation name and start timing
 * 4. Allocate gSOAP structure using soap_new__tds__[Operation]()
 * 5. Deserialize SOAP request using soap_read__tds__[Operation]()
 * 6. Record completion time
 *
 * Note: Some Device operations (GetDeviceInformation, GetSystemDateAndTime,
 * SystemReboot) have empty request structures with no parameters.
 *
 * The parsed structures are managed by the gSOAP context and should not
 * be manually freed by the caller.
 */

#include "protocol/gsoap/onvif_gsoap_device.h"

#include <stdio.h>
#include <string.h>

#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"

/**
 * @brief Parse GetDeviceInformation ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetDeviceInformation structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _tds__GetDeviceInformation from SOAP envelope
 * @note This is an empty request structure (no parameters)
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_device_information(onvif_gsoap_context_t* ctx,
                                               struct _tds__GetDeviceInformation** out) {
  /* 1. Validate parameters */
  if (!ctx || !out) {
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or output pointer");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* 2. Check request parsing is initialized */
  if (!ctx->request_state.is_initialized) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                          "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "GetDeviceInformation";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__tds__GetDeviceInformation(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetDeviceInformation request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__tds__GetDeviceInformation(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetDeviceInformation SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse GetCapabilities ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetCapabilities structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _tds__GetCapabilities from SOAP envelope
 * @note Extracts optional Category array to filter capability types
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_capabilities(onvif_gsoap_context_t* ctx,
                                         struct _tds__GetCapabilities** out) {
  /* 1. Validate parameters */
  if (!ctx || !out) {
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or output pointer");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* 2. Check request parsing is initialized */
  if (!ctx->request_state.is_initialized) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                          "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "GetCapabilities";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__tds__GetCapabilities(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetCapabilities request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__tds__GetCapabilities(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetCapabilities SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse GetSystemDateAndTime ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetSystemDateAndTime structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _tds__GetSystemDateAndTime from SOAP envelope
 * @note This is an empty request structure (no parameters)
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_system_date_and_time(onvif_gsoap_context_t* ctx,
                                                 struct _tds__GetSystemDateAndTime** out) {
  /* 1. Validate parameters */
  if (!ctx || !out) {
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or output pointer");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* 2. Check request parsing is initialized */
  if (!ctx->request_state.is_initialized) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                          "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "GetSystemDateAndTime";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__tds__GetSystemDateAndTime(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetSystemDateAndTime request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__tds__GetSystemDateAndTime(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetSystemDateAndTime SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SystemReboot ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SystemReboot structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _tds__SystemReboot from SOAP envelope
 * @note This is an empty request structure (no parameters)
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_system_reboot(onvif_gsoap_context_t* ctx,
                                      struct _tds__SystemReboot** out) {
  /* 1. Validate parameters */
  if (!ctx || !out) {
    if (ctx) {
      onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                            "Invalid parameters: NULL context or output pointer");
    }
    return ONVIF_ERROR_INVALID;
  }

  /* 2. Check request parsing is initialized */
  if (!ctx->request_state.is_initialized) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__,
                          "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "SystemReboot";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__tds__SystemReboot(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SystemReboot request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__tds__SystemReboot(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SystemReboot SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}
