/**
 * @file centralized_config.h
 * @brief Centralized configuration management system with validation
 * 
 * This module provides a unified configuration management system that eliminates
 * hardcoded values and provides consistent default handling and validation.
 */

#ifndef ONVIF_CENTRALIZED_CONFIG_H
#define ONVIF_CENTRALIZED_CONFIG_H

#include "common/onvif_types.h"
#include "utils/constants_clean.h"
#include <stddef.h>

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
 * @brief Centralized configuration manager
 */
typedef struct {
  struct application_config *app_config;
  config_section_def_t *sections;
  size_t section_count;
  int validation_enabled;
} centralized_config_t;

/**
 * @brief Initialize centralized configuration system
 * @param config Configuration manager to initialize
 * @param app_config Application configuration structure
 * @return 0 on success, negative error code on failure
 */
int centralized_config_init(centralized_config_t *config, struct application_config *app_config);

/**
 * @brief Load configuration from file with validation
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return 0 on success, negative error code on failure
 */
int centralized_config_load(centralized_config_t *config, const char *config_file);

/**
 * @brief Save configuration to file
 * @param config Configuration manager
 * @param config_file Path to configuration file
 * @return 0 on success, negative error code on failure
 */
int centralized_config_save(centralized_config_t *config, const char *config_file);

/**
 * @brief Validate configuration values
 * @param config Configuration manager
 * @return CONFIG_VALIDATION_OK on success, validation error code on failure
 */
config_validation_result_t centralized_config_validate(centralized_config_t *config);

/**
 * @brief Get configuration value with type safety
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @param value_ptr Pointer to store the value
 * @param value_type Expected value type
 * @return 0 on success, negative error code on failure
 */
int centralized_config_get_value(centralized_config_t *config, config_section_t section,
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
int centralized_config_set_value(centralized_config_t *config, config_section_t section,
                                const char *key, const void *value_ptr, config_value_type_t value_type);

/**
 * @brief Reset configuration to defaults
 * @param config Configuration manager
 * @return 0 on success, negative error code on failure
 */
int centralized_config_reset_to_defaults(centralized_config_t *config);

/**
 * @brief Get configuration parameter definition
 * @param config Configuration manager
 * @param section Configuration section
 * @param key Parameter key
 * @return Parameter definition or NULL if not found
 */
const config_parameter_t *centralized_config_get_parameter(centralized_config_t *config,
                                                          config_section_t section, const char *key);

/**
 * @brief Clean up configuration manager
 * @param config Configuration manager to clean up
 */
void centralized_config_cleanup(centralized_config_t *config);

/**
 * @brief Register custom configuration section
 * @param config Configuration manager
 * @param section_def Section definition
 * @return 0 on success, negative error code on failure
 */
int centralized_config_register_section(centralized_config_t *config, const config_section_def_t *section_def);

/**
 * @brief Get configuration summary for logging
 * @param config Configuration manager
 * @param summary Buffer to store summary
 * @param summary_size Size of summary buffer
 * @return 0 on success, negative error code on failure
 */
int centralized_config_get_summary(centralized_config_t *config, char *summary, size_t summary_size);

#endif /* ONVIF_CENTRALIZED_CONFIG_H */
