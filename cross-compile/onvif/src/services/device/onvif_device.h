#ifndef ONVIF_DEVICE_H
#define ONVIF_DEVICE_H

struct device_info {
    char manufacturer[64];
    char model[64];
    char firmware_version[32];
    char serial_number[64];
    char hardware_id[64];
};

struct device_capabilities {
    int has_analytics;
    int has_device;
    int has_events;
    int has_imaging;
    int has_media;
    int has_ptz;
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

int onvif_device_get_device_information(struct device_info *info);
int onvif_device_get_capabilities(struct device_capabilities *caps);
int onvif_device_get_system_date_time(struct system_date_time *dt);
int onvif_device_set_system_date_time(const struct system_date_time *dt);
int onvif_device_get_dns(struct dns_information *dns);
int onvif_device_get_network_interfaces(struct network_interface *interfaces, int *count);
int onvif_device_get_network_protocols(struct network_protocol *protocols, int *count);
int onvif_device_get_services(struct device_service *services, int *count);

#endif
