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
 * 5. Deserialize SOAP request using soap_get__tds__[Operation]()
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

#include "generated/soapH.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "utils/common/time_utils.h"
#include "utils/error/error_handling.h"
#include "utils/error/error_translation.h"

/* ============================================================================
 * Helper Functions - Response Callbacks
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
    return ONVIF_ERROR_INVALID;
  }

  /* Create response structure */
  struct _tds__GetDeviceInformationResponse* response =
    soap_new__tds__GetDeviceInformationResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Fill response data */
  response->Manufacturer = soap_strdup(soap, data->manufacturer);
  response->Model = soap_strdup(soap, data->model);
  response->FirmwareVersion = soap_strdup(soap, data->firmware_version);
  response->SerialNumber = soap_strdup(soap, data->serial_number);
  response->HardwareId = soap_strdup(soap, data->hardware_id);

  /* Serialize response within SOAP body */
  if (soap_put__tds__GetDeviceInformationResponse(
        soap, response, "tds:GetDeviceInformationResponse", "") != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return 0;
}

/**
 * @brief Capabilities response callback function
 */
int capabilities_response_callback(struct soap* soap, void* user_data) {
  if (!soap || !user_data) {
    return ONVIF_ERROR_INVALID;
  }

  capabilities_callback_data_t* data = (capabilities_callback_data_t*)user_data;

  /* Create response structure */
  struct _tds__GetCapabilitiesResponse* response = soap_new__tds__GetCapabilitiesResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Use provided capabilities if available, otherwise create default */
  if (data->capabilities) {
    response->Capabilities = (struct tt__Capabilities*)data->capabilities;
  } else {
    /* Create default capabilities structure */
    response->Capabilities = soap_new_tt__Capabilities(soap, 1);
    if (!response->Capabilities) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }
    /* Properly initialize the structure with default values */
    soap_default_tt__Capabilities(soap, response->Capabilities);

    /* Create minimal valid AnalyticsCapabilities to prevent segfaults */
    response->Capabilities->Analytics = soap_new_tt__AnalyticsCapabilities(soap, 1);
    if (response->Capabilities->Analytics) {
      response->Capabilities->Analytics->XAddr =
        soap_strdup(soap, "http://localhost:8080/onvif/analytics_service");
      response->Capabilities->Analytics->RuleSupport = xsd__boolean__false_;
      response->Capabilities->Analytics->AnalyticsModuleSupport = xsd__boolean__false_;
    }
  }

  /* Only create default capabilities if we created a new structure */
  if (!data->capabilities) {

    /* Build service URLs using configured device IP and port */
    char device_xaddr[256];
    char media_xaddr[256];
    char ptz_xaddr[256];
    snprintf(device_xaddr, sizeof(device_xaddr), "http://%s:%d/onvif/device_service",
             data->device_ip, data->http_port);
    snprintf(media_xaddr, sizeof(media_xaddr), "http://%s:%d/onvif/media_service", data->device_ip,
             data->http_port);
    snprintf(ptz_xaddr, sizeof(ptz_xaddr), "http://%s:%d/onvif/ptz_service", data->device_ip,
             data->http_port);

    /* Create device capabilities */
    response->Capabilities->Device = soap_new_tt__DeviceCapabilities(soap, 1);
    if (!response->Capabilities->Device) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }

    /* Set device capabilities XAddr */
    response->Capabilities->Device->XAddr = soap_strdup(soap, device_xaddr);

    /* Create media capabilities */
    response->Capabilities->Media = soap_new_tt__MediaCapabilities(soap, 1);
    if (!response->Capabilities->Media) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }

    response->Capabilities->Media->XAddr = soap_strdup(soap, media_xaddr);

    /* Create PTZ capabilities */
    response->Capabilities->PTZ = soap_new_tt__PTZCapabilities(soap, 1);
    if (!response->Capabilities->PTZ) {
      return ONVIF_ERROR_MEMORY_ALLOCATION;
    }

    response->Capabilities->PTZ->XAddr = soap_strdup(soap, ptz_xaddr);
  }

  /* Serialize response within SOAP body */
  if (soap_put__tds__GetCapabilitiesResponse(soap, response, "tds:GetCapabilitiesResponse", "") !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return 0;
}

/**
 * @brief System datetime response callback function
 */
int system_datetime_response_callback(struct soap* soap, void* user_data) {
  if (!soap || !user_data) {
    return ONVIF_ERROR_INVALID;
  }

  system_datetime_callback_data_t* data = (system_datetime_callback_data_t*)user_data;

  /* Create response structure */
  struct _tds__GetSystemDateAndTimeResponse* response =
    soap_new__tds__GetSystemDateAndTimeResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Create system date time structure */
  response->SystemDateAndTime = soap_new_tt__SystemDateTime(soap, 1);
  if (!response->SystemDateAndTime) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Set date time type to NTP or Manual */
  response->SystemDateAndTime->DateTimeType = tt__SetDateTimeType__Manual;

  /* Create UTC date time */
  response->SystemDateAndTime->UTCDateTime = soap_new_tt__DateTime(soap, 1);
  if (!response->SystemDateAndTime->UTCDateTime) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Set time */
  response->SystemDateAndTime->UTCDateTime->Time = soap_new_tt__Time(soap, 1);
  if (!response->SystemDateAndTime->UTCDateTime->Time) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->SystemDateAndTime->UTCDateTime->Time->Hour = data->tm_info.tm_hour;
  response->SystemDateAndTime->UTCDateTime->Time->Minute = data->tm_info.tm_min;
  response->SystemDateAndTime->UTCDateTime->Time->Second = data->tm_info.tm_sec;

  /* Set date */
  response->SystemDateAndTime->UTCDateTime->Date = soap_new_tt__Date(soap, 1);
  if (!response->SystemDateAndTime->UTCDateTime->Date) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->SystemDateAndTime->UTCDateTime->Date->Year = data->tm_info.tm_year + 1900;
  response->SystemDateAndTime->UTCDateTime->Date->Month = data->tm_info.tm_mon + 1;
  response->SystemDateAndTime->UTCDateTime->Date->Day = data->tm_info.tm_mday;

  /* Set timezone information */
  response->SystemDateAndTime->TimeZone = soap_new_tt__TimeZone(soap, 1);
  if (!response->SystemDateAndTime->TimeZone) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->SystemDateAndTime->TimeZone->TZ = soap_strdup(soap, "UTC");

  /* Serialize response within SOAP body */
  if (soap_put__tds__GetSystemDateAndTimeResponse(
        soap, response, "tds:GetSystemDateAndTimeResponse", "") != SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return 0;
}

/**
 * @brief Services response callback function
 */
int services_response_callback(struct soap* soap, void* user_data) {
  if (!soap || !user_data) {
    return ONVIF_ERROR_INVALID;
  }

  services_callback_data_t* data = (services_callback_data_t*)user_data;

  /* Create response structure */
  struct _tds__GetServicesResponse* response = soap_new__tds__GetServicesResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Allocate array for services - providing Device service for now */
  response->__sizeService = 1;
  response->Service = (struct tds__Service*)soap_malloc(soap, sizeof(struct tds__Service));
  if (!response->Service) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Build device service URL using configured device IP and port */
  char device_service_xaddr[256];
  snprintf(device_service_xaddr, sizeof(device_service_xaddr), "http://%s:%d/onvif/device_service",
           data->device_ip, data->http_port);

  /* Set device service information */
  response->Service[0].Namespace = soap_strdup(soap, "http://www.onvif.org/ver10/device/wsdl");
  response->Service[0].XAddr = soap_strdup(soap, device_service_xaddr);
  response->Service[0].Version = soap_new_tt__OnvifVersion(soap, 1);
  if (!response->Service[0].Version) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  response->Service[0].Version->Major = 2;
  response->Service[0].Version->Minor = 5;

  /* Serialize response within SOAP body */
  if (soap_put__tds__GetServicesResponse(soap, response, "tds:GetServicesResponse", "") !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return 0;
}

/**
 * @brief System reboot response callback function
 */
int system_reboot_response_callback(struct soap* soap, void* user_data) {
  if (!soap || !user_data) {
    return ONVIF_ERROR_INVALID;
  }

  system_reboot_callback_data_t* data = (system_reboot_callback_data_t*)user_data;

  /* Create response structure */
  struct _tds__SystemRebootResponse* response = soap_new__tds__SystemRebootResponse(soap, 1);
  if (!response) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }

  /* Set reboot message */
  response->Message = soap_strdup(soap, data->message ? data->message : "");

  /* Serialize response within SOAP body */
  if (soap_put__tds__SystemRebootResponse(soap, response, "tds:SystemRebootResponse", "") !=
      SOAP_OK) {
    return ONVIF_ERROR_SERIALIZATION_FAILED;
  }

  return 0;
}

/* ============================================================================
 * Public API - Request Parsing Functions
 * ============================================================================
 */

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
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "GetDeviceInformation", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate GetDeviceInformation structure using gSOAP managed memory */
  *out = soap_new__tds__GetDeviceInformation(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetDeviceInformation request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual GetDeviceInformation structure */
  struct _tds__GetDeviceInformation* result_ptr =
    soap_get__tds__GetDeviceInformation(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetDeviceInformation structure");
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
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "GetCapabilities", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate GetCapabilities structure using gSOAP managed memory */
  *out = soap_new__tds__GetCapabilities(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetCapabilities request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual GetCapabilities structure */
  struct _tds__GetCapabilities* result_ptr =
    soap_get__tds__GetCapabilities(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetCapabilities structure");
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
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "GetSystemDateAndTime", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate GetSystemDateAndTime structure using gSOAP managed memory */
  *out = soap_new__tds__GetSystemDateAndTime(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate GetSystemDateAndTime request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual GetSystemDateAndTime structure */
  struct _tds__GetSystemDateAndTime* result_ptr =
    soap_get__tds__GetSystemDateAndTime(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse GetSystemDateAndTime structure");
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
 * @brief Parse SystemReboot ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SystemReboot structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Parses struct _tds__SystemReboot from SOAP envelope
 * @note This is an empty request structure (no parameters)
 * @note Output structure is allocated and managed by gSOAP context
 */
int onvif_gsoap_parse_system_reboot(onvif_gsoap_context_t* ctx, struct _tds__SystemReboot** out) {
  /* 1. Validate context and begin parse operation */
  int result = onvif_gsoap_validate_and_begin_parse(ctx, out, "SystemReboot", __func__);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* 2. Allocate SystemReboot structure using gSOAP managed memory */
  *out = soap_new__tds__SystemReboot(&ctx->soap, -1);
  if (!*out) {
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_MEMORY, __func__,
                          "Failed to allocate SystemReboot request structure");
    return ONVIF_ERROR_MEMORY;
  }

  /* 3. Parse SOAP envelope */
  result = onvif_gsoap_parse_soap_envelope(ctx, __func__);
  if (result != ONVIF_SUCCESS) {
    *out = NULL;
    return result;
  }

  /* 4. Parse the actual SystemReboot structure */
  struct _tds__SystemReboot* result_ptr = soap_get__tds__SystemReboot(&ctx->soap, *out, NULL, NULL);
  if (!result_ptr || ctx->soap.error != SOAP_OK) {
    *out = NULL;
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_PARSE_FAILED, __func__,
                          "Failed to parse SystemReboot structure");
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
 * Public API - Response Generation Functions
 * ============================================================================
 */

/**
 * @brief Generate GetDeviceInformation response
 * @param ctx gSOAP context for response generation
 * @param manufacturer Device manufacturer name
 * @param model Device model name
 * @param firmware_version Firmware version string
 * @param serial_number Device serial number
 * @param hardware_id Hardware identifier
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetDeviceInformation response containing device identity
 */
int onvif_gsoap_generate_device_info_response(onvif_gsoap_context_t* ctx, const char* manufacturer,
                                              const char* model, const char* firmware_version,
                                              const char* serial_number, const char* hardware_id) {
  /* Prepare callback data */
  device_info_callback_data_t callback_data = {.manufacturer = {0},
                                               .model = {0},
                                               .firmware_version = {0},
                                               .serial_number = {0},
                                               .hardware_id = {0}};

  /* Copy strings into the callback data structure */
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

  /* Use the generic response generation with callback */
  return onvif_gsoap_generate_response_with_callback(ctx, device_info_response_callback,
                                                     &callback_data);
}

/**
 * @brief Generate GetCapabilities response
 * @param ctx gSOAP context for response generation
 * @param device_ip Device IP address for XAddr URLs
 * @param http_port HTTP port for XAddr URLs
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetCapabilities response containing service capabilities
 */
int onvif_gsoap_generate_capabilities_response(onvif_gsoap_context_t* ctx,
                                               const struct tt__Capabilities* capabilities,
                                               const char* device_ip, int http_port) {
  /* Prepare callback data */
  capabilities_callback_data_t callback_data = {
    .capabilities = capabilities, .http_port = http_port, .device_ip = {0}};

  /* Copy device IP into the callback data structure */
  strncpy(callback_data.device_ip, device_ip ? device_ip : "", sizeof(callback_data.device_ip) - 1);
  callback_data.device_ip[sizeof(callback_data.device_ip) - 1] = '\0';

  /* Use the generic response generation with callback */
  return onvif_gsoap_generate_response_with_callback(ctx, capabilities_response_callback,
                                                     &callback_data);
}

/**
 * @brief Generate GetSystemDateAndTime response
 * @param ctx gSOAP context for response generation
 * @param utc_time Pointer to struct tm containing UTC time
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetSystemDateAndTime response containing system date/time
 * @note If utc_time is NULL, uses current system time
 */
int onvif_gsoap_generate_system_date_time_response(onvif_gsoap_context_t* ctx,
                                                   const struct tm* utc_time) {
  /* Prepare callback data */
  system_datetime_callback_data_t callback_data = {.tm_info = {0}};

  /* Use provided time or get current time */
  if (utc_time) {
    callback_data.tm_info = *utc_time;
  } else {
    time_t now = time(NULL);
    struct tm* tm_utc = gmtime(&now);
    if (tm_utc) {
      callback_data.tm_info = *tm_utc;
    } else {
      /* If gmtime fails, return error */
      return ONVIF_ERROR_INVALID;
    }
  }

  /* Use the generic response generation with callback */
  return onvif_gsoap_generate_response_with_callback(ctx, system_datetime_response_callback,
                                                     &callback_data);
}

/**
 * @brief Generate GetServices response
 * @param ctx gSOAP context for response generation
 * @param include_capability Include capability information (0 or 1)
 * @param device_ip Device IP address for XAddr URLs
 * @param http_port HTTP port for XAddr URLs
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetServices response containing available services
 */
int onvif_gsoap_generate_services_response(onvif_gsoap_context_t* ctx, int include_capability,
                                           const char* device_ip, int http_port) {
  /* Prepare callback data */
  services_callback_data_t callback_data = {
    .include_capability = include_capability, .http_port = http_port, .device_ip = {0}};

  /* Copy device IP into the callback data structure */
  strncpy(callback_data.device_ip, device_ip ? device_ip : "", sizeof(callback_data.device_ip) - 1);
  callback_data.device_ip[sizeof(callback_data.device_ip) - 1] = '\0';

  /* Use the generic response generation with callback */
  return onvif_gsoap_generate_response_with_callback(ctx, services_response_callback,
                                                     &callback_data);
}
