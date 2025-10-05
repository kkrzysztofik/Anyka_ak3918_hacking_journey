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
int onvif_gsoap_parse_system_reboot(onvif_gsoap_context_t* ctx, struct _tds__SystemReboot** out);

/* ============================================================================
 * Device Service Response Callback Data Structures
 * ============================================================================
 */

#define DEVICE_MANUFACTURER_MAX_LEN 128
#define DEVICE_MODEL_MAX_LEN        128
#define FIRMWARE_VERSION_MAX_LEN    64
#define SERIAL_NUMBER_MAX_LEN       64
#define HARDWARE_ID_MAX_LEN         64

/**
 * @brief Callback data structure for device info response
 */
typedef struct {
  char manufacturer[DEVICE_MANUFACTURER_MAX_LEN];
  char model[DEVICE_MODEL_MAX_LEN];
  char firmware_version[FIRMWARE_VERSION_MAX_LEN];
  char serial_number[SERIAL_NUMBER_MAX_LEN];
  char hardware_id[HARDWARE_ID_MAX_LEN];
} device_info_callback_data_t;

/**
 * @brief Callback data structure for capabilities response
 */
typedef struct {
  const void* capabilities;
  int http_port;
  char device_ip[64];
} capabilities_callback_data_t;

/**
 * @brief Callback data structure for system datetime response
 */
typedef struct {
  struct tm tm_info;
} system_datetime_callback_data_t;

/**
 * @brief Callback data structure for services response
 */
typedef struct {
  int include_capability;
  int http_port;
  char device_ip[64];
} services_callback_data_t;

/**
 * @brief Callback data structure for system reboot response
 */
typedef struct {
  const char* message;
} system_reboot_callback_data_t;

/* ============================================================================
 * Device Service Response Callback Functions
 * ============================================================================
 */

/**
 * @brief Device info response callback function
 */
int device_info_response_callback(struct soap* soap, void* user_data);

/**
 * @brief System datetime response callback function
 */
int system_datetime_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Capabilities response callback function
 */
int capabilities_response_callback(struct soap* soap, void* user_data);

/**
 * @brief System datetime response callback function
 */
int system_datetime_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Services response callback function
 */
int services_response_callback(struct soap* soap, void* user_data);

/**
 * @brief System reboot response callback function
 */
int system_reboot_response_callback(struct soap* soap, void* user_data);

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
int onvif_gsoap_generate_device_info_response(onvif_gsoap_context_t* ctx, const char* manufacturer,
                                              const char* model, const char* firmware_version,
                                              const char* serial_number, const char* hardware_id);

/**
 * @brief Generate GetCapabilities response
 * @param ctx gSOAP context for response generation
 * @param capabilities Capabilities structure to include in response (NULL for default)
 * @param device_ip Device IP address for XAddr URLs (used if capabilities is NULL)
 * @param http_port HTTP port for XAddr URLs (used if capabilities is NULL)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetCapabilities response containing service capabilities
 * @note If capabilities is NULL, creates default capabilities with Device, Media, and PTZ services
 */
int onvif_gsoap_generate_capabilities_response(onvif_gsoap_context_t* ctx,
                                                const struct tt__Capabilities* capabilities,
                                                const char* device_ip,
                                                int http_port);

/**
 * @brief Generate GetSystemDateAndTime response
 * @param ctx gSOAP context for response generation
 * @param utc_time Pointer to struct tm containing UTC time (NULL for current time)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Generates Device service GetSystemDateAndTime response containing system date/time
 * @note If utc_time is NULL, uses current system time
 */
int onvif_gsoap_generate_system_date_time_response(onvif_gsoap_context_t* ctx,
                                                     const struct tm* utc_time);

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
                                            const char* device_ip, int http_port);

#endif /* ONVIF_GSOAP_DEVICE_H */
