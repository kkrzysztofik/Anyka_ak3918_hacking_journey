/**
 * @file config_runtime_simple.c
 * @brief Simplified config runtime implementation for command-line tools
 * @author kkrzysztofik
 * @date 2025
 */

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "utils/error/error_handling.h"
#include "utils/security/hash_utils.h"

// Constants
#define ONVIF_PASSWORD_HASH_SIZE 128

// Global variables (at top of file per project standards)
static struct application_config* g_config_runtime_app_config = NULL;      // NOLINT
static pthread_mutex_t g_config_runtime_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT
static volatile int g_config_runtime_initialized = 0;                      // NOLINT

int config_runtime_init(struct application_config* cfg) {
  if (cfg == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (g_config_runtime_initialized) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_ALREADY_EXISTS;
  }

  g_config_runtime_app_config = cfg;
  g_config_runtime_initialized = 1;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_cleanup(void) {
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

int config_runtime_is_initialized(void) {
  int initialized = 0;
  pthread_mutex_lock(&g_config_runtime_mutex);
  initialized = g_config_runtime_initialized;
  pthread_mutex_unlock(&g_config_runtime_mutex);
  return initialized;
}

int config_runtime_apply_defaults(void) {
  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  // Initialize user array to empty
  for (int i = 0; i < MAX_USERS; i++) {
    memset(g_config_runtime_app_config->users[i].username, 0, sizeof(g_config_runtime_app_config->users[i].username));
    memset(g_config_runtime_app_config->users[i].password_hash, 0, sizeof(g_config_runtime_app_config->users[i].password_hash));
    g_config_runtime_app_config->users[i].active = 0;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);
  return ONVIF_SUCCESS;
}

// Helper to find user by username
static int find_user_by_username(const char* username) {
  for (int i = 0; i < MAX_USERS; i++) {
    if (g_config_runtime_app_config->users[i].active && strcmp(g_config_runtime_app_config->users[i].username, username) == 0) {
      return i;
    }
  }
  return -1;
}

int config_runtime_add_user(const char* username, const char* password) {
  int user_index = -1;
  int result = ONVIF_ERROR;
  char password_hash[ONVIF_PASSWORD_HASH_SIZE] = {0};

  if (!username || !password) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (strlen(username) == 0 || strlen(password) == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  // Check if user already exists - if so, update password instead
  user_index = find_user_by_username(username);
  if (user_index >= 0) {
    // User exists - unlock mutex to hash password, then lock again to update
    pthread_mutex_unlock(&g_config_runtime_mutex);

    // Hash password using real hash_utils (salted SHA256)
    result = onvif_hash_password(password, password_hash, sizeof(password_hash));
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    // Update existing user's password
    pthread_mutex_lock(&g_config_runtime_mutex);
    strncpy(g_config_runtime_app_config->users[user_index].password_hash, password_hash,
            sizeof(g_config_runtime_app_config->users[user_index].password_hash) - 1);
    g_config_runtime_app_config->users[user_index].password_hash[sizeof(g_config_runtime_app_config->users[user_index].password_hash) - 1] = '\0';
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_SUCCESS;
  }

  // User doesn't exist - find empty slot for new user
  for (int i = 0; i < MAX_USERS; i++) {
    if (!g_config_runtime_app_config->users[i].active) {
      user_index = i;
      break;
    }
  }

  if (user_index == -1) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_OUT_OF_RESOURCES;
  }

  // Hash password using real hash_utils (salted SHA256)
  result = onvif_hash_password(password, password_hash, sizeof(password_hash));
  if (result != ONVIF_SUCCESS) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return result;
  }

  // Store new user
  strncpy(g_config_runtime_app_config->users[user_index].username, username, sizeof(g_config_runtime_app_config->users[user_index].username) - 1);
  g_config_runtime_app_config->users[user_index].username[sizeof(g_config_runtime_app_config->users[user_index].username) - 1] = '\0';
  strncpy(g_config_runtime_app_config->users[user_index].password_hash, password_hash,
          sizeof(g_config_runtime_app_config->users[user_index].password_hash) - 1);
  g_config_runtime_app_config->users[user_index].password_hash[sizeof(g_config_runtime_app_config->users[user_index].password_hash) - 1] = '\0';
  g_config_runtime_app_config->users[user_index].active = 1;

  pthread_mutex_unlock(&g_config_runtime_mutex);
  return ONVIF_SUCCESS;
}

int config_runtime_authenticate_user(const char* username, const char* password) {
  int user_index = -1;
  int result = ONVIF_ERROR_INVALID_PARAMETER;

  if (!username || !password) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  // Find user
  for (int i = 0; i < MAX_USERS; i++) {
    if (g_config_runtime_app_config->users[i].active && strcmp(g_config_runtime_app_config->users[i].username, username) == 0) {
      user_index = i;
      break;
    }
  }

  if (user_index == -1) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_AUTHENTICATION_FAILED;
  }

  // Verify password using real hash_utils
  result = onvif_verify_password(password, g_config_runtime_app_config->users[user_index].password_hash);
  if (result != ONVIF_SUCCESS) {
    result = ONVIF_ERROR_AUTHENTICATION_FAILED;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);
  return result;
}

// Helper to get user section field pointer
static void* get_user_field_ptr(struct user_credential* user, const char* key, config_value_type_t* out_type) {
  if (strcmp(key, "username") == 0) {
    if (out_type) {
      *out_type = CONFIG_TYPE_STRING;
    }
    return user->username;
  }
  if (strcmp(key, "password_hash") == 0) {
    if (out_type) {
      *out_type = CONFIG_TYPE_STRING;
    }
    return user->password_hash;
  }
  if (strcmp(key, "active") == 0) {
    if (out_type) {
      *out_type = CONFIG_TYPE_INT;
    }
    return &user->active;
  }
  return NULL;
}

// Helper to get section pointer
static void* get_section_ptr(config_section_t section) {
  if (g_config_runtime_app_config == NULL) {
    return NULL;
  }

  // Support only user sections for this simple implementation
  if (section >= CONFIG_SECTION_USER_1 && section <= CONFIG_SECTION_USER_8) {
    int user_index = (int)(section - CONFIG_SECTION_USER_1);
    return &g_config_runtime_app_config->users[user_index];
  }

  return NULL;
}

// Helper to get field pointer
static void* get_field_ptr(config_section_t section, const char* key, config_value_type_t* out_type) {
  void* section_ptr = get_section_ptr(section);
  if (section_ptr == NULL) {
    return NULL;
  }

  // Only handle user sections
  if (section >= CONFIG_SECTION_USER_1 && section <= CONFIG_SECTION_USER_8) {
    return get_user_field_ptr((struct user_credential*)section_ptr, key, out_type);
  }

  return NULL;
}

// Minimal schema for user sections only
static const config_schema_entry_t g_minimal_user_schema[] = {
  {CONFIG_SECTION_USER_1, "user_1", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_1, "user_1", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_1, "user_1", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_2, "user_2", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_2, "user_2", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_2, "user_2", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_3, "user_3", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_3, "user_3", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_3, "user_3", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_4, "user_4", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_4, "user_4", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_4, "user_4", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_5, "user_5", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_5, "user_5", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_5, "user_5", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_6, "user_6", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_6, "user_6", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_6, "user_6", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_7, "user_7", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_7, "user_7", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_7, "user_7", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
  {CONFIG_SECTION_USER_8, "user_8", "username", CONFIG_TYPE_STRING, 0, 0, 0, MAX_USERNAME_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_8, "user_8", "password_hash", CONFIG_TYPE_STRING, 0, 0, 0, MAX_PASSWORD_HASH_LENGTH + 1, ""},
  {CONFIG_SECTION_USER_8, "user_8", "active", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},
};

static const size_t g_minimal_user_schema_count = sizeof(g_minimal_user_schema) / sizeof(g_minimal_user_schema[0]);

const struct application_config* config_runtime_snapshot(void) {
  const struct application_config* snapshot = NULL;

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (g_config_runtime_initialized && g_config_runtime_app_config != NULL) {
    snapshot = g_config_runtime_app_config;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return snapshot;
}

const config_schema_entry_t* config_runtime_get_schema(size_t* count) {
  if (count == NULL) {
    return NULL;
  }

  *count = g_minimal_user_schema_count;
  return g_minimal_user_schema;
}

int config_runtime_get_int(config_section_t section, const char* key, int* out_value) {
  config_value_type_t field_type = CONFIG_TYPE_INT;
  void* field_ptr = NULL;

  if (key == NULL || out_value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL || (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL)) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  *out_value = *(int*)field_ptr;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_get_string(config_section_t section, const char* key, char* out_value, size_t buffer_size) {
  config_value_type_t field_type = CONFIG_TYPE_STRING;
  void* field_ptr = NULL;
  const char* str_value = NULL;
  int result = 0;

  if (key == NULL || out_value == NULL || buffer_size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL || field_type != CONFIG_TYPE_STRING) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  str_value = (const char*)field_ptr;
  result = snprintf(out_value, buffer_size, "%s", str_value);

  if (result < 0 || (size_t)result >= buffer_size) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_get_bool(config_section_t section, const char* key, int* out_value) {
  return config_runtime_get_int(section, key, out_value);
}

int config_runtime_get_float(config_section_t section, const char* key, float* out_value) {
  config_value_type_t field_type = CONFIG_TYPE_FLOAT;
  void* field_ptr = NULL;

  if (key == NULL || out_value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  if (field_type == CONFIG_TYPE_FLOAT) {
    *out_value = *(float*)field_ptr;
  } else if (field_type == CONFIG_TYPE_INT) {
    *out_value = (float)(*(int*)field_ptr);
  } else {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_set_int(config_section_t section, const char* key, int value) {
  void* field_ptr = NULL;
  config_value_type_t field_type = CONFIG_TYPE_INT;

  if (key == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL || (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL)) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    // For non-user sections, just return success (ignore)
    return ONVIF_SUCCESS;
  }

  *(int*)field_ptr = value;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_set_string(config_section_t section, const char* key, const char* value) {
  void* field_ptr = NULL;
  config_value_type_t field_type = CONFIG_TYPE_STRING;
  char* str_field = NULL;

  if (key == NULL || value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL || field_type != CONFIG_TYPE_STRING) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    // For non-user sections, just return success (ignore)
    return ONVIF_SUCCESS;
  }

  str_field = (char*)field_ptr;
  strncpy(str_field, value, MAX_PASSWORD_HASH_LENGTH);
  str_field[MAX_PASSWORD_HASH_LENGTH] = '\0';

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_set_bool(config_section_t section, const char* key, int value) {
  return config_runtime_set_int(section, key, value);
}

int config_runtime_set_float(config_section_t section, const char* key, float value) {
  void* field_ptr = NULL;
  config_value_type_t field_type = CONFIG_TYPE_FLOAT;

  if (key == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  field_ptr = get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    // For non-user sections, just return success (ignore)
    return ONVIF_SUCCESS;
  }

  if (field_type == CONFIG_TYPE_FLOAT) {
    *(float*)field_ptr = value;
  } else if (field_type == CONFIG_TYPE_INT) {
    *(int*)field_ptr = (int)value;
  } else {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

int config_runtime_process_persistence_queue(void) {
  // For this simple implementation, we just return success
  // In a real implementation, this would save to file
  return ONVIF_SUCCESS;
}
