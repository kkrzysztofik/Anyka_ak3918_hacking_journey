/*
 * onvif_device.c - ONVIF Device service implementation
 * 
 * This file implements the ONVIF Device Web Service endpoints including
 * device information, capabilities, and system management.
 */

#include <stdio.h>
#include <string.h>
#include <time.h>
#include <sys/utsname.h>
#include <stdlib.h>
#include "onvif_device.h"
#include "utils/config.h"
#include "utils/response_buffer.h"
#include "utils/xml_utils.h"
#include "utils/logging_utils.h"
#include "utils/error_handling.h"
#include "utils/constants_clean.h"
#include "utils/unified_soap_generator.h"
#include "utils/safe_string.h"
#include "utils/memory_manager.h"
#include "utils/response_helpers.h"
#include "utils/service_handler.h"
#include "utils/centralized_config.h"
#include "common/onvif_types.h"

/* Device information constants */
#define DEVICE_MANUFACTURER    "Anyka"
#define DEVICE_MODEL          "AK3918 Camera"
#define DEVICE_FIRMWARE_VER   "1.0.0"
#define DEVICE_SERIAL         "AK3918-001"
#define DEVICE_HARDWARE_ID    "1.0"

/* Static device capabilities */
static struct device_capabilities dev_caps = {
    .has_analytics = 0,
    .has_device = 1,
    .has_events = 0,
    .has_imaging = 1,
    .has_media = 1,
    .has_ptz = 1
};

int onvif_device_get_device_information(struct device_info *info) {
    ONVIF_CHECK_NULL(info);
    
    strncpy(info->manufacturer, DEVICE_MANUFACTURER, sizeof(info->manufacturer) - 1);
    strncpy(info->model, DEVICE_MODEL, sizeof(info->model) - 1);
    strncpy(info->firmware_version, DEVICE_FIRMWARE_VER, sizeof(info->firmware_version) - 1);
    strncpy(info->serial_number, DEVICE_SERIAL, sizeof(info->serial_number) - 1);
    strncpy(info->hardware_id, DEVICE_HARDWARE_ID, sizeof(info->hardware_id) - 1);
    
    return ONVIF_SUCCESS;
}

int onvif_device_get_capabilities(struct device_capabilities *caps) {
    ONVIF_CHECK_NULL(caps);
    
    *caps = dev_caps;
    return ONVIF_SUCCESS;
}

int onvif_device_get_system_date_time(struct system_date_time *dt) {
    ONVIF_CHECK_NULL(dt);
    
    time_t now = time(NULL);
    struct tm *tm_info = localtime(&now);
    
    dt->date_time_type = 0; /* Manual */
    dt->daylight_savings = 0;
    
    dt->time_zone.tz_hour = 0;  /* UTC for simplicity */
    dt->time_zone.tz_minute = 0;
    
    dt->utc_date_time.year = tm_info->tm_year + 1900;
    dt->utc_date_time.month = tm_info->tm_mon + 1;
    dt->utc_date_time.day = tm_info->tm_mday;
    dt->utc_date_time.hour = tm_info->tm_hour;
    dt->utc_date_time.minute = tm_info->tm_min;
    dt->utc_date_time.second = tm_info->tm_sec;
    
    /* Local time same as UTC for now */
    dt->local_date_time.year = dt->utc_date_time.year;
    dt->local_date_time.month = dt->utc_date_time.month;
    dt->local_date_time.day = dt->utc_date_time.day;
    dt->local_date_time.hour = dt->utc_date_time.hour;
    dt->local_date_time.minute = dt->utc_date_time.minute;
    dt->local_date_time.second = dt->utc_date_time.second;
    
    return ONVIF_SUCCESS;
}

int onvif_device_set_system_date_time(const struct system_date_time *dt) {
    ONVIF_CHECK_NULL(dt);
    
    /* TODO: Implement system time setting if needed */
    platform_log_info("SetSystemDateTime requested (not implemented)\n");
    return ONVIF_SUCCESS;
}

int onvif_device_system_reboot(void) {
    /* TODO: Implement system reboot */
    platform_log_info("SystemReboot requested (not implemented)\n");
    return ONVIF_SUCCESS;
}

int onvif_device_get_hostname(char *hostname, size_t hostname_len) {
    ONVIF_CHECK_NULL(hostname);
    if (hostname_len == 0) return ONVIF_ERROR_INVALID;
    
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
    platform_log_info("SetHostname requested: %s (not implemented)\n", hostname);
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

int onvif_device_get_network_interfaces(struct network_interface *interfaces, int *count) {
    if (!interfaces || !count) return -1;
    
    /* TODO: Enumerate actual network interfaces */
    *count = 1;
    interfaces[0].enabled = 1;
    strncpy(interfaces[0].name, "eth0", sizeof(interfaces[0].name) - 1);
    strncpy(interfaces[0].hw_address, "00:11:22:33:44:55", sizeof(interfaces[0].hw_address) - 1);
    interfaces[0].mtu = 1500;
    
    return ONVIF_SUCCESS;
}

int onvif_device_get_network_protocols(struct network_protocol *protocols, int *count) {
    if (!protocols || !count) return -1;
    
    /* Common protocols supported */
    *count = 3;
    
    /* HTTP */
    protocols[0].enabled = 1;
    strncpy(protocols[0].name, "HTTP", sizeof(protocols[0].name) - 1);
    protocols[0].port_count = 1;
    protocols[0].ports[0] = ONVIF_HTTP_STANDARD_PORT;
    
    /* HTTPS */
    protocols[1].enabled = 0;
    strncpy(protocols[1].name, "HTTPS", sizeof(protocols[1].name) - 1);
    protocols[1].port_count = 1;
    protocols[1].ports[0] = ONVIF_HTTPS_PORT_DEFAULT;
    
    /* RTSP */
    protocols[2].enabled = 1;
    strncpy(protocols[2].name, "RTSP", sizeof(protocols[2].name) - 1);
    protocols[2].port_count = 1;
    protocols[2].ports[0] = ONVIF_RTSP_PORT_DEFAULT;
    
    return ONVIF_SUCCESS;
}

int onvif_device_get_services(struct device_service *services, int *count) {
    if (!services || !count) return -1;
    
    *count = 3;
    
    /* Device Service */
    strncpy(services[0].namespace, ONVIF_DEVICE_NS, sizeof(services[0].namespace) - 1);
    snprintf(services[0].xaddr, sizeof(services[0].xaddr), "http://%s:%d%s", 
             ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT, ONVIF_DEVICE_SERVICE_PATH);
    services[0].version.major = 2;
    services[0].version.minor = 5;
    
    /* Media Service */
    strncpy(services[1].namespace, ONVIF_MEDIA_NS, sizeof(services[1].namespace) - 1);
    snprintf(services[1].xaddr, sizeof(services[1].xaddr), "http://%s:%d%s", 
             ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT, ONVIF_MEDIA_SERVICE_PATH);
    services[1].version.major = 2;
    services[1].version.minor = 5;
    
    /* PTZ Service */
    strncpy(services[2].namespace, ONVIF_PTZ_NS, sizeof(services[2].namespace) - 1);
    snprintf(services[2].xaddr, sizeof(services[2].xaddr), "http://%s:%d%s", 
             ONVIF_IP_PLACEHOLDER, ONVIF_HTTP_PORT_DEFAULT, ONVIF_PTZ_SERVICE_PATH);
    services[2].version.major = 2;
    services[2].version.minor = 5;
    
    return ONVIF_SUCCESS;
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */



/* Refactored device service implementation */

// Service handler instance
static service_handler_t g_device_handler;
static int g_handler_initialized = 0;

// Action handlers
static int handle_get_capabilities(const service_handler_config_t *config,
                                  const onvif_request_t *request,
                                  onvif_response_t *response,
                                  xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Build capabilities XML using XML builder
  xml_builder_start_element(xml_builder, "tds:Capabilities", NULL);
  
  // Analytics capabilities
  xml_builder_self_closing_element(xml_builder, "tt:Analytics",
    "XAddr", "http://[IP]:8080/onvif/analytics_service",
    "AnalyticsModuleSupport", dev_caps.has_analytics ? "true" : "false",
    "RuleSupport", dev_caps.has_analytics ? "true" : "false",
    "CellBasedSceneDescriptionSupported", dev_caps.has_analytics ? "true" : "false",
    "MulticastSupport", dev_caps.has_analytics ? "true" : "false",
    NULL);
  
  // Device capabilities
  xml_builder_self_closing_element(xml_builder, "tt:Device",
    "XAddr", "http://[IP]:8080/onvif/device_service",
    "Network", dev_caps.has_device ? "true" : "false",
    "System", dev_caps.has_device ? "true" : "false",
    "IO", dev_caps.has_device ? "true" : "false",
    "Security", dev_caps.has_device ? "true" : "false",
    NULL);
  
  // Events capabilities
  xml_builder_self_closing_element(xml_builder, "tt:Events",
    "XAddr", "http://[IP]:8080/onvif/event_service",
    "WSPullPointSupport", dev_caps.has_events ? "true" : "false",
    "WSSubscriptionPolicySupport", dev_caps.has_events ? "true" : "false",
    "WSPausableSubscriptionManagerInterfaceSupport", dev_caps.has_events ? "true" : "false",
    NULL);
  
  // Imaging capabilities
  xml_builder_self_closing_element(xml_builder, "tt:Imaging",
    "XAddr", "http://[IP]:8080/onvif/imaging_service",
    NULL);
  
  // Media capabilities
  xml_builder_self_closing_element(xml_builder, "tt:Media",
    "XAddr", "http://[IP]:8080/onvif/media_service",
    "StreamingCapabilities", dev_caps.has_media ? "true" : "false",
    NULL);
  
  // PTZ capabilities
  xml_builder_self_closing_element(xml_builder, "tt:PTZ",
    "XAddr", "http://[IP]:8080/onvif/ptz_service",
    NULL);
  
  xml_builder_end_element(xml_builder, "tds:Capabilities");
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_device_handler, "GetCapabilities", xml_content, response);
}

static int handle_get_device_information(const service_handler_config_t *config,
                                        const onvif_request_t *request,
                                        onvif_response_t *response,
                                        xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Get device information from configuration if available
  char manufacturer[64] = DEVICE_MANUFACTURER;
  char model[64] = DEVICE_MODEL;
  char firmware_version[32] = DEVICE_FIRMWARE_VER;
  char serial_number[64] = DEVICE_SERIAL;
  char hardware_id[32] = DEVICE_HARDWARE_ID;
  
  // Try to get from configuration
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_DEVICE,
                                            "manufacturer", manufacturer, CONFIG_TYPE_STRING);
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_DEVICE,
                                            "model", model, CONFIG_TYPE_STRING);
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_DEVICE,
                                            "firmware_version", firmware_version, CONFIG_TYPE_STRING);
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_DEVICE,
                                            "serial_number", serial_number, CONFIG_TYPE_STRING);
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_DEVICE,
                                            "hardware_id", hardware_id, CONFIG_TYPE_STRING);
  
  // Build device information XML
  xml_builder_element_with_text(xml_builder, "tds:Manufacturer", manufacturer, NULL);
  xml_builder_element_with_text(xml_builder, "tds:Model", model, NULL);
  xml_builder_element_with_text(xml_builder, "tds:FirmwareVersion", firmware_version, NULL);
  xml_builder_element_with_text(xml_builder, "tds:SerialNumber", serial_number, NULL);
  xml_builder_element_with_text(xml_builder, "tds:HardwareId", hardware_id, NULL);
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_device_handler, "GetDeviceInformation", xml_content, response);
}

static int handle_get_system_date_time(const service_handler_config_t *config,
                                      const onvif_request_t *request,
                                      onvif_response_t *response,
                                      xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  time_t now = time(NULL);
  struct tm *tm_info = localtime(&now);
  
  // Build system date and time XML
  xml_builder_start_element(xml_builder, "tds:SystemDateAndTime", NULL);
  
  xml_builder_element_with_text(xml_builder, "tt:DateTimeType", "Manual", NULL);
  xml_builder_element_with_text(xml_builder, "tt:DaylightSavings", "false", NULL);
  
  // Time zone
  xml_builder_start_element(xml_builder, "tt:TimeZone", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:TZ", "+00:00");
  xml_builder_end_element(xml_builder, "tt:TimeZone");
  
  // UTC date time
  xml_builder_start_element(xml_builder, "tt:UTCDateTime", NULL);
  
  // Time
  xml_builder_start_element(xml_builder, "tt:Time", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Hour", "%d", tm_info->tm_hour);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Minute", "%d", tm_info->tm_min);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Second", "%d", tm_info->tm_sec);
  xml_builder_end_element(xml_builder, "tt:Time");
  
  // Date
  xml_builder_start_element(xml_builder, "tt:Date", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Year", "%d", tm_info->tm_year + 1900);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Month", "%d", tm_info->tm_mon + 1);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Day", "%d", tm_info->tm_mday);
  xml_builder_end_element(xml_builder, "tt:Date");
  
  xml_builder_end_element(xml_builder, "tt:UTCDateTime");
  
  // Local date time (same as UTC for simplicity)
  xml_builder_start_element(xml_builder, "tt:LocalDateTime", NULL);
  
  // Time
  xml_builder_start_element(xml_builder, "tt:Time", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Hour", "%d", tm_info->tm_hour);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Minute", "%d", tm_info->tm_min);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Second", "%d", tm_info->tm_sec);
  xml_builder_end_element(xml_builder, "tt:Time");
  
  // Date
  xml_builder_start_element(xml_builder, "tt:Date", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Year", "%d", tm_info->tm_year + 1900);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Month", "%d", tm_info->tm_mon + 1);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Day", "%d", tm_info->tm_mday);
  xml_builder_end_element(xml_builder, "tt:Date");
  
  xml_builder_end_element(xml_builder, "tt:LocalDateTime");
  
  xml_builder_end_element(xml_builder, "tds:SystemDateAndTime");
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_device_handler, "GetSystemDateAndTime", xml_content, response);
}

static int handle_get_services(const service_handler_config_t *config,
                              const onvif_request_t *request,
                              onvif_response_t *response,
                              xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Get HTTP port from configuration
  int http_port = 8080;
  service_handler_get_config_value(&g_device_handler, CONFIG_SECTION_ONVIF,
                                            "http_port", &http_port, CONFIG_TYPE_INT);
  
  // Build services XML
  xml_builder_start_element(xml_builder, "tds:Service", NULL);
  
  // Device Service
  xml_builder_start_element(xml_builder, "tt:Namespace", NULL);
  xml_builder_raw_content(xml_builder, "http://www.onvif.org/ver10/device/wsdl");
  xml_builder_end_element(xml_builder, "tt:Namespace");
  
  xml_builder_start_element(xml_builder, "tt:XAddr", NULL);
  xml_builder_formatted_content(xml_builder, "http://[IP]:%d/onvif/device_service", http_port);
  xml_builder_end_element(xml_builder, "tt:XAddr");
  
  xml_builder_start_element(xml_builder, "tt:Version", NULL);
  xml_builder_element_with_text(xml_builder, "tt:Major", "2", NULL);
  xml_builder_element_with_text(xml_builder, "tt:Minor", "5", NULL);
  xml_builder_end_element(xml_builder, "tt:Version");
  
  xml_builder_end_element(xml_builder, "tds:Service");
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_device_handler, "GetServices", xml_content, response);
}

// Action definitions
static const service_action_def_t device_actions[] = {
  {ONVIF_ACTION_GET_CAPABILITIES, "GetCapabilities", handle_get_capabilities, 0},
  {ONVIF_ACTION_GET_DEVICE_INFORMATION, "GetDeviceInformation", handle_get_device_information, 0},
  {ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME, "GetSystemDateAndTime", handle_get_system_date_time, 0},
  {ONVIF_ACTION_GET_SERVICES, "GetServices", handle_get_services, 0}
};

int onvif_device_init(centralized_config_t *config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }
  
  service_handler_config_t handler_config = {
    .service_type = ONVIF_SERVICE_DEVICE,
    .service_name = "Device",
    .config = config,
    .enable_validation = 1,
    .enable_logging = 1
  };
  
  int result = service_handler_init(&g_device_handler, &handler_config,
                                             device_actions, sizeof(device_actions) / sizeof(device_actions[0]));
  
  if (result == ONVIF_SUCCESS) {
    g_handler_initialized = 1;
  }
  
  return result;
}

int onvif_device_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }
  
  return service_handler_handle_request(&g_device_handler, action, request, response);
}

void onvif_device_cleanup(void) {
  if (g_handler_initialized) {
    service_handler_cleanup(&g_device_handler);
    g_handler_initialized = 0;
  }
}
