/**
 * @file onvif_gsoap_device.h
 * @brief Device service SOAP request parsing using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides Device service request parsing functions that use
 * gSOAP's generated deserialization functions for proper ONVIF compliance.
 * Device service operations include device information, capabilities,
 * system time, and reboot functionality.
 */

#ifndef ONVIF_GSOAP_DEVICE_H
#define ONVIF_GSOAP_DEVICE_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"

/**
 * @brief Parse GetDeviceInformation ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetDeviceInformation structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note This operation has an empty request structure (no parameters)
 * @note Response contains Manufacturer, Model, FirmwareVersion, SerialNumber, HardwareId
 * @note Output structure is allocated with soap_new__tds__GetDeviceInformation()
 */
int onvif_gsoap_parse_get_device_information(onvif_gsoap_context_t* ctx,
                                               struct _tds__GetDeviceInformation** out);

/**
 * @brief Parse GetCapabilities ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetCapabilities structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts optional Category array to filter which capability types to return
 * @note Categories include: All, Analytics, Device, Events, Imaging, Media, PTZ
 * @note If Category is NULL, all capabilities are returned
 * @note Output structure is allocated with soap_new__tds__GetCapabilities()
 */
int onvif_gsoap_parse_get_capabilities(onvif_gsoap_context_t* ctx,
                                         struct _tds__GetCapabilities** out);

/**
 * @brief Parse GetSystemDateAndTime ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetSystemDateAndTime structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note This operation has an empty request structure (no parameters)
 * @note Response contains DateTimeType, DaylightSavings, TimeZone, and UTC/Local DateTime
 * @note Output structure is allocated with soap_new__tds__GetSystemDateAndTime()
 */
int onvif_gsoap_parse_get_system_date_and_time(onvif_gsoap_context_t* ctx,
                                                 struct _tds__GetSystemDateAndTime** out);

/**
 * @brief Parse SystemReboot ONVIF Device service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SystemReboot structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note This operation has an empty request structure (no parameters)
 * @note Response contains reboot message indicating when system will restart
 * @note Output structure is allocated with soap_new__tds__SystemReboot()
 */
int onvif_gsoap_parse_system_reboot(onvif_gsoap_context_t* ctx,
                                      struct _tds__SystemReboot** out);

/* ============================================================================
 * Device Service Response Generation Functions
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
int onvif_gsoap_generate_device_info_response(onvif_gsoap_context_t* ctx,
                                              const char* manufacturer,
                                              const char* model,
                                              const char* firmware_version,
                                              const char* serial_number,
                                              const char* hardware_id);

#endif /* ONVIF_GSOAP_DEVICE_H */
