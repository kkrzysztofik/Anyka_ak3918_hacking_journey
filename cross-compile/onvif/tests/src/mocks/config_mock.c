/**
 * @file config_mock.c
 * @brief Implementation of CMocka-based configuration manager mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "config_mock.h"

#include <stdbool.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

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

int __wrap_config_get_value(const config_manager_t* config, config_section_t section, const char* key, void* value, size_t value_size) {
  // Note: config_get_value doesn't exist in main project, so we can't call real function
  check_expected_ptr(config);
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(value_size);
  return (int)mock();
}

int __wrap_config_set_value(config_manager_t* config, config_section_t section, const char* key, const void* value, size_t value_size) {
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

const config_parameter_t* __wrap_config_get_parameter(const config_manager_t* config, config_section_t section, const char* key) {
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
extern int __real_config_runtime_is_initialized(void);

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

/* Declare real functions for boolean and float configuration */
extern int __real_config_runtime_get_bool(config_section_t section, const char* key, int* out_value);
extern int __real_config_runtime_set_bool(config_section_t section, const char* key, int value);
extern int __real_config_runtime_get_float(config_section_t section, const char* key, float* out_value);
extern int __real_config_runtime_set_float(config_section_t section, const char* key, float value);

int __wrap_config_runtime_get_bool(config_section_t section, const char* key, int* out_value) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_bool(section, key, out_value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(out_value);
  return (int)mock();
}

int __wrap_config_runtime_set_bool(config_section_t section, const char* key, int value) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_bool(section, key, value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected(value);
  return (int)mock();
}

int __wrap_config_runtime_get_float(config_section_t section, const char* key, float* out_value) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_float(section, key, out_value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(out_value);
  return (int)mock();
}

int __wrap_config_runtime_set_float(config_section_t section, const char* key, float value) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_float(section, key, value);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  // Note: float values are passed as int via cmocka, so we check as int
  check_expected(*(int*)&value);
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped Core Configuration Management Functions
 * ============================================================================ */

/* Declare real functions for core configuration management */
extern int __real_config_runtime_init(struct application_config* cfg);
extern int __real_config_runtime_cleanup(void);
extern int __real_config_runtime_apply_defaults(void);
extern const struct application_config* __real_config_runtime_snapshot(void);
extern uint32_t __real_config_runtime_get_generation(void);

int __wrap_config_runtime_init(struct application_config* cfg) {
  if (g_use_real_functions) {
    return __real_config_runtime_init(cfg);
  }
  function_called();
  check_expected_ptr(cfg);
  return (int)mock();
}

int __wrap_config_runtime_cleanup(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_cleanup();
  }
  function_called();
  return (int)mock();
}

int __wrap_config_runtime_is_initialized(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_is_initialized();
  }
  function_called();
  return (int)mock();
}

int __wrap_config_runtime_apply_defaults(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_apply_defaults();
  }
  function_called();
  return (int)mock();
}

const struct application_config* __wrap_config_runtime_snapshot(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_snapshot();
  }
  function_called();
  return (const struct application_config*)mock();
}

uint32_t __wrap_config_runtime_get_generation(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_generation();
  }
  function_called();
  return (uint32_t)mock();
}

/* ============================================================================
 * CMocka Wrapped Persistence Functions
 * ============================================================================ */

/* Declare real functions for persistence operations */
extern int __real_config_runtime_queue_persistence_update(config_section_t section, const char* key, const void* value, config_value_type_t type);
extern int __real_config_runtime_process_persistence_queue(void);
extern int __real_config_runtime_get_persistence_status(void);

int __wrap_config_runtime_queue_persistence_update(config_section_t section, const char* key, const void* value, config_value_type_t type) {
  if (g_use_real_functions) {
    return __real_config_runtime_queue_persistence_update(section, key, value, type);
  }
  function_called();
  check_expected(section);
  check_expected_ptr(key);
  check_expected_ptr(value);
  check_expected(type);
  return (int)mock();
}

int __wrap_config_runtime_process_persistence_queue(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_process_persistence_queue();
  }
  function_called();
  return (int)mock();
}

int __wrap_config_runtime_get_persistence_status(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_persistence_status();
  }
  function_called();
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped Stream Profile Functions
 * ============================================================================ */

/* Declare real functions for stream profile operations */
extern int __real_config_runtime_get_stream_profile(int profile_index, video_config_t* profile);
extern int __real_config_runtime_set_stream_profile(int profile_index, const video_config_t* profile);
extern int __real_config_runtime_validate_stream_profile(const video_config_t* profile);
extern int __real_config_runtime_get_stream_profile_count(void);

int __wrap_config_runtime_get_stream_profile(int profile_index, video_config_t* profile) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_stream_profile(profile_index, profile);
  }
  function_called();
  check_expected(profile_index);
  check_expected_ptr(profile);
  return (int)mock();
}

int __wrap_config_runtime_set_stream_profile(int profile_index, const video_config_t* profile) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_stream_profile(profile_index, profile);
  }
  function_called();
  check_expected(profile_index);
  check_expected_ptr(profile);
  return (int)mock();
}

int __wrap_config_runtime_validate_stream_profile(const video_config_t* profile) {
  if (g_use_real_functions) {
    return __real_config_runtime_validate_stream_profile(profile);
  }
  function_called();
  check_expected_ptr(profile);
  return (int)mock();
}

int __wrap_config_runtime_get_stream_profile_count(void) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_stream_profile_count();
  }
  function_called();
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped PTZ Preset Functions
 * ============================================================================ */

/* Declare real functions for PTZ configuration */
extern int __real_config_runtime_get_ptz_profile_presets(int profile_index, ptz_preset_list_t* presets);
extern int __real_config_runtime_set_ptz_profile_presets(int profile_index, const ptz_preset_list_t* presets);
extern int __real_config_runtime_validate_ptz_profile_presets(const ptz_preset_list_t* presets);

int __wrap_config_runtime_get_ptz_profile_presets(int profile_index, ptz_preset_list_t* presets) {
  if (g_use_real_functions) {
    return __real_config_runtime_get_ptz_profile_presets(profile_index, presets);
  }
  function_called();
  check_expected(profile_index);
  check_expected_ptr(presets);
  return (int)mock();
}

int __wrap_config_runtime_set_ptz_profile_presets(int profile_index, const ptz_preset_list_t* presets) {
  if (g_use_real_functions) {
    return __real_config_runtime_set_ptz_profile_presets(profile_index, presets);
  }
  function_called();
  check_expected(profile_index);
  check_expected_ptr(presets);
  return (int)mock();
}

int __wrap_config_runtime_validate_ptz_profile_presets(const ptz_preset_list_t* presets) {
  if (g_use_real_functions) {
    return __real_config_runtime_validate_ptz_profile_presets(presets);
  }
  function_called();
  check_expected_ptr(presets);
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped User Management Functions
 * ============================================================================ */

/* Declare real functions for user management */
extern int __real_config_runtime_hash_password(const char* password, char* hash_output, size_t output_size);
extern int __real_config_runtime_verify_password(const char* password, const char* hash);
extern int __real_config_runtime_add_user(const char* username, const char* password);
extern int __real_config_runtime_remove_user(const char* username);
extern int __real_config_runtime_update_user_password(const char* username, const char* new_password);
extern int __real_config_runtime_authenticate_user(const char* username, const char* password);
extern int __real_config_runtime_enumerate_users(char usernames[][MAX_USERNAME_LENGTH + 1], int max_users, int* user_count);

int __wrap_config_runtime_hash_password(const char* password, char* hash_output, size_t output_size) {
  if (g_use_real_functions) {
    return __real_config_runtime_hash_password(password, hash_output, output_size);
  }
  function_called();
  check_expected_ptr(password);
  check_expected_ptr(hash_output);
  check_expected(output_size);
  return (int)mock();
}

int __wrap_config_runtime_verify_password(const char* password, const char* hash) {
  if (g_use_real_functions) {
    return __real_config_runtime_verify_password(password, hash);
  }
  function_called();
  check_expected_ptr(password);
  check_expected_ptr(hash);
  return (int)mock();
}

int __wrap_config_runtime_add_user(const char* username, const char* password) {
  if (g_use_real_functions) {
    return __real_config_runtime_add_user(username, password);
  }
  function_called();
  check_expected_ptr(username);
  check_expected_ptr(password);
  return (int)mock();
}

int __wrap_config_runtime_remove_user(const char* username) {
  if (g_use_real_functions) {
    return __real_config_runtime_remove_user(username);
  }
  function_called();
  check_expected_ptr(username);
  return (int)mock();
}

int __wrap_config_runtime_update_user_password(const char* username, const char* new_password) {
  if (g_use_real_functions) {
    return __real_config_runtime_update_user_password(username, new_password);
  }
  function_called();
  check_expected_ptr(username);
  check_expected_ptr(new_password);
  return (int)mock();
}

int __wrap_config_runtime_authenticate_user(const char* username, const char* password) {
  if (g_use_real_functions) {
    return __real_config_runtime_authenticate_user(username, password);
  }
  function_called();
  check_expected_ptr(username);
  check_expected_ptr(password);
  return (int)mock();
}

int __wrap_config_runtime_enumerate_users(char usernames[][MAX_USERNAME_LENGTH + 1], int max_users, int* user_count) {
  if (g_use_real_functions) {
    return __real_config_runtime_enumerate_users(usernames, max_users, user_count);
  }
  function_called();
  check_expected_ptr(usernames);
  check_expected(max_users);
  check_expected_ptr(user_count);
  return (int)mock();
}

/* ============================================================================
 * CMocka Wrapped Storage Functions
 * ============================================================================ */

/**
 * @brief Real config_storage_save() declaration for forwarding
 */
extern int __real_config_storage_save(const char* path, const void* manager); // NOLINT(cert-dcl37-c, bugprone-reserved-identifier, cert-dcl51-cpp)

/**
 * @brief Conditional mock/real function control for config_storage_save
 */
static bool g_config_storage_use_real = false;

/**
 * @brief Control whether config_storage_save uses real function
 * @param use_real true to use real function, false for mock
 */
void config_mock_storage_use_real_function(bool use_real) {
  g_config_storage_use_real = use_real;
}

/**
 * @brief Mock for config_storage_save() to avoid file I/O in runtime tests
 *
 * This mock allows testing the persistence queue logic without requiring
 * writable /etc/jffs2/ directory.
 *
 * By default, returns ONVIF_SUCCESS for all paths. Tests that need the real
 * implementation can call config_mock_storage_use_real_function(true).
 *
 * @param path Path to save configuration file
 * @param manager Configuration manager pointer
 * @return ONVIF_SUCCESS on success (mock), or result from real function
 */
int __wrap_config_storage_save(const char* path, const void* manager) { // NOLINT(cert-dcl37-c, bugprone-reserved-identifier, cert-dcl51-cpp)
  /* Use real function if configured */
  if (g_config_storage_use_real) {
    return __real_config_storage_save(path, manager);
  }

  /* Mock implementation - avoid file I/O */
  (void)path;
  (void)manager;
  return ONVIF_SUCCESS;
}
