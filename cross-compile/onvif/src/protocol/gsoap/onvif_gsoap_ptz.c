/**
 * @file onvif_gsoap_ptz.c
 * @brief PTZ service SOAP request parsing implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements PTZ service request parsing functions using
 * gSOAP's generated deserialization for proper ONVIF compliance.
 *
 * All parsing functions follow a consistent pattern:
 * 1. Validate input parameters (NULL checks)
 * 2. Verify request parsing is initialized
 * 3. Set operation name and start timing
 * 4. Allocate gSOAP structure using soap_new__onvif3__[Operation]()
 * 5. Deserialize SOAP request using soap_read__onvif3__[Operation]()
 * 6. Record completion time
 *
 * The parsed structures are managed by the gSOAP context and should not
 * be manually freed by the caller.
 */

#include "protocol/gsoap/onvif_gsoap_ptz.h"

#include <stdio.h>
#include <string.h>

#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"

/**
 * @brief Parse GetNodes ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetNodes structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__GetNodes from SOAP envelope
 * @note Retrieves PTZ node information including capabilities
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_nodes(onvif_gsoap_context_t* ctx,
                                 struct _onvif3__GetNodes** out) {
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
  ctx->request_state.operation_name = "GetNodes";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__GetNodes(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetNodes request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__GetNodes(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetNodes SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse AbsoluteMove ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed AbsoluteMove structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__AbsoluteMove from SOAP envelope
 * @note Extracts ProfileToken, Position (PanTilt and Zoom), and optional Speed
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_absolute_move(onvif_gsoap_context_t* ctx,
                                     struct _onvif3__AbsoluteMove** out) {
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
  ctx->request_state.operation_name = "AbsoluteMove";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__AbsoluteMove(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate AbsoluteMove request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__AbsoluteMove(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse AbsoluteMove SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse GetPresets ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetPresets structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__GetPresets from SOAP envelope
 * @note Extracts ProfileToken to retrieve configured presets
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_presets(onvif_gsoap_context_t* ctx,
                                   struct _onvif3__GetPresets** out) {
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
  ctx->request_state.operation_name = "GetPresets";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__GetPresets(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetPresets request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__GetPresets(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetPresets SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SetPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__SetPreset from SOAP envelope
 * @note Extracts ProfileToken and optional PresetToken/PresetName
 * @note If PresetToken is NULL, creates new preset; otherwise updates existing
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_set_preset(onvif_gsoap_context_t* ctx,
                                  struct _onvif3__SetPreset** out) {
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
  ctx->request_state.operation_name = "SetPreset";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__SetPreset(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SetPreset request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__SetPreset(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SetPreset SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse GotoPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GotoPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__GotoPreset from SOAP envelope
 * @note Extracts ProfileToken, PresetToken, and optional Speed
 * @note PresetToken identifies which preset position to move to
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_goto_preset(onvif_gsoap_context_t* ctx,
                                   struct _onvif3__GotoPreset** out) {
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
  ctx->request_state.operation_name = "GotoPreset";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__GotoPreset(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GotoPreset request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__GotoPreset(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GotoPreset SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse RemovePreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed RemovePreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif3__RemovePreset from SOAP envelope
 * @note Extracts ProfileToken and PresetToken for preset deletion
 * @note PresetToken identifies which preset to remove from profile
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_remove_preset(onvif_gsoap_context_t* ctx,
                                     struct _onvif3__RemovePreset** out) {
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
  ctx->request_state.operation_name = "RemovePreset";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif3__RemovePreset(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate RemovePreset request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif3__RemovePreset(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse RemovePreset SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}
