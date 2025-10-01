/**
 * @file onvif_gsoap_media.c
 * @brief Media service SOAP request parsing implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements Media service request parsing functions using
 * gSOAP's generated deserialization for proper ONVIF compliance.
 */

#include "protocol/gsoap/onvif_gsoap_media.h"

#include <stdio.h>
#include <string.h>

#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"

/**
 * @brief Parse GetProfiles ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetProfiles structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note GetProfiles has no parameters, but structure must still be parsed
 */
int onvif_gsoap_parse_get_profiles(onvif_gsoap_context_t* ctx,
                                    struct _trt__GetProfiles** out) {
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
  ctx->request_state.operation_name = "GetProfiles";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__GetProfiles(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetProfiles request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__GetProfiles(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetProfiles SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse GetStreamUri ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetStreamUri structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and StreamSetup fields
 */
int onvif_gsoap_parse_get_stream_uri(onvif_gsoap_context_t* ctx,
                                      struct _trt__GetStreamUri** out) {
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
  ctx->request_state.operation_name = "GetStreamUri";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__GetStreamUri(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetStreamUri request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__GetStreamUri(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetStreamUri SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse CreateProfile ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed CreateProfile structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Name and Token fields for profile creation
 */
int onvif_gsoap_parse_create_profile(onvif_gsoap_context_t* ctx,
                                      struct _trt__CreateProfile** out) {
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
  ctx->request_state.operation_name = "CreateProfile";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__CreateProfile(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate CreateProfile request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__CreateProfile(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse CreateProfile SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse DeleteProfile ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed DeleteProfile structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken field for profile deletion
 */
int onvif_gsoap_parse_delete_profile(onvif_gsoap_context_t* ctx,
                                      struct _trt__DeleteProfile** out) {
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
  ctx->request_state.operation_name = "DeleteProfile";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__DeleteProfile(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate DeleteProfile request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__DeleteProfile(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse DeleteProfile SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SetVideoSourceConfiguration ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetVideoSourceConfiguration structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Configuration and ForcePersistence fields
 */
int onvif_gsoap_parse_set_video_source_config(onvif_gsoap_context_t* ctx,
                                               struct _trt__SetVideoSourceConfiguration** out) {
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
  ctx->request_state.operation_name = "SetVideoSourceConfiguration";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__SetVideoSourceConfiguration(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SetVideoSourceConfiguration request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__SetVideoSourceConfiguration(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SetVideoSourceConfiguration SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SetVideoEncoderConfiguration ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetVideoEncoderConfiguration structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Configuration and ForcePersistence fields
 */
int onvif_gsoap_parse_set_video_encoder_config(onvif_gsoap_context_t* ctx,
                                                struct _trt__SetVideoEncoderConfiguration** out) {
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
  ctx->request_state.operation_name = "SetVideoEncoderConfiguration";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__trt__SetVideoEncoderConfiguration(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SetVideoEncoderConfiguration request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__trt__SetVideoEncoderConfiguration(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SetVideoEncoderConfiguration SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}
