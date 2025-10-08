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
 * 4. Allocate gSOAP structure using soap_new__timg__[Operation]()
 * 5. Deserialize SOAP request using soap_get__timg__[Operation]()
 * 6. Record completion time
 *
 * The parsed structures are managed by the gSOAP context and should not
 * be manually freed by the caller.
 */

#include "protocol/gsoap/onvif_gsoap_imaging.h"

#include <stdio.h>
#include <string.h>

#include "generated/soapH.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"
#include "utils/error/error_translation.h"

/**
 * @brief Parse GetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _timg__GetImagingSettings from SOAP envelope
 * @note Extracts VideoSourceToken to identify which video source settings to retrieve
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_get_imaging_settings(onvif_gsoap_context_t* ctx,
                                           struct _timg__GetImagingSettings** out) {
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "GetImagingSettings", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate GetImagingSettings structure using gSOAP managed memory */
  *out = soap_new__timg__GetImagingSettings(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetImagingSettings request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual GetImagingSettings structure */
  struct _timg__GetImagingSettings* result_ptr =
    soap_get__timg__GetImagingSettings(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetImagingSettings structure");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 5. Finalize SOAP parsing and complete timing */
  result = onvif_gsoap_finalize_parse(ctx);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse SetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _timg__SetImagingSettings from SOAP envelope
 * @note Extracts VideoSourceToken and ImagingSettings structure
 * @note ImagingSettings contains Brightness, Contrast, ColorSaturation, Sharpness
 * @note May include BacklightCompensation, Exposure, Focus, WideDynamicRange
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_set_imaging_settings(onvif_gsoap_context_t* ctx,
                                           struct _timg__SetImagingSettings** out) {
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "SetImagingSettings", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate SetImagingSettings structure using gSOAP managed memory */
  *out = soap_new__timg__SetImagingSettings(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SetImagingSettings request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual SetImagingSettings structure */
  struct _timg__SetImagingSettings* result_ptr =
    soap_get__timg__SetImagingSettings(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SetImagingSettings structure");
    return ONVIF_ERROR_PARSE_FAILED;
  }

  /* 5. Finalize SOAP parsing and complete timing */
  result = onvif_gsoap_finalize_parse(ctx);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

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
    platform_log_error("Imaging callback: Invalid data or settings\n");
    return ONVIF_ERROR_INVALID;
  }

  struct _timg__GetImagingSettingsResponse* response =
    soap_new__timg__GetImagingSettingsResponse(soap, 1);
  if (!response) {
    platform_log_error("Imaging callback: Failed to create response structure\n");
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Create ImagingSettings structure
  response->ImagingSettings = soap_new_tt__ImagingSettings20(soap, 1);
  if (!response->ImagingSettings) {
    platform_log_error("Imaging callback: Failed to create ImagingSettings structure\n");
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Populate all imaging settings fields from data->settings
  const struct imaging_settings* settings = data->settings;

  // Brightness
  response->ImagingSettings->Brightness = soap_new_float(soap, 1);
  if (response->ImagingSettings->Brightness) {
    *(response->ImagingSettings->Brightness) = (float)settings->brightness;
  }

  // Contrast
  response->ImagingSettings->Contrast = soap_new_float(soap, 1);
  if (response->ImagingSettings->Contrast) {
    *(response->ImagingSettings->Contrast) = (float)settings->contrast;
  }

  // ColorSaturation
  response->ImagingSettings->ColorSaturation = soap_new_float(soap, 1);
  if (response->ImagingSettings->ColorSaturation) {
    *(response->ImagingSettings->ColorSaturation) = (float)settings->saturation;
  }

  // Sharpness
  response->ImagingSettings->Sharpness = soap_new_float(soap, 1);
  if (response->ImagingSettings->Sharpness) {
    *(response->ImagingSettings->Sharpness) = (float)settings->sharpness;
  }

  platform_log_debug("Imaging callback: About to serialize response with brightness=%d\n",
                     settings->brightness);

  // Serialize complete response
  if (soap_put__timg__GetImagingSettingsResponse(soap, response, "timg:GetImagingSettingsResponse",
                                                 NULL) != SOAP_OK) {
    platform_log_error("Imaging callback: Failed to serialize response\n");
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  platform_log_debug("Imaging callback: Successfully serialized response\n");
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

  struct _timg__SetImagingSettingsResponse* response =
    soap_new__timg__SetImagingSettingsResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  // Note: SetImagingSettings response is typically empty
  (void)data; // Suppress unused parameter warning

  if (soap_put__timg__SetImagingSettingsResponse(soap, response, "timg:SetImagingSettingsResponse",
                                                 NULL) != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return ONVIF_SUCCESS;
}
