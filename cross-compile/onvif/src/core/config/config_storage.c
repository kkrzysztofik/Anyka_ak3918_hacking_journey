/**
 * @file config_storage.c
 * @brief Safe INI file storage operations implementation
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include "core/config/config_storage.h"

#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"

/* Constants */
#define CONFIG_MAX_FILE_SIZE    (16 * 1024) /**< Maximum config file size: 16KB */
#define CONFIG_TEMP_SUFFIX      ".tmp"      /**< Temporary file suffix */
#define CONFIG_MAX_LINE_LENGTH  512
#define CONFIG_MAX_SECTION_NAME 64
#define CONFIG_MAX_KEY_NAME     64
#define CONFIG_MAX_VALUE_LENGTH 256

/* File system constants */
#define CONFIG_DIR_PERMISSIONS 0755 /**< Directory permissions (rwxr-xr-x) */

/* String parsing constants */
#define CONFIG_PARSE_BASE_DECIMAL 10 /**< Decimal base for string to integer parsing */

/* Checksum calculation bit shift constants */
#define CONFIG_CHECKSUM_SHIFT_1 10 /**< First hash shift value */
#define CONFIG_CHECKSUM_SHIFT_2 6  /**< Second hash shift value */
#define CONFIG_CHECKSUM_SHIFT_3 3  /**< Third hash shift value */
#define CONFIG_CHECKSUM_SHIFT_4 11 /**< Fourth hash shift value */
#define CONFIG_CHECKSUM_SHIFT_5 15 /**< Fifth hash shift value */

/* Forward declarations */
static int config_storage_check_file_size(const char* path);
static int config_storage_create_directory(const char* path);
static int config_storage_parse_ini(const char* path);
static void config_storage_trim_whitespace(char* str);
static config_section_t config_storage_parse_section_name(const char* section_name);

/* Forward declarations for INI serialization */
static int config_storage_serialize_section(char* buffer, size_t buffer_size, size_t* offset, const char* section_name);
static int config_storage_append_key_value_int(char* buffer, size_t buffer_size, size_t* offset, const char* key, int value);
static int config_storage_append_key_value_string(char* buffer, size_t buffer_size, size_t* offset, const char* key, const char* value);
static int config_storage_append_key_value_float(char* buffer, size_t buffer_size, size_t* offset, const char* key, float value);
static int config_storage_serialize_to_ini(char* buffer, size_t buffer_size);

/* ============================================================================
 * PUBLIC API - File Operations
 * ============================================================================ */

/**
 * @brief Load configuration from INI file with schema validation
 *
 * Parses INI file and loads values directly into the runtime configuration
 * using the schema-validated setters. This eliminates dependency on legacy config.c.
 */
int config_storage_load(const char* path, config_manager_t* manager) {
  int result = ONVIF_ERROR;

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
    platform_log_error("[CONFIG_STORAGE] Runtime configuration not initialized. Call "
                       "config_runtime_bootstrap() first.\n");
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Parse INI file and load values into runtime configuration */
  result = config_storage_parse_ini(path);
  if (result != ONVIF_SUCCESS) {
    config_storage_log_error("parse_ini", path, result);
    return result;
  }

  (void)manager; /* Unused - maintained for interface compatibility */

  return ONVIF_SUCCESS;
}

/**
 * @brief Save configuration to INI file
 *
 * Serializes the current runtime configuration to INI format and writes it
 * atomically to the specified path. This uses config_runtime_snapshot() to
 * get the current configuration state and config_storage_atomic_write() for
 * safe file operations.
 *
 * @param path Path to the INI file to write
 * @param manager Config manager (unused, maintained for interface compatibility)
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int config_storage_save(const char* path, const config_manager_t* manager) {
  char* buffer = NULL;
  int result = ONVIF_SUCCESS;

  if (path == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  (void)manager; /* Unused - maintained for interface compatibility */

  /* Check if runtime configuration is initialized */
  const struct application_config* snapshot = config_runtime_snapshot();
  if (snapshot == NULL) {
    platform_log_error("[CONFIG_STORAGE] Runtime configuration not initialized\n");
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Allocate buffer for INI serialization */
  buffer = (char*)malloc(CONFIG_MAX_FILE_SIZE);
  if (buffer == NULL) {
    platform_log_error("[CONFIG_STORAGE] Failed to allocate serialization buffer\n");
    return ONVIF_ERROR;
  }

  /* Serialize configuration to INI format */
  result = config_storage_serialize_to_ini(buffer, CONFIG_MAX_FILE_SIZE);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG_STORAGE] Failed to serialize configuration\n");
    free(buffer);
    return result;
  }

  /* Write to file atomically */
  size_t buffer_len = strlen(buffer);
  result = config_storage_atomic_write(path, buffer, buffer_len);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG_STORAGE] Failed to write configuration file\n");
    free(buffer);
    return result;
  }

  free(buffer);
  platform_log_info("[CONFIG_STORAGE] Configuration saved successfully to %s\n", path);
  return ONVIF_SUCCESS;
}

/**
 * @brief Reload configuration from INI file
 */
int config_storage_reload(const char* path) {
  return config_storage_load(path, NULL);
}

/* ============================================================================
 * PUBLIC API - Atomic Write Operations
 * ============================================================================ */

/**
 * @brief Perform atomic file write operation
 */
int config_storage_atomic_write(const char* path, const void* data, size_t size) {
  char temp_path[CONFIG_MAX_LINE_LENGTH];
  char dir_path[CONFIG_MAX_LINE_LENGTH];
  char* last_slash = NULL;
  FILE* temp_file = NULL;
  int result = ONVIF_SUCCESS;

  if (path == NULL || data == NULL || size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate size limit */
  if (size > CONFIG_MAX_FILE_SIZE) {
    return ONVIF_ERROR_INVALID;
  }

  /* Extract directory path and create it if needed */
  strncpy(dir_path, path, sizeof(dir_path) - 1);
  dir_path[sizeof(dir_path) - 1] = '\0';
  last_slash = strrchr(dir_path, '/');
  if (last_slash != NULL) {
    *last_slash = '\0';
    result = config_storage_create_directory(dir_path);
    if (result != ONVIF_SUCCESS) {
      platform_log_error("[CONFIG_STORAGE] Failed to create directory %s\n", dir_path);
      return result;
    }
  }

  /* Create temporary file path */
  if (snprintf(temp_path, sizeof(temp_path), "%s%s", path, CONFIG_TEMP_SUFFIX) >= (int)sizeof(temp_path)) {
    platform_log_error("[CONFIG_STORAGE] Temp path truncated\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

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
  if (fclose(temp_file) != 0) {
    config_storage_log_error("atomic_write", temp_path, errno);
    temp_file = NULL;
    (void)unlink(temp_path); /* Best-effort cleanup */
    return ONVIF_ERROR_IO;
  }
  temp_file = NULL;

  /* Atomically rename temporary file to target path */
  if (rename(temp_path, path) != 0) {
    config_storage_log_error("atomic_write", path, errno);
    result = ONVIF_ERROR_IO;
    goto cleanup;
  }

cleanup:
  if (temp_file != NULL) {
    (void)fclose(temp_file); /* Cleanup - error already occurred */
    (void)unlink(temp_path); /* Best-effort temp file cleanup */
  }

  return result;
}

/* ============================================================================
 * PUBLIC API - Validation & Checksums
 * ============================================================================ */

/**
 * @brief Validate configuration file format
 */
int config_storage_validate_file(const char* path) {
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
  FILE* file_ptr = fopen(path, "r");
  if (file_ptr == NULL) {
    return ONVIF_ERROR_IO;
  }

  char line[CONFIG_MAX_VALUE_LENGTH];
  int has_sections = 0;

  while (fgets(line, sizeof(line), file_ptr) != NULL) {
    /* Check for section headers */
    if (line[0] == '[') {
      has_sections = 1;
      break;
    }
  }

  if (fclose(file_ptr) != 0) {
    platform_log_error("[CONFIG_STORAGE] Failed to close file\n");
    return ONVIF_ERROR_IO;
  }

  /* INI files should have at least one section */
  if (!has_sections) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Calculate configuration checksum
 */
uint32_t config_storage_calculate_checksum(const void* data, size_t size) {
  uint32_t checksum = 0;
  const uint8_t* bytes = (const uint8_t*)data;

  if (data == NULL || size == 0) {
    return 0;
  }

  /* Simple checksum calculation (can be replaced with CRC32 if needed) */
  for (size_t i = 0; i < size; i++) {
    checksum += bytes[i];
    checksum += (checksum << CONFIG_CHECKSUM_SHIFT_1);
    checksum ^= (checksum >> CONFIG_CHECKSUM_SHIFT_2);
  }

  checksum += (checksum << CONFIG_CHECKSUM_SHIFT_3);
  checksum ^= (checksum >> CONFIG_CHECKSUM_SHIFT_4);
  checksum += (checksum << CONFIG_CHECKSUM_SHIFT_5);

  return checksum;
}

/* ============================================================================
 * PUBLIC API - Error Logging
 * ============================================================================ */

/**
 * @brief Log storage operation error
 */
void config_storage_log_error(const char* operation, const char* path, int error_code) {
  if (operation == NULL) {
    operation = "unknown";
  }
  if (path == NULL) {
    path = "unknown";
  }

  platform_log_error("Config storage error: %s on %s (code: %d)", operation, path, error_code);
}

/* ============================================================================
 * PRIVATE HELPERS - File Operations
 * ============================================================================ */

static int config_storage_check_file_size(const char* path) {
  struct stat file_stat;

  if (stat(path, &file_stat) != 0) {
    return ONVIF_ERROR_IO;
  }

  if (file_stat.st_size > CONFIG_MAX_FILE_SIZE) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Create directory path recursively (like mkdir -p)
 * @param path Directory path to create
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int config_storage_create_directory(const char* path) {
  char tmp[CONFIG_MAX_LINE_LENGTH];
  char* path_ptr = NULL;
  size_t len = 0;

  if (path == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (snprintf(tmp, sizeof(tmp), "%s", path) >= (int)sizeof(tmp)) {
    platform_log_error("[CONFIG_STORAGE] Path too long for directory creation\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }
  len = strlen(tmp);

  /* Remove trailing slash if present */
  if (tmp[len - 1] == '/') {
    tmp[len - 1] = '\0';
  }

  /* Create each directory in the path */
  for (path_ptr = tmp + 1; *path_ptr != '\0'; path_ptr++) {
    if (*path_ptr == '/') {
      *path_ptr = '\0';
      /* Try to create directory, ignore error if it already exists */
      if (mkdir(tmp, CONFIG_DIR_PERMISSIONS) != 0 && errno != EEXIST) {
        return ONVIF_ERROR_IO;
      }
      *path_ptr = '/';
    }
  }

  /* Create final directory */
  if (mkdir(tmp, CONFIG_DIR_PERMISSIONS) != 0 && errno != EEXIST) {
    return ONVIF_ERROR_IO;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PRIVATE HELPERS - String Processing
 * ============================================================================ */

/**
 * @brief Trim leading and trailing whitespace from string (in-place)
 */
static void config_storage_trim_whitespace(char* str) {
  char* start = NULL;
  char* end = NULL;
  size_t len = 0;

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
static config_section_t config_storage_parse_section_name(const char* section_name) {
  if (section_name == NULL) {
    return CONFIG_SECTION_ONVIF; /* Default */
  }

  if (strcmp(section_name, "http") == 0 || strcmp(section_name, "onvif") == 0) {
    return CONFIG_SECTION_ONVIF;
  }
  if (strcmp(section_name, "rtsp") == 0) {
    return CONFIG_SECTION_RTSP;
  }
  if (strcmp(section_name, "network") == 0) {
    return CONFIG_SECTION_NETWORK;
  }
  if (strcmp(section_name, "device") == 0) {
    return CONFIG_SECTION_DEVICE;
  }
  if (strcmp(section_name, "logging") == 0) {
    return CONFIG_SECTION_LOGGING;
  }
  if (strcmp(section_name, "server") == 0) {
    return CONFIG_SECTION_SERVER;
  }
  if (strcmp(section_name, "snapshot") == 0) {
    return CONFIG_SECTION_SNAPSHOT;
  }

  return CONFIG_SECTION_ONVIF; /* Default to ONVIF section */
}

/* ============================================================================
 * PRIVATE HELPERS - INI Parsing
 * ============================================================================ */

/* Helper function to parse a key=value pair from INI file */
static int config_storage_parse_key_value(const char* trimmed, const char* section_name, config_section_t current_section, // NOLINT
                                          int line_number) {                                                               // NOLINT
  char* equals = strchr(trimmed, '=');
  if (equals == NULL) {
    return ONVIF_SUCCESS; /* Not a key=value pair */
  }

  char key[CONFIG_MAX_KEY_NAME];
  char value[CONFIG_MAX_VALUE_LENGTH];
  int result = ONVIF_SUCCESS;

  *equals = '\0';
  strncpy(key, trimmed, sizeof(key) - 1);
  key[sizeof(key) - 1] = '\0';
  config_storage_trim_whitespace(key);

  strncpy(value, equals + 1, sizeof(value) - 1);
  value[sizeof(value) - 1] = '\0';
  config_storage_trim_whitespace(value);

  /* Try integer first */
  char* endptr = NULL;
  long int_value = strtol(value, &endptr, CONFIG_PARSE_BASE_DECIMAL);
  if (*endptr == '\0' && endptr != value) {
    /* Valid integer */
    result = config_runtime_set_int(current_section, key, (int)int_value);
    if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_NOT_FOUND) {
      platform_log_error("[CONFIG_STORAGE] Failed to set int %s.%s=%ld (error: %d) at line %d\n", section_name, key, int_value, result, line_number);
    }
  } else {
    /* Try as string */
    result = config_runtime_set_string(current_section, key, value);
    if (result != ONVIF_SUCCESS && result != ONVIF_ERROR_NOT_FOUND) {
      platform_log_error("[CONFIG_STORAGE] Failed to set string %s.%s=%s (error: %d) at line %d\n", section_name, key, value, result, line_number);
    }
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Parse INI file and load values into runtime configuration
 */
static int config_storage_parse_ini(const char* path) {
  FILE* file_ptr = NULL;
  char line[CONFIG_MAX_LINE_LENGTH];
  char section_name[CONFIG_MAX_SECTION_NAME] = "onvif";
  config_section_t current_section = CONFIG_SECTION_ONVIF;
  int line_number = 0;

  file_ptr = fopen(path, "r");
  if (file_ptr == NULL) {
    platform_log_error("[CONFIG_STORAGE] Failed to open config file: %s\n", path);
    return ONVIF_ERROR_IO;
  }

  while (fgets(line, sizeof(line), file_ptr) != NULL) {
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
    config_storage_parse_key_value(trimmed, section_name, current_section, line_number);
  }

  if (fclose(file_ptr) != 0) {
    platform_log_error("[CONFIG_STORAGE] Failed to close file\n");
    return ONVIF_ERROR_IO;
  }
  return ONVIF_SUCCESS;
}

/* Helper function to serialize a single configuration entry */
static int config_storage_serialize_entry(const config_schema_entry_t* entry, char* buffer, size_t buffer_size, size_t* offset) {
  int result = ONVIF_SUCCESS;

  switch (entry->type) {
  case CONFIG_TYPE_INT: {
    int value = 0;
    result = config_runtime_get_int(entry->section, entry->key, &value);
    if (result == ONVIF_SUCCESS) {
      result = config_storage_append_key_value_int(buffer, buffer_size, offset, entry->key, value);
    } else {
      platform_log_error("[CONFIG_STORAGE] Failed to get int value for %s.%s (error: %d)\n", entry->section_name, entry->key, result);
    }
    break;
  }
  case CONFIG_TYPE_FLOAT: {
    float value = 0.0F;
    result = config_runtime_get_float(entry->section, entry->key, &value);
    if (result == ONVIF_SUCCESS) {
      result = config_storage_append_key_value_float(buffer, buffer_size, offset, entry->key, value);
    } else {
      platform_log_error("[CONFIG_STORAGE] Failed to get float value for %s.%s (error: %d)\n", entry->section_name, entry->key, result);
    }
    break;
  }
  case CONFIG_TYPE_STRING: {
    char value[CONFIG_MAX_VALUE_LENGTH] = {0};
    result = config_runtime_get_string(entry->section, entry->key, value, sizeof(value));
    if (result == ONVIF_SUCCESS) {
      result = config_storage_append_key_value_string(buffer, buffer_size, offset, entry->key, value);
    } else {
      platform_log_error("[CONFIG_STORAGE] Failed to get string value for %s.%s (error: %d)\n", entry->section_name, entry->key, result);
    }
    break;
  }
  case CONFIG_TYPE_BOOL: {
    /* Booleans are stored as integers */
    int value = 0;
    result = config_runtime_get_bool(entry->section, entry->key, &value);
    if (result == ONVIF_SUCCESS) {
      result = config_storage_append_key_value_int(buffer, buffer_size, offset, entry->key, value);
    } else {
      platform_log_error("[CONFIG_STORAGE] Failed to get bool value for %s.%s (error: %d)\n", entry->section_name, entry->key, result);
    }
    break;
  }
  default:
    /* Unknown type - skip */
    platform_log_warning("[CONFIG_STORAGE] Unknown type for %s.%s\n", entry->section_name, entry->key);
    break;
  }

  return result;
}

/* ============================================================================
 * PRIVATE HELPERS - INI Serialization
 * ============================================================================ */

/**
 * @brief Append section header to INI buffer
 */
static int config_storage_serialize_section(char* buffer, size_t buffer_size, size_t* offset, const char* section_name) {
  int written = 0;

  if (buffer == NULL || section_name == NULL || offset == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Add blank line before section (except first section) */
  if (*offset > 0) {
    written = snprintf(buffer + *offset, buffer_size - *offset, "\n");
    if (written < 0 || (size_t)written >= buffer_size - *offset) {
      return ONVIF_ERROR_INVALID;
    }
    *offset += (size_t)written;
  }

  /* Add section header */
  written = snprintf(buffer + *offset, buffer_size - *offset, "[%s]\n", section_name);
  if (written < 0 || (size_t)written >= buffer_size - *offset) {
    return ONVIF_ERROR_INVALID;
  }
  *offset += (size_t)written;

  return ONVIF_SUCCESS;
}

/**
 * @brief Append integer key=value pair to INI buffer
 */
static int config_storage_append_key_value_int(char* buffer, size_t buffer_size, size_t* offset, const char* key, int value) {
  int written = 0;

  if (buffer == NULL || key == NULL || offset == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  written = snprintf(buffer + *offset, buffer_size - *offset, "%s = %d\n", key, value);
  if (written < 0 || (size_t)written >= buffer_size - *offset) {
    return ONVIF_ERROR_INVALID;
  }
  *offset += (size_t)written;

  return ONVIF_SUCCESS;
}

/**
 * @brief Append string key=value pair to INI buffer
 */
static int config_storage_append_key_value_string(char* buffer, size_t buffer_size, size_t* offset, const char* key, const char* value) {
  int written = 0;

  if (buffer == NULL || key == NULL || value == NULL || offset == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  written = snprintf(buffer + *offset, buffer_size - *offset, "%s = %s\n", key, value);
  if (written < 0 || (size_t)written >= buffer_size - *offset) {
    return ONVIF_ERROR_INVALID;
  }
  *offset += (size_t)written;

  return ONVIF_SUCCESS;
}

/**
 * @brief Append float key=value pair to INI buffer
 */
static int config_storage_append_key_value_float(char* buffer, size_t buffer_size, size_t* offset, const char* key, float value) {
  int written = 0;

  if (buffer == NULL || key == NULL || offset == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  written = snprintf(buffer + *offset, buffer_size - *offset, "%s = %.2F\n", key, value);
  if (written < 0 || (size_t)written >= buffer_size - *offset) {
    return ONVIF_ERROR_INVALID;
  }
  *offset += (size_t)written;

  return ONVIF_SUCCESS;
}

/**
 * @brief Serialize runtime configuration to INI format using schema iteration
 *
 * This function iterates through the configuration schema and serializes
 * all parameters to INI format. This approach is maintainable and extensible.
 */
static int config_storage_serialize_to_ini(char* buffer, size_t buffer_size) {
  size_t offset = 0;
  int result = ONVIF_SUCCESS;
  const config_schema_entry_t* schema = NULL;
  size_t schema_count = 0;
  const char* current_section = NULL;

  if (buffer == NULL || buffer_size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Initialize buffer */
  buffer[0] = '\0';

  /* Get configuration schema */
  schema = config_runtime_get_schema(&schema_count);
  if (schema == NULL || schema_count == 0) {
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Iterate through schema and serialize each parameter */
  for (size_t entry_idx = 0; entry_idx < schema_count; entry_idx++) {
    const config_schema_entry_t* entry = &schema[entry_idx];

    /* Write section header when section changes */
    if (current_section == NULL || strcmp(current_section, entry->section_name) != 0) {
      result = config_storage_serialize_section(buffer, buffer_size, &offset, entry->section_name);
      if (result != ONVIF_SUCCESS) {
        return result;
      }
      current_section = entry->section_name;
    }

    /* Serialize parameter based on type */
    result = config_storage_serialize_entry(entry, buffer, buffer_size, &offset);
    if (result != ONVIF_SUCCESS) {
      return result;
    }
  }

  return ONVIF_SUCCESS;
}
