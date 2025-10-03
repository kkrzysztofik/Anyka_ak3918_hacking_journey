/**
 * @file config_mock.c
 * @brief Implementation of CMocka-based configuration manager mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "config_mock.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

/* ============================================================================
 * CMocka Wrapped Configuration Functions
 * ============================================================================ */

int __wrap_config_init(config_manager_t* config, const char* config_file) {
  check_expected_ptr(config);
  check_expected_ptr(config_file);
  return (int)mock();
}

int __wrap_config_load(config_manager_t* config) {
  check_expected_ptr(config);
  return (int)mock();
}

config_validation_result_t __wrap_config_validate(const config_manager_t* config) {
  check_expected_ptr(config);
  return (config_validation_result_t)mock();
}

int __wrap_config_get_value(const config_manager_t* config, config_section_t section,
                             const char* key, void* value, size_t value_size) {
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(value_size);
  return (int)mock();
}

int __wrap_config_set_value(config_manager_t* config, config_section_t section, const char* key,
                             const void* value, size_t value_size) {
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(value_size);
  return (int)mock();
}

int __wrap_config_reset_to_defaults(config_manager_t* config) {
  check_expected_ptr(config);
  return (int)mock();
}

const config_parameter_t* __wrap_config_get_parameter(const config_manager_t* config,
                                                       config_section_t section, const char* key) {
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  return (const config_parameter_t*)mock();
}

void __wrap_config_cleanup(config_manager_t* config) {
  check_expected_ptr(config);
}

int __wrap_config_get_summary(const config_manager_t* config, char* buffer, size_t buffer_size) {
  check_expected_ptr(config);
  check_expected_ptr(buffer);
  check_expected(buffer_size);
  return (int)mock();
}

