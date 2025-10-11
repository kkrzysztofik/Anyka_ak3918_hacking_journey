/**
 * @file config_runtime.c
 * @brief Schema-driven runtime configuration manager implementation
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

#include "core/config/config_runtime.h"
#include "core/config/config.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"

/* Constants */
#define CONFIG_STRING_MAX_LEN_DEFAULT 256
#define CONFIG_STRING_MAX_LEN_STANDARD 64
#define CONFIG_STRING_MAX_LEN_SHORT 32

/* Global state variables */
static struct application_config* g_config_runtime_app_config = NULL;
static pthread_mutex_t g_config_runtime_mutex = PTHREAD_MUTEX_INITIALIZER;
static uint32_t g_config_runtime_generation = 0;
static int g_config_runtime_initialized = 0;

/* Forward declarations */
static int config_runtime_validate_section(config_section_t section);
static int config_runtime_validate_key(const char* key);
static void* config_runtime_get_section_ptr(config_section_t section);
static void* config_runtime_get_field_ptr(config_section_t section, const char* key, config_value_type_t* out_type);

/**
 * @brief Bootstrap the runtime configuration manager
 */
int config_runtime_bootstrap(struct application_config* cfg)
{
    if (cfg == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (g_config_runtime_initialized) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_ALREADY_EXISTS;
    }

    g_config_runtime_app_config = cfg;
    g_config_runtime_generation = 0;
    g_config_runtime_initialized = 1;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Shutdown the runtime configuration manager
 */
int config_runtime_shutdown(void)
{
    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    g_config_runtime_app_config = NULL;
    g_config_runtime_initialized = 0;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Apply default values for all configuration parameters
 */
int config_runtime_apply_defaults(void)
{
    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Apply default values using existing config system */
    /* For now, we set reasonable defaults directly */
    /* TODO: In Phase 4 (Schema-Driven Validation), this will load from schema */

    if (g_config_runtime_app_config->network) {
        g_config_runtime_app_config->network->rtsp_port = 554;
        g_config_runtime_app_config->network->snapshot_port = 8080;
        g_config_runtime_app_config->network->ws_discovery_port = 3702;
    }

    if (g_config_runtime_app_config->logging) {
        g_config_runtime_app_config->logging->enabled = 1;
        g_config_runtime_app_config->logging->min_level = 2;  /* NOTICE */
    }

    g_config_runtime_app_config->onvif.enabled = 1;
    g_config_runtime_app_config->onvif.http_port = 8080;
    g_config_runtime_app_config->onvif.auth_enabled = 0;

    g_config_runtime_generation++;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Get integer configuration value with validation
 */
int config_runtime_get_int(config_section_t section, const char* key, int* out_value)
{
    config_value_type_t field_type = CONFIG_TYPE_INT;
    void* field_ptr = NULL;

    if (key == NULL || out_value == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Validate type matches */
    if (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    /* Copy the value */
    *out_value = *(int*)field_ptr;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Get string configuration value with validation
 */
int config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size)
{
    config_value_type_t field_type = CONFIG_TYPE_STRING;
    void* field_ptr = NULL;
    const char* str_value = NULL;
    int result = 0;

    if (key == NULL || out_value == NULL || buffer_size == 0) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Validate type matches */
    if (field_type != CONFIG_TYPE_STRING) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    /* Copy the string safely */
    str_value = (const char*)field_ptr;
    result = snprintf(out_value, buffer_size, "%s", str_value);

    /* Check for truncation */
    if (result < 0 || (size_t)result >= buffer_size) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Get boolean configuration value with validation
 */
int config_runtime_get_bool(config_section_t section, const char* key, int* out_value)
{
    /* Booleans are stored as integers */
    return config_runtime_get_int(section, key, out_value);
}

/**
 * @brief Get float configuration value with validation
 */
int config_runtime_get_float(config_section_t section, const char* key, float* out_value)
{
    config_value_type_t field_type = CONFIG_TYPE_FLOAT;

    if (key == NULL || out_value == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    void* field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Type must be int or float */
    if (field_type == CONFIG_TYPE_INT) {
        *out_value = (float)(*(int*)field_ptr);
    } else if (field_type == CONFIG_TYPE_FLOAT) {
        *out_value = *(float*)field_ptr;
    } else {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Set integer configuration value with validation
 */
int config_runtime_set_int(config_section_t section, const char* key, int value)
{
    config_value_type_t field_type = CONFIG_TYPE_INT;
    void* field_ptr = NULL;

    if (key == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Validate type matches */
    if (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    /* Set the value */
    *(int*)field_ptr = value;

    /* Increment generation counter to signal change */
    g_config_runtime_generation++;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Set string configuration value with validation
 */
int config_runtime_set_string(config_section_t section, const char* key, const char* value)
{
    config_value_type_t field_type = CONFIG_TYPE_STRING;
    void* field_ptr = NULL;
    char* str_field = NULL;
    size_t max_len = CONFIG_STRING_MAX_LEN_DEFAULT;

    if (key == NULL || value == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Validate type matches */
    if (field_type != CONFIG_TYPE_STRING) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    /* Determine max length based on section */
    if (section == CONFIG_SECTION_LOGGING) {
        max_len = CONFIG_STRING_MAX_LEN_SHORT;  /* tag field */
    } else if (section == CONFIG_SECTION_ONVIF || section == CONFIG_SECTION_DEVICE) {
        max_len = CONFIG_STRING_MAX_LEN_STANDARD;  /* standard fields */
    }

    /* Validate string length */
    if (strlen(value) >= max_len) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    /* Copy the string safely */
    str_field = (char*)field_ptr;
    strncpy(str_field, value, max_len - 1);
    str_field[max_len - 1] = '\0';  /* Ensure null termination */

    /* Increment generation counter to signal change */
    g_config_runtime_generation++;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Set boolean configuration value with validation
 */
int config_runtime_set_bool(config_section_t section, const char* key, int value)
{
    /* Booleans are stored as integers */
    return config_runtime_set_int(section, key, value);
}

/**
 * @brief Set float configuration value with validation
 */
int config_runtime_set_float(config_section_t section, const char* key, float value)
{
    config_value_type_t field_type = CONFIG_TYPE_FLOAT;
    void* field_ptr = NULL;

    if (key == NULL) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    /* Get pointer to the field */
    field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
    if (field_ptr == NULL) {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_NOT_FOUND;
    }

    /* Type must be float or int (for conversion) */
    if (field_type == CONFIG_TYPE_FLOAT) {
        *(float*)field_ptr = value;
    } else if (field_type == CONFIG_TYPE_INT) {
        *(int*)field_ptr = (int)value;
    } else {
        pthread_mutex_unlock(&g_config_runtime_mutex);
        return ONVIF_ERROR_INVALID;
    }

    g_config_runtime_generation++;

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return ONVIF_SUCCESS;
}

/**
 * @brief Get current configuration snapshot
 */
const struct application_config* config_runtime_snapshot(void)
{
    const struct application_config* snapshot = NULL;

    pthread_mutex_lock(&g_config_runtime_mutex);

    if (g_config_runtime_initialized && g_config_runtime_app_config != NULL) {
        snapshot = g_config_runtime_app_config;
    }

    pthread_mutex_unlock(&g_config_runtime_mutex);

    return snapshot;
}

/**
 * @brief Get current configuration generation counter
 */
uint32_t config_runtime_get_generation(void)
{
    uint32_t generation;

    pthread_mutex_lock(&g_config_runtime_mutex);
    generation = g_config_runtime_generation;
    pthread_mutex_unlock(&g_config_runtime_mutex);

    return generation;
}

/* Stub implementations for remaining APIs */

int config_runtime_queue_persistence_update(config_section_t section, const char* key, const void* value, config_value_type_t type)
{
    (void)section;
    (void)key;
    (void)value;
    (void)type;
    /* TODO: Implement persistence queue */
    return ONVIF_SUCCESS;
}

int config_runtime_process_persistence_queue(void)
{
    /* TODO: Implement persistence queue processing */
    return ONVIF_SUCCESS;
}

int config_runtime_get_persistence_status(void)
{
    /* TODO: Implement persistence status */
    return 0;
}

int config_runtime_get_stream_profile(int profile_index, void* profile)
{
    (void)profile_index;
    (void)profile;
    /* TODO: Implement stream profile getter */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_set_stream_profile(int profile_index, const void* profile)
{
    (void)profile_index;
    (void)profile;
    /* TODO: Implement stream profile setter */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_validate_stream_profile(const void* profile)
{
    (void)profile;
    /* TODO: Implement stream profile validation */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_hash_password(const char* password, char* hash_output, size_t output_size)
{
    (void)password;
    (void)hash_output;
    (void)output_size;
    /* TODO: Implement password hashing */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_verify_password(const char* password, const char* hash)
{
    (void)password;
    (void)hash;
    /* TODO: Implement password verification */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_add_user(const char* username, const char* password)
{
    (void)username;
    (void)password;
    /* TODO: Implement user addition */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_remove_user(const char* username)
{
    (void)username;
    /* TODO: Implement user removal */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

int config_runtime_update_user_password(const char* username, const char* new_password)
{
    (void)username;
    (void)new_password;
    /* TODO: Implement password update */
    return ONVIF_ERROR_NOT_IMPLEMENTED;
}

/* Helper functions */

static int config_runtime_validate_section(config_section_t section)
{
    /* Validate that section is within reasonable bounds */
    if (section < CONFIG_SECTION_ONVIF || section > CONFIG_SECTION_USER_8) {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }
    return ONVIF_SUCCESS;
}

static int config_runtime_validate_key(const char* key)
{
    if (key == NULL || key[0] == '\0') {
        return ONVIF_ERROR_INVALID_PARAMETER;
    }
    return ONVIF_SUCCESS;
}

/**
 * @brief Get pointer to configuration section structure
 */
static void* config_runtime_get_section_ptr(config_section_t section)
{
    if (g_config_runtime_app_config == NULL) {
        return NULL;
    }

    switch (section) {
        case CONFIG_SECTION_ONVIF:
            return &g_config_runtime_app_config->onvif;
        case CONFIG_SECTION_NETWORK:
            return g_config_runtime_app_config->network;
        case CONFIG_SECTION_DEVICE:
            return g_config_runtime_app_config->device;
        case CONFIG_SECTION_LOGGING:
            return g_config_runtime_app_config->logging;
        case CONFIG_SECTION_SERVER:
            return g_config_runtime_app_config->server;
        case CONFIG_SECTION_MAIN_STREAM:
            return g_config_runtime_app_config->main_stream;
        case CONFIG_SECTION_SUB_STREAM:
            return g_config_runtime_app_config->sub_stream;
        case CONFIG_SECTION_IMAGING:
            return g_config_runtime_app_config->imaging;
        case CONFIG_SECTION_AUTO_DAYNIGHT:
            return g_config_runtime_app_config->auto_daynight;
        default:
            return NULL;
    }
}

/**
 * @brief Get pointer to specific field within a configuration section
 *
 * This function maps section/key pairs to actual struct field pointers.
 * It also returns the type of the field for validation.
 */
static void* config_runtime_get_field_ptr(config_section_t section, const char* key, config_value_type_t* out_type)
{
    void* section_ptr = config_runtime_get_section_ptr(section);
    if (section_ptr == NULL) {
        return NULL;
    }

    /* Map common ONVIF section keys */
    if (section == CONFIG_SECTION_ONVIF) {
        struct onvif_settings* onvif = (struct onvif_settings*)section_ptr;
        if (strcmp(key, "enabled") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &onvif->enabled;
        } else if (strcmp(key, "http_port") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &onvif->http_port;
        } else if (strcmp(key, "auth_enabled") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &onvif->auth_enabled;
        } else if (strcmp(key, "username") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return onvif->username;
        } else if (strcmp(key, "password") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return onvif->password;
        }
    }
    /* Map network section keys */
    else if (section == CONFIG_SECTION_NETWORK) {
        struct network_settings* network = (struct network_settings*)section_ptr;
        if (strcmp(key, "rtsp_port") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &network->rtsp_port;
        } else if (strcmp(key, "snapshot_port") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &network->snapshot_port;
        } else if (strcmp(key, "ws_discovery_port") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &network->ws_discovery_port;
        }
    }
    /* Map device section keys */
    else if (section == CONFIG_SECTION_DEVICE) {
        struct device_info* device = (struct device_info*)section_ptr;
        if (strcmp(key, "manufacturer") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return device->manufacturer;
        } else if (strcmp(key, "model") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return device->model;
        } else if (strcmp(key, "firmware_version") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return device->firmware_version;
        } else if (strcmp(key, "serial_number") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return device->serial_number;
        } else if (strcmp(key, "hardware_id") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return device->hardware_id;
        }
    }
    /* Map logging section keys */
    else if (section == CONFIG_SECTION_LOGGING) {
        struct logging_settings* logging = (struct logging_settings*)section_ptr;
        if (strcmp(key, "enabled") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &logging->enabled;
        } else if (strcmp(key, "use_colors") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &logging->use_colors;
        } else if (strcmp(key, "use_timestamps") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &logging->use_timestamps;
        } else if (strcmp(key, "min_level") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &logging->min_level;
        } else if (strcmp(key, "tag") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_STRING;
            return logging->tag;
        } else if (strcmp(key, "http_verbose") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &logging->http_verbose;
        }
    }
    /* Map server section keys */
    else if (section == CONFIG_SECTION_SERVER) {
        struct server_settings* server = (struct server_settings*)section_ptr;
        if (strcmp(key, "worker_threads") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->worker_threads;
        } else if (strcmp(key, "max_connections") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->max_connections;
        } else if (strcmp(key, "connection_timeout") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->connection_timeout;
        } else if (strcmp(key, "keepalive_timeout") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->keepalive_timeout;
        } else if (strcmp(key, "epoll_timeout") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->epoll_timeout;
        } else if (strcmp(key, "cleanup_interval") == 0) {
            if (out_type) *out_type = CONFIG_TYPE_INT;
            return &server->cleanup_interval;
        }
    }

    /* Key not found in this section */
    return NULL;
}
