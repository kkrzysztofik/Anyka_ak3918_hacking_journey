/**
 * @file onvif_device.h
 * @brief ONVIF Device service implementation
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef ONVIF_DEVICE_H
#define ONVIF_DEVICE_H

#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "services/common/onvif_types.h"

/**
 * @brief Device identification information - defined in config.h
 */

/**
 * @brief Device service capabilities
 */
struct device_capabilities {
  int has_analytics; /**< Analytics service available */
  int has_device;    /**< Device service available */
  int has_events;    /**< Event service available */
  int has_imaging;   /**< Imaging service available */
  int has_media;     /**< Media service available */
  int has_ptz;       /**< PTZ service available */
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

int onvif_device_system_reboot(void); /**< Reboot the system. */
/* Device service functions */

/**
 * @brief Initialize device service
 * @return 0 on success, negative error code on failure
 * @note Uses config_runtime API for configuration access
 */
int onvif_device_init(void);

/**
 * @brief Handle ONVIF device service requests by operation name
 * @param operation_name ONVIF operation name (e.g., "GetDeviceInformation")
 * @param request HTTP request structure
 * @param response Response structure
 * @return 0 on success, negative error code on failure
 */
int onvif_device_handle_operation(const char* operation_name, const http_request_t* request,
                                  http_response_t* response);

/**
 * @brief Clean up device service
 * @note Best effort cleanup - always succeeds
 */
void onvif_device_cleanup(void);

/* Utility functions */
const char* onvif_service_type_to_string(onvif_service_type_t service);
/* onvif_action_type_to_string removed - using string names directly */

#endif
