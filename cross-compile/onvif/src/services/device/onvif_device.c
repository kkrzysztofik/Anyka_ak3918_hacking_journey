/**
 * @file onvif_device.c
 * @brief ONVIF Device service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_device.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "platform/platform.h"
#include "protocol/response/onvif_service_handler.h"
#include "protocol/soap/onvif_soap.h"
#include "protocol/xml/unified_xml.h"
#include "services/common/onvif_request.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"

#define DEVICE_MANUFACTURER_DEFAULT "Anyka"
#define DEVICE_MODEL_DEFAULT "AK3918 Camera"
#define DEVICE_FIRMWARE_VER_DEFAULT "1.0.0"
#define DEVICE_SERIAL_DEFAULT "AK3918-001"
#define DEVICE_HARDWARE_ID_DEFAULT "1.0"

#define DEFAULT_HTTP_PORT 8080
#define DEFAULT_MTU 1500
#define MAX_NETWORK_INTERFACES 8
#define MAX_NETWORK_PROTOCOLS 8
#define MAX_DEVICE_SERVICES 8

#define DEVICE_MANUFACTURER_LEN 64
#define DEVICE_MODEL_LEN 64
#define DEVICE_FIRMWARE_VER_LEN 32
#define DEVICE_SERIAL_LEN 64
#define DEVICE_HARDWARE_ID_LEN 32
#define NETWORK_INTERFACE_NAME_LEN 32
#define NETWORK_HW_ADDRESS_LEN 18
#define PROTOCOL_NAME_LEN 16
#define SERVICE_NAMESPACE_LEN 128
#define SERVICE_XADDR_LEN 256

// Global device capabilities - must be non-const as capabilities may change at
// runtime
static struct device_capabilities dev_caps = {.has_analytics = 0,  // NOLINT
                                              .has_device = 1,
                                              .has_events = 0,
                                              .has_imaging = 1,
                                              .has_media = 1,
                                              .has_ptz = 1};

static int get_config_string(onvif_service_handler_instance_t *handler,
                             config_section_t section, const char *key,
                             char *value, size_t value_size,
                             const char *default_value);
static int get_config_int(onvif_service_handler_instance_t *handler,
                          config_section_t section, const char *key, int *value,
                          int default_value);
static void build_service_url(char *buffer, size_t buffer_size, int port,
                              const char *path);
static void build_date_xml(onvif_xml_builder_t *xml_builder,
                           const struct tm *tm_info) {
  onvif_xml_builder_start_element(xml_builder, "tt:Date", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Year", "%d",
                                                tm_info->tm_year + 1900);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Month", "%d",
                                                tm_info->tm_mon + 1);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Day", "%d",
                                                tm_info->tm_mday);
  onvif_xml_builder_end_element(xml_builder, "tt:Date");
}

static void build_time_xml(onvif_xml_builder_t *xml_builder,
                           const struct tm *tm_info) {
  onvif_xml_builder_start_element(xml_builder, "tt:Time", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Hour", "%d",
                                                tm_info->tm_hour);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Minute", "%d",
                                                tm_info->tm_min);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Second", "%d",
                                                tm_info->tm_sec);
  onvif_xml_builder_end_element(xml_builder, "tt:Time");
}

static void build_date_time_xml(onvif_xml_builder_t *xml_builder,
                                const struct tm *tm_info,
                                const char *element_name) {
  onvif_xml_builder_start_element(xml_builder, element_name, NULL);
  build_date_xml(xml_builder, tm_info);
  build_time_xml(xml_builder, tm_info);
  onvif_xml_builder_end_element(xml_builder, element_name);
}
static int handle_device_validation_error(const error_context_t *context,
                                          const error_result_t *result,
                                          onvif_response_t *response);
static int handle_device_system_error(const error_context_t *context,
                                      const error_result_t *result,
                                      onvif_response_t *response);

int onvif_device_get_device_information(struct device_info *info) {
  ONVIF_CHECK_NULL(info);

  strncpy(info->manufacturer, DEVICE_MANUFACTURER_DEFAULT,
          DEVICE_MANUFACTURER_LEN - 1);
  info->manufacturer[DEVICE_MANUFACTURER_LEN - 1] = '\0';

  strncpy(info->model, DEVICE_MODEL_DEFAULT, DEVICE_MODEL_LEN - 1);
  info->model[DEVICE_MODEL_LEN - 1] = '\0';

  strncpy(info->firmware_version, DEVICE_FIRMWARE_VER_DEFAULT,
          DEVICE_FIRMWARE_VER_LEN - 1);
  info->firmware_version[DEVICE_FIRMWARE_VER_LEN - 1] = '\0';

  strncpy(info->serial_number, DEVICE_SERIAL_DEFAULT, DEVICE_SERIAL_LEN - 1);
  info->serial_number[DEVICE_SERIAL_LEN - 1] = '\0';

  strncpy(info->hardware_id, DEVICE_HARDWARE_ID_DEFAULT,
          DEVICE_HARDWARE_ID_LEN - 1);
  info->hardware_id[DEVICE_HARDWARE_ID_LEN - 1] = '\0';

  return ONVIF_SUCCESS;
}

int onvif_device_get_capabilities(struct device_capabilities *caps) {
  ONVIF_CHECK_NULL(caps);

  *caps = dev_caps;
  return ONVIF_SUCCESS;
}

int onvif_device_get_system_date_time(struct system_date_time *date_time) {
  ONVIF_CHECK_NULL(date_time);

  time_t now = time(NULL);
  struct tm *tm_info = localtime(&now);

  date_time->date_time_type = 0;
  date_time->daylight_savings = 0;

  date_time->time_zone.tz_hour = 0;
  date_time->time_zone.tz_minute = 0;

  date_time->utc_date_time.year = tm_info->tm_year + 1900;
  date_time->utc_date_time.month = tm_info->tm_mon + 1;
  date_time->utc_date_time.day = tm_info->tm_mday;
  date_time->utc_date_time.hour = tm_info->tm_hour;
  date_time->utc_date_time.minute = tm_info->tm_min;
  date_time->utc_date_time.second = tm_info->tm_sec;

  date_time->local_date_time.year = date_time->utc_date_time.year;
  date_time->local_date_time.month = date_time->utc_date_time.month;
  date_time->local_date_time.day = date_time->utc_date_time.day;
  date_time->local_date_time.hour = date_time->utc_date_time.hour;
  date_time->local_date_time.minute = date_time->utc_date_time.minute;
  date_time->local_date_time.second = date_time->utc_date_time.second;

  return ONVIF_SUCCESS;
}

int onvif_device_set_system_date_time(
    const struct system_date_time *date_time) {
  ONVIF_CHECK_NULL(date_time);

  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Device", "SetSystemDateTime",
                           SERVICE_LOG_INFO);
  service_log_not_implemented(&log_ctx, "SetSystemDateTime");
  return ONVIF_SUCCESS;
}

int onvif_device_system_reboot(void) {
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Device", "SystemReboot",
                           SERVICE_LOG_INFO);
  service_log_info(&log_ctx, "System reboot requested via ONVIF");

  platform_log_notice(
      "ONVIF SystemReboot requested - initiating system reboot\n");

  int result = -1;

  // Note: system() call is acceptable here for reboot functionality
  // as it's a controlled command with no user input
  result = system("reboot");  // NOLINT
  if (result == 0) {
    platform_log_info("System reboot initiated via 'reboot' command\n");
    return ONVIF_SUCCESS;
  }

  // If all methods failed, log error and return failure
  platform_log_error(
      "All reboot methods failed - system may not support reboot\n");
  service_log_operation_failure(&log_ctx, "system_reboot", -1,
                                "All reboot methods failed");
  return ONVIF_ERROR;
}

int onvif_device_get_hostname(char *hostname, size_t hostname_len) {
  ONVIF_CHECK_NULL(hostname);
  if (hostname_len == 0) {
    return ONVIF_ERROR_INVALID;
  }

  struct utsname uts;
  if (uname(&uts) == 0) {
    strncpy(hostname, uts.nodename, hostname_len - 1);
    hostname[hostname_len - 1] = '\0';
    return ONVIF_SUCCESS;
  }

  /* Fallback hostname */
  strncpy(hostname, "anyka-camera", hostname_len - 1);
  hostname[hostname_len - 1] = '\0';
  return ONVIF_SUCCESS;
}

int onvif_device_set_hostname(const char *hostname) {
  ONVIF_CHECK_NULL(hostname);

  /* TODO: Implement hostname setting if needed */
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Device", "SetHostname", SERVICE_LOG_INFO);
  service_log_info(&log_ctx, "SetHostname requested: %s (not implemented)",
                   hostname);
  return ONVIF_SUCCESS;
}

int onvif_device_get_dns(struct dns_information *dns) {
  ONVIF_CHECK_NULL(dns);

  /* TODO: Read DNS settings from system or config */
  dns->from_dhcp = 1;
  dns->dns_manual_count = 0;
  dns->search_domain[0] = '\0';

  return ONVIF_SUCCESS;
}

int onvif_device_get_network_interfaces(struct network_interface *interfaces,
                                        int *count) {
  if (!interfaces || !count) {
    return ONVIF_ERROR_INVALID;
  }

  /* TODO: Enumerate actual network interfaces */
  *count = 1;
  interfaces[0].enabled = 1;
  strncpy(interfaces[0].name, "eth0", NETWORK_INTERFACE_NAME_LEN - 1);
  interfaces[0].name[NETWORK_INTERFACE_NAME_LEN - 1] = '\0';
  strncpy(interfaces[0].hw_address, "00:11:22:33:44:55",
          NETWORK_HW_ADDRESS_LEN - 1);
  interfaces[0].hw_address[NETWORK_HW_ADDRESS_LEN - 1] = '\0';
  interfaces[0].mtu = DEFAULT_MTU;

  return ONVIF_SUCCESS;
}

int onvif_device_get_network_protocols(struct network_protocol *protocols,
                                       int *count) {
  if (!protocols || !count) {
    return ONVIF_ERROR_INVALID;
  }

  /* Common protocols supported */
  *count = 3;

  /* HTTP */
  protocols[0].enabled = 1;
  strncpy(protocols[0].name, "HTTP", PROTOCOL_NAME_LEN - 1);
  protocols[0].name[PROTOCOL_NAME_LEN - 1] = '\0';
  protocols[0].port_count = 1;
  protocols[0].ports[0] = ONVIF_HTTP_STANDARD_PORT;

  /* HTTPS */
  protocols[1].enabled = 0;
  strncpy(protocols[1].name, "HTTPS", PROTOCOL_NAME_LEN - 1);
  protocols[1].name[PROTOCOL_NAME_LEN - 1] = '\0';
  protocols[1].port_count = 1;
  protocols[1].ports[0] = ONVIF_HTTPS_PORT_DEFAULT;

  /* RTSP */
  protocols[2].enabled = 1;
  strncpy(protocols[2].name, "RTSP", PROTOCOL_NAME_LEN - 1);
  protocols[2].name[PROTOCOL_NAME_LEN - 1] = '\0';
  protocols[2].port_count = 1;
  protocols[2].ports[0] = ONVIF_RTSP_PORT_DEFAULT;

  return ONVIF_SUCCESS;
}

int onvif_device_get_services(struct device_service *services, int *count) {
  if (!services || !count) {
    return ONVIF_ERROR_INVALID;
  }

  *count = 3;

  /* Device Service */
  strncpy(services[0].namespace, ONVIF_DEVICE_NS, SERVICE_NAMESPACE_LEN - 1);
  services[0].namespace[SERVICE_NAMESPACE_LEN - 1] = '\0';
  if (snprintf(services[0].xaddr, SERVICE_XADDR_LEN, "http://%s:%d%s",
               ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT,
               ONVIF_DEVICE_SERVICE_PATH) >= SERVICE_XADDR_LEN) {
    return ONVIF_ERROR;
  }
  services[0].version.major = 2;
  services[0].version.minor = 5;

  /* Media Service */
  strncpy(services[1].namespace, ONVIF_MEDIA_NS, SERVICE_NAMESPACE_LEN - 1);
  services[1].namespace[SERVICE_NAMESPACE_LEN - 1] = '\0';
  if (snprintf(services[1].xaddr, SERVICE_XADDR_LEN, "http://%s:%d%s",
               ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT,
               ONVIF_MEDIA_SERVICE_PATH) >= SERVICE_XADDR_LEN) {
    return ONVIF_ERROR;
  }
  services[1].version.major = 2;
  services[1].version.minor = 5;

  /* PTZ Service */
  strncpy(services[2].namespace, ONVIF_PTZ_NS, SERVICE_NAMESPACE_LEN - 1);
  services[2].namespace[SERVICE_NAMESPACE_LEN - 1] = '\0';
  if (snprintf(services[2].xaddr, SERVICE_XADDR_LEN, "http://%s:%d%s",
               ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT,
               ONVIF_PTZ_SERVICE_PATH) >= SERVICE_XADDR_LEN) {
    return ONVIF_ERROR;
  }
  services[2].version.major = 2;
  services[2].version.minor = 5;

  return ONVIF_SUCCESS;
}

/* Helper Functions */

static int get_config_string(onvif_service_handler_instance_t *handler,
                             config_section_t section, const char *key,
                             char *value, size_t value_size,
                             const char *default_value) {
  if (!handler || !key || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Try to get from configuration
  // Simplified config access - just use default values for now
  int result = ONVIF_SUCCESS;
  if (result != ONVIF_SUCCESS && default_value) {
    strncpy(value, default_value, value_size - 1);
    value[value_size - 1] = '\0';
    return ONVIF_SUCCESS;
  }
  if (default_value) {
    strncpy(value, default_value, value_size - 1);
    value[value_size - 1] = '\0';
  } else {
    value[0] = '\0';
  }

  return result;
}

static int get_config_int(onvif_service_handler_instance_t *handler,
                          config_section_t section, const char *key, int *value,
                          int default_value) {
  if (!handler || !key || !value) {
    return ONVIF_ERROR_INVALID;
  }

  // Try to get from configuration
  // Simplified config access - just use default values for now
  *value = default_value;
  int result = ONVIF_SUCCESS;
  if (result != ONVIF_SUCCESS) {
    *value = default_value;
    return ONVIF_SUCCESS;
  }

  return result;
}

static void build_service_url(char *buffer, size_t buffer_size, int port,
                              const char *path) {
  if (!buffer || buffer_size == 0 || !path) {
    return;
  }

  if (snprintf(buffer, buffer_size, "http://[IP]:%d%s", port, path) >=
      buffer_size) {
    service_log_context_t log_ctx;
    service_log_init_context(&log_ctx, "Device", "build_service_url",
                             SERVICE_LOG_ERROR);
    service_log_operation_failure(&log_ctx, "build_service_url", -1,
                                  "Buffer too small");
  }
}

// Removed - now using common_xml_build_time_element,
// common_xml_build_date_element, and common_xml_build_datetime_element from
// common_xml_builder.h

static int handle_device_validation_error(const error_context_t *context,
                                          const error_result_t *result,
                                          onvif_response_t *response) {
  if (!context || !result || !response) {
    return ONVIF_ERROR_INVALID;
  }

  platform_log_error("Device validation failed: %s", result->error_message);
  return onvif_generate_fault_response(response, result->soap_fault_code,
                                       result->soap_fault_string);
}

static int handle_device_system_error(const error_context_t *context,
                                      const error_result_t *result,
                                      onvif_response_t *response) {
  if (!context || !result || !response) {
    return ONVIF_ERROR_INVALID;
  }

  platform_log_error("Device system error: %s", result->error_message);
  return onvif_generate_fault_response(response, result->soap_fault_code,
                                       result->soap_fault_string);
}

/* Refactored device service implementation */

// Service handler instance - must be non-const as it's initialized at runtime
static onvif_service_handler_instance_t
    g_device_handler;  // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
static int g_handler_initialized = 0;  // NOLINT

// Action handlers
static int handle_get_capabilities(const service_handler_config_t *config,
                                   const onvif_request_t *request,
                                   onvif_response_t *response,
                                   onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Device", "GetCapabilities",
                     "capabilities_retrieval");

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing",
                                  response);
  }

  // Get HTTP port from configuration with fallback
  int http_port = DEFAULT_HTTP_PORT;
  if (get_config_int(&g_device_handler, CONFIG_SECTION_ONVIF, "http_port",
                     &http_port, DEFAULT_HTTP_PORT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }

  // Build capabilities XML using XML builder
  onvif_xml_builder_start_element(xml_builder, "tds:Capabilities", NULL);

  // Analytics capabilities
  onvif_xml_builder_self_closing_element(
      xml_builder, "tt:Analytics", "XAddr",
      "http://[IP]:8080/onvif/analytics_service", "AnalyticsModuleSupport",
      dev_caps.has_analytics ? "true" : "false", "RuleSupport",
      dev_caps.has_analytics ? "true" : "false",
      "CellBasedSceneDescriptionSupported",
      dev_caps.has_analytics ? "true" : "false", "MulticastSupport",
      dev_caps.has_analytics ? "true" : "false", NULL);

  // Device capabilities
  onvif_xml_builder_self_closing_element(
      xml_builder, "tt:Device", "XAddr",
      "http://[IP]:8080/onvif/device_service", "Network",
      dev_caps.has_device ? "true" : "false", "System",
      dev_caps.has_device ? "true" : "false", "IO",
      dev_caps.has_device ? "true" : "false", "Security",
      dev_caps.has_device ? "true" : "false", NULL);

  // Events capabilities
  onvif_xml_builder_self_closing_element(
      xml_builder, "tt:Events", "XAddr", "http://[IP]:8080/onvif/event_service",
      "WSPullPointSupport", dev_caps.has_events ? "true" : "false",
      "WSSubscriptionPolicySupport", dev_caps.has_events ? "true" : "false",
      "WSPausableSubscriptionManagerInterfaceSupport",
      dev_caps.has_events ? "true" : "false", NULL);

  // Imaging capabilities
  onvif_xml_builder_self_closing_element(
      xml_builder, "tt:Imaging", "XAddr",
      "http://[IP]:8080/onvif/imaging_service", NULL);

  // Media capabilities
  onvif_xml_builder_self_closing_element(
      xml_builder, "tt:Media", "XAddr", "http://[IP]:8080/onvif/media_service",
      "StreamingCapabilities", dev_caps.has_media ? "true" : "false", NULL);

  // PTZ capabilities
  onvif_xml_builder_self_closing_element(xml_builder, "tt:PTZ", "XAddr",
                                         "http://[IP]:8080/onvif/ptz_service",
                                         NULL);

  onvif_xml_builder_end_element(xml_builder, "tds:Capabilities");

  // Generate success response
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  // Generate simple success response
  response->status_code = 200;
  strncpy(response->body, xml_content, sizeof(response->body) - 1);
  response->body[sizeof(response->body) - 1] = '\0';
  return ONVIF_SUCCESS;
}

static int handle_get_device_information(const service_handler_config_t *config,
                                         const onvif_request_t *request,
                                         onvif_response_t *response,
                                         onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Device", "GetDeviceInformation",
                     "device_info_retrieval");

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing",
                                  response);
  }

  // Get device information from configuration if available
  char manufacturer[DEVICE_MANUFACTURER_LEN];
  char model[DEVICE_MODEL_LEN];
  char firmware_version[DEVICE_FIRMWARE_VER_LEN];
  char serial_number[DEVICE_SERIAL_LEN];
  char hardware_id[DEVICE_HARDWARE_ID_LEN];

  // Try to get from configuration with fallback to defaults
  if (get_config_string(&g_device_handler, CONFIG_SECTION_DEVICE,
                        "manufacturer", manufacturer, sizeof(manufacturer),
                        DEVICE_MANUFACTURER_DEFAULT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }
  if (get_config_string(&g_device_handler, CONFIG_SECTION_DEVICE, "model",
                        model, sizeof(model),
                        DEVICE_MODEL_DEFAULT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }
  if (get_config_string(&g_device_handler, CONFIG_SECTION_DEVICE,
                        "firmware_version", firmware_version,
                        sizeof(firmware_version),
                        DEVICE_FIRMWARE_VER_DEFAULT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }
  if (get_config_string(&g_device_handler, CONFIG_SECTION_DEVICE,
                        "serial_number", serial_number, sizeof(serial_number),
                        DEVICE_SERIAL_DEFAULT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }
  if (get_config_string(&g_device_handler, CONFIG_SECTION_DEVICE, "hardware_id",
                        hardware_id, sizeof(hardware_id),
                        DEVICE_HARDWARE_ID_DEFAULT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }

  // Build device information XML
  onvif_xml_builder_element_with_text(xml_builder, "tds:Manufacturer",
                                      manufacturer, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:Model", model, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:FirmwareVersion",
                                      firmware_version, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:SerialNumber",
                                      serial_number, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:HardwareId",
                                      hardware_id, NULL);

  // Generate success response
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  // Generate simple success response
  response->status_code = 200;
  strncpy(response->body, xml_content, sizeof(response->body) - 1);
  response->body[sizeof(response->body) - 1] = '\0';
  return ONVIF_SUCCESS;
}

static int handle_get_system_date_time(const service_handler_config_t *config,
                                       const onvif_request_t *request,
                                       onvif_response_t *response,
                                       onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Device", "GetSystemDateAndTime",
                     "system_time_retrieval");

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing",
                                  response);
  }

  time_t now = time(NULL);
  if (now == (time_t)-1) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }

  struct tm *tm_info = localtime(&now);
  if (!tm_info) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }

  // Build system date and time XML
  onvif_xml_builder_start_element(xml_builder, "tds:SystemDateAndTime", NULL);

  onvif_xml_builder_element_with_text(xml_builder, "tt:DateTimeType", "Manual",
                                      NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tt:DaylightSavings",
                                      "false", NULL);

  // Time zone
  onvif_xml_builder_start_element(xml_builder, "tt:TimeZone", NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:TZ", "+00:00");
  onvif_xml_builder_end_element(xml_builder, "tt:TimeZone");

  // UTC date time
  onvif_xml_builder_start_element(xml_builder, "tt:UTCDateTime", NULL);
  build_date_time_xml(xml_builder, tm_info, "tt:Date");
  build_time_xml(xml_builder, tm_info);
  onvif_xml_builder_end_element(xml_builder, "tt:UTCDateTime");

  // Local date time (same as UTC for simplicity)
  onvif_xml_builder_start_element(xml_builder, "tt:LocalDateTime", NULL);
  build_date_time_xml(xml_builder, tm_info, "tt:Date");
  build_time_xml(xml_builder, tm_info);
  onvif_xml_builder_end_element(xml_builder, "tt:LocalDateTime");

  onvif_xml_builder_end_element(xml_builder, "tds:SystemDateAndTime");

  // Generate success response
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  // Generate simple success response
  response->status_code = 200;
  strncpy(response->body, xml_content, sizeof(response->body) - 1);
  response->body[sizeof(response->body) - 1] = '\0';
  return ONVIF_SUCCESS;
}

static int handle_get_services(const service_handler_config_t *config,
                               const onvif_request_t *request,
                               onvif_response_t *response,
                               onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Device", "GetServices",
                     "service_list_retrieval");

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing",
                                  response);
  }

  // Get HTTP port from configuration with fallback
  int http_port = DEFAULT_HTTP_PORT;
  if (get_config_int(&g_device_handler, CONFIG_SECTION_ONVIF, "http_port",
                     &http_port, DEFAULT_HTTP_PORT) != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER,
                                         "Internal server error");
  }

  // Build services XML directly
  onvif_xml_builder_start_element(xml_builder, "tds:Services", NULL);

  char device_xaddr[256];
  if (snprintf(device_xaddr, sizeof(device_xaddr),
               "http://[IP]:%d/onvif/device_service",
               http_port) >= sizeof(device_xaddr)) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "xaddr_build",
                               response);
  }

  // Device service
  onvif_xml_builder_start_element(xml_builder, "tds:Service", NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:Namespace",
                                      "http://www.onvif.org/ver10/device/wsdl",
                                      NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tds:XAddr", device_xaddr,
                                      NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tds:Version",
                                                "2.5");
  onvif_xml_builder_end_element(xml_builder, "tds:Service");

  onvif_xml_builder_end_element(xml_builder, "tds:Services");

  // Generate simple success response
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  response->status_code = 200;
  strncpy(response->body, xml_content, sizeof(response->body) - 1);
  response->body[sizeof(response->body) - 1] = '\0';
  return ONVIF_SUCCESS;
}

static int handle_system_reboot(const service_handler_config_t *config,
                                const onvif_request_t *request,
                                onvif_response_t *response,
                                onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Device", "SystemReboot", "system_reboot");

  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing",
                                  response);
  }

  // Call the system reboot function
  int result = onvif_device_system_reboot();
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "system_reboot_execution",
                               response);
  }

  // Build empty response XML (SystemReboot doesn't return data)
  onvif_xml_builder_start_element(xml_builder, "tds:SystemRebootResponse",
                                  NULL);
  onvif_xml_builder_end_element(xml_builder, "tds:SystemRebootResponse");

  // Generate success response
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  // Generate simple success response
  response->status_code = 200;
  strncpy(response->body, xml_content, sizeof(response->body) - 1);
  response->body[sizeof(response->body) - 1] = '\0';
  return ONVIF_SUCCESS;
}

// Action definitions
static const service_action_def_t device_actions[] = {
    {ONVIF_ACTION_GET_CAPABILITIES, "GetCapabilities", handle_get_capabilities,
     0},
    {ONVIF_ACTION_GET_DEVICE_INFORMATION, "GetDeviceInformation",
     handle_get_device_information, 0},
    {ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME, "GetSystemDateAndTime",
     handle_get_system_date_time, 0},
    {ONVIF_ACTION_GET_SERVICES, "GetServices", handle_get_services, 0},
    {ONVIF_ACTION_SYSTEM_REBOOT, "SystemReboot", handle_system_reboot, 0}};

int onvif_device_init(config_manager_t *config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {
      .service_type = ONVIF_SERVICE_DEVICE,
      .service_name = "Device",
      .config = config,
      .enable_validation = 1,
      .enable_logging = 1};

  int result = onvif_service_handler_init(
      &g_device_handler, &handler_config, device_actions,
      sizeof(device_actions) / sizeof(device_actions[0]));

  if (result == ONVIF_SUCCESS) {
    // Register device-specific error handlers
    // Error handler registration not implemented yet
    // Error handler registration not implemented yet

    g_handler_initialized = 1;
  }

  return result;
}

int onvif_device_handle_request(onvif_action_type_t action,
                                const onvif_request_t *request,
                                onvif_response_t *response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }

  return onvif_service_handler_handle_request(&g_device_handler, action,
                                              request, response);
}

void onvif_device_cleanup(void) {
  if (g_handler_initialized) {
    error_context_t error_ctx;
    error_context_init(&error_ctx, "Device", "Cleanup", "service_cleanup");

    onvif_service_handler_cleanup(&g_device_handler);

    // Unregister error handlers
    // Error handler unregistration not implemented yet

    g_handler_initialized = 0;

    // Check for memory leaks
    memory_manager_check_leaks();
  }
}
