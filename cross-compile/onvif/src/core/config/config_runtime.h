/**
 * @file config_runtime.h
 * @brief Schema-driven runtime configuration manager for ONVIF daemon
 *
 * Provides centralized configuration management with:
 * - Schema-driven validation with type checking and bounds enforcement
 * - Runtime configuration updates with immediate in-memory changes
 * - Typed getter/setter functions with validation
 * - Thread-safe operations with generation counters
 * - Async persistence queue for configuration updates
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#ifndef CONFIG_RUNTIME_H
#define CONFIG_RUNTIME_H

#include <stddef.h>
#include <stdint.h>
#include <pthread.h>

#include "core/config/config.h"
#include "services/common/onvif_types.h"

/**
 * @defgroup config_runtime Runtime Configuration Manager
 * @brief Schema-driven runtime configuration with validation and persistence
 * @{
 */

/* Note: config_section_t and config_value_type_t are defined in config.h
 * The following new sections will need to be added to config.h enum:
 * - CONFIG_SECTION_MEDIA
 * - CONFIG_SECTION_PTZ
 * - CONFIG_SECTION_SNAPSHOT
 * - CONFIG_SECTION_STREAM_PROFILE_1
 * - CONFIG_SECTION_STREAM_PROFILE_2
 * - CONFIG_SECTION_STREAM_PROFILE_3
 * - CONFIG_SECTION_STREAM_PROFILE_4
 * - CONFIG_SECTION_USER_1 through CONFIG_SECTION_USER_8
 */

/**
 * @brief Configuration schema entry with validation rules
 */
typedef struct {
    config_section_t section;         /**< Configuration section */
    const char* section_name;         /**< Section name string */
    const char* key;                  /**< Configuration key */
    config_value_type_t type;         /**< Value type (from config.h) */
    int required;                     /**< Is this entry required? */
    int min_value;                    /**< Minimum value (for int/float) */
    int max_value;                    /**< Maximum value (for int/float) */
    size_t max_length;                /**< Maximum length (for strings) */
    const char* default_literal;      /**< Default value as string */
} config_schema_entry_t;

/**
 * @brief Configuration persistence queue entry
 */
typedef struct {
    config_section_t section;         /**< Configuration section */
    char key[64];                     /**< Configuration key */
    config_value_type_t type;         /**< Value type (from config.h) */
    union {
        int int_value;
        float float_value;
        char string_value[256];
    } value;                          /**< Value union */
    uint64_t timestamp;               /**< Queue timestamp */
} persistence_queue_entry_t;

/* Core Runtime Manager APIs */

/**
 * @brief Bootstrap the runtime configuration manager
 *
 * Initializes the runtime manager with the provided configuration structure.
 * This must be called before any other runtime configuration operations.
 *
 * @param[in,out] cfg Application configuration structure to manage
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_bootstrap(struct application_config* cfg);

/**
 * @brief Shutdown the runtime configuration manager
 *
 * Cleans up resources and flushes any pending persistence operations.
 *
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_shutdown(void);

/**
 * @brief Apply default values for all configuration parameters
 *
 * Sets all configuration parameters to their schema-defined defaults.
 *
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_apply_defaults(void);

/* Typed Getter Functions */

/**
 * @brief Get integer configuration value with validation
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[out] out_value Output integer value
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_get_int(config_section_t section, const char* key, int* out_value);

/**
 * @brief Get string configuration value with validation
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[out] out_value Output string buffer
 * @param[in] buffer_size Size of output buffer
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size);

/**
 * @brief Get boolean configuration value with validation
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[out] out_value Output boolean value (0 or 1)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_get_bool(config_section_t section, const char* key, int* out_value);

/**
 * @brief Get float configuration value with validation
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[out] out_value Output float value
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_get_float(config_section_t section, const char* key, float* out_value);

/* Typed Setter Functions */

/**
 * @brief Set integer configuration value with validation
 *
 * Validates against schema, updates in-memory immediately, queues for persistence.
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[in] value Integer value to set
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_set_int(config_section_t section, const char* key, int value);

/**
 * @brief Set string configuration value with validation
 *
 * Validates against schema, updates in-memory immediately, queues for persistence.
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[in] value String value to set
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_set_string(config_section_t section, const char* key, const char* value);

/**
 * @brief Set boolean configuration value with validation
 *
 * Validates against schema, updates in-memory immediately, queues for persistence.
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[in] value Boolean value to set (0 or 1)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_set_bool(config_section_t section, const char* key, int value);

/**
 * @brief Set float configuration value with validation
 *
 * Validates against schema, updates in-memory immediately, queues for persistence.
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[in] value Float value to set
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_set_float(config_section_t section, const char* key, float value);

/* Runtime Status and Snapshot APIs */

/**
 * @brief Get current configuration snapshot
 *
 * Returns a read-only pointer to the current configuration state.
 * This pointer is valid until the next configuration update.
 *
 * @return Pointer to current configuration, or NULL on error
 */
const struct application_config* config_runtime_snapshot(void);

/**
 * @brief Get current configuration generation counter
 *
 * The generation counter increments with each configuration update.
 * Used to detect configuration changes.
 *
 * @return Current generation counter value
 */
uint32_t config_runtime_get_generation(void);

/* Persistence Queue Management */

/**
 * @brief Queue a configuration update for async persistence
 *
 * Internal function - called automatically by setter functions.
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @param[in] value Value to persist
 * @param[in] type Value type
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_queue_persistence_update(config_section_t section, const char* key, const void* value, config_value_type_t type);

/**
 * @brief Process pending persistence queue entries
 *
 * Processes all queued configuration updates and persists to storage.
 *
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_process_persistence_queue(void);

/**
 * @brief Get persistence queue status
 *
 * Returns the number of pending persistence operations.
 *
 * @return Number of pending operations, or -1 on error
 */
int config_runtime_get_persistence_status(void);

/* Stream Profile Management (User Story 4) */

/**
 * @brief Get stream profile configuration
 *
 * @param[in] profile_index Profile index (0-3)
 * @param[out] profile Output profile structure
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_get_stream_profile(int profile_index, void* profile);

/**
 * @brief Set stream profile configuration
 *
 * @param[in] profile_index Profile index (0-3)
 * @param[in] profile Input profile structure
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_set_stream_profile(int profile_index, const void* profile);

/**
 * @brief Validate stream profile parameters
 *
 * @param[in] profile Profile structure to validate
 * @return ONVIF_SUCCESS if valid, error code on failure
 */
int config_runtime_validate_stream_profile(const void* profile);

/* User Credential Management (User Story 5) */

/**
 * @brief Hash password using SHA256
 *
 * @param[in] password Plaintext password
 * @param[out] hash_output Output hash buffer (64 hex chars + null)
 * @param[in] output_size Size of output buffer (must be >= 65)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_hash_password(const char* password, char* hash_output, size_t output_size);

/**
 * @brief Verify password against stored hash
 *
 * @param[in] password Plaintext password to verify
 * @param[in] hash Stored password hash
 * @return ONVIF_SUCCESS if password matches, error code on failure
 */
int config_runtime_verify_password(const char* password, const char* hash);

/**
 * @brief Add a new user account
 *
 * @param[in] username Username (3-32 alphanumeric chars)
 * @param[in] password Plaintext password
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_add_user(const char* username, const char* password);

/**
 * @brief Remove a user account
 *
 * @param[in] username Username to remove
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_remove_user(const char* username);

/**
 * @brief Update user password
 *
 * @param[in] username Username
 * @param[in] new_password New plaintext password
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_runtime_update_user_password(const char* username, const char* new_password);

/**
 * @}
 */

#endif /* CONFIG_RUNTIME_H */
