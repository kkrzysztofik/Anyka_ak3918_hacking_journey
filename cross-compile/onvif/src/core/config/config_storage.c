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

    /* TODO: Implement INI parsing and population of runtime manager */

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

    /* TODO: Implement atomic write with temp file + rename */

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

    /* TODO: Add INI format validation */

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
    /* TODO: Implement using platform_log_error() */
    fprintf(stderr, "Config storage error: %s on %s (code: %d)\n",
            operation ? operation : "unknown",
            path ? path : "unknown",
            error_code);
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
