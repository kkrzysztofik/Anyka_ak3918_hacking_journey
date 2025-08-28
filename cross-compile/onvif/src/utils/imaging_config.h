#ifndef IMAGING_CONFIG_H
#define IMAGING_CONFIG_H

struct imaging_config {
    int brightness;
    int contrast;
    int saturation;
    int sharpness;
};

/* Forward declaration - use the one from onvif_imaging.h */
struct auto_daynight_config;
struct imaging_settings;

int imaging_config_load(const char *path, struct imaging_config *cfg);
int imaging_config_save(const char *path, const struct imaging_config *cfg);
int imaging_config_load_defaults(struct imaging_settings *cfg);
int imaging_config_load_auto(const char *path, struct auto_daynight_config *cfg);
int imaging_config_save_auto(const char *path, const struct auto_daynight_config *cfg);

#endif