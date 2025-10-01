/**
 * @file onvif_gsoap_imaging.c
 * @brief Imaging service SOAP request parsing implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements Imaging service request parsing functions using
 * gSOAP's generated deserialization for proper ONVIF compliance.
 *
 * All parsing functions follow a consistent pattern:
 * 1. Validate input parameters (NULL checks)
 * 2. Verify request parsing is initialized
 * 3. Set operation name and start timing
 * 4. Allocate gSOAP structure using soap_new__onvif4__[Operation]()
 * 5. Deserialize SOAP request using soap_read__onvif4__[Operation]()
 * 6. Record completion time
 *
 * The parsed structures are managed by the gSOAP context and should not
 * be manually freed by the caller.
 */

#include "protocol/gsoap/onvif_gsoap_imaging.h"

#include <stdio.h>
#include <string.h>

#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"

/**
 * @brief Parse GetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif4__GetImagingSettings from SOAP envelope
 * @note Extracts VideoSourceToken to identify which video source settings to retrieve
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_imaging_settings(onvif_gsoap_context_t* ctx,
                                           struct _onvif4__GetImagingSettings** out) {
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
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__, "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "GetImagingSettings";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif4__GetImagingSettings(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetImagingSettings request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif4__GetImagingSettings(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetImagingSettings SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _onvif4__SetImagingSettings from SOAP envelope
 * @note Extracts VideoSourceToken and ImagingSettings structure
 * @note ImagingSettings contains Brightness, Contrast, ColorSaturation, Sharpness
 * @note May include BacklightCompensation, Exposure, Focus, WideDynamicRange
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_set_imaging_settings(onvif_gsoap_context_t* ctx,
                                           struct _onvif4__SetImagingSettings** out) {
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
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__, "Request parsing not initialized");
    return ONVIF_ERROR_INVALID;
  }

  /* 3. Record operation name and start timing */
  ctx->request_state.operation_name = "SetImagingSettings";
  ctx->request_state.parse_start_time = get_timestamp_us();

  /* 4. Allocate structure using gSOAP managed memory */
  *out = soap_new__onvif4__SetImagingSettings(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SetImagingSettings request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 5. Use gSOAP generated deserialization function */
  if (soap_read__onvif4__SetImagingSettings(&ctx->soap, *out) != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SetImagingSettings SOAP request");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 6. Record parse completion time */
  ctx->request_state.parse_end_time = get_timestamp_us();

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Imaging Service Utility Functions
 * ============================================================================
 */

/**
 * @brief Parse day/night mode string to enum value
 * @param mode_str String representation ("Auto", "Day", "Night")
 * @param mode Output pointer to receive parsed day_night_mode enum
 * @note Converts ONVIF day/night mode string to internal enum representation
 * @note If mode_str is NULL or unrecognized, mode is unchanged
 */
void parse_daynight_mode(const char* mode_str, enum day_night_mode* mode) {
  if (!mode_str || !mode) {
    return;
  }

  if (strcmp(mode_str, "Auto") == 0) {
    *mode = DAY_NIGHT_AUTO;
  } else if (strcmp(mode_str, "Day") == 0) {
    *mode = DAY_NIGHT_DAY;
  } else if (strcmp(mode_str, "Night") == 0) {
    *mode = DAY_NIGHT_NIGHT;
  }
}

/**
 * @brief Parse IR LED mode string to enum value
 * @param mode_str String representation ("Off", "On", "Auto")
 * @param mode Output pointer to receive parsed ir_led_mode enum
 * @note Converts ONVIF IR LED mode string to internal enum representation
 * @note If mode_str is NULL or unrecognized, mode is unchanged
 */
void parse_ir_led_mode(const char* mode_str, enum ir_led_mode* mode) {
  if (!mode_str || !mode) {
    return;
  }

  if (strcmp(mode_str, "Off") == 0) {
    *mode = IR_LED_OFF;
  } else if (strcmp(mode_str, "On") == 0) {
    *mode = IR_LED_ON;
  } else if (strcmp(mode_str, "Auto") == 0) {
    *mode = IR_LED_AUTO;
  }
}

/* ============================================================================
 * Imaging Service Response Callback Functions
 * ============================================================================
 */

/**
 * @brief Generate imaging settings response
 * @param soap gSOAP context
 * @param user_data Callback data containing imaging settings
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Creates SOAP response with imaging settings
 */
int imaging_settings_response_callback(struct soap* soap, void* user_data) {
  imaging_settings_callback_data_t* data = (imaging_settings_callback_data_t*)user_data;

  if (!data || !data->settings) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif4__GetImagingSettingsResponse* response =
    soap_new__onvif4__GetImagingSettingsResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Note: Imaging settings response generation would go here
  // For now, create minimal response structure

  if (soap_put__onvif4__GetImagingSettingsResponse(
        soap, response, "onvif4:GetImagingSettingsResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Generate set imaging settings response
 * @param soap gSOAP context
 * @param user_data Callback data (unused for empty response)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Creates empty SOAP response for SetImagingSettings
 */
int set_imaging_settings_response_callback(struct soap* soap, void* user_data) {
  set_imaging_settings_callback_data_t* data = (set_imaging_settings_callback_data_t*)user_data;

  if (!data) {
    return ONVIF_ERROR_INVALID;
  }

  struct _onvif4__SetImagingSettingsResponse* response =
    soap_new__onvif4__SetImagingSettingsResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Note: SetImagingSettings response is typically empty
  (void)data; // Suppress unused parameter warning

  if (soap_put__onvif4__SetImagingSettingsResponse(
        soap, response, "onvif4:SetImagingSettingsResponse", NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}
