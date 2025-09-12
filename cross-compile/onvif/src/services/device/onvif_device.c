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
#include "config.h"
#include "../utils/response_buffer.h"
#include "../utils/xml_utils.h"
#include "../utils/logging_utils.h"
#include "../utils/error_handling.h"
#include "../utils/constants_clean.h"
#include "../common/onvif_types.h"

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

/* SOAP XML generation helpers */
static void soap_fault_response(char *response, size_t response_size, const char *fault_code, const char *fault_string) {
    snprintf(response, response_size, 
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <soap:Fault>\n"
        "      <soap:Code>\n"
        "        <soap:Value>%s</soap:Value>\n"
        "      </soap:Code>\n"
        "      <soap:Reason>\n"
        "        <soap:Text>%s</soap:Text>\n"
        "      </soap:Reason>\n"
        "    </soap:Fault>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", fault_code, fault_string);
}

static void soap_success_response(char *response, size_t response_size, const char *action, const char *body_content) {
    snprintf(response, response_size,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <tds:%sResponse xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\n"
        "      %s\n"
        "    </tds:%sResponse>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", action, body_content, action);
}

/* XML parsing helpers - now using xml_utils module */


/**
 * @brief Handle ONVIF device service requests
 */
int onvif_device_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!request || !response) {
        return ONVIF_ERROR;
    }
    
    // Use response buffer management for automatic cleanup
    response_buffer_t resp_buf;
    if (response_buffer_init(&resp_buf, response) != 0) {
        return ONVIF_ERROR;
    }
    response->body_length = 0;
    
    switch (action) {
        case ONVIF_ACTION_GET_CAPABILITIES: {
            struct device_capabilities caps;
            if (onvif_device_get_capabilities(&caps) == 0) {
                char caps_xml[ONVIF_XML_BUFFER_SIZE];
                snprintf(caps_xml, sizeof(caps_xml),
                    "<tds:Capabilities>\n"
                    "  <tt:Analytics XAddr=\"http://[IP]:8080/onvif/analytics_service\" "
                    "AnalyticsModuleSupport=\"%s\" RuleSupport=\"%s\" "
                    "CellBasedSceneDescriptionSupported=\"%s\" "
                    "MulticastSupport=\"%s\" />\n"
                    "  <tt:Device XAddr=\"http://[IP]:8080/onvif/device_service\" "
                    "Network=\"%s\" System=\"%s\" IO=\"%s\" Security=\"%s\" />\n"
                    "  <tt:Events XAddr=\"http://[IP]:8080/onvif/event_service\" "
                    "WSPullPointSupport=\"%s\" WSSubscriptionPolicySupport=\"%s\" "
                    "WSPausableSubscriptionManagerInterfaceSupport=\"%s\" />\n"
                    "  <tt:Imaging XAddr=\"http://[IP]:8080/onvif/imaging_service\" />\n"
                    "  <tt:Media XAddr=\"http://[IP]:8080/onvif/media_service\" "
                    "StreamingCapabilities=\"%s\" />\n"
                    "  <tt:PTZ XAddr=\"http://[IP]:8080/onvif/ptz_service\" />\n"
                    "</tds:Capabilities>",
                    caps.has_analytics ? "true" : "false",
                    caps.has_analytics ? "true" : "false", 
                    caps.has_analytics ? "true" : "false",
                    caps.has_analytics ? "true" : "false",
                    caps.has_device ? "true" : "false",
                    caps.has_device ? "true" : "false",
                    caps.has_device ? "true" : "false", 
                    caps.has_device ? "true" : "false",
                    caps.has_events ? "true" : "false",
                    caps.has_events ? "true" : "false",
                    caps.has_events ? "true" : "false",
                    caps.has_media ? "true" : "false");
                
                response_buffer_set_body_printf(&resp_buf,
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <tds:GetCapabilitiesResponse xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\n"
                    "      %s\n"
                    "    </tds:GetCapabilitiesResponse>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>", caps_xml);
            } else {
                response_buffer_set_body_printf(&resp_buf,
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <soap:Fault>\n"
                    "      <soap:Code>\n"
                    "        <soap:Value>soap:Receiver</soap:Value>\n"
                    "      </soap:Code>\n"
                    "      <soap:Reason>\n"
                    "        <soap:Text>Failed to get capabilities</soap:Text>\n"
                    "      </soap:Reason>\n"
                    "    </soap:Fault>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>");
            }
            break;
        }
        
        case ONVIF_ACTION_GET_DEVICE_INFORMATION: {
            struct device_info info;
            if (onvif_device_get_device_information(&info) == 0) {
                char info_xml[ONVIF_XML_BUFFER_SIZE];
                snprintf(info_xml, sizeof(info_xml),
                    "<tds:Manufacturer>%s</tds:Manufacturer>\n"
                    "<tds:Model>%s</tds:Model>\n"
                    "<tds:FirmwareVersion>%s</tds:FirmwareVersion>\n"
                    "<tds:SerialNumber>%s</tds:SerialNumber>\n"
                    "<tds:HardwareId>%s</tds:HardwareId>",
                    info.manufacturer, info.model, info.firmware_version,
                    info.serial_number, info.hardware_id);
                
                response_buffer_set_body_printf(&resp_buf,
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <tds:GetDeviceInformationResponse xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\n"
                    "      %s\n"
                    "    </tds:GetDeviceInformationResponse>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>", info_xml);
            } else {
                response_buffer_set_body_printf(&resp_buf,
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <soap:Fault>\n"
                    "      <soap:Code>\n"
                    "        <soap:Value>soap:Receiver</soap:Value>\n"
                    "      </soap:Code>\n"
                    "      <soap:Reason>\n"
                    "        <soap:Text>Failed to get device information</soap:Text>\n"
                    "      </soap:Reason>\n"
                    "    </soap:Fault>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>");
            }
            break;
        }
        
        case ONVIF_ACTION_GET_SYSTEM_DATE_AND_TIME: {
            struct system_date_time dt;
            if (onvif_device_get_system_date_time(&dt) == 0) {
                char dt_xml[ONVIF_XML_BUFFER_SIZE];
                snprintf(dt_xml, sizeof(dt_xml),
                    "<tds:SystemDateAndTime>\n"
                    "  <tt:DateTimeType>%s</tt:DateTimeType>\n"
                    "  <tt:DaylightSavings>%s</tt:DaylightSavings>\n"
                    "  <tt:TimeZone>\n"
                    "    <tt:TZ xmlns:tt=\"http://www.onvif.org/ver10/schema\">%s%02d:%02d</tt:TZ>\n"
                    "  </tt:TimeZone>\n"
                    "  <tt:UTCDateTime>\n"
                    "    <tt:Time xmlns:tt=\"http://www.onvif.org/ver10/schema\">\n"
                    "      <tt:Hour>%d</tt:Hour>\n"
                    "      <tt:Minute>%d</tt:Minute>\n"
                    "      <tt:Second>%d</tt:Second>\n"
                    "    </tt:Time>\n"
                    "    <tt:Date xmlns:tt=\"http://www.onvif.org/ver10/schema\">\n"
                    "      <tt:Year>%d</tt:Year>\n"
                    "      <tt:Month>%d</tt:Month>\n"
                    "      <tt:Day>%d</tt:Day>\n"
                    "    </tt:Date>\n"
                    "  </tt:UTCDateTime>\n"
                    "  <tt:LocalDateTime>\n"
                    "    <tt:Time xmlns:tt=\"http://www.onvif.org/ver10/schema\">\n"
                    "      <tt:Hour>%d</tt:Hour>\n"
                    "      <tt:Minute>%d</tt:Minute>\n"
                    "      <tt:Second>%d</tt:Second>\n"
                    "    </tt:Time>\n"
                    "    <tt:Date xmlns:tt=\"http://www.onvif.org/ver10/schema\">\n"
                    "      <tt:Year>%d</tt:Year>\n"
                    "      <tt:Month>%d</tt:Month>\n"
                    "      <tt:Day>%d</tt:Day>\n"
                    "    </tt:Date>\n"
                    "  </tt:LocalDateTime>\n"
                    "</tds:SystemDateAndTime>",
                    dt.date_time_type ? "NTP" : "Manual",
                    dt.daylight_savings ? "true" : "false",
                    dt.time_zone.tz_hour >= 0 ? "+" : "",
                    dt.time_zone.tz_hour, dt.time_zone.tz_minute,
                    dt.utc_date_time.hour, dt.utc_date_time.minute, dt.utc_date_time.second,
                    dt.utc_date_time.year, dt.utc_date_time.month, dt.utc_date_time.day,
                    dt.local_date_time.hour, dt.local_date_time.minute, dt.local_date_time.second,
                    dt.local_date_time.year, dt.local_date_time.month, dt.local_date_time.day);
                
                soap_success_response(response->body, ONVIF_RESPONSE_BUFFER_SIZE, "GetSystemDateAndTime", dt_xml);
                response->body_length = strlen(response->body);
            } else {
                soap_fault_response(response->body, ONVIF_RESPONSE_BUFFER_SIZE, SOAP_FAULT_RECEIVER, "Failed to get system date and time");
                response->body_length = strlen(response->body);
            }
            break;
        }
        
        case ONVIF_ACTION_GET_SERVICES: {
            struct device_service services[10];
            int count = 0;
            if (onvif_device_get_services(services, &count) == 0) {
                char services_xml[2048];
                strcpy(services_xml, "<tds:Service>\n");
                
                for (int i = 0; i < count; i++) {
                    char service_xml[512];
                    snprintf(service_xml, sizeof(service_xml),
                        "  <tt:Namespace>%s</tt:Namespace>\n"
                        "  <tt:XAddr>%s</tt:XAddr>\n"
                        "  <tt:Version>\n"
                        "    <tt:Major>%d</tt:Major>\n"
                        "    <tt:Minor>%d</tt:Minor>\n"
                        "  </tt:Version>\n",
                        services[i].namespace, services[i].xaddr,
                        services[i].version.major, services[i].version.minor);
                    strcat(services_xml, service_xml);
                }
                strcat(services_xml, "</tds:Service>");
                
                soap_success_response(response->body, ONVIF_RESPONSE_BUFFER_SIZE, "GetServices", services_xml);
                response->body_length = strlen(response->body);
            } else {
                soap_fault_response(response->body, ONVIF_RESPONSE_BUFFER_SIZE, SOAP_FAULT_RECEIVER, "Failed to get services");
                response->body_length = strlen(response->body);
            }
            break;
        }
        
        default:
            response_buffer_set_body_printf(&resp_buf,
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                "  <soap:Body>\n"
                "    <soap:Fault>\n"
                "      <soap:Code>\n"
                "        <soap:Value>soap:Receiver</soap:Value>\n"
                "      </soap:Code>\n"
                "      <soap:Reason>\n"
                "        <soap:Text>Unsupported action</soap:Text>\n"
                "      </soap:Reason>\n"
                "    </soap:Fault>\n"
                "  </soap:Body>\n"
                "</soap:Envelope>");
            break;
    }
    
    // Response buffer will be automatically cleaned up when it goes out of scope
    return response->body_length;
}
