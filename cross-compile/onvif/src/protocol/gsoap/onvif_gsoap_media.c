/**
 * @file onvif_gsoap_media.c
 * @brief Media service SOAP request parsing implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements Media service request parsing functions using
 * gSOAP's generated deserialization for proper ONVIF compliance.
 *
 * All parsing functions follow a consistent pattern:
 * 1. Validate input parameters (NULL checks)
 * 2. Verify request parsing is initialized
 * 3. Set operation name and start timing
 * 4. Allocate gSOAP structure using soap_new__trt__[Operation]()
 * 5. Deserialize SOAP request using soap_read__trt__[Operation]()
 * 6. Record completion time
 *
 * The parsed structures are managed by the gSOAP context and should not
 * be manually freed by the caller.
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
 * @note Parses struct _trt__GetProfiles from SOAP envelope
 * @note GetProfiles has no request parameters (empty structure)
 * @note Output structure is allocated and managed by gSOAP context
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
 * @note Parses struct _trt__GetStreamUri from SOAP envelope
 * @note Extracts ProfileToken (char*) and StreamSetup (Protocol, Transport) fields
 * @note Output structure is allocated and managed by gSOAP context
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
 * @note Parses struct _trt__CreateProfile from SOAP envelope
 * @note Extracts Name (char*) and Token (char*) fields for profile creation
 * @note Output structure is allocated and managed by gSOAP context
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
 * @note Parses struct _trt__DeleteProfile from SOAP envelope
 * @note Extracts ProfileToken (char*) field for profile deletion
 * @note Output structure is allocated and managed by gSOAP context
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
 * @note Parses struct _trt__SetVideoSourceConfiguration from SOAP envelope
 * @note Extracts Configuration (Name, Token, Bounds, SourceToken) and ForcePersistence (bool)
 * @note Output structure is allocated and managed by gSOAP context
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
 * @note Parses struct _trt__SetVideoEncoderConfiguration from SOAP envelope
 * @note Extracts Configuration (Name, Token, Encoding, Resolution, Quality, RateControl) and ForcePersistence (bool)
 * @note Output structure is allocated and managed by gSOAP context
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

/* ============================================================================
 * Media Service Response Generation - Callback Functions
 * ============================================================================
 */

int media_profiles_response_callback(struct soap* soap, void* user_data) {
  media_profiles_callback_data_t* data = (media_profiles_callback_data_t*)user_data;

  if (!data || !data->profiles) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__GetProfilesResponse* response = soap_new__trt__GetProfilesResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->__sizeProfiles = data->profile_count;
  response->Profiles = soap_new_tt__Profile(soap, data->profile_count);
  if (!response->Profiles) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  for (int i = 0; i < data->profile_count; i++) {
    const struct media_profile* src_profile = &data->profiles[i];
    struct tt__Profile* profile = &response->Profiles[i];

    profile->token = soap_strdup(soap, src_profile->token);
    profile->Name = soap_strdup(soap, src_profile->name);
    profile->fixed = soap_new_xsd__boolean(soap, 1);
    if (profile->fixed) {
      *(profile->fixed) = src_profile->fixed ? xsd__boolean__true_ : xsd__boolean__false_;
    }

    profile->VideoSourceConfiguration = soap_new_tt__VideoSourceConfiguration(soap, 1);
    if (profile->VideoSourceConfiguration) {
      profile->VideoSourceConfiguration->token = soap_strdup(soap, "VideoSourceConfig0");
      profile->VideoSourceConfiguration->Name = soap_strdup(soap, "Video Source Configuration");
      profile->VideoSourceConfiguration->UseCount = 1;
      profile->VideoSourceConfiguration->SourceToken =
        soap_strdup(soap, src_profile->video_source.source_token);
      profile->VideoSourceConfiguration->Bounds = soap_new_tt__IntRectangle(soap, 1);
      if (profile->VideoSourceConfiguration->Bounds) {
        profile->VideoSourceConfiguration->Bounds->x = src_profile->video_source.bounds.x;
        profile->VideoSourceConfiguration->Bounds->y = src_profile->video_source.bounds.y;
        profile->VideoSourceConfiguration->Bounds->width = src_profile->video_source.bounds.width;
        profile->VideoSourceConfiguration->Bounds->height = src_profile->video_source.bounds.height;
      }
    }

    profile->VideoEncoderConfiguration = soap_new_tt__VideoEncoderConfiguration(soap, 1);
    if (profile->VideoEncoderConfiguration) {
      profile->VideoEncoderConfiguration->token =
        soap_strdup(soap, src_profile->video_encoder.token);
      profile->VideoEncoderConfiguration->Name = soap_strdup(soap, "Video Encoder Configuration");
      profile->VideoEncoderConfiguration->UseCount = 1;
      profile->VideoEncoderConfiguration->Encoding = tt__VideoEncoding__H264;
      profile->VideoEncoderConfiguration->Resolution = soap_new_tt__VideoResolution(soap, 1);
      if (profile->VideoEncoderConfiguration->Resolution) {
        profile->VideoEncoderConfiguration->Resolution->Width =
          src_profile->video_encoder.resolution.width;
        profile->VideoEncoderConfiguration->Resolution->Height =
          src_profile->video_encoder.resolution.height;
      }
      profile->VideoEncoderConfiguration->Quality = src_profile->video_encoder.quality;
    }

    profile->AudioSourceConfiguration = soap_new_tt__AudioSourceConfiguration(soap, 1);
    if (profile->AudioSourceConfiguration) {
      profile->AudioSourceConfiguration->token = soap_strdup(soap, "AudioSourceConfig0");
      profile->AudioSourceConfiguration->Name = soap_strdup(soap, "Audio Source Configuration");
      profile->AudioSourceConfiguration->UseCount = 1;
      profile->AudioSourceConfiguration->SourceToken =
        soap_strdup(soap, src_profile->audio_source.source_token);
    }

    profile->AudioEncoderConfiguration = soap_new_tt__AudioEncoderConfiguration(soap, 1);
    if (profile->AudioEncoderConfiguration) {
      profile->AudioEncoderConfiguration->token =
        soap_strdup(soap, src_profile->audio_encoder.token);
      profile->AudioEncoderConfiguration->Name = soap_strdup(soap, "Audio Encoder Configuration");
      profile->AudioEncoderConfiguration->UseCount = 1;
      profile->AudioEncoderConfiguration->Encoding = tt__AudioEncoding__AAC;
      profile->AudioEncoderConfiguration->Bitrate = src_profile->audio_encoder.bitrate;
      profile->AudioEncoderConfiguration->SampleRate = src_profile->audio_encoder.sample_rate;
    }

    profile->PTZConfiguration = soap_new_tt__PTZConfiguration(soap, 1);
    if (profile->PTZConfiguration) {
      profile->PTZConfiguration->token = soap_strdup(soap, "PTZConfig0");
      profile->PTZConfiguration->Name = soap_strdup(soap, "PTZ Configuration");
      profile->PTZConfiguration->UseCount = 1;
      profile->PTZConfiguration->NodeToken = soap_strdup(soap, src_profile->ptz.node_token);
      profile->PTZConfiguration->DefaultAbsolutePantTiltPositionSpace =
        soap_strdup(soap, src_profile->ptz.default_absolute_pan_tilt_position_space);
      profile->PTZConfiguration->DefaultAbsoluteZoomPositionSpace =
        soap_strdup(soap, src_profile->ptz.default_absolute_zoom_position_space);
      profile->PTZConfiguration->DefaultRelativePanTiltTranslationSpace =
        soap_strdup(soap, src_profile->ptz.default_relative_pan_tilt_translation_space);
      profile->PTZConfiguration->DefaultRelativeZoomTranslationSpace =
        soap_strdup(soap, src_profile->ptz.default_relative_zoom_translation_space);
      profile->PTZConfiguration->DefaultContinuousPanTiltVelocitySpace =
        soap_strdup(soap, src_profile->ptz.default_continuous_pan_tilt_velocity_space);
      profile->PTZConfiguration->DefaultContinuousZoomVelocitySpace =
        soap_strdup(soap, src_profile->ptz.default_continuous_zoom_velocity_space);
    }
  }

  if (soap_put__trt__GetProfilesResponse(soap, response, "trt:GetProfilesResponse", NULL) !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_stream_uri_response_callback(struct soap* soap, void* user_data) {
  media_stream_uri_callback_data_t* data = (media_stream_uri_callback_data_t*)user_data;

  if (!data || !data->uri) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__GetStreamUriResponse* response = soap_new__trt__GetStreamUriResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->MediaUri = soap_new_tt__MediaUri(soap, 1);
  if (!response->MediaUri) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->MediaUri->Uri = soap_strdup(soap, data->uri->uri);
  response->MediaUri->InvalidAfterConnect =
    data->uri->invalid_after_connect ? xsd__boolean__true_ : xsd__boolean__false_;
  response->MediaUri->InvalidAfterReboot =
    data->uri->invalid_after_reboot ? xsd__boolean__true_ : xsd__boolean__false_;
  static const char* timeout_str = "PT60S";
  response->MediaUri->Timeout = soap_strdup(soap, timeout_str);

  if (soap_put__trt__GetStreamUriResponse(soap, response, "trt:GetStreamUriResponse", NULL) !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_create_profile_response_callback(struct soap* soap, void* user_data) {
  media_create_profile_callback_data_t* data = (media_create_profile_callback_data_t*)user_data;

  if (!data || !data->profile) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__CreateProfileResponse* response = soap_new__trt__CreateProfileResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->Profile = soap_new_tt__Profile(soap, 1);
  if (!response->Profile) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  const struct media_profile* src_profile = data->profile;
  struct tt__Profile* profile = response->Profile;

  profile->token = soap_strdup(soap, src_profile->token);
  profile->Name = soap_strdup(soap, src_profile->name);
  profile->fixed = soap_new_xsd__boolean(soap, 1);
  if (profile->fixed) {
    *(profile->fixed) = src_profile->fixed ? xsd__boolean__true_ : xsd__boolean__false_;
  }

  if (soap_put__trt__CreateProfileResponse(soap, response, "trt:CreateProfileResponse", NULL) !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_delete_profile_response_callback(struct soap* soap, void* user_data) {
  media_delete_profile_callback_data_t* data = (media_delete_profile_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__DeleteProfileResponse* response = soap_new__trt__DeleteProfileResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__DeleteProfileResponse(soap, response, "trt:DeleteProfileResponse", NULL) !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_set_video_source_config_response_callback(struct soap* soap, void* user_data) {
  media_set_video_source_config_callback_data_t* data =
    (media_set_video_source_config_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__SetVideoSourceConfigurationResponse* response =
    soap_new__trt__SetVideoSourceConfigurationResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__SetVideoSourceConfigurationResponse(
        soap, response, "trt:SetVideoSourceConfigurationResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_set_video_encoder_config_response_callback(struct soap* soap, void* user_data) {
  media_set_video_encoder_config_callback_data_t* data =
    (media_set_video_encoder_config_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__SetVideoEncoderConfigurationResponse* response =
    soap_new__trt__SetVideoEncoderConfigurationResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__SetVideoEncoderConfigurationResponse(
        soap, response, "trt:SetVideoEncoderConfigurationResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_start_multicast_response_callback(struct soap* soap, void* user_data) {
  media_start_multicast_callback_data_t* data = (media_start_multicast_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__StartMulticastStreamingResponse* response =
    soap_new__trt__StartMulticastStreamingResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__StartMulticastStreamingResponse(
        soap, response, "trt:StartMulticastStreamingResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_stop_multicast_response_callback(struct soap* soap, void* user_data) {
  media_stop_multicast_callback_data_t* data = (media_stop_multicast_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__StopMulticastStreamingResponse* response =
    soap_new__trt__StopMulticastStreamingResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__StopMulticastStreamingResponse(
        soap, response, "trt:StopMulticastStreamingResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_get_metadata_configs_response_callback(struct soap* soap, void* user_data) {
  media_get_metadata_configs_callback_data_t* data =
    (media_get_metadata_configs_callback_data_t*)user_data;

  if (!data || !data->configs) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__GetMetadataConfigurationsResponse* response =
    soap_new__trt__GetMetadataConfigurationsResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->__sizeConfigurations = data->config_count;
  response->Configurations = soap_new_tt__MetadataConfiguration(soap, data->config_count);
  if (!response->Configurations) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  for (int i = 0; i < data->config_count; i++) {
    const struct metadata_configuration* src_config = &data->configs[i];
    struct tt__MetadataConfiguration* config = &response->Configurations[i];

    config->token = soap_strdup(soap, src_config->token);
    config->Name = soap_strdup(soap, src_config->name);
    config->UseCount = src_config->use_count;
    config->SessionTimeout = soap_strdup(soap, "PT60S");
    config->Analytics = soap_new_xsd__boolean(soap, 1);
    if (config->Analytics) {
      *(config->Analytics) = src_config->analytics ? xsd__boolean__true_ : xsd__boolean__false_;
    }
    config->Multicast = soap_new_tt__MulticastConfiguration(soap, 1);
    if (config->Multicast) {
      config->Multicast->Address = soap_new_tt__IPAddress(soap, 1);
      if (config->Multicast->Address) {
        config->Multicast->Address->Type = tt__IPType__IPv4;
        config->Multicast->Address->IPv4Address = soap_strdup(soap, src_config->multicast.address);
      }
      config->Multicast->Port = src_config->multicast.port;
      config->Multicast->TTL = src_config->multicast.ttl;
      config->Multicast->AutoStart =
        src_config->multicast.auto_start ? xsd__boolean__true_ : xsd__boolean__false_;
    }
  }

  if (soap_put__trt__GetMetadataConfigurationsResponse(
        soap, response, "trt:GetMetadataConfigurationsResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

int media_set_metadata_config_response_callback(struct soap* soap, void* user_data) {
  media_set_metadata_config_callback_data_t* data =
    (media_set_metadata_config_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _trt__SetMetadataConfigurationResponse* response =
    soap_new__trt__SetMetadataConfigurationResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  if (soap_put__trt__SetMetadataConfigurationResponse(
        soap, response, "trt:SetMetadataConfigurationResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Media Service Response Generation - Public API Functions
 * ============================================================================
 */

int onvif_gsoap_generate_profiles_response(onvif_gsoap_context_t* ctx,
                                           const struct media_profile* profiles,
                                           int profile_count) {
  if (!ctx || !ctx->soap || !profiles || profile_count <= 0) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for profiles response");
    return ONVIF_ERROR_INVALID;
  }

  media_profiles_callback_data_t callback_data = {
    .profiles = profiles,
    .profile_count = profile_count
  };

  return onvif_gsoap_generate_response_with_callback(ctx, media_profiles_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_stream_uri_response(onvif_gsoap_context_t* ctx,
                                             const struct stream_uri* uri) {
  if (!ctx || !ctx->soap || !uri) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for stream URI response");
    return ONVIF_ERROR_INVALID;
  }

  media_stream_uri_callback_data_t callback_data = { .uri = uri };

  return onvif_gsoap_generate_response_with_callback(ctx, media_stream_uri_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_create_profile_response(onvif_gsoap_context_t* ctx,
                                                 const struct media_profile* profile) {
  if (!ctx || !ctx->soap || !profile) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for create profile response");
    return ONVIF_ERROR_INVALID;
  }

  media_create_profile_callback_data_t callback_data = { .profile = profile };

  return onvif_gsoap_generate_response_with_callback(ctx, media_create_profile_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_delete_profile_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL, "Invalid parameters for delete profile response");
    return ONVIF_ERROR_INVALID;
  }

  media_delete_profile_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx, media_delete_profile_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_set_video_source_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for set video source configuration response");
    return ONVIF_ERROR_INVALID;
  }

  media_set_video_source_config_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx,
                                                     media_set_video_source_config_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_set_video_encoder_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for set video encoder configuration response");
    return ONVIF_ERROR_INVALID;
  }

  media_set_video_encoder_config_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx,
                                                     media_set_video_encoder_config_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_start_multicast_streaming_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for start multicast streaming response");
    return ONVIF_ERROR_INVALID;
  }

  media_start_multicast_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx, media_start_multicast_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_stop_multicast_streaming_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for stop multicast streaming response");
    return ONVIF_ERROR_INVALID;
  }

  media_stop_multicast_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx, media_stop_multicast_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_get_metadata_configurations_response(
  onvif_gsoap_context_t* ctx, const struct metadata_configuration* configs, int count) {
  if (!ctx || !ctx->soap || !configs || count < 0) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for get metadata configurations response");
    return ONVIF_ERROR_INVALID;
  }

  media_get_metadata_configs_callback_data_t callback_data = {
    .configs = configs,
    .config_count = count
  };

  return onvif_gsoap_generate_response_with_callback(ctx,
                                                     media_get_metadata_configs_response_callback,
                                                     &callback_data);
}

int onvif_gsoap_generate_set_metadata_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    onvif_gsoap_set_error(ctx ? ctx->soap : NULL,
                          "Invalid parameters for set metadata configuration response");
    return ONVIF_ERROR_INVALID;
  }

  media_set_metadata_config_callback_data_t callback_data = { .message = "" };

  return onvif_gsoap_generate_response_with_callback(ctx,
                                                     media_set_metadata_config_response_callback,
                                                     &callback_data);
}
