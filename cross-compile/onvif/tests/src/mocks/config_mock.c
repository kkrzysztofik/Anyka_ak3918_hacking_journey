/**
 * @file config_mock.c
 * @brief Implementation of configuration manager mock functions using generic mock framework
 * @author kkrzysztofik
 * @date 2025
 */

#include "config_mock.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../common/generic_mock_framework.h"
#include "core/config/config.h"

/* ============================================================================
 * Mock Operations
 * ============================================================================ */

enum config_operations {
  CONFIG_OP_CREATE = 0,
  CONFIG_OP_DESTROY,
  CONFIG_OP_SET_STRING,
  CONFIG_OP_SET_INT,
  CONFIG_OP_GET_STRING,
  CONFIG_OP_GET_INT,
  CONFIG_OP_VALIDATE,
  CONFIG_OP_COUNT
};

/* ============================================================================
 * Mock Instance
 * ============================================================================ */

// Create mock instance using generic framework
GENERIC_MOCK_CREATE(config, CONFIG_OP_COUNT);

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

void config_mock_init(void) {
  generic_mock_init(&config_mock);

  // Set default results (all operations succeed)
  for (int i = 0; i < CONFIG_OP_COUNT; i++) {
    generic_mock_set_operation_result(&config_mock, i, 0);
  }
}

void config_mock_cleanup(void) {
  generic_mock_cleanup(&config_mock);
}

/* ============================================================================
 * Config Mock Functions
 * ============================================================================ */

config_manager_t* mock_config_manager_create(void) {
  (void)generic_mock_execute_operation(&config_mock, CONFIG_OP_CREATE, NULL);

  config_manager_t* config = malloc(sizeof(config_manager_t));
  if (!config) {
    return NULL;
  }

  memset(config, 0, sizeof(config_manager_t));
  config->validation_enabled = 1;

  return config;
}

void mock_config_manager_destroy(config_manager_t* config) {
  (void)generic_mock_execute_operation(&config_mock, CONFIG_OP_DESTROY, config);

  if (config) {
    free(config);
  }
}

int mock_config_set_string(config_manager_t* config, const char* section, const char* key,
                           const char* value) {
  (void)config;
  (void)section;
  (void)key;
  (void)value;

  return generic_mock_execute_operation(&config_mock, CONFIG_OP_SET_STRING, NULL);
}

int mock_config_set_int(config_manager_t* config, const char* section, const char* key, int value) {
  (void)config;
  (void)section;
  (void)key;
  (void)value;

  return generic_mock_execute_operation(&config_mock, CONFIG_OP_SET_INT, NULL);
}

int mock_config_get_string(config_manager_t* config, const char* section, const char* key,
                           char* value, size_t size, const char* default_value) {
  (void)config;
  (void)section;
  (void)key;

  int result = generic_mock_execute_operation(&config_mock, CONFIG_OP_GET_STRING, NULL);

  // Return default value
  if (value && size > 0 && default_value) {
    strncpy(value, default_value, size - 1);
    value[size - 1] = '\0';
  }

  return result;
}

int mock_config_get_int(config_manager_t* config, const char* section, const char* key, int* value,
                        int default_value) {
  (void)config;
  (void)section;
  (void)key;

  int result = generic_mock_execute_operation(&config_mock, CONFIG_OP_GET_INT, NULL);

  // Return default value
  if (value) {
    *value = default_value;
  }

  return result;
}

int mock_config_validate(config_manager_t* config) {
  (void)config;
  return generic_mock_execute_operation(&config_mock, CONFIG_OP_VALIDATE, config);
}
