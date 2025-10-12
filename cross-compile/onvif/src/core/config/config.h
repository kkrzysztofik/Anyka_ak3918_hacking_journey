/**
 * @file config.h
 * @brief Configuration management system with validation
 * @author kkrzysztofik
 * @date 2025

 */

#ifndef ONVIF_CONFIG_H
#define ONVIF_CONFIG_H

#include <stddef.h>

#include "services/common/video_config_types.h"

/* Forward declarations for imaging structures */
struct imaging_settings;
struct auto_daynight_config;
struct network_settings;
struct device_info;

/* Core ONVIF daemon settings */
struct onvif_settings {
  int enabled;       /* daemon enable flag */
  int http_port;     /* HTTP/SOAP port */
  int auth_enabled;  /* authentication enable flag */
  char username[64]; /* auth user (optional) */
  char password[64]; /* auth password (optional) */
};

/* Network settings for ONVIF services */
struct network_settings {
  int rtsp_port;         /* RTSP server port */
  int snapshot_port;     /* Snapshot service port */
  int ws_discovery_port; /* WS-Discovery port */
};

/* Device information for ONVIF identification */
struct device_info {
  char manufacturer[64];     /* Device manufacturer name */
  char model[64];            /* Device model name */
  char firmware_version[32]; /* Firmware version string */
  char serial_number[64];    /* Device serial number */
  char hardware_id[64];      /* Hardware identification */
};

/* Logging configuration */
struct logging_settings {
  int enabled;        /* Enable/disable logging */
  int use_colors;     /* Enable/disable color output */
  int use_timestamps; /* Enable/disable timestamps */
  int min_level;      /* Minimum log level (0=ERROR, 1=WARNING, 2=NOTICE, 3=INFO,
                         4=DEBUG) */
  char tag[32];       /* Log tag identifier */
  int http_verbose;   /* Enable full HTTP/SOAP request/response body logging */
};

/* HTTP server configuration */
struct server_settings {
  int worker_threads;     /* Number of worker threads (1-32) */
  int max_connections;    /* Maximum concurrent connections (1-1000) */
  int connection_timeout; /* Connection timeout in seconds (5-300) */
  int keepalive_timeout;  /* Keep-alive timeout in seconds (1-60) */
  int epoll_timeout;      /* Epoll event timeout in milliseconds (100-5000) */
  int cleanup_interval;   /* Periodic cleanup interval in seconds (1-60) */
};

/* User credentials for ONVIF authentication (User Story 5) */
#define MAX_USERS 8
#define MAX_USERNAME_LENGTH 32
#define MAX_PASSWORD_HASH_LENGTH 128  /* Salted SHA256: salt$hash (32+1+64 hex chars) */

struct user_credential {
  char username[MAX_USERNAME_LENGTH + 1]; /* Username (3-32 alphanumeric chars) */
  char password_hash[MAX_PASSWORD_HASH_LENGTH + 1]; /* Salted SHA256: salt$hash format */
  int active; /* Is this user slot active? */
};

/* Full application configuration */
struct application_config {
  struct onvif_settings onvif;                /* core ONVIF settings */
  struct imaging_settings* imaging;           /* imaging tuning */
  struct auto_daynight_config* auto_daynight; /* day/night auto thresholds */
  struct network_settings* network;           /* network service settings */
  struct device_info* device;                 /* device identification info */
  struct logging_settings* logging;           /* logging configuration */
  struct server_settings* server;             /* HTTP server configuration */
  video_config_t* main_stream;                /* main stream (vs0) configuration */
  video_config_t* sub_stream;                 /* sub stream (vs1) configuration */
  video_config_t* stream_profile_1;           /* stream profile 1 configuration (User Story 4) */
  video_config_t* stream_profile_2;           /* stream profile 2 configuration (User Story 4) */
  video_config_t* stream_profile_3;           /* stream profile 3 configuration (User Story 4) */
  video_config_t* stream_profile_4;           /* stream profile 4 configuration (User Story 4) */
  struct user_credential users[MAX_USERS];    /* user credentials array (User Story 5) */
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
  CONFIG_SECTION_DEVICE,
  CONFIG_SECTION_LOGGING,
  CONFIG_SECTION_SERVER,
  CONFIG_SECTION_MAIN_STREAM,
  CONFIG_SECTION_SUB_STREAM,
  CONFIG_SECTION_MEDIA,
  CONFIG_SECTION_PTZ,
  CONFIG_SECTION_SNAPSHOT,
  CONFIG_SECTION_STREAM_PROFILE_1,
  CONFIG_SECTION_STREAM_PROFILE_2,
  CONFIG_SECTION_STREAM_PROFILE_3,
  CONFIG_SECTION_STREAM_PROFILE_4,
  CONFIG_SECTION_USER_1,
  CONFIG_SECTION_USER_2,
  CONFIG_SECTION_USER_3,
  CONFIG_SECTION_USER_4,
  CONFIG_SECTION_USER_5,
  CONFIG_SECTION_USER_6,
  CONFIG_SECTION_USER_7,
  CONFIG_SECTION_USER_8
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
  const char* key;
  config_value_type_t type;
  void* value_ptr;
  size_t value_size;
  int min_value;
  int max_value;
  const char* default_value;
  int required;
} config_parameter_t;

/**
 * @brief Configuration section definition
 */
typedef struct {
  config_section_t section;
  const char* section_name;
  config_parameter_t* parameters;
  size_t parameter_count;
} config_section_def_t;

/**
 * @brief Configuration manager
 */
typedef struct {
  struct application_config* app_config;
  config_section_def_t* sections;
  size_t section_count;
  int validation_enabled;
} config_manager_t;

/**
 * @brief Initialize configuration system
 * @param config Configuration manager to initialize
 * @param app_config Application configuration structure
 * @return 0 on success, negative error code on failure
 */
int config_init(config_manager_t* config, struct application_config* app_config);

/**
 * @brief Load configuration from file with validation
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return 0 on success, negative error code on failure
 */
int config_load(config_manager_t* config, const char* config_file);

/**
 * @brief Validate configuration values
 * @param config Configuration manager
 * @return CONFIG_VALIDATION_OK on success, validation error code on failure
 */
config_validation_result_t config_validate(config_manager_t* config);

/**
 * @brief Get configuration value with type safety
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to store the value
 * @param value_type Expected value type
 * @return 0 on success, negative error code on failure
 */
int config_get_value(config_manager_t* config, config_section_t section, const char* key,
                     void* value_ptr, config_value_type_t value_type);

/**
 * @brief Set configuration value with validation
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to the value
 * @param value_type Value type
 * @return 0 on success, negative error code on failure
 */
int config_set_value(config_manager_t* config, config_section_t section, const char* key,
                     const void* value_ptr, config_value_type_t value_type);

/**
 * @brief Reset configuration to defaults
 * @param config Configuration manager
 * @return 0 on success, negative error code on failure
 */
int config_reset_to_defaults(config_manager_t* config);

/**
 * @brief Get configuration parameter definition
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @return Parameter definition or NULL if not found
 */
const config_parameter_t* config_get_parameter(config_manager_t* config, config_section_t section,
                                               const char* key);

/**
 * @brief Clean up configuration manager
 * @param config Configuration manager to clean up
 */
void config_cleanup(config_manager_t* config);

/**
 * @brief Get configuration summary for logging
 * @param config Configuration manager
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return 0 on success, negative error code on failure
 */
int config_get_summary(config_manager_t* config, char* summary, size_t summary_size);

#endif /* ONVIF_CONFIG_H */
