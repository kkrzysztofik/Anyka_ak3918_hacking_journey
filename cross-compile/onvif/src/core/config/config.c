/**
 * @file config.c
 * @brief Configuration management system implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "config.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "platform/platform.h"
#include "services/common/onvif_imaging_types.h"
#include "utils/error/error_handling.h"
#include "utils/string/string_shims.h"
#include "utils/validation/common_validation.h"

// Default configuration values
#define DEFAULT_HTTP_PORT     8080
#define DEFAULT_RTSP_PORT     554
#define DEFAULT_SNAPSHOT_PORT 3000
#define DEFAULT_USERNAME      "admin"
#define DEFAULT_PASSWORD      "admin"
#define DEFAULT_BRIGHTNESS    50
#define DEFAULT_CONTRAST      50
#define DEFAULT_SATURATION    50
#define DEFAULT_SHARPNESS     50
#define DEFAULT_HUE           0

// Configuration file parsing constants
#define MAX_LINE_LENGTH         512
#define MAX_SECTION_NAME_LENGTH 128
#define MAX_KEY_LENGTH          128
#define MAX_VALUE_LENGTH        256

// Configuration validation constants
#define MIN_USERNAME_LENGTH 1
#define MAX_USERNAME_LENGTH 32
#define MIN_PASSWORD_LENGTH 1
#define MAX_PASSWORD_LENGTH 32
#define MIN_PORT_VALUE      1
#define MAX_PORT_VALUE      65535

// Configuration parameter definitions
// Note: These arrays are modified at runtime in link_parameters_to_config()
// to point to actual configuration structure members
static const config_parameter_t onvif_parameters_init[] = {
  {"enabled", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 1},
  {"http_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "8080", 1},
  {"auth_enabled", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"username", CONFIG_TYPE_STRING, NULL, 64, 0, 0, DEFAULT_USERNAME, 1},
  {"password", CONFIG_TYPE_STRING, NULL, 64, 0, 0, DEFAULT_PASSWORD, 1}};

// Runtime copies that can be modified
static config_parameter_t onvif_parameters[5]; // NOLINT

static const config_parameter_t imaging_parameters_init[] = {
  {"brightness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"contrast", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"saturation", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"sharpness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"hue", CONFIG_TYPE_INT, NULL, sizeof(int), -180, 180, "0", 0}};

static config_parameter_t imaging_parameters[5]; // NOLINT

static const config_parameter_t auto_daynight_parameters_init[] = {
  {"auto_day_night_enable", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"day_night_mode", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 2, "2", 0},
  {"day_to_night_lum", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 10000, "6400", 0},
  {"night_to_day_lum", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 10000, "2048", 0},
  {"lock_time", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 3600000, "900000", 0}};

static config_parameter_t auto_daynight_parameters[5]; // NOLINT

static const config_parameter_t network_parameters_init[] = {
  {"rtsp_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "554", 0},
  {"snapshot_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "3000", 0},
  {"ws_discovery_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "3702", 0}};

static config_parameter_t network_parameters[3]; // NOLINT

static const config_parameter_t device_parameters_init[] = {
  {"manufacturer", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "Anyka", 0},
  {"model", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "AK3918 Camera", 0},
  {"firmware_version", CONFIG_TYPE_STRING, NULL, 32, 0, 0, "1.0.0", 0},
  {"serial_number", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "AK3918-001", 0},
  {"hardware_id", CONFIG_TYPE_STRING, NULL, 32, 0, 0, "1.0", 0}};

static config_parameter_t device_parameters[5]; // NOLINT

static const config_parameter_t logging_parameters_init[] = {
  {"enabled", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"use_colors", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"use_timestamps", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"min_level", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 4, "1", 0},
  {"tag", CONFIG_TYPE_STRING, NULL, 32, 0, 0, "ONVIF", 0},
  {"http_verbose", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0}};

static config_parameter_t logging_parameters[6]; // NOLINT

static const config_parameter_t server_parameters_init[] = {
  {"worker_threads", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 32, "8", 0},
  {"max_connections", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 1000, "100", 0},
  {"connection_timeout", CONFIG_TYPE_INT, NULL, sizeof(int), 5, 300, "30", 0},
  {"keepalive_timeout", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 60, "15", 0},
  {"epoll_timeout", CONFIG_TYPE_INT, NULL, sizeof(int), 100, 5000, "500", 0},
  {"cleanup_interval", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 60, "5", 0}};

static config_parameter_t server_parameters[6]; // NOLINT

static const config_parameter_t main_stream_parameters_init[] = {
  {"main_fps", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 60, "25", 0},
  {"main_kbps", CONFIG_TYPE_INT, NULL, sizeof(int), 100, 10000, "2048", 0}};

static config_parameter_t main_stream_parameters[2]; // NOLINT

static const config_parameter_t sub_stream_parameters_init[] = {
  {"sub_fps", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 60, "25", 0},
  {"sub_kbps", CONFIG_TYPE_INT, NULL, sizeof(int), 100, 5000, "800", 0}};

static config_parameter_t sub_stream_parameters[2]; // NOLINT

/**
 * @brief Initialize runtime parameter arrays from const init arrays
 * @note This function copies the const initialization data to runtime arrays
 */
static void initialize_parameter_arrays(void) {
  // Copy ONVIF parameters
  memcpy(onvif_parameters, onvif_parameters_init, sizeof(onvif_parameters));

  // Copy imaging parameters
  memcpy(imaging_parameters, imaging_parameters_init, sizeof(imaging_parameters));

  // Copy auto day/night parameters
  memcpy(auto_daynight_parameters, auto_daynight_parameters_init, sizeof(auto_daynight_parameters));

  // Copy network parameters
  memcpy(network_parameters, network_parameters_init, sizeof(network_parameters));

  // Copy device parameters
  memcpy(device_parameters, device_parameters_init, sizeof(device_parameters));

  // Copy logging parameters
  memcpy(logging_parameters, logging_parameters_init, sizeof(logging_parameters));

  // Copy server parameters
  memcpy(server_parameters, server_parameters_init, sizeof(server_parameters));

  // Copy main stream parameters
  memcpy(main_stream_parameters, main_stream_parameters_init, sizeof(main_stream_parameters));

  // Copy sub stream parameters
  memcpy(sub_stream_parameters, sub_stream_parameters_init, sizeof(sub_stream_parameters));
}

// Section definitions
static const config_section_def_t default_sections[] = {
  {CONFIG_SECTION_ONVIF, "onvif", onvif_parameters,
   sizeof(onvif_parameters) / sizeof(onvif_parameters[0])},
  {CONFIG_SECTION_IMAGING, "imaging", imaging_parameters,
   sizeof(imaging_parameters) / sizeof(imaging_parameters[0])},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "autoir", auto_daynight_parameters,
   sizeof(auto_daynight_parameters) / sizeof(auto_daynight_parameters[0])},
  {CONFIG_SECTION_NETWORK, "network", network_parameters,
   sizeof(network_parameters) / sizeof(network_parameters[0])},
  {CONFIG_SECTION_DEVICE, "device", device_parameters,
   sizeof(device_parameters) / sizeof(device_parameters[0])},
  {CONFIG_SECTION_LOGGING, "logging", logging_parameters,
   sizeof(logging_parameters) / sizeof(logging_parameters[0])},
  {CONFIG_SECTION_SERVER, "server", server_parameters,
   sizeof(server_parameters) / sizeof(server_parameters[0])},
  {CONFIG_SECTION_MAIN_STREAM, "onvif", main_stream_parameters,
   sizeof(main_stream_parameters) / sizeof(main_stream_parameters[0])},
  {CONFIG_SECTION_SUB_STREAM, "onvif", sub_stream_parameters,
   sizeof(sub_stream_parameters) / sizeof(sub_stream_parameters[0])}};

/**
 * @brief Case-insensitive string comparison for configuration keys
 * @param first_string First string to compare
 * @param second_string Second string to compare
 * @return 1 if strings are equal (case-insensitive), 0 otherwise
 * @note Uses strcasecmp from the SDK
 */
static int key_equals(const char* first_string, const char* second_string) {
  if (!first_string || !second_string) {
    platform_log_error("key_equals() called with NULL parameter (first_string=%p, "
                       "second_string=%p)",
                       first_string, second_string);
    return 0;
  }
  return strcasecmp(first_string, second_string) == 0;
}

/**
 * @brief Check if a section name is known/valid
 * @param section_name Section name string to check
 * @return 1 if section is known, 0 if unknown
 * @note Returns 0 if section_name is NULL
 */
static int is_known_section(const char* section_name) {
  if (!section_name) {
    return 0;
  }

  for (size_t i = 0; i < sizeof(default_sections) / sizeof(default_sections[0]); i++) {
    if (key_equals(section_name, default_sections[i].section_name)) {
      return 1;
    }
  }
  return 0;
}

/**
 * @brief Parse section name and return corresponding section type
 * @param section_name Section name string to parse
 * @return Corresponding config_section_t or CONFIG_SECTION_ONVIF as default
 * @note Returns default section if section_name is NULL or not found
 */
static config_section_t parse_section_name(const char* section_name) {
  if (!section_name) {
    platform_log_error("parse_section_name() called with NULL section_name parameter");
    return CONFIG_SECTION_ONVIF;
  }

  for (size_t i = 0; i < sizeof(default_sections) / sizeof(default_sections[0]); i++) {
    if (key_equals(section_name, default_sections[i].section_name)) {
      return default_sections[i].section;
    }
  }
  return CONFIG_SECTION_ONVIF; // Default
}

/**
 * @brief Find a configuration parameter by section and key
 * @param config Configuration manager
 * @param section Section to search in
 * @param key Parameter key to find
 * @return Pointer to parameter if found, NULL otherwise
 * @note Returns NULL if config is NULL or key is NULL
 */
static config_parameter_t* find_parameter(config_manager_t* config, config_section_t section,
                                          const char* key) {
  if (!config || !key) {
    platform_log_error("find_parameter() called with NULL parameter (config=%p, key=%p)", config,
                       key);
    return NULL;
  }

  for (size_t i = 0; i < config->section_count; i++) {
    if (config->sections[i].section == section) {
      for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
        if (key_equals(key, config->sections[i].parameters[j].key)) {
          return &config->sections[i].parameters[j];
        }
      }
    }
  }
  return NULL;
}

/**
 * @brief Parse integer value from string with error handling
 * @param value String value to parse
 * @param param Parameter for error reporting
 * @return Parsed integer value
 */
static long parse_integer_value(const char* value, const config_parameter_t* param) {
  char* end_ptr = NULL;
  long int_val = strtol(value, &end_ptr, 10);

  // Check for conversion errors
  if (end_ptr == value || *end_ptr != '\0' || int_val < INT_MIN || int_val > INT_MAX) {
    platform_log_warning("invalid integer value '%s' for parameter '%s', using default", value,
                         param->key ? param->key : "unknown");
    if (param->default_value) {
      int_val = strtol(param->default_value, &end_ptr, 10);
      if (end_ptr == param->default_value || *end_ptr != '\0') {
        int_val = 0; // Fallback to 0 if default is also invalid
      }
    } else {
      int_val = 0;
    }
  }

  return int_val;
}

/**
 * @brief Validate port number parameter
 * @param int_val Integer value to validate
 * @param param Parameter being validated
 * @return Validated integer value
 */
static long validate_port_parameter(long int_val, const config_parameter_t* param) {
  validation_result_t validation =
    validate_int(param->key, (int)int_val, MIN_PORT_VALUE, MAX_PORT_VALUE);
  if (!validation_is_valid(&validation)) {
    platform_log_warning("port validation failed for %s: %s, using default", param->key,
                         validation_get_error_message(&validation));
    // Use default value instead of failing
    if (param->default_value) {
      char* end_ptr = NULL;
      int_val = strtol(param->default_value, &end_ptr, 10);
      if (end_ptr == param->default_value || *end_ptr != '\0') {
        int_val = 0; // Fallback to 0 if default is also invalid
      }
    }
  }
  return int_val;
}

/**
 * @brief Validate integer parameter with range checking
 * @param int_val Integer value to validate
 * @param param Parameter being validated
 * @return Validated integer value
 */
static long validate_range_parameter(long int_val, const config_parameter_t* param) {
  validation_result_t validation =
    validate_int(param->key, (int)int_val, param->min_value, param->max_value);
  if (!validation_is_valid(&validation)) {
    platform_log_warning("integer validation failed for %s: %s, clamping to valid range",
                         param->key, validation_get_error_message(&validation));
    // Clamp to valid range instead of failing
    if (int_val < param->min_value) {
      int_val = param->min_value;
    }
    if (int_val > param->max_value) {
      int_val = param->max_value;
    }
  }
  return int_val;
}

/**
 * @brief Check if parameter is a port number
 * @param param Parameter to check
 * @return 1 if port parameter, 0 otherwise
 */
static int is_port_parameter(const config_parameter_t* param) {
  return (key_equals(param->key, "http_port") || key_equals(param->key, "rtsp_port") ||
          key_equals(param->key, "snapshot_port") || key_equals(param->key, "ws_discovery_port"));
}

/**
 * @brief Set integer parameter value with validation
 * @param param Parameter to set value for
 * @param value String value to set
 * @return 1 on success, 0 on failure
 */
static int set_int_parameter_value(config_parameter_t* param, const char* value) {
  long int_val = parse_integer_value(value, param);

  // Apply appropriate validation based on parameter type
  if (is_port_parameter(param)) {
    int_val = validate_port_parameter(int_val, param);
  } else if (param->min_value != param->max_value) {
    int_val = validate_range_parameter(int_val, param);
  }

  *(int*)param->value_ptr = int_val;
  return 1;
}

/**
 * @brief Set boolean parameter value
 * @param param Parameter to set value for
 * @param value String value to set
 * @return 1 on success, 0 on failure
 */
static int set_bool_parameter_value(config_parameter_t* param, const char* value) {
  int bool_val = (key_equals(value, "true") || key_equals(value, "1") || key_equals(value, "yes"));
  *(int*)param->value_ptr = bool_val;
  return 1;
}

/**
 * @brief Set string parameter value with validation
 * @param param Parameter to set value for
 * @param value String value to set
 * @return 1 on success, 0 on failure
 */
static int set_string_parameter_value(config_parameter_t* param, const char* value) {
  // Validate string length based on parameter type
  validation_result_t validation;
  if (key_equals(param->key, "username")) {
    validation = validate_string("username", value, MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH, 0);
  } else if (key_equals(param->key, "password")) {
    validation = validate_string("password", value, MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH, 0);
  } else {
    validation = validate_string(param->key, value, 0, param->value_size - 1, 1);
  }

  if (!validation_is_valid(&validation)) {
    platform_log_warning("string validation failed for %s: %s, using truncated value", param->key,
                         validation_get_error_message(&validation));
    // Continue with truncated value instead of failing
  }

  size_t value_len = strlen(value);
  size_t copy_len = (value_len < param->value_size - 1) ? value_len : param->value_size - 1;
  strncpy((char*)param->value_ptr, value, copy_len);
  ((char*)param->value_ptr)[copy_len] = '\0';
  return 1;
}

/**
 * @brief Set float parameter value with validation
 * @param param Parameter to set value for
 * @param value String value to set
 * @return 1 on success, 0 on failure
 */
static int set_float_parameter_value(config_parameter_t* param, const char* value) {
  char* end_ptr = NULL;
  double float_val = strtod(value, &end_ptr);

  // Check for conversion errors
  if (end_ptr == value || *end_ptr != '\0') {
    platform_log_warning("invalid float value '%s' for parameter '%s', using default", value,
                         param->key ? param->key : "unknown");
    if (param->default_value) {
      float_val = strtod(param->default_value, &end_ptr);
      if (end_ptr == param->default_value || *end_ptr != '\0') {
        float_val = 0.0f; // Fallback to 0.0 if default is also invalid
      }
    } else {
      float_val = 0.0f;
    }
  }
  *(float*)param->value_ptr = (float)float_val;
  return 1;
}

/**
 * @brief Set parameter value with validation and bounds checking
 * @param param Parameter to set value for
 * @param value String value to set
 * @note Validates input and applies bounds checking for numeric types
 */
static void set_parameter_value(config_parameter_t* param, const char* value) {
  if (!param || !value) {
    platform_log_warning("set_parameter_value() called with NULL parameter (param=%p, value=%p)",
                         param, value);
    return;
  }

  if (!param->value_ptr) {
    platform_log_debug("parameter '%s' has no value pointer, skipping",
                       param->key ? param->key : "unknown");
    return;
  }

  switch (param->type) {
  case CONFIG_TYPE_INT:
    set_int_parameter_value(param, value);
    break;
  case CONFIG_TYPE_BOOL:
    set_bool_parameter_value(param, value);
    break;
  case CONFIG_TYPE_STRING:
    set_string_parameter_value(param, value);
    break;
  case CONFIG_TYPE_FLOAT:
    set_float_parameter_value(param, value);
    break;
  }
}

/**
 * @brief Set all parameters to their default values
 * @param config Configuration manager
 * @note Only sets parameters that have default values defined
 */
static void set_default_values(config_manager_t* config) {
  if (!config) {
    platform_log_error("set_default_values() called with NULL config parameter");
    return;
  }

  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t* param = &config->sections[i].parameters[j];
      if (param->default_value && param->value_ptr) {
        set_parameter_value(param, param->default_value);
      }
    }
  }
}

/**
 * @brief Link parameter definitions to actual configuration structure members
 * @param config Configuration manager
 * @note This function must be called after config_init to establish proper
 * memory links
 */
static void link_parameters_to_config(config_manager_t* config) {
  if (!config || !config->app_config) {
    platform_log_error("link_parameters_to_config() called with NULL parameter (config=%p, "
                       "app_config=%p)",
                       config, config ? config->app_config : NULL);
    return;
  }

  // Link ONVIF parameters
  onvif_parameters[0].value_ptr = &config->app_config->onvif.enabled;
  onvif_parameters[1].value_ptr = &config->app_config->onvif.http_port;
  onvif_parameters[2].value_ptr = &config->app_config->onvif.auth_enabled;
  onvif_parameters[3].value_ptr = config->app_config->onvif.username;
  onvif_parameters[4].value_ptr = config->app_config->onvif.password;

  // Link imaging parameters
  if (config->app_config->imaging) {
    imaging_parameters[0].value_ptr = &config->app_config->imaging->brightness;
    imaging_parameters[1].value_ptr = &config->app_config->imaging->contrast;
    imaging_parameters[2].value_ptr = &config->app_config->imaging->saturation;
    imaging_parameters[3].value_ptr = &config->app_config->imaging->sharpness;
    imaging_parameters[4].value_ptr = &config->app_config->imaging->hue;
  }

  // Link auto day/night parameters (mapped to device parameter names)
  if (config->app_config->auto_daynight) {
    auto_daynight_parameters[0].value_ptr =
      &config->app_config->auto_daynight->enable_auto_switching;
    auto_daynight_parameters[1].value_ptr = &config->app_config->auto_daynight->mode;
    auto_daynight_parameters[2].value_ptr =
      &config->app_config->auto_daynight->day_to_night_threshold;
    auto_daynight_parameters[3].value_ptr =
      &config->app_config->auto_daynight->night_to_day_threshold;
    auto_daynight_parameters[4].value_ptr = &config->app_config->auto_daynight->lock_time_seconds;
  }

  // Link network parameters
  if (config->app_config->network) {
    network_parameters[0].value_ptr = &config->app_config->network->rtsp_port;
    network_parameters[1].value_ptr = &config->app_config->network->snapshot_port;
    network_parameters[2].value_ptr = &config->app_config->network->ws_discovery_port;
  }

  // Link device parameters
  if (config->app_config->device) {
    device_parameters[0].value_ptr = config->app_config->device->manufacturer;
    device_parameters[1].value_ptr = config->app_config->device->model;
    device_parameters[2].value_ptr = config->app_config->device->firmware_version;
    device_parameters[3].value_ptr = config->app_config->device->serial_number;
    device_parameters[4].value_ptr = config->app_config->device->hardware_id;
  }

  // Link logging parameters
  if (config->app_config->logging) {
    logging_parameters[0].value_ptr = &config->app_config->logging->enabled;
    logging_parameters[1].value_ptr = &config->app_config->logging->use_colors;
    logging_parameters[2].value_ptr = &config->app_config->logging->use_timestamps;
    logging_parameters[3].value_ptr = &config->app_config->logging->min_level;
    logging_parameters[4].value_ptr = config->app_config->logging->tag;
    logging_parameters[5].value_ptr = &config->app_config->logging->http_verbose;
  }

  // Link server parameters
  if (config->app_config->server) {
    server_parameters[0].value_ptr = &config->app_config->server->worker_threads;
    server_parameters[1].value_ptr = &config->app_config->server->max_connections;
    server_parameters[2].value_ptr = &config->app_config->server->connection_timeout;
    server_parameters[3].value_ptr = &config->app_config->server->keepalive_timeout;
    server_parameters[4].value_ptr = &config->app_config->server->epoll_timeout;
    server_parameters[5].value_ptr = &config->app_config->server->cleanup_interval;
  }

  // Link main stream parameters
  if (config->app_config->main_stream) {
    main_stream_parameters[0].value_ptr = &config->app_config->main_stream->fps;
    main_stream_parameters[1].value_ptr = &config->app_config->main_stream->bitrate;
  }

  // Link sub stream parameters
  if (config->app_config->sub_stream) {
    sub_stream_parameters[0].value_ptr = &config->app_config->sub_stream->fps;
    sub_stream_parameters[1].value_ptr = &config->app_config->sub_stream->bitrate;
  }
}

/**
 * @brief Validate a single line in configuration file
 * @param line Line to validate
 * @param has_valid_section Pointer to flag indicating valid section found
 * @param line_number Line number for error reporting
 * @param has_valid_key_value Pointer to flag indicating valid key=value found
 * @return 1 if line is valid, 0 if invalid
 */
static int validate_config_line(const char* line, int* has_valid_section, int line_number,
                                int* has_valid_key_value) {
  // Check for line truncation
  if (strlen(line) == MAX_LINE_LENGTH - 1 && line[MAX_LINE_LENGTH - 2] != '\n') {
    platform_log_warning("line %d too long in config file", line_number);
    return 0;
  }

  char trimmed_line[MAX_LINE_LENGTH];
  strncpy(trimmed_line, line, sizeof(trimmed_line) - 1);
  trimmed_line[sizeof(trimmed_line) - 1] = '\0';
  trim_whitespace(trimmed_line);

  // Skip empty lines and comments
  if (!trimmed_line[0] || trimmed_line[0] == '#' || trimmed_line[0] == ';') {
    return 1;
  }

  // Validate section header
  if (trimmed_line[0] == '[') {
    char* end = strchr(trimmed_line, ']');
    if (!end) {
      platform_log_warning("malformed section header at line %d", line_number);
      return 0;
    }

    size_t len = end - trimmed_line - 1;
    if (len == 0 || len >= MAX_SECTION_NAME_LENGTH) {
      platform_log_warning("invalid section name at line %d", line_number);
      return 0;
    }

    *has_valid_section = 1;
    return 1;
  }

  // Validate key=value pair
  char* equals_sign = strchr(trimmed_line, '=');
  if (!equals_sign) {
    platform_log_warning("malformed key=value pair at line %d", line_number);
    return 0;
  }

  // Validate key length
  size_t key_len = equals_sign - trimmed_line;
  if (key_len == 0 || key_len >= MAX_KEY_LENGTH) {
    platform_log_warning("invalid key length at line %d", line_number);
    return 0;
  }

  // Validate value length
  size_t value_len = strlen(equals_sign + 1);
  if (value_len >= MAX_VALUE_LENGTH) {
    platform_log_warning("value too long at line %d", line_number);
    return 0;
  }

  *has_valid_key_value = 1;
  return 1;
}

/**
 * @brief Validate configuration file format and content
 * @param config_file Path to configuration file
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 * @note Performs basic format validation without loading values
 */
static int validate_config_file_format(const char* config_file) {
  if (!config_file) {
    platform_log_error("validate_config_file_format() called with NULL config_file parameter");
    return ONVIF_VALIDATION_FAILED;
  }

  FILE* file_ptr = fopen(config_file, "r");
  if (!file_ptr) {
    platform_log_error("failed to open config file '%s' for validation: %s", config_file,
                       strerror(errno));
    return ONVIF_VALIDATION_FAILED;
  }

  char line[MAX_LINE_LENGTH];
  int line_number = 0;
  int has_valid_section = 0;
  int has_valid_key_value = 0;

  while (fgets(line, sizeof(line), file_ptr)) {
    line_number++;

    if (!validate_config_line(line, &has_valid_section, line_number, &has_valid_key_value)) {
      (void)fclose(file_ptr);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  (void)fclose(file_ptr);

  // Basic validation: file should have at least one section and one key=value
  // pair
  if (!has_valid_section || !has_valid_key_value) {
    platform_log_warning("config file appears to be empty or malformed");
    return ONVIF_VALIDATION_FAILED;
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Convert device-specific parameter values to ONVIF format
 * @param config Configuration manager
 */
static void convert_device_values_to_onvif(config_manager_t* config) {
  if (!config || !config->app_config) {
    platform_log_warning("convert_device_values_to_onvif() called with NULL parameter "
                         "(config=%p, app_config=%p)",
                         config, config ? config->app_config : NULL);
    return;
  }

  if (!config->app_config->auto_daynight) {
    platform_log_info("auto_daynight config not available, skipping conversion");
    return;
  }

  struct auto_daynight_config* auto_config = config->app_config->auto_daynight;

  // Convert device luminance values (0-10000) to ONVIF threshold values (0-100)
  // Device uses: day_to_night_lum = 6400, night_to_day_lum = 2048
  // ONVIF expects: day_to_night_threshold = 30, night_to_day_threshold = 70
  if (auto_config->day_to_night_threshold > 0) {
    // Convert from device range (0-10000) to ONVIF range (0-100)
    auto_config->day_to_night_threshold = (auto_config->day_to_night_threshold * 100) / 10000;
    if (auto_config->day_to_night_threshold > 100) {
      auto_config->day_to_night_threshold = 100;
    }
  }

  if (auto_config->night_to_day_threshold > 0) {
    // Convert from device range (0-10000) to ONVIF range (0-100)
    auto_config->night_to_day_threshold = (auto_config->night_to_day_threshold * 100) / 10000;
    if (auto_config->night_to_day_threshold > 100) {
      auto_config->night_to_day_threshold = 100;
    }
  }

  // Convert device lock time (microseconds) to ONVIF lock time (seconds)
  if (auto_config->lock_time_seconds > 0) {
    // Device uses microseconds, ONVIF uses seconds
    auto_config->lock_time_seconds = auto_config->lock_time_seconds / 1000000;
    if (auto_config->lock_time_seconds < 1) {
      auto_config->lock_time_seconds = 1;
    }
    if (auto_config->lock_time_seconds > 3600) {
      auto_config->lock_time_seconds = 3600;
    }
  }

  // Device day_night_mode values: 0=night, 1=day, 2=auto
  // ONVIF day_night_mode values: 0=auto, 1=day, 2=night
  if (auto_config->mode == 2) {
    auto_config->mode = 0; // Device auto -> ONVIF auto
  } else if (auto_config->mode == 1) {
    auto_config->mode = 1; // Device day -> ONVIF day
  } else if (auto_config->mode == 0) {
    auto_config->mode = 2; // Device night -> ONVIF night
  }
}

/**
 * @brief Initialize configuration system with application config
 * @param config Configuration manager to initialize
 * @param app_config Application configuration structure
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note This function must be called before any other config operations
 */
int config_init(config_manager_t* config, struct application_config* app_config) {
  if (!config || !app_config) {
    platform_log_error("config_init() called with NULL parameter (config=%p, app_config=%p)",
                       config, app_config);
    return ONVIF_ERROR_INVALID;
  }

  memset(config, 0, sizeof(config_manager_t));
  config->app_config = app_config;
  config->sections = (config_section_def_t*)default_sections;
  config->section_count = sizeof(default_sections) / sizeof(default_sections[0]);
  config->validation_enabled = 1;

  // Initialize runtime parameter arrays from const init data
  initialize_parameter_arrays();

  // Link parameters to configuration structure
  link_parameters_to_config(config);

  // Set default values
  set_default_values(config);

  // Convert device-specific values to ONVIF format
  convert_device_values_to_onvif(config);

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse a section header from a line
 * @param line Line containing section header
 * @param line_number Line number for error reporting
 * @param current_section Buffer to store section name
 * @param current_section_type Pointer to store section type
 * @param current_section_known Pointer to flag indicating if section is known
 * @return 1 if section parsed successfully, 0 if error
 */
static int parse_section_header(const char* line, int line_number, char* current_section,
                                config_section_t* current_section_type,
                                int* current_section_known) {
  char* end = strchr(line, ']');
  if (end) {
    size_t len = end - line - 1;
    if (len >= MAX_SECTION_NAME_LENGTH) {
      platform_log_warning("section name too long at line %d, truncating", line_number);
      len = MAX_SECTION_NAME_LENGTH - 1;
    }
    strncpy(current_section, line + 1, len);
    current_section[len] = '\0';
    trim_whitespace(current_section);
    *current_section_known = is_known_section(current_section);
    if (*current_section_known) {
      *current_section_type = parse_section_name(current_section);
    } else {
      platform_log_info("processing unknown section '[%s]' at line %d", current_section,
                        line_number);
    }
  } else {
    platform_log_warning("malformed section header at line %d, skipping", line_number);
    strcpy(current_section, "onvif");
    *current_section_type = CONFIG_SECTION_ONVIF;
    *current_section_known = 1;
  }
  return 1;
}

/**
 * @brief Parse a key=value pair from a line
 * @param line Line containing key=value pair
 * @param line_number Line number for error reporting
 * @param key Buffer to store key
 * @param value Buffer to store value
 * @return 1 if key=value parsed successfully, 0 if error
 */
static int parse_key_value_pair(const char* line, int line_number, char* key, char* value) {
  char* equals_sign = strchr(line, '=');
  if (!equals_sign) {
    platform_log_warning("malformed key=value pair at line %d, skipping", line_number);
    return 0;
  }

  *equals_sign = '\0';

  // Safely copy key
  size_t key_len = strlen(line);
  if (key_len >= MAX_KEY_LENGTH) {
    platform_log_warning("key too long at line %d, truncating", line_number);
    key_len = MAX_KEY_LENGTH - 1;
  }
  strncpy(key, line, key_len);
  key[key_len] = '\0';

  // Safely copy value
  size_t value_len = strlen(equals_sign + 1);
  if (value_len >= MAX_VALUE_LENGTH) {
    platform_log_warning("value too long at line %d, truncating", line_number);
    value_len = MAX_VALUE_LENGTH - 1;
  }
  strncpy(value, equals_sign + 1, value_len);
  value[value_len] = '\0';

  // Strip inline comments starting with ';' or '#'
  char* comment = strpbrk(value, ";#");
  if (comment) {
    *comment = '\0';
  }

  trim_whitespace(key);
  trim_whitespace(value);

  // Skip empty keys
  if (strlen(key) == 0) {
    platform_log_warning("empty key at line %d, skipping", line_number);
    return 0;
  }

  return 1;
}

/**
 * @brief Process a single line from configuration file
 * @param line Line to process
 * @param line_number Line number for error reporting
 * @param config Configuration manager
 * @param current_section Current section name
 * @param current_section_type Current section type
 * @param current_section_known Flag indicating if current section is known
 * @return 1 if line processed successfully, 0 if error
 */
static int process_config_line(const char* line, int line_number, config_manager_t* config,
                               char* current_section, config_section_t* current_section_type,
                               int* current_section_known) {
  // Strip UTF-8 BOM at start of file/line if present
  if ((unsigned char)line[0] == 0xEF && (unsigned char)line[1] == 0xBB &&
      (unsigned char)line[2] == 0xBF) {
    size_t remaining = strlen(line + 3);
    memmove((char*)line, line + 3, remaining + 1);
  }

  // Check for line truncation
  if (strlen(line) == MAX_LINE_LENGTH - 1 && line[MAX_LINE_LENGTH - 2] != '\n') {
    platform_log_warning("line %d too long, truncated", line_number);
    // Skip to next line
    return 1;
  }

  char trimmed_line[MAX_LINE_LENGTH];
  strncpy(trimmed_line, line, sizeof(trimmed_line) - 1);
  trimmed_line[sizeof(trimmed_line) - 1] = '\0';
  trim_whitespace(trimmed_line);

  // Skip empty lines and comments
  if (!trimmed_line[0] || trimmed_line[0] == '#' || trimmed_line[0] == ';') {
    return 1;
  }

  // Parse section header
  if (trimmed_line[0] == '[') {
    return parse_section_header(trimmed_line, line_number, current_section, current_section_type,
                                current_section_known);
  }

  // Parse key=value pairs
  char key[MAX_KEY_LENGTH];
  char value[MAX_VALUE_LENGTH];
  if (!parse_key_value_pair(trimmed_line, line_number, key, value)) {
    return 1; // Continue processing other lines
  }

  // Find and set parameter
  if (*current_section_known) {
    config_parameter_t* param = find_parameter(config, *current_section_type, key);
    if (param) {
      set_parameter_value(param, value);
    } else {
      platform_log_info("unknown parameter '%s' in section '[%s]' at line %d, skipping", key,
                        current_section, line_number);
    }
  }

  return 1;
}

/**
 * @brief Open configuration file with fallback options
 * @param config_file Path to configuration file
 * @return FILE pointer on success, NULL on failure
 */
static FILE* open_config_file(const char* config_file) {
  FILE* file_ptr = fopen(config_file, "r");
  if (!file_ptr) {
    // Try alternate filename
    if (strcmp(config_file, ONVIF_CONFIG_FILE) == 0) {
      file_ptr = fopen("/etc/jffs2/anyka_cfg.ini", "r");
    }
    if (!file_ptr) {
      platform_log_error("could not open config file '%s': %s (tried alternate: "
                         "/etc/jffs2/anyka_cfg.ini)",
                         config_file, strerror(errno));
    }
  }
  return file_ptr;
}

/**
 * @brief Load configuration from INI file with comprehensive error handling
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR_IO on
 * failure
 * @note Tolerant parser: supports UTF-8 BOM, strips inline comments (';' or
 * '#'), ignores unknown sections/keys, clamps invalid numeric ranges, and logs
 *       non-fatal issues. Only known sections/keys are applied to
 * configuration.
 */
int config_load(config_manager_t* config, const char* config_file) {
  if (!config || !config_file) {
    platform_log_error("config_load() called with NULL parameter (config=%p, config_file=%p)",
                       config, config_file);
    return ONVIF_ERROR_INVALID;
  }

  FILE* file_ptr = open_config_file(config_file);
  if (!file_ptr) {
    return ONVIF_ERROR_IO;
  }

  char line[MAX_LINE_LENGTH];
  char current_section[MAX_SECTION_NAME_LENGTH] = "";
  config_section_t current_section_type = CONFIG_SECTION_ONVIF;
  int current_section_known = 0;
  int line_number = 0;

  while (fgets(line, sizeof(line), file_ptr)) {
    line_number++;

    // Check for line truncation and skip if too long
    if (strlen(line) == sizeof(line) - 1 && line[sizeof(line) - 2] != '\n') {
      platform_log_warning("line %d too long, truncated", line_number);
      // Skip to next line
      int character = 0;
      while ((character = fgetc(file_ptr)) != EOF && character != '\n') {
        // Skip characters until newline
      }
      continue;
    }

    process_config_line(line, line_number, config, current_section, &current_section_type,
                        &current_section_known);
  }

  if (file_ptr) {
    (void)fclose(file_ptr);
  }
  return ONVIF_SUCCESS;
}

/**
 * @brief Validate all configuration parameters
 * @param config Configuration manager
 * @return CONFIG_VALIDATION_OK if valid, error code if validation fails
 * @note Checks required parameters and value ranges
 */
config_validation_result_t config_validate(config_manager_t* config) {
  if (!config) {
    platform_log_error("config_validate() called with NULL config parameter");
    return CONFIG_VALIDATION_OK;
  }

  if (!config->validation_enabled) {
    platform_log_debug("config validation is disabled");
    return CONFIG_VALIDATION_OK;
  }

  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t* param = &config->sections[i].parameters[j];

      if (param->required && !param->value_ptr) {
        platform_log_error("required parameter '%s' in section %d has no value pointer", param->key,
                           config->sections[i].section);
        return CONFIG_VALIDATION_MISSING_REQUIRED;
      }

      if (param->type == CONFIG_TYPE_INT && param->value_ptr) {
        int value = *(int*)param->value_ptr;
        if (param->min_value != param->max_value &&
            (value < param->min_value || value > param->max_value)) {
          platform_log_error("parameter '%s' value %d is out of range [%d, %d] in section %d",
                             param->key, value, param->min_value, param->max_value,
                             config->sections[i].section);
          return CONFIG_VALIDATION_OUT_OF_RANGE;
        }
      }

      if (param->type == CONFIG_TYPE_BOOL && param->value_ptr) {
        int value = *(int*)param->value_ptr;
        if (value != 0 && value != 1) {
          platform_log_error(
            "parameter '%s' value %d is invalid for boolean type (must be 0 or 1) in section %d",
            param->key, value, config->sections[i].section);
          return CONFIG_VALIDATION_INVALID_VALUE;
        }
      }
    }
  }

  return CONFIG_VALIDATION_OK;
}

/**
 * @brief Get configuration value with type safety
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to store the value
 * @param value_type Expected value type
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or
 * ONVIF_ERROR_NOT_FOUND on failure
 * @note Validates type compatibility before copying value
 */
int config_get_value(config_manager_t* config, config_section_t section, const char* key,
                     void* value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    platform_log_error("config_get_value() called with NULL parameter (config=%p, key=%p, "
                       "value_ptr=%p)",
                       config, key, value_ptr);
    return ONVIF_ERROR_INVALID;
  }

  config_parameter_t* param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    platform_log_error("parameter '%s' not found or type mismatch (expected %d, got %d) in "
                       "section %d",
                       key, value_type, param ? param->type : -1, section);
    return ONVIF_ERROR_NOT_FOUND;
  }

  if (param->value_ptr) {
    memcpy(value_ptr, param->value_ptr, param->value_size);
    return ONVIF_SUCCESS;
  }

  platform_log_error("parameter '%s' has no value pointer in section %d", key, section);
  return ONVIF_ERROR_NOT_FOUND;
}

/**
 * @brief Set configuration value with validation
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to the value
 * @param value_type Value type
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or
 * ONVIF_ERROR_NOT_FOUND on failure
 * @note Validates type compatibility before setting value
 */
int config_set_value(config_manager_t* config, config_section_t section, const char* key,
                     const void* value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    platform_log_error("config_set_value() called with NULL parameter (config=%p, key=%p, "
                       "value_ptr=%p)",
                       config, key, value_ptr);
    return ONVIF_ERROR_INVALID;
  }

  config_parameter_t* param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    platform_log_error("parameter '%s' not found or type mismatch (expected %d, got %d) in "
                       "section %d",
                       key, value_type, param ? param->type : -1, section);
    return ONVIF_ERROR_NOT_FOUND;
  }

  if (param->value_ptr) {
    memcpy(param->value_ptr, value_ptr, param->value_size);

    // Validate the value after setting it
    if (config->validation_enabled) {
      config_validation_result_t validation_result = config_validate(config);
      if (validation_result != CONFIG_VALIDATION_OK) {
        return ONVIF_ERROR_INVALID;
      }
    }

    return ONVIF_SUCCESS;
  }

  platform_log_error("parameter '%s' has no value pointer in section %d", key, section);
  return ONVIF_ERROR_NOT_FOUND;
}

/**
 * @brief Reset all configuration parameters to their default values
 * @param config Configuration manager
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Only resets parameters that have default values defined
 */
int config_reset_to_defaults(config_manager_t* config) {
  if (!config) {
    platform_log_error("config_reset_to_defaults() called with NULL config parameter");
    return ONVIF_ERROR_INVALID;
  }

  set_default_values(config);
  return ONVIF_SUCCESS;
}

/**
 * @brief Get configuration parameter definition
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @return Parameter definition or NULL if not found
 * @note Returns NULL if config is NULL or key is NULL
 */
const config_parameter_t* config_get_parameter(config_manager_t* config, config_section_t section,
                                               const char* key) {
  if (!config || !key) {
    platform_log_error("config_get_parameter() called with NULL parameter (config=%p, key=%p)",
                       config, key);
    return NULL;
  }

  return find_parameter(config, section, key);
}

/**
 * @brief Clean up configuration manager
 * @param config Configuration manager to clean up
 * @note Safely clears all configuration data
 */
void config_cleanup(config_manager_t* config) {
  if (config) {
    memset(config, 0, sizeof(config_manager_t));
  } else {
    platform_log_warning("config_cleanup() called with NULL config parameter");
  }
}

/**
 * @brief Get configuration summary for logging and debugging
 * @param config Configuration manager
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR on
 * failure
 * @note Creates a human-readable summary of all configuration values
 */
int config_get_summary(config_manager_t* config, char* summary, size_t summary_size) {
  if (!config || !summary) {
    platform_log_error("config_get_summary() called with NULL parameter (config=%p, "
                       "summary=%p)",
                       config, summary);
    return ONVIF_ERROR_INVALID;
  }

  if (!config->app_config) {
    platform_log_error("app_config is NULL in config_get_summary()");
    return ONVIF_ERROR_INVALID;
  }

  int result = snprintf(
    summary, summary_size,
    "ONVIF: enabled=%d, port=%d, auth_enabled=%d, user=%s\n"
    "Imaging: brightness=%d, contrast=%d, saturation=%d, sharpness=%d, "
    "hue=%d\n"
    "Auto Day/Night: enabled=%d, mode=%d, thresholds=%d/%d, lock_time=%ds",
    config->app_config->onvif.enabled, config->app_config->onvif.http_port,
    config->app_config->onvif.auth_enabled, config->app_config->onvif.username,
    config->app_config->imaging ? config->app_config->imaging->brightness : 0,
    config->app_config->imaging ? config->app_config->imaging->contrast : 0,
    config->app_config->imaging ? config->app_config->imaging->saturation : 0,
    config->app_config->imaging ? config->app_config->imaging->sharpness : 0,
    config->app_config->imaging ? config->app_config->imaging->hue : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->enable_auto_switching
                                      : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->mode : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->day_to_night_threshold
                                      : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->night_to_day_threshold
                                      : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->lock_time_seconds : 0);

  if (result < 0) {
    platform_log_error("snprintf failed in config_get_summary()");
    return ONVIF_ERROR;
  }

  if ((size_t)result >= summary_size) {
    platform_log_error("summary buffer too small (required=%d, available=%zu)", result,
                       summary_size);
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}
