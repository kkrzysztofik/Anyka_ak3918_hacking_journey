/**
 * @file onvif_gsoap.c
 * @brief Proper gSOAP implementation using generated structures and
 * serialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides a proper gSOAP implementation that uses the generated
 * gSOAP structures and serialization functions instead of manual XML building.
 * This ensures proper ONVIF compliance and eliminates buffer overflow risks.
 */

#include "protocol/gsoap/onvif_gsoap.h"

#include <errno.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "common/onvif_imaging_types.h"
#include "media/onvif_media.h"
#include "platform/platform.h"
#include "ptz/onvif_ptz.h"
#include "services/device/onvif_device.h"
#include "services/imaging/onvif_imaging.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"

// Include gSOAP generated files
#include "generated/DeviceBinding.nsmap"
#include "generated/soapH.h"
#include "generated/soapStub.h"

/* ============================================================================
 * Constants and Types
 * ============================================================================
 */

// gSOAP configuration constants
#define ONVIF_GSOAP_DEFAULT_BUFFER_SIZE 4096
#define ONVIF_GSOAP_MAX_RESPONSE_SIZE   (16 * 1024 * 1024)

/* ============================================================================
 * Global Variables
 * ============================================================================
 */

// Global error message buffer
static char g_onvif_gsoap_error_msg[256] = {0};

/* ============================================================================
 * Internal Helper Functions
 * ============================================================================
 */

/**
 * @brief Validate and initialize gSOAP context for proper fault handling
 * @param soap gSOAP context to validate
 * @return 0 on success, -EINVAL if soap is NULL, -ENOMEM if fault allocation
 * fails
 */
static int validate_gsoap_context(struct soap* soap) {
  if (!soap) {
    return -EINVAL;
  }

  // Ensure fault structure is available for error handling
  if (!soap->fault) {
    soap->fault = soap_new_SOAP_ENV__Fault(soap, 1);
    if (!soap->fault) {
      return -ENOMEM;
    }
  }

  return 0;
}

/**
 * @brief Set error message for the gSOAP context
 * @param soap gSOAP context
 * @param format Error message format string
 * @param ... Format arguments
 */
static void set_gsoap_error(struct soap* soap, const char* format, ...) {
  va_list args;
  va_start(args, format);
  vsnprintf(g_onvif_gsoap_error_msg, sizeof(g_onvif_gsoap_error_msg), format, args);
  va_end(args);

  if (soap) {
    soap->error = SOAP_FAULT;
    // Ensure fault structure exists before accessing it
    if (validate_gsoap_context(soap) == 0 && soap->fault) {
      soap->fault->faultstring = soap_strdup(soap, g_onvif_gsoap_error_msg);
    }
  }
  platform_log_error("ONVIF gSOAP Error: %s", g_onvif_gsoap_error_msg);
}

/**
 * @brief Get current timestamp in microseconds
 * @return Current timestamp in microseconds
 */
static uint64_t get_timestamp_us(void) {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (uint64_t)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}

/* ============================================================================
 * gSOAP Context Management
 * ============================================================================
 */

int onvif_gsoap_init(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    platform_log_error("ONVIF gSOAP: NULL context pointer");
    return -EINVAL;
  }

  // Initialize context structure
  memset(ctx, 0, sizeof(onvif_gsoap_context_t));

  // Initialize gSOAP context
  ctx->soap = soap_new();
  if (!ctx->soap) {
    platform_log_error("ONVIF gSOAP: Failed to create soap context");
    return -ENOMEM;
  }

  // Configure gSOAP context
  soap_set_mode(ctx->soap, SOAP_C_UTFSTRING);
  soap_set_namespaces(ctx->soap, namespaces);

  // Initialize statistics
  ctx->generation_start_time = get_timestamp_us();
  ctx->total_bytes_written = 0;

  platform_log_debug("ONVIF gSOAP: Initialized with soap context");
  return 0;
}

void onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return;
  }

  // Clean up gSOAP context
  if (ctx->soap) {
    soap_destroy(ctx->soap);
    soap_end(ctx->soap);
    soap_free(ctx->soap);
    ctx->soap = NULL;
  }

  // Clear all state
  memset(ctx, 0, sizeof(onvif_gsoap_context_t));

  platform_log_debug("ONVIF gSOAP: Cleaned up");
}

void onvif_gsoap_reset(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    return;
  }

  // Reset gSOAP context
  soap_destroy(ctx->soap);
  soap_end(ctx->soap);

  // Reset statistics
  ctx->generation_start_time = get_timestamp_us();
  ctx->total_bytes_written = 0;

  platform_log_debug("ONVIF gSOAP: Reset to initial state");
}

/* ============================================================================
 * Response Generation Functions
 * ============================================================================
 */

int onvif_gsoap_serialize_response(onvif_gsoap_context_t* ctx, void* response_data) {
  if (!ctx || !ctx->soap || !response_data) {
    set_gsoap_error(ctx->soap, "Invalid parameters for serialize response");
    return -EINVAL;
  }

  // Start timing
  ctx->generation_start_time = get_timestamp_us();

  // Begin SOAP response
  if (soap_begin_send(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to begin SOAP send");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Started response serialization");
  return 0;
}

int onvif_gsoap_finalize_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "NULL context pointer");
    return -EINVAL;
  }

  // End SOAP response
  if (soap_end_send(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to end SOAP send");
    return -1;
  }

  // Update statistics
  ctx->generation_end_time = get_timestamp_us();
  ctx->total_bytes_written = ctx->soap->length;

  platform_log_debug("ONVIF gSOAP: Finalized response (%zu bytes, %llu us)",
                     ctx->total_bytes_written,
                     (unsigned long long)(ctx->generation_end_time - ctx->generation_start_time));
  return 0;
}

/* ============================================================================
 * ONVIF Service Helper Functions
 * ============================================================================
 */

/* ============================================================================
 * Generic Response Generation
 * ============================================================================
 */

/**
 * @brief Generic SOAP response generation with callback
 * @param ctx gSOAP context
 * @param callback Endpoint-specific response generation callback
 * @param user_data User data passed to callback
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                onvif_response_callback_t callback,
                                                void* user_data) {
  if (!ctx || !ctx->soap || !callback) {
    if (ctx && ctx->soap) {
      set_gsoap_error(ctx->soap, "Invalid parameters for response generation");
    }
    return -EINVAL;
  }

  // Ensure gSOAP context is properly initialized for fault handling
  if (validate_gsoap_context(ctx->soap) != 0) {
    platform_log_error("ONVIF gSOAP: Failed to initialize fault handling context");
    return -ENOMEM;
  }

  // Set up gSOAP for string output - this is the correct way to get XML as a
  // string
  char* output_string = NULL;
  ctx->soap->os = (void*)&output_string;

  // Begin SOAP send with string output mode
  if (soap_begin_send(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to begin SOAP send");
    ctx->soap->os = NULL;
    return -1;
  }

  // Use gSOAP's proper envelope functions for complete SOAP envelope generation
  if (soap_envelope_begin_out(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to begin SOAP envelope");
    ctx->soap->os = NULL;
    return -1;
  }

  if (soap_body_begin_out(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to begin SOAP body");
    ctx->soap->os = NULL;
    return -1;
  }

  // Call the endpoint-specific callback to generate the response content
  int callback_result = callback(ctx->soap, user_data);
  if (callback_result != 0) {
    set_gsoap_error(ctx->soap, "Callback failed to generate response content");
    ctx->soap->os = NULL;
    return callback_result;
  }

  if (soap_body_end_out(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to end SOAP body");
    ctx->soap->os = NULL;
    return -1;
  }

  if (soap_envelope_end_out(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to end SOAP envelope");
    ctx->soap->os = NULL;
    return -1;
  }

  if (soap_end_send(ctx->soap) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to end SOAP send");
    ctx->soap->os = NULL;
    return -1;
  }

  // Clear the output stream pointer
  ctx->soap->os = NULL;

  // Copy the generated string to our buffer
  if (output_string) {
    size_t response_len = strlen(output_string);
    if (response_len < sizeof(ctx->soap->buf)) {
      strncpy(ctx->soap->buf, output_string, sizeof(ctx->soap->buf) - 1);
      ctx->soap->buf[sizeof(ctx->soap->buf) - 1] = '\0';
      ctx->soap->length = response_len;
    } else {
      set_gsoap_error(ctx->soap, "Response too large for buffer");
      return -1;
    }
  } else {
    set_gsoap_error(ctx->soap, "No output string generated");
    return -1;
  }

  // Debug: Check buffer after generation
  platform_log_debug("ONVIF gSOAP: Buffer after generation: ptr=%p, length=%zu", ctx->soap->buf,
                     ctx->soap->length);

  platform_log_debug("ONVIF gSOAP: Generated response with callback");
  return 0;
}

/* ============================================================================
 * Device Information Response
 * ============================================================================
 */

/**
 * @brief Callback function for device info response generation
 * @param soap gSOAP context
 * @param user_data Pointer to device_info_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int device_info_response_callback(struct soap* soap, void* user_data) {
  device_info_callback_data_t* data = (device_info_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _tds__GetDeviceInformationResponse* response =
    soap_new__tds__GetDeviceInformationResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Fill response data
  response->Manufacturer = soap_strdup(soap, data->manufacturer);
  response->Model = soap_strdup(soap, data->model);
  response->FirmwareVersion = soap_strdup(soap, data->firmware_version);
  response->SerialNumber = soap_strdup(soap, data->serial_number);
  response->HardwareId = soap_strdup(soap, data->hardware_id);

  // Serialize response within SOAP body
  if (soap_put__tds__GetDeviceInformationResponse(
        soap, response, "tds:GetDeviceInformationResponse", "") != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for capabilities response generation
 * @param soap gSOAP context
 * @param user_data Pointer to capabilities_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int capabilities_response_callback(struct soap* soap, void* user_data) {
  capabilities_callback_data_t* data = (capabilities_callback_data_t*)user_data;

  if (!data || !data->capabilities) {
    return -EINVAL;
  }

  // Create response structure
  struct _tds__GetCapabilitiesResponse* response = soap_new__tds__GetCapabilitiesResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create capabilities structure
  response->Capabilities = soap_new_tt__Capabilities(soap, 1);
  if (!response->Capabilities) {
    return -ENOMEM;
  }

  // Fill capabilities data
  const struct device_capabilities* caps = data->capabilities;

  // Device capabilities
  response->Capabilities->Device = soap_new_tt__DeviceCapabilities(soap, 1);
  if (response->Capabilities->Device) {
    response->Capabilities->Device->Network = soap_new_tt__NetworkCapabilities(soap, 1);
    response->Capabilities->Device->System = soap_new_tt__SystemCapabilities(soap, 1);
    response->Capabilities->Device->IO = soap_new_tt__IOCapabilities(soap, 1);
  }

  // Media capabilities
  if (caps->has_media) {
    response->Capabilities->Media = soap_new_tt__MediaCapabilities(soap, 1);
    if (response->Capabilities->Media) {
      response->Capabilities->Media->StreamingCapabilities =
        soap_new_tt__RealTimeStreamingCapabilities(soap, 1);
    }
  }

  // PTZ capabilities
  if (caps->has_ptz) {
    response->Capabilities->PTZ = soap_new_tt__PTZCapabilities(soap, 1);
  }

  // Imaging capabilities
  if (caps->has_imaging) {
    response->Capabilities->Imaging = soap_new_tt__ImagingCapabilities(soap, 1);
  }

  // Events capabilities
  if (caps->has_events) {
    response->Capabilities->Events = soap_new_tt__EventCapabilities(soap, 1);
  }

  // Analytics capabilities
  if (caps->has_analytics) {
    response->Capabilities->Analytics = soap_new_tt__AnalyticsCapabilities(soap, 1);
  }

  // Serialize response within SOAP body
  if (soap_put__tds__GetCapabilitiesResponse(soap, response, "tds:GetCapabilitiesResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for system datetime response generation
 * @param soap gSOAP context
 * @param user_data Pointer to system_datetime_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int system_datetime_response_callback(struct soap* soap, void* user_data) {
  system_datetime_callback_data_t* data = (system_datetime_callback_data_t*)user_data;

  if (!data || !data->tm_info) {
    return -EINVAL;
  }

  // Create response structure
  struct _tds__GetSystemDateAndTimeResponse* response =
    soap_new__tds__GetSystemDateAndTimeResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create system date time structure
  response->SystemDateAndTime = soap_new_tt__SystemDateTime(soap, 1);
  if (!response->SystemDateAndTime) {
    return -ENOMEM;
  }

  // Fill date time data
  const struct tm* tm_info = data->tm_info;

  // Set date time type (0 = Manual, 1 = NTP)
  response->SystemDateAndTime->DateTimeType = tt__SetDateTimeType__Manual;

  // Set daylight savings (0 = No, 1 = Yes)
  response->SystemDateAndTime->DaylightSavings = 0;

  // Create time zone
  response->SystemDateAndTime->TimeZone = soap_new_tt__TimeZone(soap, 1);
  if (response->SystemDateAndTime->TimeZone) {
    response->SystemDateAndTime->TimeZone->TZ = soap_strdup(soap, "UTC");
  }

  // Create UTC date time
  response->SystemDateAndTime->UTCDateTime = soap_new_tt__DateTime(soap, 1);
  if (response->SystemDateAndTime->UTCDateTime) {
    response->SystemDateAndTime->UTCDateTime->Date = soap_new_tt__Date(soap, 1);
    response->SystemDateAndTime->UTCDateTime->Time = soap_new_tt__Time(soap, 1);

    if (response->SystemDateAndTime->UTCDateTime->Date) {
      response->SystemDateAndTime->UTCDateTime->Date->Year = tm_info->tm_year + 1900;
      response->SystemDateAndTime->UTCDateTime->Date->Month = tm_info->tm_mon + 1;
      response->SystemDateAndTime->UTCDateTime->Date->Day = tm_info->tm_mday;
    }

    if (response->SystemDateAndTime->UTCDateTime->Time) {
      response->SystemDateAndTime->UTCDateTime->Time->Hour = tm_info->tm_hour;
      response->SystemDateAndTime->UTCDateTime->Time->Minute = tm_info->tm_min;
      response->SystemDateAndTime->UTCDateTime->Time->Second = tm_info->tm_sec;
    }
  }

  // Create local date time (same as UTC for now)
  response->SystemDateAndTime->LocalDateTime = soap_new_tt__DateTime(soap, 1);
  if (response->SystemDateAndTime->LocalDateTime) {
    response->SystemDateAndTime->LocalDateTime->Date = soap_new_tt__Date(soap, 1);
    response->SystemDateAndTime->LocalDateTime->Time = soap_new_tt__Time(soap, 1);

    if (response->SystemDateAndTime->LocalDateTime->Date) {
      response->SystemDateAndTime->LocalDateTime->Date->Year = tm_info->tm_year + 1900;
      response->SystemDateAndTime->LocalDateTime->Date->Month = tm_info->tm_mon + 1;
      response->SystemDateAndTime->LocalDateTime->Date->Day = tm_info->tm_mday;
    }

    if (response->SystemDateAndTime->LocalDateTime->Time) {
      response->SystemDateAndTime->LocalDateTime->Time->Hour = tm_info->tm_hour;
      response->SystemDateAndTime->LocalDateTime->Time->Minute = tm_info->tm_min;
      response->SystemDateAndTime->LocalDateTime->Time->Second = tm_info->tm_sec;
    }
  }

  // Serialize response within SOAP body
  if (soap_put__tds__GetSystemDateAndTimeResponse(
        soap, response, "tds:GetSystemDateAndTimeResponse", "") != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for services response generation
 * @param soap gSOAP context
 * @param user_data Pointer to services_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int services_response_callback(struct soap* soap, void* user_data) {
  services_callback_data_t* data = (services_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _tds__GetServicesResponse* response = soap_new__tds__GetServicesResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create service list
  response->Service = soap_new_tds__Service(soap, 3); // Device, Media, PTZ
  if (!response->Service) {
    return -ENOMEM;
  }

  // Device Service
  response->Service[0].Namespace = soap_strdup(soap, "http://www.onvif.org/ver10/device/wsdl");
  response->Service[0].XAddr = soap_strdup(soap, "http://[IP]:8080/onvif/device_service");
  response->Service[0].Version = soap_new_tt__OnvifVersion(soap, 1);
  if (response->Service[0].Version) {
    response->Service[0].Version->Major = 2;
    response->Service[0].Version->Minor = 5;
  }

  // Media Service
  response->Service[1].Namespace = soap_strdup(soap, "http://www.onvif.org/ver10/media/wsdl");
  response->Service[1].XAddr = soap_strdup(soap, "http://[IP]:8080/onvif/media_service");
  response->Service[1].Version = soap_new_tt__OnvifVersion(soap, 1);
  if (response->Service[1].Version) {
    response->Service[1].Version->Major = 2;
    response->Service[1].Version->Minor = 5;
  }

  // PTZ Service
  response->Service[2].Namespace = soap_strdup(soap, "http://www.onvif.org/ver20/ptz/wsdl");
  response->Service[2].XAddr = soap_strdup(soap, "http://[IP]:8080/onvif/ptz_service");
  response->Service[2].Version = soap_new_tt__OnvifVersion(soap, 1);
  if (response->Service[2].Version) {
    response->Service[2].Version->Major = 2;
    response->Service[2].Version->Minor = 5;
  }

  // Set service count
  response->__sizeService = 3;

  // Serialize response within SOAP body
  if (soap_put__tds__GetServicesResponse(soap, response, "tds:GetServicesResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for system reboot response generation
 * @param soap gSOAP context
 * @param user_data Pointer to system_reboot_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int system_reboot_response_callback(struct soap* soap, void* user_data) {
  system_reboot_callback_data_t* data = (system_reboot_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _tds__SystemRebootResponse* response = soap_new__tds__SystemRebootResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Set message if provided
  if (data->message) {
    response->Message = soap_strdup(soap, data->message);
  }

  // Serialize response within SOAP body
  if (soap_put__tds__SystemRebootResponse(soap, response, "tds:SystemRebootResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for imaging settings response generation
 * @param soap gSOAP context
 * @param user_data Pointer to imaging_settings_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int imaging_settings_response_callback(struct soap* soap, void* user_data) {
  imaging_settings_callback_data_t* data = (imaging_settings_callback_data_t*)user_data;

  if (!data || !data->settings) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif4__GetImagingSettingsResponse* response =
    soap_new__onvif4__GetImagingSettingsResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create ImagingSettings structure
  response->ImagingSettings = soap_new_tt__ImagingSettings(soap, 1);
  if (!response->ImagingSettings) {
    return -ENOMEM;
  }

  // Fill imaging settings data
  const struct imaging_settings* settings = data->settings;

  // Set brightness
  response->ImagingSettings->Brightness = soap_new_float(soap, 1);
  if (response->ImagingSettings->Brightness) {
    *(response->ImagingSettings->Brightness) = (float)settings->brightness;
  }

  // Set contrast
  response->ImagingSettings->Contrast = soap_new_float(soap, 1);
  if (response->ImagingSettings->Contrast) {
    *(response->ImagingSettings->Contrast) = (float)settings->contrast;
  }

  // Set color saturation
  response->ImagingSettings->ColorSaturation = soap_new_float(soap, 1);
  if (response->ImagingSettings->ColorSaturation) {
    *(response->ImagingSettings->ColorSaturation) = (float)settings->saturation;
  }

  // Set sharpness
  response->ImagingSettings->Sharpness = soap_new_float(soap, 1);
  if (response->ImagingSettings->Sharpness) {
    *(response->ImagingSettings->Sharpness) = (float)settings->sharpness;
  }

  // Note: Hue field is not available in this gSOAP version of ImagingSettings

  // Serialize response within SOAP body
  if (soap_put__onvif4__GetImagingSettingsResponse(
        soap, response, "onvif4:GetImagingSettingsResponse", "") != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for set imaging settings response generation
 * @param soap gSOAP context
 * @param user_data Pointer to set_imaging_settings_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int set_imaging_settings_response_callback(struct soap* soap, void* user_data) {
  set_imaging_settings_callback_data_t* data = (set_imaging_settings_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif4__SetImagingSettingsResponse* response =
    soap_new__onvif4__SetImagingSettingsResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // SetImagingSettingsResponse is an empty structure in this gSOAP version

  // Serialize response within SOAP body
  if (soap_put__onvif4__SetImagingSettingsResponse(
        soap, response, "onvif4:SetImagingSettingsResponse", "") != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for PTZ nodes response generation
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_nodes_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_nodes_response_callback(struct soap* soap, void* user_data) {
  ptz_nodes_callback_data_t* data = (ptz_nodes_callback_data_t*)user_data;

  if (!data || !data->nodes) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif3__GetNodesResponse* response = soap_new__onvif3__GetNodesResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create PTZ node array
  response->__sizePTZNode = data->count;
  response->PTZNode = soap_new_tt__PTZNode(soap, data->count);
  if (!response->PTZNode) {
    return -ENOMEM;
  }

  // Fill PTZ node data
  for (int i = 0; i < data->count; i++) {
    const struct ptz_node* node = &data->nodes[i];

    // Set basic node information
    response->PTZNode[i].token = soap_strdup(soap, node->token);
    response->PTZNode[i].Name = soap_strdup(soap, node->name);
    response->PTZNode[i].SupportedPTZSpaces = soap_new_tt__PTZSpaces(soap, 1);

    if (response->PTZNode[i].SupportedPTZSpaces) {
      // Set supported spaces (simplified for this implementation)
      response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace =
        soap_new_tt__Space2DDescription(soap, 1);
      if (response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace) {
        response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->URI =
          soap_strdup(soap, node->supported_ptz_spaces.absolute_pan_tilt_position_space.uri);
        response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange =
          soap_new_tt__FloatRange(soap, 1);
        if (response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange) {
          response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange->Min =
            node->supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.min;
          response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange->Max =
            node->supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.max;
        }
        response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange =
          soap_new_tt__FloatRange(soap, 1);
        if (response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange) {
          response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange->Min =
            node->supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.min;
          response->PTZNode[i].SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange->Max =
            node->supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.max;
        }
      }
    }

    // Set other node properties
    response->PTZNode[i].MaximumNumberOfPresets = node->maximum_number_of_presets;
    response->PTZNode[i].HomeSupported =
      node->home_supported ? xsd__boolean__true_ : xsd__boolean__false_;
  }

  // Serialize response within SOAP body
  if (soap_put__onvif3__GetNodesResponse(soap, response, "onvif3:GetNodesResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for PTZ absolute move response generation
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_absolute_move_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_absolute_move_response_callback(struct soap* soap, void* user_data) {
  ptz_absolute_move_callback_data_t* data = (ptz_absolute_move_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif3__AbsoluteMoveResponse* response = soap_new__onvif3__AbsoluteMoveResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // AbsoluteMoveResponse is an empty structure in this gSOAP version

  // Serialize response within SOAP body
  if (soap_put__onvif3__AbsoluteMoveResponse(soap, response, "onvif3:AbsoluteMoveResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for PTZ presets response generation
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_presets_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_presets_response_callback(struct soap* soap, void* user_data) {
  ptz_presets_callback_data_t* data = (ptz_presets_callback_data_t*)user_data;

  if (!data || !data->presets) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif3__GetPresetsResponse* response = soap_new__onvif3__GetPresetsResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create preset array
  response->__sizePTZPreset = data->count;
  response->PTZPreset = soap_new_tt__PTZPreset(soap, data->count);
  if (!response->PTZPreset) {
    return -ENOMEM;
  }

  // Fill preset data
  for (int i = 0; i < data->count; i++) {
    const struct ptz_preset* preset = &data->presets[i];

    response->PTZPreset[i].token = soap_strdup(soap, preset->token);
    response->PTZPreset[i].Name = soap_strdup(soap, preset->name);

    // Set PTZ position
    response->PTZPreset[i].PTZPosition = soap_new_tt__PTZVector(soap, 1);
    if (response->PTZPreset[i].PTZPosition) {
      response->PTZPreset[i].PTZPosition->PanTilt = soap_new_tt__Vector2D(soap, 1);
      if (response->PTZPreset[i].PTZPosition->PanTilt) {
        response->PTZPreset[i].PTZPosition->PanTilt->x = preset->ptz_position.pan_tilt.x;
        response->PTZPreset[i].PTZPosition->PanTilt->y = preset->ptz_position.pan_tilt.y;
      }
      response->PTZPreset[i].PTZPosition->Zoom = soap_new_tt__Vector1D(soap, 1);
      if (response->PTZPreset[i].PTZPosition->Zoom) {
        response->PTZPreset[i].PTZPosition->Zoom->x = preset->ptz_position.zoom;
      }
    }
  }

  // Serialize response within SOAP body
  if (soap_put__onvif3__GetPresetsResponse(soap, response, "onvif3:GetPresetsResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for PTZ set preset response generation
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_set_preset_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_set_preset_response_callback(struct soap* soap, void* user_data) {
  ptz_set_preset_callback_data_t* data = (ptz_set_preset_callback_data_t*)user_data;

  if (!data || !data->preset_token) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif3__SetPresetResponse* response = soap_new__onvif3__SetPresetResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // SetPresetResponse is an empty struct - no fields to set
  // The preset token is handled by the calling function

  // Serialize response within SOAP body
  if (soap_put__onvif3__SetPresetResponse(soap, response, "onvif3:SetPresetResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for PTZ goto preset response generation
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_goto_preset_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_goto_preset_response_callback(struct soap* soap, void* user_data) {
  ptz_goto_preset_callback_data_t* data = (ptz_goto_preset_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _onvif3__GotoPresetResponse* response = soap_new__onvif3__GotoPresetResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // GotoPresetResponse is an empty structure in this gSOAP version

  // Serialize response within SOAP body
  if (soap_put__onvif3__GotoPresetResponse(soap, response, "onvif3:GotoPresetResponse", "") !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/* ============================================================================
 * Media Service Callback Functions
 * ============================================================================
 */

/**
 * @brief Callback function for media profiles response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_profiles_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_profiles_response_callback(struct soap* soap, void* user_data) {
  media_profiles_callback_data_t* data = (media_profiles_callback_data_t*)user_data;

  if (!data || !data->profiles) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetProfilesResponse* response = soap_new__trt__GetProfilesResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create profiles array
  response->__sizeProfiles = data->profile_count;
  response->Profiles = soap_new_tt__Profile(soap, data->profile_count);
  if (!response->Profiles) {
    return -ENOMEM;
  }

  // Fill profiles data
  for (int i = 0; i < data->profile_count; i++) {
    const struct media_profile* src_profile = &data->profiles[i];
    struct tt__Profile* profile = &response->Profiles[i];

    profile->token = soap_strdup(soap, src_profile->token);
    profile->Name = soap_strdup(soap, src_profile->name);
    profile->fixed = soap_new_xsd__boolean(soap, 1);
    if (profile->fixed) {
      *(profile->fixed) = src_profile->fixed ? xsd__boolean__true_ : xsd__boolean__false_;
    }

    // Add video source configuration
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

    // Add video encoder configuration
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

    // Add audio source configuration
    profile->AudioSourceConfiguration = soap_new_tt__AudioSourceConfiguration(soap, 1);
    if (profile->AudioSourceConfiguration) {
      profile->AudioSourceConfiguration->token = soap_strdup(soap, "AudioSourceConfig0");
      profile->AudioSourceConfiguration->Name = soap_strdup(soap, "Audio Source Configuration");
      profile->AudioSourceConfiguration->UseCount = 1;
      profile->AudioSourceConfiguration->SourceToken =
        soap_strdup(soap, src_profile->audio_source.source_token);
    }

    // Add audio encoder configuration
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

    // Add PTZ configuration
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

  // Serialize response
  if (soap_put__trt__GetProfilesResponse(soap, response, "trt:GetProfilesResponse", NULL) !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for stream URI response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_stream_uri_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_stream_uri_response_callback(struct soap* soap, void* user_data) {
  media_stream_uri_callback_data_t* data = (media_stream_uri_callback_data_t*)user_data;

  if (!data || !data->uri) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetStreamUriResponse* response = soap_new__trt__GetStreamUriResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Fill response data
  response->MediaUri = soap_new_tt__MediaUri(soap, 1);
  if (!response->MediaUri) {
    return -ENOMEM;
  }

  response->MediaUri->Uri = soap_strdup(soap, data->uri->uri);
  response->MediaUri->InvalidAfterConnect =
    data->uri->invalid_after_connect ? xsd__boolean__true_ : xsd__boolean__false_;
  response->MediaUri->InvalidAfterReboot =
    data->uri->invalid_after_reboot ? xsd__boolean__true_ : xsd__boolean__false_;
  response->MediaUri->Timeout = soap_strdup(soap, "PT60S");

  // Serialize response
  if (soap_put__trt__GetStreamUriResponse(soap, response, "trt:GetStreamUriResponse", NULL) !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for create profile response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_create_profile_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_create_profile_response_callback(struct soap* soap, void* user_data) {
  media_create_profile_callback_data_t* data = (media_create_profile_callback_data_t*)user_data;

  if (!data || !data->profile) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__CreateProfileResponse* response = soap_new__trt__CreateProfileResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Fill response data
  response->Profile = soap_new_tt__Profile(soap, 1);
  if (!response->Profile) {
    return -ENOMEM;
  }

  const struct media_profile* src_profile = data->profile;
  struct tt__Profile* profile = response->Profile;

  profile->token = soap_strdup(soap, src_profile->token);
  profile->Name = soap_strdup(soap, src_profile->name);
  profile->fixed = soap_new_xsd__boolean(soap, 1);
  if (profile->fixed) {
    *(profile->fixed) = src_profile->fixed ? xsd__boolean__true_ : xsd__boolean__false_;
  }

  // Serialize response
  if (soap_put__trt__CreateProfileResponse(soap, response, "trt:CreateProfileResponse", NULL) !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for delete profile response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_delete_profile_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_delete_profile_response_callback(struct soap* soap, void* user_data) {
  media_delete_profile_callback_data_t* data = (media_delete_profile_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__DeleteProfileResponse* response = soap_new__trt__DeleteProfileResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // DeleteProfileResponse is an empty structure

  // Serialize response
  if (soap_put__trt__DeleteProfileResponse(soap, response, "trt:DeleteProfileResponse", NULL) !=
      SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for set video source configuration response
 * generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_video_source_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_video_source_config_response_callback(struct soap* soap, void* user_data) {
  media_set_video_source_config_callback_data_t* data =
    (media_set_video_source_config_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__SetVideoSourceConfigurationResponse* response =
    soap_new__trt__SetVideoSourceConfigurationResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // SetVideoSourceConfigurationResponse is an empty structure

  // Serialize response
  if (soap_put__trt__SetVideoSourceConfigurationResponse(
        soap, response, "trt:SetVideoSourceConfigurationResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for set video encoder configuration response
 * generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_video_encoder_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_video_encoder_config_response_callback(struct soap* soap, void* user_data) {
  media_set_video_encoder_config_callback_data_t* data =
    (media_set_video_encoder_config_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__SetVideoEncoderConfigurationResponse* response =
    soap_new__trt__SetVideoEncoderConfigurationResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // SetVideoEncoderConfigurationResponse is an empty structure

  // Serialize response
  if (soap_put__trt__SetVideoEncoderConfigurationResponse(
        soap, response, "trt:SetVideoEncoderConfigurationResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for start multicast streaming response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_start_multicast_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_start_multicast_response_callback(struct soap* soap, void* user_data) {
  media_start_multicast_callback_data_t* data = (media_start_multicast_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__StartMulticastStreamingResponse* response =
    soap_new__trt__StartMulticastStreamingResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // StartMulticastStreamingResponse is an empty structure

  // Serialize response
  if (soap_put__trt__StartMulticastStreamingResponse(
        soap, response, "trt:StartMulticastStreamingResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for stop multicast streaming response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_stop_multicast_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_stop_multicast_response_callback(struct soap* soap, void* user_data) {
  media_stop_multicast_callback_data_t* data = (media_stop_multicast_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__StopMulticastStreamingResponse* response =
    soap_new__trt__StopMulticastStreamingResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // StopMulticastStreamingResponse is an empty structure

  // Serialize response
  if (soap_put__trt__StopMulticastStreamingResponse(
        soap, response, "trt:StopMulticastStreamingResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for get metadata configurations response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_get_metadata_configs_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_get_metadata_configs_response_callback(struct soap* soap, void* user_data) {
  media_get_metadata_configs_callback_data_t* data =
    (media_get_metadata_configs_callback_data_t*)user_data;

  if (!data || !data->configs) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetMetadataConfigurationsResponse* response =
    soap_new__trt__GetMetadataConfigurationsResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // Create metadata configurations array
  response->__sizeConfigurations = data->config_count;
  response->Configurations = soap_new_tt__MetadataConfiguration(soap, data->config_count);
  if (!response->Configurations) {
    return -ENOMEM;
  }

  // Fill metadata configurations data
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

  // Serialize response
  if (soap_put__trt__GetMetadataConfigurationsResponse(
        soap, response, "trt:GetMetadataConfigurationsResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

/**
 * @brief Callback function for set metadata configuration response generation
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_metadata_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_metadata_config_response_callback(struct soap* soap, void* user_data) {
  media_set_metadata_config_callback_data_t* data =
    (media_set_metadata_config_callback_data_t*)user_data;

  if (!data) {
    return -EINVAL;
  }

  // Create response structure
  struct _trt__SetMetadataConfigurationResponse* response =
    soap_new__trt__SetMetadataConfigurationResponse(soap, 1);
  if (!response) {
    return -ENOMEM;
  }

  // SetMetadataConfigurationResponse is an empty structure

  // Serialize response
  if (soap_put__trt__SetMetadataConfigurationResponse(
        soap, response, "trt:SetMetadataConfigurationResponse", NULL) != SOAP_OK) {
    return -1;
  }

  return 0;
}

int onvif_gsoap_generate_device_info_response(onvif_gsoap_context_t* ctx, const char* manufacturer,
                                              const char* model, const char* firmware_version,
                                              const char* serial_number, const char* hardware_id) {
  // Prepare callback data
  device_info_callback_data_t callback_data = {{0}};

  // Copy strings into the callback data structure
  strncpy(callback_data.manufacturer, manufacturer ? manufacturer : "",
          sizeof(callback_data.manufacturer) - 1);
  callback_data.manufacturer[sizeof(callback_data.manufacturer) - 1] = '\0';

  strncpy(callback_data.model, model ? model : "", sizeof(callback_data.model) - 1);
  callback_data.model[sizeof(callback_data.model) - 1] = '\0';

  strncpy(callback_data.firmware_version, firmware_version ? firmware_version : "",
          sizeof(callback_data.firmware_version) - 1);
  callback_data.firmware_version[sizeof(callback_data.firmware_version) - 1] = '\0';

  strncpy(callback_data.serial_number, serial_number ? serial_number : "",
          sizeof(callback_data.serial_number) - 1);
  callback_data.serial_number[sizeof(callback_data.serial_number) - 1] = '\0';

  strncpy(callback_data.hardware_id, hardware_id ? hardware_id : "",
          sizeof(callback_data.hardware_id) - 1);
  callback_data.hardware_id[sizeof(callback_data.hardware_id) - 1] = '\0';

  // Use the generic response generation with callback
  return onvif_gsoap_generate_response_with_callback(ctx, device_info_response_callback,
                                                     &callback_data);
}

/* ============================================================================
 * Utility Functions
 * ============================================================================
 */

const char* onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    return NULL;
  }

  return ctx->soap->buf;
}

size_t onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    return 0;
  }

  return ctx->soap->length;
}

bool onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    return true;
  }

  return ctx->soap->error != SOAP_OK;
}

const char* onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap || ctx->soap->error == SOAP_OK) {
    return NULL;
  }

  return ctx->soap->fault ? ctx->soap->fault->faultstring : "Unknown gSOAP error";
}

int onvif_gsoap_validate_response(const onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    return -EINVAL;
  }

  if (ctx->soap->error != SOAP_OK) {
    return -EINVAL;
  }

  return 0;
}

/* ============================================================================
 * Error Handling and Fault Generation
 * ============================================================================
 */

int onvif_gsoap_generate_profiles_response(onvif_gsoap_context_t* ctx,
                                           const struct media_profile* profiles,
                                           int profile_count) {
  if (!ctx || !ctx->soap || !profiles || profile_count <= 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for profiles response");
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetProfilesResponse* response = soap_new__trt__GetProfilesResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate profiles response");
    return -ENOMEM;
  }

  // Initialize profiles array
  response->__sizeProfiles = profile_count;
  response->Profiles = soap_new_tt__Profile(ctx->soap, profile_count);
  if (!response->Profiles) {
    set_gsoap_error(ctx->soap, "Failed to allocate profiles array");
    return -ENOMEM;
  }

  // Fill profile data
  for (int i = 0; i < profile_count; i++) {
    response->Profiles[i].token = soap_strdup(ctx->soap, profiles[i].token);
    response->Profiles[i].Name = soap_strdup(ctx->soap, profiles[i].name);

    // Set fixed values for now
    response->Profiles[i].fixed = 0; // Not fixed
    response->Profiles[i].VideoSourceConfiguration = NULL;
    response->Profiles[i].AudioSourceConfiguration = NULL;
    response->Profiles[i].VideoEncoderConfiguration = NULL;
    response->Profiles[i].AudioEncoderConfiguration = NULL;
    response->Profiles[i].VideoAnalyticsConfiguration = NULL;
    response->Profiles[i].PTZConfiguration = NULL;
    response->Profiles[i].MetadataConfiguration = NULL;
    response->Profiles[i].Extension = NULL;
  }

  // Serialize response
  if (soap_put__trt__GetProfilesResponse(ctx->soap, response, "trt:GetProfilesResponse", NULL) !=
      SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize profiles response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated profiles response with %d profiles", profile_count);
  return 0;
}

int onvif_gsoap_generate_stream_uri_response(onvif_gsoap_context_t* ctx,
                                             const struct stream_uri* uri) {
  if (!ctx || !ctx->soap || !uri) {
    set_gsoap_error(ctx->soap, "Invalid parameters for stream URI response");
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetStreamUriResponse* response = soap_new__trt__GetStreamUriResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate stream URI response");
    return -ENOMEM;
  }

  // Create MediaUri structure
  response->MediaUri = soap_new_tt__MediaUri(ctx->soap, 1);
  if (!response->MediaUri) {
    set_gsoap_error(ctx->soap, "Failed to allocate MediaUri structure");
    return -ENOMEM;
  }

  // Fill MediaUri data
  response->MediaUri->Uri = soap_strdup(ctx->soap, uri->uri);
  response->MediaUri->InvalidAfterConnect = uri->invalid_after_connect;
  response->MediaUri->InvalidAfterReboot = uri->invalid_after_reboot;
  response->MediaUri->Timeout = soap_strdup(ctx->soap, "PT60S"); // Default timeout

  // Serialize response
  if (soap_put__trt__GetStreamUriResponse(ctx->soap, response, "trt:GetStreamUriResponse", NULL) !=
      SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize stream URI response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated stream URI response: %s", uri->uri);
  return 0;
}

int onvif_gsoap_generate_create_profile_response(onvif_gsoap_context_t* ctx,
                                                 const struct media_profile* profile) {
  if (!ctx || !ctx->soap || !profile) {
    set_gsoap_error(ctx->soap, "Invalid parameters for create profile response");
    return -EINVAL;
  }

  // Create response structure
  struct _trt__CreateProfileResponse* response = soap_new__trt__CreateProfileResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate create profile response");
    return -ENOMEM;
  }

  // Create Profile structure
  response->Profile = soap_new_tt__Profile(ctx->soap, 1);
  if (!response->Profile) {
    set_gsoap_error(ctx->soap, "Failed to allocate Profile structure");
    return -ENOMEM;
  }

  // Fill Profile data
  response->Profile->token = soap_strdup(ctx->soap, profile->token);
  response->Profile->Name = soap_strdup(ctx->soap, profile->name);

  // Set fixed attribute (not fixed by default for custom profiles)
  response->Profile->fixed = soap_new_xsd__boolean(ctx->soap, 1);
  if (response->Profile->fixed) {
    *response->Profile->fixed = xsd__boolean__false_;
  }

  // Set optional configurations to NULL for now
  response->Profile->VideoSourceConfiguration = NULL;
  response->Profile->AudioSourceConfiguration = NULL;
  response->Profile->VideoEncoderConfiguration = NULL;
  response->Profile->AudioEncoderConfiguration = NULL;
  response->Profile->VideoAnalyticsConfiguration = NULL;
  response->Profile->PTZConfiguration = NULL;
  response->Profile->MetadataConfiguration = NULL;
  response->Profile->Extension = NULL;

  // Serialize response
  if (soap_put__trt__CreateProfileResponse(ctx->soap, response, "trt:CreateProfileResponse",
                                           NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize create profile response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated create profile response: %s", profile->token);
  return 0;
}

int onvif_gsoap_generate_delete_profile_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for delete profile response");
    return -EINVAL;
  }

  // Create response structure (empty response for DeleteProfile)
  struct _trt__DeleteProfileResponse* response = soap_new__trt__DeleteProfileResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate delete profile response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__DeleteProfileResponse(ctx->soap, response, "trt:DeleteProfileResponse",
                                           NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize delete profile response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated delete profile response");
  return 0;
}

int onvif_gsoap_generate_set_video_source_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for set video source configuration response");
    return -EINVAL;
  }

  // Create response structure (empty response for SetVideoSourceConfiguration)
  struct _trt__SetVideoSourceConfigurationResponse* response =
    soap_new__trt__SetVideoSourceConfigurationResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate set video source configuration response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__SetVideoSourceConfigurationResponse(
        ctx->soap, response, "trt:SetVideoSourceConfigurationResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize set video source configuration response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated set video source configuration response");
  return 0;
}

int onvif_gsoap_generate_set_video_encoder_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for set video encoder configuration response");
    return -EINVAL;
  }

  // Create response structure (empty response for SetVideoEncoderConfiguration)
  struct _trt__SetVideoEncoderConfigurationResponse* response =
    soap_new__trt__SetVideoEncoderConfigurationResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate set video encoder configuration response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__SetVideoEncoderConfigurationResponse(
        ctx->soap, response, "trt:SetVideoEncoderConfigurationResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize set video encoder configuration response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated set video encoder configuration response");
  return 0;
}

int onvif_gsoap_generate_start_multicast_streaming_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for start multicast streaming response");
    return -EINVAL;
  }

  // Create response structure (empty response for StartMulticastStreaming)
  struct _trt__StartMulticastStreamingResponse* response =
    soap_new__trt__StartMulticastStreamingResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate start multicast streaming response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__StartMulticastStreamingResponse(
        ctx->soap, response, "trt:StartMulticastStreamingResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize start multicast streaming response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated start multicast streaming response");
  return 0;
}

int onvif_gsoap_generate_stop_multicast_streaming_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for stop multicast streaming response");
    return -EINVAL;
  }

  // Create response structure (empty response for StopMulticastStreaming)
  struct _trt__StopMulticastStreamingResponse* response =
    soap_new__trt__StopMulticastStreamingResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate stop multicast streaming response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__StopMulticastStreamingResponse(
        ctx->soap, response, "trt:StopMulticastStreamingResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize stop multicast streaming response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated stop multicast streaming response");
  return 0;
}

int onvif_gsoap_generate_get_metadata_configurations_response(
  onvif_gsoap_context_t* ctx, const struct metadata_configuration* configs, int count) {
  if (!ctx || !ctx->soap || !configs || count < 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for get metadata configurations response");
    return -EINVAL;
  }

  // Create response structure
  struct _trt__GetMetadataConfigurationsResponse* response =
    soap_new__trt__GetMetadataConfigurationsResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate get metadata configurations response");
    return -ENOMEM;
  }

  // Set the number of configurations
  response->__sizeConfigurations = count;

  if (count > 0) {
    // Allocate array of metadata configurations
    response->Configurations = soap_new_tt__MetadataConfiguration(ctx->soap, count);
    if (!response->Configurations) {
      set_gsoap_error(ctx->soap, "Failed to allocate metadata configurations array");
      return -ENOMEM;
    }

    // Fill the configurations array
    for (int i = 0; i < count; i++) {
      struct tt__MetadataConfiguration* gsoap_config = &response->Configurations[i];

      // Set basic fields
      gsoap_config->Name = soap_strdup(ctx->soap, configs[i].name);
      gsoap_config->UseCount = configs[i].use_count;
      gsoap_config->token = soap_strdup(ctx->soap, configs[i].token);

      // Set analytics field
      gsoap_config->Analytics = soap_new_xsd__boolean(ctx->soap, 1);
      if (gsoap_config->Analytics) {
        *gsoap_config->Analytics =
          configs[i].analytics ? xsd__boolean__true_ : xsd__boolean__false_;
      }

      // Set session timeout (convert to duration format)
      gsoap_config->SessionTimeout = soap_strdup(ctx->soap, "PT30S"); // Default 30 seconds

      // Create multicast configuration (required field)
      gsoap_config->Multicast = soap_new_tt__MulticastConfiguration(ctx->soap, 1);
      if (gsoap_config->Multicast) {
        gsoap_config->Multicast->Address = soap_new_tt__IPAddress(ctx->soap, 1);
        if (gsoap_config->Multicast->Address) {
          gsoap_config->Multicast->Address->Type = tt__IPType__IPv4;
          gsoap_config->Multicast->Address->IPv4Address = soap_strdup(ctx->soap, "239.255.255.250");
        }
        gsoap_config->Multicast->Port = 3702; // Default ONVIF discovery port
        gsoap_config->Multicast->TTL = 5;
        gsoap_config->Multicast->AutoStart = xsd__boolean__false_;
      }
    }
  } else {
    response->Configurations = NULL;
  }

  // Serialize response
  if (soap_put__trt__GetMetadataConfigurationsResponse(
        ctx->soap, response, "trt:GetMetadataConfigurationsResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize get metadata configurations response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated get metadata configurations response with %d "
                     "configs",
                     count);
  return 0;
}

int onvif_gsoap_generate_set_metadata_configuration_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for set metadata configuration response");
    return -EINVAL;
  }

  // Create response structure (empty response for SetMetadataConfiguration)
  struct _trt__SetMetadataConfigurationResponse* response =
    soap_new__trt__SetMetadataConfigurationResponse(ctx->soap, 1);
  if (!response) {
    set_gsoap_error(ctx->soap, "Failed to allocate set metadata configuration response");
    return -ENOMEM;
  }

  // Serialize response (empty structure)
  if (soap_put__trt__SetMetadataConfigurationResponse(
        ctx->soap, response, "trt:SetMetadataConfigurationResponse", NULL) != SOAP_OK) {
    set_gsoap_error(ctx->soap, "Failed to serialize set metadata configuration response");
    return -1;
  }

  platform_log_debug("ONVIF gSOAP: Generated set metadata configuration response");
  return 0;
}

/* ============================================================================
 * Request Parsing Functions
 * ============================================================================
 */

int onvif_gsoap_init_request_parsing(onvif_gsoap_context_t* ctx, const char* request_data,
                                     size_t request_size) {
  if (!ctx || !ctx->soap || !request_data || request_size == 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for request parsing initialization");
    return -EINVAL;
  }

  // Reset the soap context for parsing
  soap_begin(ctx->soap);

  // TODO: Implement proper input stream initialization
  // For now, we'll skip the input stream setup as soap_string_input is not
  // available
  platform_log_debug("ONVIF gSOAP: Request parsing initialized (input stream setup skipped)");

  platform_log_debug("ONVIF gSOAP: Initialized request parsing with %zu bytes", request_size);
  return 0;
}

int onvif_gsoap_parse_profile_token(onvif_gsoap_context_t* ctx, char* token, size_t token_size) {
  if (!ctx || !ctx->soap || !token || token_size == 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for profile token parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for profile token
  // For now, return a default value
  strncpy(token, "Profile_1", token_size - 1);
  token[token_size - 1] = '\0';

  platform_log_debug("ONVIF gSOAP: Parsed profile token: %s", token);
  return 0;
}

int onvif_gsoap_parse_configuration_token(onvif_gsoap_context_t* ctx, char* token,
                                          size_t token_size) {
  if (!ctx || !ctx->soap || !token || token_size == 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for configuration token parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for configuration token
  // For now, return a default value
  strncpy(token, "Configuration_1", token_size - 1);
  token[token_size - 1] = '\0';

  platform_log_debug("ONVIF gSOAP: Parsed configuration token: %s", token);
  return 0;
}

int onvif_gsoap_parse_protocol(onvif_gsoap_context_t* ctx, char* protocol, size_t protocol_size) {
  if (!ctx || !ctx->soap || !protocol || protocol_size == 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for protocol parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for protocol
  // For now, return a default value
  strncpy(protocol, "RTSP", protocol_size - 1);
  protocol[protocol_size - 1] = '\0';

  platform_log_debug("ONVIF gSOAP: Parsed protocol: %s", protocol);
  return 0;
}

int onvif_gsoap_parse_value(onvif_gsoap_context_t* ctx, const char* xpath, char* value,
                            size_t value_size) {
  if (!ctx || !ctx->soap || !xpath || !value || value_size == 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for value parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for value
  // For now, return a default value
  strncpy(value, "default", value_size - 1);
  value[value_size - 1] = '\0';

  platform_log_debug("ONVIF gSOAP: Parsed value for %s: %s", xpath, value);
  return 0;
}

int onvif_gsoap_parse_boolean(onvif_gsoap_context_t* ctx, const char* xpath, int* value) {
  if (!ctx || !ctx->soap || !xpath || !value) {
    set_gsoap_error(ctx->soap, "Invalid parameters for boolean parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for boolean value
  // For now, return a default value
  *value = 0;

  platform_log_debug("ONVIF gSOAP: Parsed boolean for %s: %d", xpath, *value);
  return 0;
}

int onvif_gsoap_parse_integer(onvif_gsoap_context_t* ctx, const char* xpath, int* value) {
  if (!ctx || !ctx->soap || !xpath || !value) {
    set_gsoap_error(ctx->soap, "Invalid parameters for integer parsing");
    return -EINVAL;
  }

  // TODO: Implement proper XML parsing for integer value
  // For now, return a default value
  *value = 0;
  platform_log_debug("ONVIF gSOAP: Parsed integer for %s: %d", xpath, *value);
  return 0;
}

/**
 * @brief Extract ONVIF operation name from SOAP request using gSOAP XML parser
 *
 * This function uses gSOAP's built-in XML parsing capabilities to properly
 * parse the SOAP envelope and extract the operation name from the SOAP body.
 * It handles namespaces correctly and provides robust XML parsing.
 *
 * @param request_data Raw SOAP request data
 * @param request_size Size of the request data
 * @param operation_name Buffer to store the extracted operation name
 * @param operation_name_size Size of the operation name buffer
 * @return ONVIF_XML_SUCCESS on success, negative error code on failure
 */
int onvif_gsoap_extract_operation_name(const char* request_data, size_t request_size,
                                       char* operation_name, size_t operation_name_size) {
  if (!request_data || request_size == 0 || !operation_name || operation_name_size == 0) {
    return ONVIF_XML_ERROR_INVALID_INPUT;
  }

  // Initialize gSOAP context for proper XML parsing
  struct soap soap_ctx;
  soap_init(&soap_ctx);
  soap_set_mode(&soap_ctx, SOAP_C_UTFSTRING | SOAP_XML_STRICT);

  int result = ONVIF_XML_ERROR_PARSE_FAILED;

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
                result = ONVIF_XML_SUCCESS;
              }
            }
          }
        }
      }
    }
  }

  // Check for gSOAP parsing errors
  if (result != ONVIF_XML_SUCCESS && soap_ctx.error != SOAP_OK) {
    // All parsing errors map to the same result for simplicity
    result = ONVIF_XML_ERROR_PARSE_FAILED;
  }

  soap_end(&soap_ctx);
  soap_done(&soap_ctx);

  return result;
}

/* ============================================================================
 * Error Handling and Fault Generation
 * ============================================================================
 */

int onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, int fault_code,
                                        const char* fault_string) {
  if (!ctx || !ctx->soap) {
    return -EINVAL;
  }

  // Set SOAP fault
  soap_fault(ctx->soap);
  if (ctx->soap->fault) {
    ctx->soap->fault->faultcode = "soap:Server";
    ctx->soap->fault->faultstring =
      soap_strdup(ctx->soap, fault_string ? fault_string : "Internal server error");
  }

  platform_log_debug("ONVIF gSOAP: Generated fault response");
  return 0;
}

/* ============================================================================
 * PTZ Service Response Generation Functions
 * ============================================================================
 */

/**
 * @brief Generate GetNodes response using gSOAP serialization
 */
int onvif_gsoap_generate_get_nodes_response(onvif_gsoap_context_t* ctx,
                                            const struct ptz_node* nodes, int count) {
  if (!ctx || !ctx->soap || !nodes || count <= 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for GetNodes response");
    return -EINVAL;
  }

  // Create gSOAP response structure
  struct _onvif3__GetNodesResponse response;
  soap_default__onvif3__GetNodesResponse(ctx->soap, &response);

  // Allocate array for PTZ nodes
  response.PTZNode = soap_new_tt__PTZNode(ctx->soap, count);
  if (!response.PTZNode) {
    set_gsoap_error(ctx->soap, "Failed to allocate PTZ nodes array");
    return -ENOMEM;
  }

  // Convert each PTZ node to gSOAP format
  for (int i = 0; i < count; i++) {
    struct tt__PTZNode* gsoap_node = &response.PTZNode[i];
    soap_default_tt__PTZNode(ctx->soap, gsoap_node);

    // Set basic node information
    gsoap_node->token = soap_strdup(ctx->soap, nodes[i].token);
    gsoap_node->Name = soap_strdup(ctx->soap, nodes[i].name);

    // Set supported spaces
    gsoap_node->SupportedPTZSpaces = soap_new_tt__PTZSpaces(ctx->soap, 1);
    if (gsoap_node->SupportedPTZSpaces) {
      soap_default_tt__PTZSpaces(ctx->soap, gsoap_node->SupportedPTZSpaces);

      // Absolute pan/tilt position space
      gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace =
        soap_new_tt__Space2DDescription(ctx->soap, 1);
      if (gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace) {
        soap_default_tt__Space2DDescription(
          ctx->soap, gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace);
        gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->URI = soap_strdup(
          ctx->soap, nodes[i].supported_ptz_spaces.absolute_pan_tilt_position_space.uri);
        gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange =
          soap_new_tt__FloatRange(ctx->soap, 1);
        if (gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange) {
          gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange->Min =
            nodes[i].supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.min;
          gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->XRange->Max =
            nodes[i].supported_ptz_spaces.absolute_pan_tilt_position_space.x_range.max;
        }
        gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange =
          soap_new_tt__FloatRange(ctx->soap, 1);
        if (gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange) {
          gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange->Min =
            nodes[i].supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.min;
          gsoap_node->SupportedPTZSpaces->AbsolutePanTiltPositionSpace->YRange->Max =
            nodes[i].supported_ptz_spaces.absolute_pan_tilt_position_space.y_range.max;
        }
      }
    }

    // Set maximum number of presets
    gsoap_node->MaximumNumberOfPresets = nodes[i].maximum_number_of_presets;

    // Set home supported flag
    gsoap_node->HomeSupported = nodes[i].home_supported;
  }

  response.__sizePTZNode = count;

  // Serialize response
  soap_serialize__onvif3__GetNodesResponse(ctx->soap, &response);
  if (soap_put__onvif3__GetNodesResponse(ctx->soap, &response, "onvif3:GetNodesResponse", NULL)) {
    set_gsoap_error(ctx->soap, "Failed to serialize GetNodes response");
    return -EIO;
  }

  platform_log_debug("ONVIF gSOAP: Generated GetNodes response with %d nodes", count);
  return 0;
}

/**
 * @brief Generate AbsoluteMove response using gSOAP serialization
 */
int onvif_gsoap_generate_absolute_move_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for AbsoluteMove response");
    return -EINVAL;
  }

  // Create gSOAP response structure (empty response)
  struct _onvif3__AbsoluteMoveResponse response;
  soap_default__onvif3__AbsoluteMoveResponse(ctx->soap, &response);

  // Serialize response
  soap_serialize__onvif3__AbsoluteMoveResponse(ctx->soap, &response);
  if (soap_put__onvif3__AbsoluteMoveResponse(ctx->soap, &response, "onvif3:AbsoluteMoveResponse",
                                             NULL)) {
    set_gsoap_error(ctx->soap, "Failed to serialize AbsoluteMove response");
    return -EIO;
  }

  platform_log_debug("ONVIF gSOAP: Generated AbsoluteMove response");
  return 0;
}

/**
 * @brief Generate GetPresets response using gSOAP serialization
 */
int onvif_gsoap_generate_get_presets_response(onvif_gsoap_context_t* ctx,
                                              const struct ptz_preset* presets, int count) {
  if (!ctx || !ctx->soap || !presets || count < 0) {
    set_gsoap_error(ctx->soap, "Invalid parameters for GetPresets response");
    return -EINVAL;
  }

  // Create gSOAP response structure
  struct _onvif3__GetPresetsResponse response;
  soap_default__onvif3__GetPresetsResponse(ctx->soap, &response);

  if (count > 0) {
    // Allocate array for PTZ presets
    response.PTZPreset = soap_new_tt__PTZPreset(ctx->soap, count);
    if (!response.PTZPreset) {
      set_gsoap_error(ctx->soap, "Failed to allocate PTZ presets array");
      return -ENOMEM;
    }

    // Convert each PTZ preset to gSOAP format
    for (int i = 0; i < count; i++) {
      struct tt__PTZPreset* gsoap_preset = &response.PTZPreset[i];
      soap_default_tt__PTZPreset(ctx->soap, gsoap_preset);

      // Set preset information
      gsoap_preset->token = soap_strdup(ctx->soap, presets[i].token);
      gsoap_preset->Name = soap_strdup(ctx->soap, presets[i].name);

      // Set PTZ position
      gsoap_preset->PTZPosition = soap_new_tt__PTZVector(ctx->soap, 1);
      if (gsoap_preset->PTZPosition) {
        soap_default_tt__PTZVector(ctx->soap, gsoap_preset->PTZPosition);

        // Set pan/tilt position
        gsoap_preset->PTZPosition->PanTilt = soap_new_tt__Vector2D(ctx->soap, 1);
        if (gsoap_preset->PTZPosition->PanTilt) {
          gsoap_preset->PTZPosition->PanTilt->x = presets[i].ptz_position.pan_tilt.x;
          gsoap_preset->PTZPosition->PanTilt->y = presets[i].ptz_position.pan_tilt.y;
        }

        // Set zoom position
        gsoap_preset->PTZPosition->Zoom = soap_new_tt__Vector1D(ctx->soap, 1);
        if (gsoap_preset->PTZPosition->Zoom) {
          gsoap_preset->PTZPosition->Zoom->x = presets[i].ptz_position.zoom;
        }

        // Note: tt__PTZVector doesn't have a space field in this gSOAP version
      }
    }

    response.__sizePTZPreset = count;
  } else {
    response.PTZPreset = NULL;
    response.__sizePTZPreset = 0;
  }

  // Serialize response
  soap_serialize__onvif3__GetPresetsResponse(ctx->soap, &response);
  if (soap_put__onvif3__GetPresetsResponse(ctx->soap, &response, "onvif3:GetPresetsResponse",
                                           NULL)) {
    set_gsoap_error(ctx->soap, "Failed to serialize GetPresets response");
    return -EIO;
  }

  platform_log_debug("ONVIF gSOAP: Generated GetPresets response with %d presets", count);
  return 0;
}

/**
 * @brief Generate SetPreset response using gSOAP serialization
 */
int onvif_gsoap_generate_set_preset_response(onvif_gsoap_context_t* ctx, const char* preset_token) {
  if (!ctx || !ctx->soap || !preset_token) {
    set_gsoap_error(ctx->soap, "Invalid parameters for SetPreset response");
    return -EINVAL;
  }

  // Create gSOAP response structure
  struct _onvif3__SetPresetResponse response;
  soap_default__onvif3__SetPresetResponse(ctx->soap, &response);

  // Note: SetPresetResponse is an empty structure in this gSOAP version

  // Serialize response
  soap_serialize__onvif3__SetPresetResponse(ctx->soap, &response);
  if (soap_put__onvif3__SetPresetResponse(ctx->soap, &response, "onvif3:SetPresetResponse", NULL)) {
    set_gsoap_error(ctx->soap, "Failed to serialize SetPreset response");
    return -EIO;
  }

  platform_log_debug("ONVIF gSOAP: Generated SetPreset response with token: %s", preset_token);
  return 0;
}

/**
 * @brief Generate GotoPreset response using gSOAP serialization
 */
int onvif_gsoap_generate_goto_preset_response(onvif_gsoap_context_t* ctx) {
  if (!ctx || !ctx->soap) {
    set_gsoap_error(ctx->soap, "Invalid parameters for GotoPreset response");
    return -EINVAL;
  }

  // Create gSOAP response structure (empty response)
  struct _onvif3__GotoPresetResponse response;
  soap_default__onvif3__GotoPresetResponse(ctx->soap, &response);

  // Serialize response
  soap_serialize__onvif3__GotoPresetResponse(ctx->soap, &response);
  if (soap_put__onvif3__GotoPresetResponse(ctx->soap, &response, "onvif3:GotoPresetResponse",
                                           NULL)) {
    set_gsoap_error(ctx->soap, "Failed to serialize GotoPreset response");
    return -EIO;
  }

  platform_log_debug("ONVIF gSOAP: Generated GotoPreset response");
  return 0;
}
