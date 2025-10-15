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
#define CONFIG_MAX_LINE_LENGTH 512
#define CONFIG_MAX_SECTION_NAME 64
#define CONFIG_MAX_KEY_NAME 64
#define CONFIG_MAX_VALUE_LENGTH 256

/* Forward declarations */
static int config_storage_check_file_size(const char* path);
static int config_storage_parse_ini(const char* path);
static void config_storage_trim_whitespace(char* str);
static config_section_t config_storage_parse_section_name(const char* section_name);

/**
 * @brief Load configuration from INI file with schema validation
 *
 * Parses INI file and loads values directly into the runtime configuration
 * using the schema-validated setters. This eliminates dependency on legacy config.c.
 */
int config_storage_load(const char* path, config_manager_t* manager)
{
    int result;

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

    /* Check if runtime configuration is initialized */
    /* If not initialized, caller should bootstrap first */
    const struct application_config* snapshot = config_runtime_snapshot();
    if (snapshot == NULL) {
        platform_log_error("[CONFIG_STORAGE] Runtime configuration not initialized. Call config_runtime_bootstrap() first.\n");
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Parse INI file and load values into runtime configuration */
    result = config_storage_parse_ini(path);
    if (result != ONVIF_SUCCESS) {
        config_storage_log_error("parse_ini", path, result);
        return result;
    }

    (void)manager;  /* Unused - maintained for interface compatibility */

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

/**
 * @brief Trim leading and trailing whitespace from string (in-place)
 */
static void config_storage_trim_whitespace(char* str)
{
    char* start;
    char* end;
    size_t len;

    if (str == NULL || *str == '\0') {
        return;
    }

    /* Find first non-whitespace character */
    start = str;
    while (*start == ' ' || *start == '\t' || *start == '\r' || *start == '\n') {
        start++;
    }

    /* All spaces? */
    if (*start == '\0') {
        *str = '\0';
        return;
    }

    /* Find last non-whitespace character */
    end = start + strlen(start) - 1;
    while (end > start && (*end == ' ' || *end == '\t' || *end == '\r' || *end == '\n')) {
        end--;
    }

    /* Calculate trimmed length and move string to beginning */
    len = (size_t)(end - start + 1);
    if (start != str) {
        memmove(str, start, len);
    }
    str[len] = '\0';
}

/**
 * @brief Parse section name to config_section_t enum
 */
static config_section_t config_storage_parse_section_name(const char* section_name)
{
    if (section_name == NULL) {
        return CONFIG_SECTION_ONVIF;  /* Default */
    }

    if (strcmp(section_name, "http") == 0 || strcmp(section_name, "onvif") == 0) {
        return CONFIG_SECTION_ONVIF;
    } else if (strcmp(section_name, "rtsp") == 0) {
        return CONFIG_SECTION_RTSP;
    } else if (strcmp(section_name, "network") == 0) {
        return CONFIG_SECTION_NETWORK;
    } else if (strcmp(section_name, "device") == 0) {
        return CONFIG_SECTION_DEVICE;
    } else if (strcmp(section_name, "logging") == 0) {
        return CONFIG_SECTION_LOGGING;
    } else if (strcmp(section_name, "server") == 0) {
        return CONFIG_SECTION_SERVER;
    } else if (strcmp(section_name, "snapshot") == 0) {
        return CONFIG_SECTION_SNAPSHOT;
    }

    return CONFIG_SECTION_ONVIF;  /* Default to ONVIF section */
}

/**
 * @brief Parse INI file and load values into runtime configuration
 */
static int config_storage_parse_ini(const char* path)
{
    FILE* fp;
    char line[CONFIG_MAX_LINE_LENGTH];
    char section_name[CONFIG_MAX_SECTION_NAME] = "onvif";
    config_section_t current_section = CONFIG_SECTION_ONVIF;
    int line_number = 0;
    int result = ONVIF_SUCCESS;

    fp = fopen(path, "r");
    if (fp == NULL) {
        platform_log_error("[CONFIG_STORAGE] Failed to open config file: %s\n", path);
        return ONVIF_ERROR_IO;
    }

    while (fgets(line, sizeof(line), fp) != NULL) {
        line_number++;
        char* trimmed = line;
        config_storage_trim_whitespace(trimmed);

        /* Skip empty lines and comments */
        if (trimmed[0] == '\0' || trimmed[0] == '#' || trimmed[0] == ';') {
            continue;
        }

        /* Parse section header */
        if (trimmed[0] == '[') {
            char* section_end = strchr(trimmed, ']');
            if (section_end != NULL) {
                *section_end = '\0';
                strncpy(section_name, trimmed + 1, sizeof(section_name) - 1);
                section_name[sizeof(section_name) - 1] = '\0';
                config_storage_trim_whitespace(section_name);
                current_section = config_storage_parse_section_name(section_name);
                continue;
            }
        }

        /* Parse key=value pairs */
        char* equals = strchr(trimmed, '=');
        if (equals != NULL) {
            char key[CONFIG_MAX_KEY_NAME];
            char value[CONFIG_MAX_VALUE_LENGTH];

            *equals = '\0';
            strncpy(key, trimmed, sizeof(key) - 1);
            key[sizeof(key) - 1] = '\0';
            config_storage_trim_whitespace(key);

            strncpy(value, equals + 1, sizeof(value) - 1);
            value[sizeof(value) - 1] = '\0';
            config_storage_trim_whitespace(value);

            /* Attempt to set the value using runtime configuration setters */
            /* Try integer first */
            char* endptr;
            long int_value = strtol(value, &endptr, 10);
            if (*endptr == '\0' && endptr != value) {
                /* Valid integer */
                result = config_runtime_set_int(current_section, key, (int)int_value);
                if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_NOT_FOUND) {
                    platform_log_error("[CONFIG_STORAGE] Failed to set int %s.%s=%ld (error: %d) at line %d\n",
                                     section_name, key, int_value, result, line_number);
                }
            } else {
                /* Try as string */
                result = config_runtime_set_string(current_section, key, value);
                if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_NOT_FOUND) {
                    platform_log_error("[CONFIG_STORAGE] Failed to set string %s.%s=%s (error: %d) at line %d\n",
                                     section_name, key, value, result, line_number);
                }
            }
        }
    }

    fclose(fp);
    return ONVIF_SUCCESS;
}
