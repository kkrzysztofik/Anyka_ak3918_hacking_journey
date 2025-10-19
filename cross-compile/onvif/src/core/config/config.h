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

/* Common buffer size constants */
#define CONFIG_STRING_SHORT_LEN  32 /* Short string buffers */
#define CONFIG_STRING_MEDIUM_LEN 64 /* Medium string buffers */

/* Network port constants */
#define HTTP_PORT_DEFAULT 8080 /* Default HTTP port for ONVIF services */

/* Forward declarations for imaging structures */
struct imaging_settings;
struct auto_daynight_config;
struct network_settings;
struct device_info;
struct ptz_preset_profile;

/* Core ONVIF daemon settings */
struct onvif_settings {
  int enabled;      /* daemon enable flag */
  int http_port;    /* HTTP/SOAP port */
  int auth_enabled; /* authentication enable flag */
};

/* Network settings for ONVIF services */
struct network_settings {
  int rtsp_port;         /* RTSP server port */
  int snapshot_port;     /* Snapshot service port */
  int ws_discovery_port; /* WS-Discovery port */
};

/* Device information for ONVIF identification */
struct device_info {
  char manufacturer[CONFIG_STRING_MEDIUM_LEN];    /* Device manufacturer name */
  char model[CONFIG_STRING_MEDIUM_LEN];           /* Device model name */
  char firmware_version[CONFIG_STRING_SHORT_LEN]; /* Firmware version string */
  char serial_number[CONFIG_STRING_MEDIUM_LEN];   /* Device serial number */
  char hardware_id[CONFIG_STRING_MEDIUM_LEN];     /* Hardware identification */
};

/* Logging configuration */
struct logging_settings {
  int enabled;                       /* Enable/disable logging */
  int use_colors;                    /* Enable/disable color output */
  int use_timestamps;                /* Enable/disable timestamps */
  int min_level;                     /* Minimum log level (0=ERROR, 1=WARNING, 2=NOTICE, 3=INFO,
                                        4=DEBUG) */
  char tag[CONFIG_STRING_SHORT_LEN]; /* Log tag identifier */
  int http_verbose;                  /* Enable full HTTP/SOAP request/response body logging */
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

/* Snapshot service configuration (Service Integration - T086) */
struct snapshot_settings {
  int width;                            /* Snapshot image width (160-2048 pixels, default 640) */
  int height;                           /* Snapshot image height (120-2048 pixels, default 480) */
  int quality;                          /* JPEG quality (1-100, default 85) */
  char format[CONFIG_STRING_SHORT_LEN]; /* Image format (default "jpeg") */
};

/* User credentials for ONVIF authentication (User Story 5) */
#define MAX_USERS                8
#define MAX_USERNAME_LENGTH      32
#define MAX_PASSWORD_HASH_LENGTH 128 /* Salted SHA256: salt$hash (32+1+64 hex chars) */

struct user_credential {
  char username[MAX_USERNAME_LENGTH + 1];           /* Username (3-32 alphanumeric chars) */
  char password_hash[MAX_PASSWORD_HASH_LENGTH + 1]; /* Salted SHA256: salt$hash format */
  int active;                                       /* Is this user slot active? */
};

/* PTZ Preset Profile Storage - 4 presets max per profile */
struct ptz_preset_profile {
  int preset_count;                             /* Number of active presets (0-4) */
  char preset1_token[CONFIG_STRING_MEDIUM_LEN]; /* Preset 1 token */
  char preset1_name[CONFIG_STRING_MEDIUM_LEN];  /* Preset 1 name */
  float preset1_pan;                            /* Preset 1 pan position (-180 to 180) */
  float preset1_tilt;                           /* Preset 1 tilt position (-90 to 90) */
  float preset1_zoom;                           /* Preset 1 zoom level (0 to 1) */
  char preset2_token[CONFIG_STRING_MEDIUM_LEN]; /* Preset 2 token */
  char preset2_name[CONFIG_STRING_MEDIUM_LEN];  /* Preset 2 name */
  float preset2_pan;                            /* Preset 2 pan position (-180 to 180) */
  float preset2_tilt;                           /* Preset 2 tilt position (-90 to 90) */
  float preset2_zoom;                           /* Preset 2 zoom level (0 to 1) */
  char preset3_token[CONFIG_STRING_MEDIUM_LEN]; /* Preset 3 token */
  char preset3_name[CONFIG_STRING_MEDIUM_LEN];  /* Preset 3 name */
  float preset3_pan;                            /* Preset 3 pan position (-180 to 180) */
  float preset3_tilt;                           /* Preset 3 tilt position (-90 to 90) */
  float preset3_zoom;                           /* Preset 3 zoom level (0 to 1) */
  char preset4_token[CONFIG_STRING_MEDIUM_LEN]; /* Preset 4 token */
  char preset4_name[CONFIG_STRING_MEDIUM_LEN];  /* Preset 4 name */
  float preset4_pan;                            /* Preset 4 pan position (-180 to 180) */
  float preset4_tilt;                           /* Preset 4 tilt position (-90 to 90) */
  float preset4_zoom;                           /* Preset 4 zoom level (0 to 1) */
};

/* Full application configuration */
struct application_config {
  struct onvif_settings onvif;                     /* core ONVIF settings */
  struct imaging_settings* imaging;                /* imaging tuning */
  struct auto_daynight_config* auto_daynight;      /* day/night auto thresholds */
  struct network_settings* network;                /* network service settings */
  struct device_info* device;                      /* device identification info */
  struct logging_settings* logging;                /* logging configuration */
  struct server_settings* server;                  /* HTTP server configuration */
  struct snapshot_settings* snapshot;              /* snapshot service configuration (T086) */
  video_config_t* main_stream;                     /* main stream (vs0) configuration */
  video_config_t* sub_stream;                      /* sub stream (vs1) configuration */
  video_config_t* stream_profile_1;                /* stream profile 1 configuration (User Story 4) */
  video_config_t* stream_profile_2;                /* stream profile 2 configuration (User Story 4) */
  video_config_t* stream_profile_3;                /* stream profile 3 configuration (User Story 4) */
  video_config_t* stream_profile_4;                /* stream profile 4 configuration (User Story 4) */
  struct ptz_preset_profile* ptz_preset_profile_1; /* PTZ preset profile 1 storage */
  struct ptz_preset_profile* ptz_preset_profile_2; /* PTZ preset profile 2 storage */
  struct ptz_preset_profile* ptz_preset_profile_3; /* PTZ preset profile 3 storage */
  struct ptz_preset_profile* ptz_preset_profile_4; /* PTZ preset profile 4 storage */
  struct user_credential users[MAX_USERS];         /* user credentials array (User Story 5) */
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
  CONFIG_SECTION_PTZ_PRESET_PROFILE_1,
  CONFIG_SECTION_PTZ_PRESET_PROFILE_2,
  CONFIG_SECTION_PTZ_PRESET_PROFILE_3,
  CONFIG_SECTION_PTZ_PRESET_PROFILE_4,
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
typedef enum { CONFIG_TYPE_INT, CONFIG_TYPE_STRING, CONFIG_TYPE_BOOL, CONFIG_TYPE_FLOAT } config_value_type_t;

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

#endif /* ONVIF_CONFIG_H */
