/**
 * @file config_mock.c
 * @brief Implementation of configuration manager mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "config_mock.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Mock state
static int g_config_mock_initialized = 0;  // NOLINT
static int g_config_validation_result = 0; // NOLINT

/**
 * @brief Create a mock configuration manager
 * @return Pointer to mock configuration manager
 */
config_manager_t* mock_config_manager_create(void) {
  config_manager_t* config = malloc(sizeof(config_manager_t));
  if (!config) {
    return NULL;
  }

  memset(config, 0, sizeof(config_manager_t));
  config->validation_enabled = 1;

  return config;
}

/**
 * @brief Destroy a mock configuration manager
 * @param config Configuration manager to destroy
 */
void mock_config_manager_destroy(config_manager_t* config) {
  if (config) {
    free(config);
  }
}

/**
 * @brief Mock set string configuration value
 * @param config Configuration manager
 * @param section Section name
 * @param key Key name
 * @param value String value
 * @return 0 on success
 */
int mock_config_set_string(config_manager_t* config, const char* section, const char* key,
                           const char* value) {
  (void)config;
  (void)section;
  (void)key;
  (void)value;
  return 0; // Always succeed in mock
}

/**
 * @brief Mock set integer configuration value
 * @param config Configuration manager
 * @param section Section name
 * @param key Key name
 * @param value Integer value
 * @return 0 on success
 */
int mock_config_set_int(config_manager_t* config, const char* section, const char* key, int value) {
  (void)config;
  (void)section;
  (void)key;
  (void)value;
  return 0; // Always succeed in mock
}

/**
 * @brief Mock get string configuration value
 * @param config Configuration manager
 * @param section Section name
 * @param key Key name
 * @param value Buffer to store value
 * @param size Size of buffer
 * @param default_value Default value if not found
 * @return 0 on success
 */
int mock_config_get_string(config_manager_t* config, const char* section, const char* key,
                           char* value, size_t size, const char* default_value) {
  (void)config;
  (void)section;
  (void)key;

  if (value && size > 0 && default_value) {
    strncpy(value, default_value, size - 1);
    value[size - 1] = '\0';
  }

  return 0; // Always succeed in mock
}

/**
 * @brief Mock get integer configuration value
 * @param config Configuration manager
 * @param section Section name
 * @param key Key name
 * @param value Pointer to store value
 * @param default_value Default value if not found
 * @return 0 on success
 */
int mock_config_get_int(config_manager_t* config, const char* section, const char* key, int* value,
                        int default_value) {
  (void)config;
  (void)section;
  (void)key;

  if (value) {
    *value = default_value;
  }

  return 0; // Always succeed in mock
}

/**
 * @brief Mock configuration validation
 * @param config Configuration manager
 * @return Validation result
 */
int mock_config_validate(config_manager_t* config) {
  (void)config;
  return g_config_validation_result;
}

/**
 * @brief Initialize configuration mock
 */
void config_mock_init(void) {
  g_config_mock_initialized = 1;
  g_config_validation_result = 0; // CONFIG_VALIDATION_OK
}

/**
 * @brief Cleanup configuration mock
 */
void config_mock_cleanup(void) {
  g_config_mock_initialized = 0;
  g_config_validation_result = 0;
}
