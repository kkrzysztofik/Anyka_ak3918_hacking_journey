/**
 * @file config_mock.c
 * @brief Implementation of CMocka-based configuration manager mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "config_mock.h"

#include <stdbool.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void config_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * CMocka Wrapped Configuration Functions
 * ============================================================================ */

int __wrap_config_init(config_manager_t* config, const char* config_file) {
  // Note: config_init doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected_ptr(config_file);
  return (int)mock();
}

int __wrap_config_load(config_manager_t* config) {
  // Note: config_load doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  return (int)mock();
}

config_validation_result_t __wrap_config_validate(const config_manager_t* config) {
  // config_validate doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  return (config_validation_result_t)mock();
}

int __wrap_config_get_value(const config_manager_t* config, config_section_t section,
                            const char* key, void* value, size_t value_size) {
  // Note: config_get_value doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(value_size);
  return (int)mock();
}

int __wrap_config_set_value(config_manager_t* config, config_section_t section, const char* key,
                            const void* value, size_t value_size) {
  // Note: config_set_value doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(value_size);
  return (int)mock();
}

int __wrap_config_reset_to_defaults(config_manager_t* config) {
  // Note: config_reset_to_defaults doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  return (int)mock();
}

const config_parameter_t* __wrap_config_get_parameter(const config_manager_t* config,
                                                      config_section_t section, const char* key) {
  // Note: config_get_parameter doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  return (const config_parameter_t*)mock();
}

void __wrap_config_cleanup(config_manager_t* config) {
  // config_cleanup doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
}

int __wrap_config_get_summary(const config_manager_t* config, char* buffer, size_t buffer_size) {
  // Note: config_get_summary doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected_ptr(buffer);
  check_expected(buffer_size);
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped Runtime Configuration Functions
 * ============================================================================ */

/* Declare real functions for runtime configuration */
extern int __real_config_runtime_get_int(config_section_t section, const char* key, int* out_value);
extern int __real_config_runtime_set_int(config_section_t section, const char* key, int value);
extern int __real_config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size);
extern int __real_config_runtime_set_string(config_section_t section, const char* key, const char* value);

int __wrap_config_runtime_get_int(config_section_t section, const char* key, int* out_value) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_int(section, key, out_value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(out_value);
  return (int)mock();
}

int __wrap_config_runtime_set_int(config_section_t section, const char* key, int value) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_int(section, key, value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected(value);
  return (int)mock();
}

int __wrap_config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_string(section, key, out_value, buffer_size);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(out_value);
  check_expected(buffer_size);
  return (int)mock();
}

int __wrap_config_runtime_set_string(config_section_t section, const char* key, const char* value) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_string(section, key, value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  return (int)mock();
}
