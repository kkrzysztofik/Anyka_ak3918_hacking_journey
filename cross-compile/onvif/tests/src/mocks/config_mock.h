/**
 * @file config_mock.h
 * @brief Header for configuration manager mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef CONFIG_MOCK_H
#define CONFIG_MOCK_H

#include "core/config/config.h"

// Mock configuration manager functions
config_manager_t* mock_config_manager_create(void);
void mock_config_manager_destroy(config_manager_t* config);

// Mock configuration set functions
int mock_config_set_string(config_manager_t* config, const char* section, const char* key,
                           const char* value);
int mock_config_set_int(config_manager_t* config, const char* section, const char* key, int value);

// Mock configuration get functions
int mock_config_get_string(config_manager_t* config, const char* section, const char* key,
                           char* value, size_t size, const char* default_value);
int mock_config_get_int(config_manager_t* config, const char* section, const char* key, int* value,
                        int default_value);

// Mock configuration validation functions
int mock_config_validate(config_manager_t* config);

// Mock configuration initialization
void config_mock_init(void);
void config_mock_cleanup(void);

#endif // CONFIG_MOCK_H
