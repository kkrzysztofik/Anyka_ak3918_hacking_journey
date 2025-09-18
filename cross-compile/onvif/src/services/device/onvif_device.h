/**
 * @file onvif_device.h
 * @brief ONVIF Device service implementation
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef ONVIF_DEVICE_H
#define ONVIF_DEVICE_H

#include "core/config/config.h"
#include "services/common/onvif_request.h"
#include "services/common/onvif_types.h"

/**
 * @brief Device identification information - defined in config.h
 */

/**
 * @brief Device service capabilities
 */
struct device_capabilities {
    int has_analytics;    /**< Analytics service available */
    int has_device;       /**< Device service available */
    int has_events;       /**< Event service available */
    int has_imaging;      /**< Imaging service available */
    int has_media;        /**< Media service available */
    int has_ptz;          /**< PTZ service available */
};

struct system_date_time {
    int date_time_type;
    int daylight_savings;
    struct {
        int tz_hour;
        int tz_minute;
    } time_zone;
    struct {
        int year;
        int month;
        int day;
        int hour;
        int minute;
        int second;
    } utc_date_time;
    struct {
        int year;
        int month;
        int day;
        int hour;
        int minute;
        int second;
    } local_date_time;
};

struct dns_information {
    int from_dhcp;
    int dns_manual_count;
    char search_domain[256];
};

struct network_interface {
    int enabled;
    char name[32];
    char hw_address[18];
    int mtu;
};

struct network_protocol {
    int enabled;
    char name[16];
    int port_count;
    int ports[8];
};

struct device_service {
    char namespace[128];
    char xaddr[256];
    struct {
        int major;
        int minor;
    } version;
};

int onvif_device_get_device_information(struct device_info *info); /**< Populate device identification fields. */
int onvif_device_get_capabilities(struct device_capabilities *caps); /**< Report supported services. */
int onvif_device_get_system_date_time(struct system_date_time *dt); /**< Retrieve system time. */
int onvif_device_set_system_date_time(const struct system_date_time *dt); /**< Set system time. */
int onvif_device_system_reboot(void); /**< Reboot the system. */
int onvif_device_get_dns(struct dns_information *dns); /**< Get DNS settings. */
int onvif_device_get_network_interfaces(struct network_interface *interfaces, int *count); /**< Enumerate NICs. */
int onvif_device_get_network_protocols(struct network_protocol *protocols, int *count); /**< Enumerate protocols & ports. */
int onvif_device_get_services(struct device_service *services, int *count); /**< List device services and versions. */
/* Device service functions */

/**
 * @brief Initialize device service
 * @param config Centralized configuration
 * @return 0 on success, negative error code on failure
 */
int onvif_device_init(config_manager_t *config);

/**
 * @brief Handle ONVIF device service requests
 * @param action Action type
 * @param request Request structure
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int onvif_device_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response);

/**
 * @brief Clean up device service
 */
void onvif_device_cleanup(void);

#endif
