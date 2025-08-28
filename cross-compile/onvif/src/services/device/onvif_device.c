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
#include "onvif_device.h"
#include "config.h"

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
    if (!info) return -1;
    
    strncpy(info->manufacturer, DEVICE_MANUFACTURER, sizeof(info->manufacturer) - 1);
    strncpy(info->model, DEVICE_MODEL, sizeof(info->model) - 1);
    strncpy(info->firmware_version, DEVICE_FIRMWARE_VER, sizeof(info->firmware_version) - 1);
    strncpy(info->serial_number, DEVICE_SERIAL, sizeof(info->serial_number) - 1);
    strncpy(info->hardware_id, DEVICE_HARDWARE_ID, sizeof(info->hardware_id) - 1);
    
    return 0;
}

int onvif_device_get_capabilities(struct device_capabilities *caps) {
    if (!caps) return -1;
    
    *caps = dev_caps;
    return 0;
}

int onvif_device_get_system_date_time(struct system_date_time *dt) {
    if (!dt) return -1;
    
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
    
    return 0;
}

int onvif_device_set_system_date_time(const struct system_date_time *dt) {
    if (!dt) return -1;
    
    /* TODO: Implement system time setting if needed */
    printf("SetSystemDateTime requested (not implemented)\n");
    return 0;
}

int onvif_device_system_reboot(void) {
    /* TODO: Implement system reboot */
    printf("SystemReboot requested (not implemented)\n");
    return 0;
}

int onvif_device_get_hostname(char *hostname, size_t hostname_len) {
    if (!hostname || hostname_len == 0) return -1;
    
    struct utsname uts;
    if (uname(&uts) == 0) {
        strncpy(hostname, uts.nodename, hostname_len - 1);
        hostname[hostname_len - 1] = '\0';
        return 0;
    }
    
    /* Fallback hostname */
    strncpy(hostname, "anyka-camera", hostname_len - 1);
    hostname[hostname_len - 1] = '\0';
    return 0;
}

int onvif_device_set_hostname(const char *hostname) {
    if (!hostname) return -1;
    
    /* TODO: Implement hostname setting if needed */
    printf("SetHostname requested: %s (not implemented)\n", hostname);
    return 0;
}

int onvif_device_get_dns(struct dns_information *dns) {
    if (!dns) return -1;
    
    /* TODO: Read DNS settings from system or config */
    dns->from_dhcp = 1;
    dns->dns_manual_count = 0;
    dns->search_domain[0] = '\0';
    
    return 0;
}

int onvif_device_get_network_interfaces(struct network_interface *interfaces, int *count) {
    if (!interfaces || !count) return -1;
    
    /* TODO: Enumerate actual network interfaces */
    *count = 1;
    interfaces[0].enabled = 1;
    strncpy(interfaces[0].name, "eth0", sizeof(interfaces[0].name) - 1);
    strncpy(interfaces[0].hw_address, "00:11:22:33:44:55", sizeof(interfaces[0].hw_address) - 1);
    interfaces[0].mtu = 1500;
    
    return 0;
}

int onvif_device_get_network_protocols(struct network_protocol *protocols, int *count) {
    if (!protocols || !count) return -1;
    
    /* Common protocols supported */
    *count = 3;
    
    /* HTTP */
    protocols[0].enabled = 1;
    strncpy(protocols[0].name, "HTTP", sizeof(protocols[0].name) - 1);
    protocols[0].port_count = 1;
    protocols[0].ports[0] = 80;
    
    /* HTTPS */
    protocols[1].enabled = 0;
    strncpy(protocols[1].name, "HTTPS", sizeof(protocols[1].name) - 1);
    protocols[1].port_count = 1;
    protocols[1].ports[0] = 443;
    
    /* RTSP */
    protocols[2].enabled = 1;
    strncpy(protocols[2].name, "RTSP", sizeof(protocols[2].name) - 1);
    protocols[2].port_count = 1;
    protocols[2].ports[0] = 554;
    
    return 0;
}

int onvif_device_get_services(struct device_service *services, int *count) {
    if (!services || !count) return -1;
    
    *count = 3;
    
    /* Device Service */
    strncpy(services[0].namespace, "http://www.onvif.org/ver10/device/wsdl", sizeof(services[0].namespace) - 1);
    strncpy(services[0].xaddr, "http://[IP]:8080/onvif/device_service", sizeof(services[0].xaddr) - 1);
    services[0].version.major = 2;
    services[0].version.minor = 5;
    
    /* Media Service */
    strncpy(services[1].namespace, "http://www.onvif.org/ver10/media/wsdl", sizeof(services[1].namespace) - 1);
    strncpy(services[1].xaddr, "http://[IP]:8080/onvif/media_service", sizeof(services[1].xaddr) - 1);
    services[1].version.major = 2;
    services[1].version.minor = 5;
    
    /* PTZ Service */
    strncpy(services[2].namespace, "http://www.onvif.org/ver20/ptz/wsdl", sizeof(services[2].namespace) - 1);
    strncpy(services[2].xaddr, "http://[IP]:8080/onvif/ptz_service", sizeof(services[2].xaddr) - 1);
    services[2].version.major = 2;
    services[2].version.minor = 5;
    
    return 0;
}
