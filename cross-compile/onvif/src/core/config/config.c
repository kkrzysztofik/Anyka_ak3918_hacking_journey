/**
 * @file config.c
 * @brief Configuration management system implementation
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <errno.h>

#include "config.h"
#include "utils/error/error_handling.h"
#include "utils/string/string_shims.h"
#include "utils/validation/common_validation.h"
#include "services/common/onvif_imaging_types.h"
#include "platform/platform.h"

// Default configuration values
#define DEFAULT_HTTP_PORT 8080
#define DEFAULT_RTSP_PORT 554
#define DEFAULT_SNAPSHOT_PORT 3000
#define DEFAULT_USERNAME "admin"
#define DEFAULT_PASSWORD "admin"
#define DEFAULT_BRIGHTNESS 50
#define DEFAULT_CONTRAST 50
#define DEFAULT_SATURATION 50
#define DEFAULT_SHARPNESS 50
#define DEFAULT_HUE 0

// Configuration file parsing constants
#define MAX_LINE_LENGTH 512
#define MAX_SECTION_NAME_LENGTH 128
#define MAX_KEY_LENGTH 128
#define MAX_VALUE_LENGTH 256

// Configuration validation constants
#define MIN_USERNAME_LENGTH 1
#define MAX_USERNAME_LENGTH 32
#define MIN_PASSWORD_LENGTH 1
#define MAX_PASSWORD_LENGTH 32
#define MIN_PORT_VALUE 1
#define MAX_PORT_VALUE 65535

// Configuration parameter definitions
static config_parameter_t onvif_parameters[] = {
  {"enabled", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 1},
  {"http_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "8080", 1},
  {"username", CONFIG_TYPE_STRING, NULL, 64, 0, 0, DEFAULT_USERNAME, 1},
  {"password", CONFIG_TYPE_STRING, NULL, 64, 0, 0, DEFAULT_PASSWORD, 1}
};

static config_parameter_t imaging_parameters[] = {
  {"brightness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"contrast", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"saturation", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"sharpness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"hue", CONFIG_TYPE_INT, NULL, sizeof(int), -180, 180, "0", 0}
};

static config_parameter_t auto_daynight_parameters[] = {
  {"auto_day_night_enable", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"day_night_mode", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 2, "2", 0},
  {"day_to_night_lum", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 10000, "6400", 0},
  {"night_to_day_lum", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 10000, "2048", 0},
  {"lock_time", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 3600000, "900000", 0}
};

static config_parameter_t network_parameters[] = {
  {"rtsp_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "554", 0},
  {"snapshot_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "3000", 0},
  {"ws_discovery_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "3702", 0}
};

static config_parameter_t device_parameters[] = {
  {"manufacturer", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "Anyka", 0},
  {"model", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "AK3918 Camera", 0},
  {"firmware_version", CONFIG_TYPE_STRING, NULL, 32, 0, 0, "1.0.0", 0},
  {"serial_number", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "AK3918-001", 0},
  {"hardware_id", CONFIG_TYPE_STRING, NULL, 32, 0, 0, "1.0", 0}
};

// Section definitions
static config_section_def_t default_sections[] = {
  {CONFIG_SECTION_ONVIF, "onvif", onvif_parameters, sizeof(onvif_parameters) / sizeof(onvif_parameters[0])},
  {CONFIG_SECTION_IMAGING, "imaging", imaging_parameters, sizeof(imaging_parameters) / sizeof(imaging_parameters[0])},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "autoir", auto_daynight_parameters, sizeof(auto_daynight_parameters) / sizeof(auto_daynight_parameters[0])},
  {CONFIG_SECTION_NETWORK, "network", network_parameters, sizeof(network_parameters) / sizeof(network_parameters[0])},
  {CONFIG_SECTION_DEVICE, "device", device_parameters, sizeof(device_parameters) / sizeof(device_parameters[0])}
};


/**
 * @brief Case-insensitive string comparison for configuration keys
 * @param a First string to compare
 * @param b Second string to compare
 * @return 1 if strings are equal (case-insensitive), 0 otherwise
 * @note Uses strcasecmp from the SDK
 */
static int key_equals(const char *a, const char *b) {
  if (!a || !b) {
    platform_log_error("error: key_equals() called with NULL parameter (a=%p, b=%p)\n", a, b);
    return 0;
  }
  return strcasecmp(a, b) == 0;
}

/**
 * @brief Check if a section name is known/valid
 * @param section_name Section name string to check
 * @return 1 if section is known, 0 if unknown
 * @note Returns 0 if section_name is NULL
 */
static int is_known_section(const char *section_name) {
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
static config_section_t parse_section_name(const char *section_name) {
  if (!section_name) {
    platform_log_error("error: parse_section_name() called with NULL section_name parameter\n");
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
static config_parameter_t *find_parameter(config_manager_t *config, config_section_t section, const char *key) {
  if (!config || !key) {
    platform_log_error("error: find_parameter() called with NULL parameter (config=%p, key=%p)\n", config, key);
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
 * @brief Set parameter value with validation and bounds checking
 * @param param Parameter to set value for
 * @param value String value to set
 * @note Validates input and applies bounds checking for numeric types
 */
static void set_parameter_value(config_parameter_t *param, const char *value) {
  if (!param || !value) {
    platform_log_warning("warning: set_parameter_value() called with NULL parameter (param=%p, value=%p)\n", 
                        param, value);
    return;
  }
  
  if (!param->value_ptr) {
    platform_log_debug("debug: parameter '%s' has no value pointer, skipping\n", 
                       param->key ? param->key : "unknown");
    return;
  }
  
  switch (param->type) {
    case CONFIG_TYPE_INT: {
      int int_val = atoi(value);
      
      // Use validation utilities for port numbers
      validation_result_t validation;
      if (key_equals(param->key, "http_port") || key_equals(param->key, "rtsp_port") || 
          key_equals(param->key, "snapshot_port") || key_equals(param->key, "ws_discovery_port")) {
        validation = validate_int(int_val, param->key, MIN_PORT_VALUE, MAX_PORT_VALUE);
        if (!validation_is_valid(&validation)) {
          platform_log_warning("warning: port validation failed for %s: %s, using default\n", 
                              param->key, validation_get_error_message(&validation));
          // Use default value instead of failing
          if (param->default_value) {
            int_val = atoi(param->default_value);
          }
        }
      } else if (param->min_value != param->max_value) {
        validation = validate_int(int_val, param->key, param->min_value, param->max_value);
        if (!validation_is_valid(&validation)) {
          platform_log_warning("warning: integer validation failed for %s: %s, clamping to valid range\n", 
                              param->key, validation_get_error_message(&validation));
          // Clamp to valid range instead of failing
          if (int_val < param->min_value) int_val = param->min_value;
          if (int_val > param->max_value) int_val = param->max_value;
        }
      }
      
      *(int*)param->value_ptr = int_val;
      break;
    }
    case CONFIG_TYPE_BOOL: {
      int bool_val = (key_equals(value, "true") || key_equals(value, "1") || key_equals(value, "yes"));
      *(int*)param->value_ptr = bool_val;
      break;
    }
    case CONFIG_TYPE_STRING: {
      // Validate string length based on parameter type
      validation_result_t validation;
      if (key_equals(param->key, "username")) {
        validation = validate_string(value, "username", MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH, 0);
      } else if (key_equals(param->key, "password")) {
        validation = validate_string(value, "password", MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH, 0);
      } else {
        validation = validate_string(value, param->key, 0, param->value_size - 1, 1);
      }
      
      if (!validation_is_valid(&validation)) {
        platform_log_warning("warning: string validation failed for %s: %s, using truncated value\n", 
                            param->key, validation_get_error_message(&validation));
        // Continue with truncated value instead of failing
      }
      
      size_t value_len = strlen(value);
      size_t copy_len = (value_len < param->value_size - 1) ? value_len : param->value_size - 1;
      strncpy((char*)param->value_ptr, value, copy_len);
      ((char*)param->value_ptr)[copy_len] = '\0';
      break;
    }
    case CONFIG_TYPE_FLOAT: {
      float float_val = (float)atof(value);
      *(float*)param->value_ptr = float_val;
      break;
    }
  }
}

/**
 * @brief Set all parameters to their default values
 * @param config Configuration manager
 * @note Only sets parameters that have default values defined
 */
static void set_default_values(config_manager_t *config) {
  if (!config) {
    platform_log_error("error: set_default_values() called with NULL config parameter\n");
    return;
  }
  
  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t *param = &config->sections[i].parameters[j];
      if (param->default_value && param->value_ptr) {
        set_parameter_value(param, param->default_value);
      }
    }
  }
}

/**
 * @brief Link parameter definitions to actual configuration structure members
 * @param config Configuration manager
 * @note This function must be called after config_init to establish proper memory links
 */
static void link_parameters_to_config(config_manager_t *config) {
  if (!config || !config->app_config) {
    platform_log_error("error: link_parameters_to_config() called with NULL parameter (config=%p, app_config=%p)\n", 
                      config, config ? config->app_config : NULL);
    return;
  }
  
  // Link ONVIF parameters
  onvif_parameters[0].value_ptr = &config->app_config->onvif.enabled;
  onvif_parameters[1].value_ptr = &config->app_config->onvif.http_port;
  onvif_parameters[2].value_ptr = config->app_config->onvif.username;
  onvif_parameters[3].value_ptr = config->app_config->onvif.password;
  
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
    auto_daynight_parameters[0].value_ptr = &config->app_config->auto_daynight->enable_auto_switching;
    auto_daynight_parameters[1].value_ptr = &config->app_config->auto_daynight->mode;
    auto_daynight_parameters[2].value_ptr = &config->app_config->auto_daynight->day_to_night_threshold;
    auto_daynight_parameters[3].value_ptr = &config->app_config->auto_daynight->night_to_day_threshold;
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
}

/**
 * @brief Validate configuration file format and content
 * @param config_file Path to configuration file
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 * @note Performs basic format validation without loading values
 */
static int validate_config_file_format(const char *config_file) {
  if (!config_file) {
    platform_log_error("error: validate_config_file_format() called with NULL config_file parameter\n");
    return ONVIF_VALIDATION_FAILED;
  }
  
  FILE *fp = fopen(config_file, "r");
  if (!fp) {
    platform_log_error("error: failed to open config file '%s' for validation: %s\n", 
                      config_file, strerror(errno));
    return ONVIF_VALIDATION_FAILED;
  }
  
  char line[MAX_LINE_LENGTH];
  int line_number = 0;
  int has_valid_section = 0;
  int has_valid_key_value = 0;
  
  while (fgets(line, sizeof(line), fp)) {
    line_number++;
    
    // Check for line truncation
    if (strlen(line) == sizeof(line) - 1 && line[sizeof(line) - 2] != '\n') {
      platform_log_warning("warning: line %d too long in config file\n", line_number);
      fclose(fp);
      return ONVIF_VALIDATION_FAILED;
    }
    
    trim_whitespace(line);
    
    // Skip empty lines and comments
    if (!line[0] || line[0] == '#' || line[0] == ';') {
      continue;
    }
    
    // Validate section header
    if (line[0] == '[') {
      char *end = strchr(line, ']');
      if (!end) {
        platform_log_warning("warning: malformed section header at line %d\n", line_number);
        fclose(fp);
        return ONVIF_VALIDATION_FAILED;
      }
      
      size_t len = end - line - 1;
      if (len == 0 || len >= MAX_SECTION_NAME_LENGTH) {
        platform_log_warning("warning: invalid section name at line %d\n", line_number);
        fclose(fp);
        return ONVIF_VALIDATION_FAILED;
      }
      
      has_valid_section = 1;
      continue;
    }
    
    // Validate key=value pair
    char *eq = strchr(line, '=');
    if (!eq) {
      platform_log_warning("warning: malformed key=value pair at line %d\n", line_number);
      fclose(fp);
      return ONVIF_VALIDATION_FAILED;
    }
    
    // Validate key length
    size_t key_len = eq - line;
    if (key_len == 0 || key_len >= MAX_KEY_LENGTH) {
      platform_log_warning("warning: invalid key length at line %d\n", line_number);
      fclose(fp);
      return ONVIF_VALIDATION_FAILED;
    }
    
    // Validate value length
    size_t value_len = strlen(eq + 1);
    if (value_len >= MAX_VALUE_LENGTH) {
      platform_log_warning("warning: value too long at line %d\n", line_number);
      fclose(fp);
      return ONVIF_VALIDATION_FAILED;
    }
    
    has_valid_key_value = 1;
  }
  
  fclose(fp);
  
  // Basic validation: file should have at least one section and one key=value pair
  if (!has_valid_section || !has_valid_key_value) {
    platform_log_warning("warning: config file appears to be empty or malformed\n");
    return ONVIF_VALIDATION_FAILED;
  }
  
  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Convert device-specific parameter values to ONVIF format
 * @param config Configuration manager
 */
static void convert_device_values_to_onvif(config_manager_t *config) {
  if (!config || !config->app_config) {
    platform_log_warning("warning: convert_device_values_to_onvif() called with NULL parameter (config=%p, app_config=%p)\n", 
                        config, config ? config->app_config : NULL);
    return;
  }
  
  if (!config->app_config->auto_daynight) {
    platform_log_info("info: auto_daynight config not available, skipping conversion\n");
    return;
  }
  
  struct auto_daynight_config *auto_config = config->app_config->auto_daynight;
  
  // Convert device luminance values (0-10000) to ONVIF threshold values (0-100)
  // Device uses: day_to_night_lum = 6400, night_to_day_lum = 2048
  // ONVIF expects: day_to_night_threshold = 30, night_to_day_threshold = 70
  if (auto_config->day_to_night_threshold > 0) {
    // Convert from device range (0-10000) to ONVIF range (0-100)
    auto_config->day_to_night_threshold = (auto_config->day_to_night_threshold * 100) / 10000;
    if (auto_config->day_to_night_threshold > 100) auto_config->day_to_night_threshold = 100;
  }
  
  if (auto_config->night_to_day_threshold > 0) {
    // Convert from device range (0-10000) to ONVIF range (0-100)
    auto_config->night_to_day_threshold = (auto_config->night_to_day_threshold * 100) / 10000;
    if (auto_config->night_to_day_threshold > 100) auto_config->night_to_day_threshold = 100;
  }
  
  // Convert device lock time (microseconds) to ONVIF lock time (seconds)
  if (auto_config->lock_time_seconds > 0) {
    // Device uses microseconds, ONVIF uses seconds
    auto_config->lock_time_seconds = auto_config->lock_time_seconds / 1000000;
    if (auto_config->lock_time_seconds < 1) auto_config->lock_time_seconds = 1;
    if (auto_config->lock_time_seconds > 3600) auto_config->lock_time_seconds = 3600;
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
int config_init(config_manager_t *config, struct application_config *app_config) {
  if (!config || !app_config) {
    platform_log_error("error: config_init() called with NULL parameter (config=%p, app_config=%p)\n", 
                      config, app_config);
    return ONVIF_ERROR_INVALID;
  }
  
  memset(config, 0, sizeof(config_manager_t));
  config->app_config = app_config;
  config->sections = default_sections;
  config->section_count = sizeof(default_sections) / sizeof(default_sections[0]);
  config->validation_enabled = 1;
  
  // Link parameters to configuration structure
  link_parameters_to_config(config);
  
  // Set default values
  set_default_values(config);
  
  // Convert device-specific values to ONVIF format
  convert_device_values_to_onvif(config);
  
  return ONVIF_SUCCESS;
}

/**
 * @brief Load configuration from INI file with comprehensive error handling
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR_IO on failure
 * @note Tolerant parser: supports UTF-8 BOM, strips inline comments (';' or '#'),
 *       ignores unknown sections/keys, clamps invalid numeric ranges, and logs
 *       non-fatal issues. Only known sections/keys are applied to configuration.
 */
int config_load(config_manager_t *config, const char *config_file) {
  if (!config || !config_file) {
    platform_log_error("error: config_load() called with NULL parameter (config=%p, config_file=%p)\n", 
                      config, config_file);
    return ONVIF_ERROR_INVALID;
  }
  
  // Skip strict validation - be more tolerant of file format issues
  // The parser will handle errors gracefully during actual parsing
  
  FILE *fp = fopen(config_file, "r");
  if (!fp) {
    // Try alternate filename
    if (strcmp(config_file, ONVIF_CONFIG_FILE) == 0) {
      fp = fopen("/etc/jffs2/anyka_cfg.ini", "r");
    }
    if (!fp) {
      platform_log_error("error: could not open config file '%s': %s (tried alternate: /etc/jffs2/anyka_cfg.ini)\n", 
                        config_file, strerror(errno));
      return ONVIF_ERROR_IO;
    }
  }
  
  char line[MAX_LINE_LENGTH];
  char current_section[MAX_SECTION_NAME_LENGTH] = "";
  config_section_t current_section_type = CONFIG_SECTION_ONVIF;
  int current_section_known = 0;
  int line_number = 0;
  
  while (fgets(line, sizeof(line), fp)) {
    line_number++;
    
    // Strip UTF-8 BOM at start of file/line if present
    if ((unsigned char)line[0] == 0xEF && (unsigned char)line[1] == 0xBB && (unsigned char)line[2] == 0xBF) {
      size_t remaining = strlen(line + 3);
      memmove(line, line + 3, remaining + 1);
    }
    
    // Check for line truncation
    if (strlen(line) == sizeof(line) - 1 && line[sizeof(line) - 2] != '\n') {
      platform_log_warning("warning: line %d too long, truncated\n", line_number);
      // Skip to next line
      int c;
      while ((c = fgetc(fp)) != EOF && c != '\n');
      continue;
    }
    
    trim_whitespace(line);
    
    // Skip empty lines and comments
    if (!line[0] || line[0] == '#' || line[0] == ';') {
      continue;
    }
    
    // Parse section header
    if (line[0] == '[') {
      char *end = strchr(line, ']');
      if (end) {
        size_t len = end - line - 1;
        if (len >= sizeof(current_section)) {
          platform_log_warning("warning: section name too long at line %d, truncating\n", line_number);
          len = sizeof(current_section) - 1;
        }
        strncpy(current_section, line + 1, len);
        current_section[len] = '\0';
        trim_whitespace(current_section);
        current_section_known = is_known_section(current_section);
        if (current_section_known) {
          current_section_type = parse_section_name(current_section);
        } else {
          // Keep previous section_type but mark as unknown to skip key=value application
          platform_log_info("info: processing unknown section '[%s]' at line %d\n", 
                           current_section, line_number);
        }
      } else {
        platform_log_warning("warning: malformed section header at line %d, skipping\n", line_number);
        // Reset to default section to continue processing
        strcpy(current_section, "onvif");
        current_section_type = CONFIG_SECTION_ONVIF;
        current_section_known = 1;
      }
      continue;
    }
    
    // Parse key=value pairs
    char *eq = strchr(line, '=');
    if (!eq) {
      platform_log_warning("warning: malformed key=value pair at line %d, skipping\n", line_number);
      continue;
    }
    
    *eq = '\0';
    char key[MAX_KEY_LENGTH], value[MAX_VALUE_LENGTH];
    
    // Safely copy key
    size_t key_len = strlen(line);
    if (key_len >= sizeof(key)) {
      platform_log_warning("warning: key too long at line %d, truncating\n", line_number);
      key_len = sizeof(key) - 1;
    }
    strncpy(key, line, key_len);
    key[key_len] = '\0';
    
    // Safely copy value
    size_t value_len = strlen(eq + 1);
    if (value_len >= sizeof(value)) {
      platform_log_warning("warning: value too long at line %d, truncating\n", line_number);
      value_len = sizeof(value) - 1;
    }
    strncpy(value, eq + 1, value_len);
    value[value_len] = '\0';
    
    // Strip inline comments starting with ';' or '#'
    char *comment = strpbrk(value, ";#");
    if (comment) {
      *comment = '\0';
    }
    
    trim_whitespace(key);
    trim_whitespace(value);
    
    // Skip empty keys or values
    if (strlen(key) == 0) {
      platform_log_warning("warning: empty key at line %d, skipping\n", line_number);
      continue;
    }
    
    // Find and set parameter
    if (current_section_known) {
      config_parameter_t *param = find_parameter(config, current_section_type, key);
      if (param) {
        set_parameter_value(param, value);
      } else {
        // Known section but unknown parameter
        platform_log_info("info: unknown parameter '%s' in section '[%s]' at line %d, skipping\n", 
                         key, current_section, line_number);
      }
    } else {
      // Unknown section: skip keys silently
    }
  }
  
  if (fp) {
    fclose(fp);
  }
  return ONVIF_SUCCESS;
}

/**
 * @brief Validate all configuration parameters
 * @param config Configuration manager
 * @return CONFIG_VALIDATION_OK if valid, error code if validation fails
 * @note Checks required parameters and value ranges
 */
config_validation_result_t config_validate(config_manager_t *config) {
  if (!config) {
    platform_log_error("error: config_validate() called with NULL config parameter\n");
    return CONFIG_VALIDATION_OK;
  }
  
  if (!config->validation_enabled) {
    platform_log_debug("debug: config validation is disabled\n");
    return CONFIG_VALIDATION_OK;
  }
  
  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t *param = &config->sections[i].parameters[j];
      
      if (param->required && !param->value_ptr) {
        platform_log_error("error: required parameter '%s' in section %d has no value pointer\n", 
                          param->key, config->sections[i].section);
        return CONFIG_VALIDATION_MISSING_REQUIRED;
      }
      
      if (param->type == CONFIG_TYPE_INT && param->value_ptr) {
        int value = *(int*)param->value_ptr;
        if (param->min_value != param->max_value && (value < param->min_value || value > param->max_value)) {
          platform_log_error("error: parameter '%s' value %d is out of range [%d, %d] in section %d\n", 
                            param->key, value, param->min_value, param->max_value, config->sections[i].section);
          return CONFIG_VALIDATION_OUT_OF_RANGE;
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
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR_NOT_FOUND on failure
 * @note Validates type compatibility before copying value
 */
int config_get_value(config_manager_t *config, config_section_t section,
                                const char *key, void *value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    platform_log_error("error: config_get_value() called with NULL parameter (config=%p, key=%p, value_ptr=%p)\n", 
                      config, key, value_ptr);
    return ONVIF_ERROR_INVALID;
  }
  
  config_parameter_t *param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    platform_log_error("error: parameter '%s' not found or type mismatch (expected %d, got %d) in section %d\n", 
                      key, value_type, param ? param->type : -1, section);
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  if (param->value_ptr) {
    memcpy(value_ptr, param->value_ptr, param->value_size);
    return ONVIF_SUCCESS;
  }
  
  platform_log_error("error: parameter '%s' has no value pointer in section %d\n", key, section);
  return ONVIF_ERROR_NOT_FOUND;
}

/**
 * @brief Set configuration value with validation
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to the value
 * @param value_type Value type
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR_NOT_FOUND on failure
 * @note Validates type compatibility before setting value
 */
int config_set_value(config_manager_t *config, config_section_t section,
                                const char *key, const void *value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    platform_log_error("error: config_set_value() called with NULL parameter (config=%p, key=%p, value_ptr=%p)\n", 
                      config, key, value_ptr);
    return ONVIF_ERROR_INVALID;
  }
  
  config_parameter_t *param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    platform_log_error("error: parameter '%s' not found or type mismatch (expected %d, got %d) in section %d\n", 
                      key, value_type, param ? param->type : -1, section);
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  if (param->value_ptr) {
    memcpy(param->value_ptr, value_ptr, param->value_size);
    return ONVIF_SUCCESS;
  }
  
  platform_log_error("error: parameter '%s' has no value pointer in section %d\n", key, section);
  return ONVIF_ERROR_NOT_FOUND;
}

/**
 * @brief Reset all configuration parameters to their default values
 * @param config Configuration manager
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Only resets parameters that have default values defined
 */
int config_reset_to_defaults(config_manager_t *config) {
  if (!config) {
    platform_log_error("error: config_reset_to_defaults() called with NULL config parameter\n");
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
const config_parameter_t *config_get_parameter(config_manager_t *config,
                                                          config_section_t section, const char *key) {
  if (!config || !key) {
    platform_log_error("error: config_get_parameter() called with NULL parameter (config=%p, key=%p)\n", 
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
void config_cleanup(config_manager_t *config) {
  if (config) {
    memset(config, 0, sizeof(config_manager_t));
  } else {
    platform_log_warning("warning: config_cleanup() called with NULL config parameter\n");
  }
}

/**
 * @brief Get configuration summary for logging and debugging
 * @param config Configuration manager
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID or ONVIF_ERROR on failure
 * @note Creates a human-readable summary of all configuration values
 */
int config_get_summary(config_manager_t *config, char *summary, size_t summary_size) {
  if (!config || !summary) {
    platform_log_error("error: config_get_summary() called with NULL parameter (config=%p, summary=%p)\n", 
                      config, summary);
    return ONVIF_ERROR_INVALID;
  }
  
  if (!config->app_config) {
    platform_log_error("error: app_config is NULL in config_get_summary()\n");
    return ONVIF_ERROR_INVALID;
  }
  
  int result = snprintf(summary, summary_size,
    "ONVIF: enabled=%d, port=%d, user=%s\n"
    "Imaging: brightness=%d, contrast=%d, saturation=%d, sharpness=%d, hue=%d\n"
    "Auto Day/Night: enabled=%d, mode=%d, thresholds=%d/%d, lock_time=%ds",
    config->app_config->onvif.enabled,
    config->app_config->onvif.http_port,
    config->app_config->onvif.username,
    config->app_config->imaging ? config->app_config->imaging->brightness : 0,
    config->app_config->imaging ? config->app_config->imaging->contrast : 0,
    config->app_config->imaging ? config->app_config->imaging->saturation : 0,
    config->app_config->imaging ? config->app_config->imaging->sharpness : 0,
    config->app_config->imaging ? config->app_config->imaging->hue : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->enable_auto_switching : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->mode : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->day_to_night_threshold : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->night_to_day_threshold : 0,
    config->app_config->auto_daynight ? config->app_config->auto_daynight->lock_time_seconds : 0);
  
  if (result < 0) {
    platform_log_error("error: snprintf failed in config_get_summary()\n");
    return ONVIF_ERROR;
  }
  
  if ((size_t)result >= summary_size) {
    platform_log_error("error: summary buffer too small (required=%d, available=%zu)\n", result, summary_size);
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}
