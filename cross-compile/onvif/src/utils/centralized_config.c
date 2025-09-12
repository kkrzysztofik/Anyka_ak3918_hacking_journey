/**
 * @file centralized_config.c
 * @brief Centralized configuration management system implementation
 */

#include "centralized_config.h"
#include "utils/error_handling.h"
#include "utils/safe_string.h"
#include "utils/config.h"
#include "common/onvif_imaging_types.h"
#include "platform/platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <errno.h>

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

// Configuration parameter definitions
static config_parameter_t onvif_parameters[] = {
  {"enabled", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 1},
  {"http_port", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 65535, "8080", 1},
  {"username", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "admin", 1},
  {"password", CONFIG_TYPE_STRING, NULL, 64, 0, 0, "admin", 1}
};

static config_parameter_t imaging_parameters[] = {
  {"brightness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"contrast", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"saturation", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"sharpness", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "50", 0},
  {"hue", CONFIG_TYPE_INT, NULL, sizeof(int), -180, 180, "0", 0}
};

static config_parameter_t auto_daynight_parameters[] = {
  {"enable_auto_switching", CONFIG_TYPE_BOOL, NULL, sizeof(int), 0, 1, "1", 0},
  {"mode", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 2, "0", 0},
  {"day_to_night_threshold", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "30", 0},
  {"night_to_day_threshold", CONFIG_TYPE_INT, NULL, sizeof(int), 0, 100, "70", 0},
  {"lock_time_seconds", CONFIG_TYPE_INT, NULL, sizeof(int), 1, 3600, "5", 0}
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

static void trim_whitespace(char *s) {
  char *start = s;
  char *end = s + strlen(s) - 1;
  
  // Trim leading whitespace
  while (isspace((unsigned char)*start)) start++;
  
  // Trim trailing whitespace
  while (end >= start && isspace((unsigned char)*end)) end--;
  
  // Move trimmed string to beginning
  if (start != s) {
    memmove(s, start, end - start + 1);
  }
  s[end - start + 1] = '\0';
}

static int key_equals(const char *a, const char *b) {
  while (*a && *b) {
    if (tolower((unsigned char)*a) != tolower((unsigned char)*b)) {
      return 0;
    }
    a++;
    b++;
  }
  return *a == '\0' && *b == '\0';
}

static config_section_t parse_section_name(const char *section_name) {
  for (size_t i = 0; i < sizeof(default_sections) / sizeof(default_sections[0]); i++) {
    if (key_equals(section_name, default_sections[i].section_name)) {
      return default_sections[i].section;
    }
  }
  return CONFIG_SECTION_ONVIF; // Default
}

static config_parameter_t *find_parameter(centralized_config_t *config, config_section_t section, const char *key) {
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

static void set_parameter_value(config_parameter_t *param, const char *value) {
  if (!param || !value) return;
  
  switch (param->type) {
    case CONFIG_TYPE_INT: {
      int int_val = atoi(value);
      if (param->min_value != param->max_value) {
        if (int_val < param->min_value) int_val = param->min_value;
        if (int_val > param->max_value) int_val = param->max_value;
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
      safe_strcpy((char*)param->value_ptr, param->value_size, value);
      break;
    }
    case CONFIG_TYPE_FLOAT: {
      float float_val = (float)atof(value);
      *(float*)param->value_ptr = float_val;
      break;
    }
  }
}

static void set_default_values(centralized_config_t *config) {
  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t *param = &config->sections[i].parameters[j];
      if (param->default_value) {
        set_parameter_value(param, param->default_value);
      }
    }
  }
}

static void link_parameters_to_config(centralized_config_t *config) {
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
  
  // Link auto day/night parameters
  if (config->app_config->auto_daynight) {
    auto_daynight_parameters[0].value_ptr = &config->app_config->auto_daynight->enable_auto_switching;
    auto_daynight_parameters[1].value_ptr = &config->app_config->auto_daynight->mode;
    auto_daynight_parameters[2].value_ptr = &config->app_config->auto_daynight->day_to_night_threshold;
    auto_daynight_parameters[3].value_ptr = &config->app_config->auto_daynight->night_to_day_threshold;
    auto_daynight_parameters[4].value_ptr = &config->app_config->auto_daynight->lock_time_seconds;
  }
}

int centralized_config_init(centralized_config_t *config, struct application_config *app_config) {
  if (!config || !app_config) {
    return ONVIF_ERROR_INVALID;
  }
  
  memset(config, 0, sizeof(centralized_config_t));
  config->app_config = app_config;
  config->sections = default_sections;
  config->section_count = sizeof(default_sections) / sizeof(default_sections[0]);
  config->validation_enabled = 1;
  
  // Link parameters to configuration structure
  link_parameters_to_config(config);
  
  // Set default values
  set_default_values(config);
  
  return ONVIF_SUCCESS;
}

int centralized_config_load(centralized_config_t *config, const char *config_file) {
  if (!config || !config_file) {
    return ONVIF_ERROR_INVALID;
  }
  
  FILE *fp = fopen(config_file, "r");
  if (!fp) {
    // Try alternate filename
    if (strcmp(config_file, ONVIF_CONFIG_FILE) == 0) {
      fp = fopen("/etc/jffs2/anyka_cfg.ini", "r");
    }
    if (!fp) {
      platform_log_warning("warning: could not open %s: %s (using defaults)\n", 
                          config_file, strerror(errno));
      return -1;
    }
  }
  
  char line[512];
  char current_section[128] = "";
  config_section_t current_section_type = CONFIG_SECTION_ONVIF;
  
  while (fgets(line, sizeof(line), fp)) {
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
        if (len >= sizeof(current_section)) len = sizeof(current_section) - 1;
        strncpy(current_section, line + 1, len);
        current_section[len] = '\0';
        trim_whitespace(current_section);
        current_section_type = parse_section_name(current_section);
      }
      continue;
    }
    
    // Parse key=value pairs
    char *eq = strchr(line, '=');
    if (!eq) continue;
    
    *eq = '\0';
    char key[128], value[256];
    strncpy(key, line, sizeof(key) - 1);
    key[sizeof(key) - 1] = '\0';
    strncpy(value, eq + 1, sizeof(value) - 1);
    value[sizeof(value) - 1] = '\0';
    trim_whitespace(key);
    trim_whitespace(value);
    
    // Find and set parameter
    config_parameter_t *param = find_parameter(config, current_section_type, key);
    if (param) {
      set_parameter_value(param, value);
    }
  }
  
  fclose(fp);
  return ONVIF_SUCCESS;
}

config_validation_result_t centralized_config_validate(centralized_config_t *config) {
  if (!config || !config->validation_enabled) {
    return CONFIG_VALIDATION_OK;
  }
  
  for (size_t i = 0; i < config->section_count; i++) {
    for (size_t j = 0; j < config->sections[i].parameter_count; j++) {
      config_parameter_t *param = &config->sections[i].parameters[j];
      
      if (param->required && !param->value_ptr) {
        return CONFIG_VALIDATION_MISSING_REQUIRED;
      }
      
      if (param->type == CONFIG_TYPE_INT && param->value_ptr) {
        int value = *(int*)param->value_ptr;
        if (param->min_value != param->max_value && (value < param->min_value || value > param->max_value)) {
          return CONFIG_VALIDATION_OUT_OF_RANGE;
        }
      }
    }
  }
  
  return CONFIG_VALIDATION_OK;
}

int centralized_config_get_value(centralized_config_t *config, config_section_t section,
                                const char *key, void *value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }
  
  config_parameter_t *param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  if (param->value_ptr) {
    memcpy(value_ptr, param->value_ptr, param->value_size);
    return ONVIF_SUCCESS;
  }
  
  return ONVIF_ERROR_NOT_FOUND;
}

int centralized_config_set_value(centralized_config_t *config, config_section_t section,
                                const char *key, const void *value_ptr, config_value_type_t value_type) {
  if (!config || !key || !value_ptr) {
    return ONVIF_ERROR_INVALID;
  }
  
  config_parameter_t *param = find_parameter(config, section, key);
  if (!param || param->type != value_type) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  if (param->value_ptr) {
    memcpy(param->value_ptr, value_ptr, param->value_size);
    return ONVIF_SUCCESS;
  }
  
  return ONVIF_ERROR_NOT_FOUND;
}

int centralized_config_reset_to_defaults(centralized_config_t *config) {
  if (!config) {
    return ONVIF_ERROR_INVALID;
  }
  
  set_default_values(config);
  return ONVIF_SUCCESS;
}

const config_parameter_t *centralized_config_get_parameter(centralized_config_t *config,
                                                          config_section_t section, const char *key) {
  if (!config || !key) {
    return NULL;
  }
  
  return find_parameter(config, section, key);
}

void centralized_config_cleanup(centralized_config_t *config) {
  if (config) {
    memset(config, 0, sizeof(centralized_config_t));
  }
}

int centralized_config_get_summary(centralized_config_t *config, char *summary, size_t summary_size) {
  if (!config || !summary) {
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
  
  return (result < 0 || (size_t)result >= summary_size) ? ONVIF_ERROR : ONVIF_SUCCESS;
}
