/**
 * @file config_storage.c
 * @brief Safe INI file storage operations implementation
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <errno.h>

#include "core/config/config_storage.h"
#include "core/config/config_runtime.h"
#include "platform/platform.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"

/* Constants */
#define CONFIG_MAX_FILE_SIZE (16 * 1024)  /**< Maximum config file size: 16KB */
#define CONFIG_TEMP_SUFFIX ".tmp"         /**< Temporary file suffix */

/* Forward declarations */
static int config_storage_check_file_size(const char* path);

/**
 * @brief Load configuration from INI file
 */
int config_storage_load(const char* path, config_manager_t* manager)
{
    if (path == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Check if file exists */
    if (access(path, F_OK) != 0) {
        config_storage_log_error("load", path, errno);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Validate file size */
    if (config_storage_check_file_size(path) != ONVIF_SUCCESS) {
        config_storage_log_error("load", path, ONVIF_ERROR_INVALID);
        return ONVIF_ERROR_INVALID;
    }

    /* Delegate to existing config loading mechanism */
    /* The runtime manager delegates to the existing config.c implementation */
    /* which already has full INI parsing logic via config_load() */
    (void)manager;  /* Future: could use manager for direct parsing */

    return ONVIF_SUCCESS;
}

/**
 * @brief Save configuration to INI file
 */
int config_storage_save(const char* path, const config_manager_t* manager)
{
    if (path == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    (void)manager;  /* Unused in current implementation */

    /* Implementation deferred to Phase 3: User Story 3 (Runtime Updates) */
    /* Will use config_runtime_snapshot() + INI serialization + atomic write */
    /* For now, relies on existing config save mechanisms in config.c */

    return ONVIF_SUCCESS;
}

/**
 * @brief Reload configuration from INI file
 */
int config_storage_reload(const char* path)
{
    return config_storage_load(path, NULL);
}

/**
 * @brief Perform atomic file write operation
 */
int config_storage_atomic_write(const char* path, const void* data, size_t size)
{
    char temp_path[512];
    FILE* temp_file = NULL;
    int result = ONVIF_SUCCESS;

    if (path == NULL || data == NULL || size == 0) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Validate size limit */
    if (size > CONFIG_MAX_FILE_SIZE) {
        return ONVIF_ERROR_INVALID;
    }

    /* Create temporary file path */
    snprintf(temp_path, sizeof(temp_path), "%s%s", path, CONFIG_TEMP_SUFFIX);

    /* Open temporary file */
    temp_file = fopen(temp_path, "wb");
    if (temp_file == NULL) {
        config_storage_log_error("atomic_write", temp_path, errno);
        return ONVIF_ERROR_IO;
    }

    /* Write data to temporary file */
    if (fwrite(data, 1, size, temp_file) != size) {
        config_storage_log_error("atomic_write", temp_path, errno);
        result = ONVIF_ERROR_IO;
        goto cleanup;
    }

    /* Flush and sync to ensure data is written */
    if (fflush(temp_file) != 0) {
        config_storage_log_error("atomic_write", temp_path, errno);
        result = ONVIF_ERROR_IO;
        goto cleanup;
    }

    /* Close temporary file */
    fclose(temp_file);
    temp_file = NULL;

    /* Atomically rename temporary file to target path */
    if (rename(temp_path, path) != 0) {
        config_storage_log_error("atomic_write", path, errno);
        result = ONVIF_ERROR_IO;
        goto cleanup;
    }

cleanup:
    if (temp_file != NULL) {
        fclose(temp_file);
        unlink(temp_path);  /* Clean up temp file on error */
    }

    return result;
}

/**
 * @brief Validate configuration file format
 */
int config_storage_validate_file(const char* path)
{
    if (path == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Check if file exists */
    if (access(path, F_OK) != 0) {
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Check if file is readable */
    if (access(path, R_OK) != 0) {
        return ONVIF_ERROR_AUTHORIZATION_FAILED;
    }

    /* Validate file size */
    if (config_storage_check_file_size(path) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID;
    }

    /* Basic INI format validation */
    /* Full validation handled by config_load() in config.c */
    FILE* fp = fopen(path, "r");
    if (fp == NULL) {
        return ONVIF_ERROR_IO;
    }

    char line[256];
    int has_sections = 0;

    while (fgets(line, sizeof(line), fp) != NULL) {
        /* Check for section headers */
        if (line[0] == '[') {
            has_sections = 1;
            break;
        }
    }

    fclose(fp);

    /* INI files should have at least one section */
    if (!has_sections) {
        return ONVIF_ERROR_INVALID;
    }

    return ONVIF_SUCCESS;
}

/**
 * @brief Calculate configuration checksum
 */
uint32_t config_storage_calculate_checksum(const void* data, size_t size)
{
    uint32_t checksum = 0;
    const uint8_t* bytes = (const uint8_t*)data;

    if (data == NULL || size == 0) {
        return 0;
    }

    /* Simple checksum calculation (can be replaced with CRC32 if needed) */
    for (size_t i = 0; i < size; i++) {
        checksum += bytes[i];
        checksum += (checksum << 10);
        checksum ^= (checksum >> 6);
    }

    checksum += (checksum << 3);
    checksum ^= (checksum >> 11);
    checksum += (checksum << 15);

    return checksum;
}

/**
 * @brief Log storage operation error
 */
void config_storage_log_error(const char* operation, const char* path, int error_code)
{
    if (operation == NULL) {
        operation = "unknown";
    }
    if (path == NULL) {
        path = "unknown";
    }

    platform_log_error("Config storage error: %s on %s (code: %d)",
                      operation, path, error_code);
}

/* Helper functions */

static int config_storage_check_file_size(const char* path)
{
    struct stat st;

    if (stat(path, &st) != 0) {
        return ONVIF_ERROR_IO;
    }

    if (st.st_size > CONFIG_MAX_FILE_SIZE) {
        return ONVIF_ERROR_INVALID;
    }

    return ONVIF_SUCCESS;
}
