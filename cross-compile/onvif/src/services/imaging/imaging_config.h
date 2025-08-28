#ifndef IMAGING_CONFIG_H
#define IMAGING_CONFIG_H

/* Forward declarations */
struct auto_daynight_config;
struct imaging_settings;

/* Function declarations */
int imaging_config_load_defaults(struct imaging_settings *settings);
int imaging_config_save(const struct imaging_settings *settings);
int imaging_config_load(struct imaging_settings *settings);
int imaging_config_load_auto(const char *path, struct auto_daynight_config *cfg);
int imaging_config_save_auto(const char *path, const struct auto_daynight_config *cfg);

#endif /* IMAGING_CONFIG_H */
