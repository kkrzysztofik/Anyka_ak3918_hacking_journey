#ifndef ONVIF_CONFIG_H
#define ONVIF_CONFIG_H

struct onvif_config {
    int enabled;
    int http_port;
    char username[64];
    char password[64];
};

int onvif_config_load(struct onvif_config *cfg, const char *config_file);

#endif
