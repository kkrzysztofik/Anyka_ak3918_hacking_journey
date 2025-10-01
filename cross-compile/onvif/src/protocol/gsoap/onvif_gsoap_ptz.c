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

/* ============================================================================
 * PTZ Service Response Generation - Callback Functions
 * ============================================================================
 */

int ptz_nodes_response_callback(struct soap* soap, void* user_data) {
  ptz_nodes_callback_data_t* data = (ptz_nodes_callback_data_t*)user_data;

  if (!data || !data->nodes) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif3__GetNodesResponse* response = soap_new__onvif3__GetNodesResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->__sizePTZNode = data->count;
  response->PTZNode = soap_new_onvif3__PTZNode(soap, data->count);
  if (!response->PTZNode) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  for (int i = 0; i < data->count; i++) {
    const struct ptz_node* src_node = &data->nodes[i];
    struct onvif3__PTZNode* node = &response->PTZNode[i];

    node->token = soap_strdup(soap, src_node->token);
    node->Name = soap_strdup(soap, src_node->name);
    node->FixedHomePosition = src_node->fixed_home_position ? xsd__boolean__true_ : xsd__boolean__false_;
  }

  if (soap_put__onvif3__GetNodesResponse(soap, response, "onvif3:GetNodesResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int ptz_absolute_move_response_callback(struct soap* soap, void* user_data) {
  ptz_absolute_move_callback_data_t* data = (ptz_absolute_move_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif3__AbsoluteMoveResponse* response = soap_new__onvif3__AbsoluteMoveResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__onvif3__AbsoluteMoveResponse(soap, response, "onvif3:AbsoluteMoveResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int ptz_presets_response_callback(struct soap* soap, void* user_data) {
  ptz_presets_callback_data_t* data = (ptz_presets_callback_data_t*)user_data;

  if (!data || !data->presets) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif3__GetPresetsResponse* response = soap_new__onvif3__GetPresetsResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->__sizePreset = data->count;
  response->Preset = soap_new_onvif3__PTZPreset(soap, data->count);
  if (!response->Preset) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  for (int i = 0; i < data->count; i++) {
    const struct ptz_preset* src_preset = &data->presets[i];
    struct onvif3__PTZPreset* preset = &response->Preset[i];

    preset->token = soap_strdup(soap, src_preset->token);
    preset->Name = soap_strdup(soap, src_preset->name);
    
    preset->PTZPosition = soap_new_onvif3__PTZVector(soap, 1);
    if (preset->PTZPosition) {
      preset->PTZPosition->PanTilt = soap_new_onvif3__Vector2D(soap, 1);
      if (preset->PTZPosition->PanTilt) {
        preset->PTZPosition->PanTilt->x = src_preset->position.pan;
        preset->PTZPosition->PanTilt->y = src_preset->position.tilt;
        preset->PTZPosition->PanTilt->space = soap_strdup(soap, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace");
      }
      
      preset->PTZPosition->Zoom = soap_new_onvif3__Vector1D(soap, 1);
      if (preset->PTZPosition->Zoom) {
        preset->PTZPosition->Zoom->x = src_preset->position.zoom;
        preset->PTZPosition->Zoom->space = soap_strdup(soap, "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace");
      }
    }
  }

  if (soap_put__onvif3__GetPresetsResponse(soap, response, "onvif3:GetPresetsResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int ptz_set_preset_response_callback(struct soap* soap, void* user_data) {
  ptz_set_preset_callback_data_t* data = (ptz_set_preset_callback_data_t*)user_data;

  if (!data || !data->preset_token) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif3__SetPresetResponse* response = soap_new__onvif3__SetPresetResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->PresetToken = soap_strdup(soap, data->preset_token);

  if (soap_put__onvif3__SetPresetResponse(soap, response, "onvif3:SetPresetResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int ptz_goto_preset_response_callback(struct soap* soap, void* user_data) {
  ptz_goto_preset_callback_data_t* data = (ptz_goto_preset_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif3__GotoPresetResponse* response = soap_new__onvif3__GotoPresetResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__onvif3__GotoPresetResponse(soap, response, "onvif3:GotoPresetResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PTZ Service Response Generation - Public API Functions
 * ============================================================================
 */

int onvif_gsoap_generate_get_nodes_response(onvif_gsoap_context_t* ctx,
                                            const struct ptz_node* nodes,
                                            int count) {
  if (!ctx || !ctx->soap || !nodes || count <= 0) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for get nodes response");
    return ONVIF_ERROR_INVALID;
  }

  ptz_nodes_callback_data_t callback_data = {
    .nodes = nodes,
    .count = count
  };

  return onvif_gsoap_generate_response_with_callback(ctx, ptz_nodes_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_absolute_move_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for absolute move response");
    return ONVIF_ERROR_INVALID;
  }

  ptz_absolute_move_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx, ptz_absolute_move_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_get_presets_response(onvif_gsoap_context_t* ctx,
                                              const struct ptz_preset* presets,
                                              int count) {
  if (!ctx || !ctx->soap || !presets || count < 0) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for get presets response");
    return ONVIF_ERROR_INVALID;
  }

  ptz_presets_callback_data_t callback_data = {
    .presets = presets,
    .count = count
  };

  return onvif_gsoap_generate_response_with_callback(ctx, ptz_presets_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_set_preset_response(onvif_gsoap_context_t* ctx, const char* preset_token) {
  if (!ctx || !ctx->soap || !preset_token) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for set preset response");
    return ONVIF_ERROR_INVALID;
  }

  ptz_set_preset_callback_data_t callback_data = { .preset_token = preset_token };

  return onvif_gsoap_generate_response_with_callback(ctx, ptz_set_preset_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_goto_preset_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for goto preset response");
    return ONVIF_ERROR_INVALID;
  }

  ptz_goto_preset_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx, ptz_goto_preset_response_callback,
                                                     &callback_data);
}
