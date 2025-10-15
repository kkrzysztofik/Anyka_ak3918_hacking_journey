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
#include "core/config/config_runtime.h"

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
 * CMocka Wrapped Runtime Configuration Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_get_int
 * @param section Configuration section
 * @param key Configuration key
 * @param out_value Output integer value
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_int(config_section_t section, const char* key, int* out_value);

/**
 * @brief CMocka wrapped config_runtime_set_int
 * @param section Configuration section
 * @param key Configuration key
 * @param value Integer value to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_int(config_section_t section, const char* key, int value);

/**
 * @brief CMocka wrapped config_runtime_get_string
 * @param section Configuration section
 * @param key Configuration key
 * @param out_value Output string buffer
 * @param buffer_size Size of output buffer
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size);

/**
 * @brief CMocka wrapped config_runtime_set_string
 * @param section Configuration section
 * @param key Configuration key
 * @param value String value to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_string(config_section_t section, const char* key, const char* value);

/**
 * @brief CMocka wrapped config_runtime_get_bool
 * @param section Configuration section
 * @param key Configuration key
 * @param out_value Output boolean value
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_bool(config_section_t section, const char* key, int* out_value);

/**
 * @brief CMocka wrapped config_runtime_set_bool
 * @param section Configuration section
 * @param key Configuration key
 * @param value Boolean value to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_bool(config_section_t section, const char* key, int value);

/**
 * @brief CMocka wrapped config_runtime_get_float
 * @param section Configuration section
 * @param key Configuration key
 * @param out_value Output float value
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_float(config_section_t section, const char* key, float* out_value);

/**
 * @brief CMocka wrapped config_runtime_set_float
 * @param section Configuration section
 * @param key Configuration key
 * @param value Float value to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_float(config_section_t section, const char* key, float value);

/* ============================================================================
 * CMocka Wrapped Core Configuration Management Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_init
 * @param cfg Application configuration structure
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_init(struct application_config* cfg);

/**
 * @brief CMocka wrapped config_runtime_cleanup
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_cleanup(void);

/**
 * @brief CMocka wrapped config_runtime_apply_defaults
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_apply_defaults(void);

/**
 * @brief CMocka wrapped config_runtime_snapshot
 * @return Application configuration snapshot (configured via will_return)
 */
const struct application_config* __wrap_config_runtime_snapshot(void);

/**
 * @brief CMocka wrapped config_runtime_get_generation
 * @return Generation counter (configured via will_return)
 */
uint32_t __wrap_config_runtime_get_generation(void);

/* ============================================================================
 * CMocka Wrapped Persistence Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_queue_persistence_update
 * @param section Configuration section
 * @param key Configuration key
 * @param value Value to persist
 * @param type Value type
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_queue_persistence_update(config_section_t section, const char* key,
                                                   const void* value, config_value_type_t type);

/**
 * @brief CMocka wrapped config_runtime_process_persistence_queue
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_process_persistence_queue(void);

/**
 * @brief CMocka wrapped config_runtime_get_persistence_status
 * @return Status code (configured via will_return)
 */
int __wrap_config_runtime_get_persistence_status(void);

/* ============================================================================
 * CMocka Wrapped Stream Profile Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_get_stream_profile
 * @param profile_index Profile index (0-3)
 * @param profile Output profile configuration
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_stream_profile(int profile_index, video_config_t* profile);

/**
 * @brief CMocka wrapped config_runtime_set_stream_profile
 * @param profile_index Profile index (0-3)
 * @param profile Profile configuration to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_stream_profile(int profile_index, const video_config_t* profile);

/**
 * @brief CMocka wrapped config_runtime_validate_stream_profile
 * @param profile Profile configuration to validate
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_validate_stream_profile(const video_config_t* profile);

/**
 * @brief CMocka wrapped config_runtime_get_stream_profile_count
 * @return Profile count (configured via will_return)
 */
int __wrap_config_runtime_get_stream_profile_count(void);

/* ============================================================================
 * CMocka Wrapped PTZ Preset Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_get_ptz_profile_presets
 * @param profile_index PTZ profile index (1-4)
 * @param presets Output preset list
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_get_ptz_profile_presets(int profile_index, ptz_preset_list_t* presets);

/**
 * @brief CMocka wrapped config_runtime_set_ptz_profile_presets
 * @param profile_index PTZ profile index (1-4)
 * @param presets Preset list to set
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_set_ptz_profile_presets(int profile_index,
                                                  const ptz_preset_list_t* presets);

/**
 * @brief CMocka wrapped config_runtime_validate_ptz_profile_presets
 * @param presets Preset list to validate
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_validate_ptz_profile_presets(const ptz_preset_list_t* presets);

/* ============================================================================
 * CMocka Wrapped User Management Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped config_runtime_hash_password
 * @param password Password to hash
 * @param hash_output Output hash buffer
 * @param output_size Size of output buffer
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_hash_password(const char* password, char* hash_output, size_t output_size);

/**
 * @brief CMocka wrapped config_runtime_verify_password
 * @param password Password to verify
 * @param hash Hash to verify against
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_verify_password(const char* password, const char* hash);

/**
 * @brief CMocka wrapped config_runtime_add_user
 * @param username Username to add
 * @param password Password for user
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_add_user(const char* username, const char* password);

/**
 * @brief CMocka wrapped config_runtime_remove_user
 * @param username Username to remove
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_remove_user(const char* username);

/**
 * @brief CMocka wrapped config_runtime_update_user_password
 * @param username Username to update
 * @param new_password New password
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_update_user_password(const char* username, const char* new_password);

/**
 * @brief CMocka wrapped config_runtime_authenticate_user
 * @param username Username to authenticate
 * @param password Password to verify
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_authenticate_user(const char* username, const char* password);

/**
 * @brief CMocka wrapped config_runtime_enumerate_users
 * @param usernames Output usernames array
 * @param max_users Maximum users to enumerate
 * @param user_count Output user count
 * @return Result code (configured via will_return)
 */
int __wrap_config_runtime_enumerate_users(char usernames[][MAX_USERNAME_LENGTH + 1], int max_users,
                                          int* user_count);

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
