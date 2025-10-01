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
 * @brief Capabilities response callback function (stub)
 */
int capabilities_response_callback(struct soap* soap, void* user_data) {
  (void)soap;
  (void)user_data;
  /* TODO: Implement capabilities response generation */
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

/**
 * @brief System datetime response callback function (stub)
 */
int system_datetime_response_callback(struct soap* soap, void* user_data) {
  (void)soap;
  (void)user_data;
  /* TODO: Implement system datetime response generation */
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

/**
 * @brief Services response callback function (stub)
 */
int services_response_callback(struct soap* soap, void* user_data) {
  (void)soap;
  (void)user_data;
  /* TODO: Implement services response generation */
  return ONVIF_ERROR_NOT_IMPLEMENTED;
}

/**
 * @brief System reboot response callback function (stub)
 */
int system_reboot_response_callback(struct soap* soap, void* user_data) {
  (void)soap;
  (void)user_data;
  /* TODO: Implement system reboot response generation */
  return ONVIF_ERROR_NOT_IMPLEMENTED;
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
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__, "Request parsing not initialized");
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
    onvif_gsoap_set_error(ctx, ONVIF_ERROR_INVALID, __func__, "Request parsing not initialized");
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
int onvif_gsoap_parse_system_reboot(onvif_gsoap_context_t* ctx, struct _tds__SystemReboot** out) {
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
