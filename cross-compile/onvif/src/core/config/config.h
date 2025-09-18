/**
 * @file config.h
 * @brief Configuration management system with validation
 * 
 * This module provides a unified configuration management system that eliminates
 * hardcoded values and provides consistent default handling and validation.
 */

#ifndef ONVIF_CONFIG_H
#define ONVIF_CONFIG_H

#include <stddef.h>

#include "common/onvif_types.h"
#include "common/onvif_constants.h"

/* Forward declarations for imaging structures */
struct imaging_settings;
struct auto_daynight_config;
struct network_settings;
struct device_info;

/* Core ONVIF daemon settings */
struct onvif_settings {
    int enabled;               /* daemon enable flag */
    int http_port;             /* HTTP/SOAP port */
    char username[64];         /* auth user (optional) */
    char password[64];         /* auth password (optional) */
};

/* Network settings for ONVIF services */
struct network_settings {
    int rtsp_port;             /* RTSP server port */
    int snapshot_port;         /* Snapshot service port */
    int ws_discovery_port;     /* WS-Discovery port */
};

/* Device information for ONVIF identification */
struct device_info {
    char manufacturer[64];     /* Device manufacturer name */
    char model[64];            /* Device model name */
    char firmware_version[32]; /* Firmware version string */
    char serial_number[64];    /* Device serial number */
    char hardware_id[64];      /* Hardware identification */
};

/* Full application configuration */
struct application_config {
    struct onvif_settings onvif;                 /* core ONVIF settings */
    struct imaging_settings *imaging;            /* imaging tuning */
    struct auto_daynight_config *auto_daynight;  /* day/night auto thresholds */
    struct network_settings *network;            /* network service settings */
    struct device_info *device;                  /* device identification info */
};

/**
 * @brief Configuration validation result
 */
typedef enum {
  CONFIG_VALIDATION_OK,
  CONFIG_VALIDATION_INVALID_VALUE,
  CONFIG_VALIDATION_OUT_OF_RANGE,
  CONFIG_VALIDATION_MISSING_REQUIRED,
  CONFIG_VALIDATION_INVALID_FORMAT
} config_validation_result_t;

/**
 * @brief Configuration section types
 */
typedef enum {
  CONFIG_SECTION_ONVIF,
  CONFIG_SECTION_IMAGING,
  CONFIG_SECTION_AUTO_DAYNIGHT,
  CONFIG_SECTION_NETWORK,
  CONFIG_SECTION_RTSP,
  CONFIG_SECTION_DEVICE
} config_section_t;

/**
 * @brief Configuration value types
 */
typedef enum {
  CONFIG_TYPE_INT,
  CONFIG_TYPE_STRING,
  CONFIG_TYPE_BOOL,
  CONFIG_TYPE_FLOAT
} config_value_type_t;

/**
 * @brief Configuration parameter definition
 */
typedef struct {
  const char *key;
  config_value_type_t type;
  void *value_ptr;
  size_t value_size;
  int min_value;
  int max_value;
  const char *default_value;
  int required;
} config_parameter_t;

/**
 * @brief Configuration section definition
 */
typedef struct {
  config_section_t section;
  const char *section_name;
  config_parameter_t *parameters;
  size_t parameter_count;
} config_section_def_t;

/**
 * @brief Configuration manager
 */
typedef struct {
  struct application_config *app_config;
  config_section_def_t *sections;
  size_t section_count;
  int validation_enabled;
} config_manager_t;

/**
 * @brief Initialize configuration system
 * @param config Configuration manager to initialize
 * @param app_config Application configuration structure
 * @return 0 on success, negative error code on failure
 */
int config_init(config_manager_t *config, struct application_config *app_config);

/**
 * @brief Load configuration from file with validation
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return 0 on success, negative error code on failure
 */
int config_load(config_manager_t *config, const char *config_file);


/**
 * @brief Validate configuration values
 * @param config Configuration manager
 * @return CONFIG_VALIDATION_OK on success, validation error code on failure
 */
config_validation_result_t config_validate(config_manager_t *config);

/**
 * @brief Get configuration value with type safety
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to store the value
 * @param value_type Expected value type
 * @return 0 on success, negative error code on failure
 */
int config_get_value(config_manager_t *config, config_section_t section,
                                const char *key, void *value_ptr, config_value_type_t value_type);

/**
 * @brief Set configuration value with validation
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to the value
 * @param value_type Value type
 * @return 0 on success, negative error code on failure
 */
int config_set_value(config_manager_t *config, config_section_t section,
                                const char *key, const void *value_ptr, config_value_type_t value_type);

/**
 * @brief Reset configuration to defaults
 * @param config Configuration manager
 * @return 0 on success, negative error code on failure
 */
int config_reset_to_defaults(config_manager_t *config);

/**
 * @brief Get configuration parameter definition
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @return Parameter definition or NULL if not found
 */
const config_parameter_t *config_get_parameter(config_manager_t *config,
                                                          config_section_t section, const char *key);

/**
 * @brief Clean up configuration manager
 * @param config Configuration manager to clean up
 */
void config_cleanup(config_manager_t *config);


/**
 * @brief Get configuration summary for logging
 * @param config Configuration manager
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return 0 on success, negative error code on failure
 */
int config_get_summary(config_manager_t *config, char *summary, size_t summary_size);

#endif /* ONVIF_CONFIG_H */
