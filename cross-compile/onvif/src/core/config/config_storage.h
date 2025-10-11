/**
 * @file config_storage.h
 * @brief Safe INI file storage operations with atomic writes
 *
 * Provides reliable configuration persistence with:
 * - Atomic file operations using temp-file + rename pattern
 * - INI parsing with validation and error handling
 * - Checksum handling for integrity verification
 * - Graceful fallback to defaults on load failure
 * - Structured error logging
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#ifndef CONFIG_STORAGE_H
#define CONFIG_STORAGE_H

#include <stddef.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"

/**
 * @defgroup config_storage Configuration Storage Layer
 * @brief Atomic INI file operations for configuration persistence
 * @{
 */

/* Core Storage APIs */

/**
 * @brief Load configuration from INI file
 *
 * Loads configuration from the specified file path and populates the
 * runtime configuration manager. Performs validation and falls back
 * to defaults on corruption or missing file.
 *
 * @param[in] path Path to INI configuration file
 * @param[in,out] manager Configuration manager instance (unused, for future compatibility)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_storage_load(const char* path, config_manager_t* manager);

/**
 * @brief Save configuration to INI file
 *
 * Saves the current runtime configuration to the specified file path
 * using atomic write operations (temp file + rename).
 *
 * @param[in] path Path to INI configuration file
 * @param[in] manager Configuration manager instance (unused, for future compatibility)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_storage_save(const char* path, const config_manager_t* manager);

/**
 * @brief Reload configuration from INI file
 *
 * Convenience function that reloads configuration from the specified path.
 * Equivalent to config_storage_load() but with simpler interface.
 *
 * @param[in] path Path to INI configuration file
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_storage_reload(const char* path);

/* Atomic Write Operations */

/**
 * @brief Perform atomic file write operation
 *
 * Writes data to a temporary file and atomically renames it to the target path.
 * This prevents corruption if write operation is interrupted.
 *
 * @param[in] path Target file path
 * @param[in] data Data to write
 * @param[in] size Size of data in bytes
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int config_storage_atomic_write(const char* path, const void* data, size_t size);

/* Validation and Integrity */

/**
 * @brief Validate configuration file format
 *
 * Checks if the file at the specified path is a valid INI file
 * with proper format and within size limits.
 *
 * @param[in] path Path to configuration file
 * @return ONVIF_SUCCESS if valid, error code on failure
 */
int config_storage_validate_file(const char* path);

/**
 * @brief Calculate configuration checksum
 *
 * Computes a checksum for the configuration data to detect corruption.
 *
 * @param[in] data Configuration data
 * @param[in] size Size of data in bytes
 * @return Checksum value
 */
uint32_t config_storage_calculate_checksum(const void* data, size_t size);

/* Error Handling and Logging */

/**
 * @brief Log storage operation error
 *
 * Internal function for logging storage-related errors with context.
 *
 * @param[in] operation Operation name
 * @param[in] path File path involved
 * @param[in] error_code Error code
 */
void config_storage_log_error(const char* operation, const char* path, int error_code);

/**
 * @}
 */

#endif /* CONFIG_STORAGE_H */
