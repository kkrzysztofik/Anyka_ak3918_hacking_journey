/**
 * @file config.h
 * @brief Load/save application configuration (core ONVIF + imaging + day/night).
 *
 * The configuration is stored in a simple INI-like file specified by
 * ONVIF_CONFIG_FILE. This module provides structured access and a
 * singleton accessor after initial load.
 */
#ifndef APPLICATION_CONFIG_H
#define APPLICATION_CONFIG_H

/* Forward declarations for imaging structures */
#include "../services/imaging/onvif_imaging.h" /* brings imaging_settings & auto_daynight_config */

/* Core ONVIF daemon settings */
struct onvif_settings {
    int enabled;               /* daemon enable flag */
    int http_port;             /* HTTP/SOAP port */
    char username[64];         /* auth user (optional) */
    char password[64];         /* auth password (optional) */
};

/* Full application configuration */
struct application_config {
    struct onvif_settings onvif;                 /* core ONVIF settings */
    struct imaging_settings imaging;            /* imaging tuning */
    struct auto_daynight_config auto_daynight;  /* day/night auto thresholds */
};

/**
 * @brief Load configuration from file, supplying defaults if missing.
 * @param cfg Output structure to populate.
 * @param config_file Path to config file.
 * @return 0 on success, non-zero on error (defaults still applied).
 */
int config_load(struct application_config *cfg, const char *config_file);

/**
 * @brief Access the last-loaded configuration singleton (read-only).
 * @return Pointer to immutable configuration or NULL if not loaded.
 */
const struct application_config *config_get(void);

/**
 * @brief Persist imaging settings section back to config file.
 * @param settings Imaging settings to save.
 * @return 0 on success, negative on failure.
 */
int config_save_imaging(const struct imaging_settings *settings);
/**
 * @brief Persist auto day/night configuration back to file.
 * @param cfg Day/night thresholds configuration.
 * @return 0 on success, negative on failure.
 */
int config_save_auto_daynight(const struct auto_daynight_config *cfg);

#endif
