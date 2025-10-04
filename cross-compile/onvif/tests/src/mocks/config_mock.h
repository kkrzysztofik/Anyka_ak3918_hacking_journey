/**
 * @file config_mock.h
 * @brief CMocka-based configuration manager mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef CONFIG_MOCK_H
#define CONFIG_MOCK_H

#include <stdbool.h>

#include "cmocka_wrapper.h"
#include "core/config/config.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * CMocka Wrapped Configuration Functions
 * ============================================================================
 * All configuration functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped configuration initialization
 * @param config Configuration manager pointer
 * @param config_file Configuration file path
 * @return Result code (configured via will_return)
 */
int __wrap_config_init(config_manager_t* config, const char* config_file);

/**
 * @brief CMocka wrapped configuration load
 * @param config Configuration manager pointer
 * @return Result code (configured via will_return)
 */
int __wrap_config_load(config_manager_t* config);

/**
 * @brief CMocka wrapped configuration validation
 * @param config Configuration manager pointer
 * @return Validation result (configured via will_return)
 */
config_validation_result_t __wrap_config_validate(const config_manager_t* config);

/**
 * @brief CMocka wrapped configuration get value
 * @param config Configuration manager pointer
 * @param section Configuration section
 * @param key Configuration key
 * @param value Output value pointer
 * @param value_size Size of value buffer
 * @return Result code (configured via will_return)
 */
int __wrap_config_get_value(const config_manager_t* config, config_section_t section,
                            const char* key, void* value, size_t value_size);

/**
 * @brief CMocka wrapped configuration set value
 * @param config Configuration manager pointer
 * @param section Configuration section
 * @param key Configuration key
 * @param value Value pointer
 * @param value_size Size of value
 * @return Result code (configured via will_return)
 */
int __wrap_config_set_value(config_manager_t* config, config_section_t section, const char* key,
                            const void* value, size_t value_size);

/**
 * @brief CMocka wrapped configuration reset to defaults
 * @param config Configuration manager pointer
 * @return Result code (configured via will_return)
 */
int __wrap_config_reset_to_defaults(config_manager_t* config);

/**
 * @brief CMocka wrapped configuration get parameter
 * @param config Configuration manager pointer
 * @param section Configuration section
 * @param key Configuration key
 * @return Parameter pointer (configured via will_return)
 */
const config_parameter_t* __wrap_config_get_parameter(const config_manager_t* config,
                                                      config_section_t section, const char* key);

/**
 * @brief CMocka wrapped configuration cleanup
 * @param config Configuration manager pointer
 */
void __wrap_config_cleanup(config_manager_t* config);

/**
 * @brief CMocka wrapped configuration get summary
 * @param config Configuration manager pointer
 * @param buffer Output buffer
 * @param buffer_size Size of buffer
 * @return Result code (configured via will_return)
 */
int __wrap_config_get_summary(const config_manager_t* config, char* buffer, size_t buffer_size);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void config_mock_use_real_function(bool use_real);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful configuration initialization
 */
#define EXPECT_CONFIG_INIT_SUCCESS()                                                               \
  expect_any(__wrap_config_init, config);                                                          \
  expect_any(__wrap_config_init, config_file);                                                     \
  will_return(__wrap_config_init, 0)

/**
 * @brief Set up expectations for configuration initialization failure
 * @param error_code Error code to return
 */
#define EXPECT_CONFIG_INIT_ERROR(error_code)                                                       \
  expect_any(__wrap_config_init, config);                                                          \
  expect_any(__wrap_config_init, config_file);                                                     \
  will_return(__wrap_config_init, error_code)

/**
 * @brief Set up expectations for successful configuration load
 */
#define EXPECT_CONFIG_LOAD_SUCCESS()                                                               \
  expect_any(__wrap_config_load, config);                                                          \
  will_return(__wrap_config_load, 0)

/**
 * @brief Set up expectations for successful configuration validation
 */
#define EXPECT_CONFIG_VALIDATE_SUCCESS()                                                           \
  expect_any(__wrap_config_validate, config);                                                      \
  will_return(__wrap_config_validate, CONFIG_VALIDATION_OK)

/**
 * @brief Set up expectations for configuration validation failure
 * @param result Validation result to return
 */
#define EXPECT_CONFIG_VALIDATE_ERROR(result)                                                       \
  expect_any(__wrap_config_validate, config);                                                      \
  will_return(__wrap_config_validate, result)

/**
 * @brief Set up expectations for successful configuration get value
 * @param sect Configuration section
 * @param k Configuration key
 */
#define EXPECT_CONFIG_GET_VALUE_SUCCESS(sect, k)                                                   \
  expect_any(__wrap_config_get_value, config);                                                     \
  expect_value(__wrap_config_get_value, section, sect);                                            \
  expect_string(__wrap_config_get_value, key, k);                                                  \
  expect_any(__wrap_config_get_value, value);                                                      \
  expect_any(__wrap_config_get_value, value_size);                                                 \
  will_return(__wrap_config_get_value, 0)

/**
 * @brief Set up expectations for successful configuration set value
 * @param sect Configuration section
 * @param k Configuration key
 */
#define EXPECT_CONFIG_SET_VALUE_SUCCESS(sect, k)                                                   \
  expect_any(__wrap_config_set_value, config);                                                     \
  expect_value(__wrap_config_set_value, section, sect);                                            \
  expect_string(__wrap_config_set_value, key, k);                                                  \
  expect_any(__wrap_config_set_value, value);                                                      \
  expect_any(__wrap_config_set_value, value_size);                                                 \
  will_return(__wrap_config_set_value, 0)

/**
 * @brief Set up expectations for successful configuration cleanup
 */
#define EXPECT_CONFIG_CLEANUP() expect_any(__wrap_config_cleanup, config)

#ifdef __cplusplus
}
#endif

#endif // CONFIG_MOCK_H
