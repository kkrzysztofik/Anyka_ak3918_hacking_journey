/**
 * @file onvif_gsoap_imaging.h
 * @brief Imaging service SOAP request parsing using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides Imaging service request parsing functions that use
 * gSOAP's generated deserialization functions for proper ONVIF compliance.
 * Imaging service operations control camera image settings including
 * brightness, contrast, saturation, sharpness, and other visual parameters.
 */

#ifndef ONVIF_GSOAP_IMAGING_H
#define ONVIF_GSOAP_IMAGING_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "services/common/onvif_imaging_types.h"

/**
 * @brief Parse GetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts VideoSourceToken to identify which video source settings to retrieve
 * @note Response contains ImagingSettings with Brightness, Contrast, Saturation, Sharpness, etc.
 * @note Output structure is allocated with soap_new__timg__GetImagingSettings()
 */
int onvif_gsoap_parse_get_imaging_settings(onvif_gsoap_context_t* ctx, struct _timg__GetImagingSettings** out);

/**
 * @brief Parse SetImagingSettings ONVIF Imaging service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetImagingSettings structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts VideoSourceToken and ImagingSettings structure
 * @note ImagingSettings contains Brightness, Contrast, ColorSaturation, Sharpness values
 * @note May also include BacklightCompensation, Exposure, Focus, WideDynamicRange settings
 * @note ForcePersistence flag indicates whether settings should persist across reboots
 * @note Output structure is allocated with soap_new__timg__SetImagingSettings()
 */
int onvif_gsoap_parse_set_imaging_settings(onvif_gsoap_context_t* ctx, struct _timg__SetImagingSettings** out);

/* ============================================================================
 * Imaging Service Response Callback Data Structures
 * ============================================================================
 */

/**
 * @brief Callback data structure for imaging settings response
 */
typedef struct {
  const struct imaging_settings* settings;
} imaging_settings_callback_data_t;

/**
 * @brief Callback data structure for set imaging settings response
 */
typedef struct {
  const char* message;
} set_imaging_settings_callback_data_t;

/* ============================================================================
 * Imaging Service Response Callback Functions
 * ============================================================================
 */

/**
 * @brief Imaging settings response callback function
 */
int imaging_settings_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Set imaging settings response callback function
 */
int set_imaging_settings_response_callback(struct soap* soap, void* user_data);

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
void parse_daynight_mode(const char* mode_str, enum day_night_mode* mode);

/**
 * @brief Parse IR LED mode string to enum value
 * @param mode_str String representation ("Off", "On", "Auto")
 * @param mode Output pointer to receive parsed ir_led_mode enum
 * @note Converts ONVIF IR LED mode string to internal enum representation
 * @note If mode_str is NULL or unrecognized, mode is unchanged
 */
void parse_ir_led_mode(const char* mode_str, enum ir_led_mode* mode);

#endif /* ONVIF_GSOAP_IMAGING_H */
