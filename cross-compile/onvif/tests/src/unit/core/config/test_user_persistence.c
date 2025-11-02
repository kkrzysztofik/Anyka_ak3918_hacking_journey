/**
 * @file test_user_persistence.c
 * @brief Unit tests for user credentials persistence functionality
 * @author Generated for user persistence testing
 * @date 2025
 */

#include "unit/core/config/test_user_persistence.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "platform/platform.h"

// Module under test
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "utils/error/error_handling.h"

// Test mocks
#include "mocks/config_mock.h"
#include "mocks/network_mock.h"

/* Test configuration file path */
#define TEST_CONFIG_FILE "/tmp/test_user_config.ini"

/* Test setup and teardown */
static int setup_test_config(void** state) {
  /* Enable real config functions instead of mocks */
  config_mock_use_real_function(true);

  /* Enable real config storage functions for file I/O */
  config_mock_storage_use_real_function(true);

  /* Enable real network functions for file I/O */
  network_mock_use_real_function(true);

  /* Platform mock will handle platform_get_executable_path calls automatically */

  struct application_config* app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(app_config);

  int result = config_runtime_init(app_config);
  assert_int_equal(result, ONVIF_SUCCESS);

  result = config_runtime_apply_defaults();
  assert_int_equal(result, ONVIF_SUCCESS);

  *state = app_config;
  return 0;
}

static int teardown_test_config(void** state) {
  struct application_config* app_config = (struct application_config*)*state;

  config_runtime_cleanup();
  free(app_config);

  /* Restore mock behavior for other test suites */
  config_mock_use_real_function(false);

  /* Clean up test file */
  unlink(TEST_CONFIG_FILE);

  return 0;
}

/* Test user schema entries are included in schema */
static void test_user_schema_entries_exist(void** state) {
  (void)state;

  size_t schema_count = 0;
  const config_schema_entry_t* schema = config_runtime_get_schema(&schema_count);

  assert_non_null(schema);
  assert_true(schema_count > 0);

  /* Check that user schema entries exist */
  int user_1_found = 0;
  int user_2_found = 0;

  for (size_t i = 0; i < schema_count; i++) {
    if (schema[i].section == CONFIG_SECTION_USER_1 && strcmp(schema[i].key, "username") == 0) {
      user_1_found = 1;
      assert_string_equal(schema[i].section_name, "user_1");
      assert_int_equal(schema[i].type, CONFIG_TYPE_STRING);
    }
    if (schema[i].section == CONFIG_SECTION_USER_2 && strcmp(schema[i].key, "password_hash") == 0) {
      user_2_found = 1;
      assert_string_equal(schema[i].section_name, "user_2");
      assert_int_equal(schema[i].type, CONFIG_TYPE_STRING);
    }
  }

  assert_true(user_1_found);
  assert_true(user_2_found);
}

/* Test user field handlers using public API */
static void test_user_field_pointer_handlers(void** state) {
  (void)state;

  /* Test getting user field values using public API */
  char username[64];
  int result = config_runtime_get_string(CONFIG_SECTION_USER_1, "username", username, sizeof(username));
  assert_int_equal(result, ONVIF_SUCCESS);

  char password_hash[128];
  result = config_runtime_get_string(CONFIG_SECTION_USER_1, "password_hash", password_hash, sizeof(password_hash));
  assert_int_equal(result, ONVIF_SUCCESS);

  int active;
  result = config_runtime_get_int(CONFIG_SECTION_USER_1, "active", &active);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Test setting user field values */
  result = config_runtime_set_string(CONFIG_SECTION_USER_1, "username", "testuser");
  assert_int_equal(result, ONVIF_SUCCESS);

  result = config_runtime_set_string(CONFIG_SECTION_USER_1, "password_hash", "test_hash");
  assert_int_equal(result, ONVIF_SUCCESS);

  result = config_runtime_set_int(CONFIG_SECTION_USER_1, "active", 1);
  assert_int_equal(result, ONVIF_SUCCESS);
}

/* Test user add operation with persistence queue */
static void test_user_add_with_persistence_queue(void** state) {
  (void)state;

  /* Add a user */
  int result = config_runtime_add_user("testuser", "testpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Check that persistence queue has entries */
  int queue_status = config_runtime_get_persistence_status();
  assert_true(queue_status > 0);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify queue is empty after processing */
  queue_status = config_runtime_get_persistence_status();
  assert_int_equal(queue_status, 0);

  /* Verify user was added to runtime config */
  char username[64];
  result = config_runtime_get_string(CONFIG_SECTION_USER_1, "username", username, sizeof(username));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(username, "testuser");
}

/* Test user authentication after persistence */
static void test_user_authentication_after_persistence(void** state) {
  (void)state;

  /* Add a user */
  int result = config_runtime_add_user("authtest", "authpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Test authentication */
  result = config_runtime_authenticate_user("authtest", "authpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Test wrong password */
  result = config_runtime_authenticate_user("authtest", "wrongpass");
  assert_int_equal(result, ONVIF_ERROR_AUTHENTICATION_FAILED);

  /* Test non-existent user */
  result = config_runtime_authenticate_user("nonexistent", "pass");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/* Test user remove operation with persistence queue */
static void test_user_remove_with_persistence_queue(void** state) {
  (void)state;

  /* Add a user first */
  int result = config_runtime_add_user("removetest", "removepass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify user exists */
  result = config_runtime_authenticate_user("removetest", "removepass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Remove user */
  result = config_runtime_remove_user("removetest");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Check that persistence queue has entries */
  int queue_status = config_runtime_get_persistence_status();
  assert_true(queue_status > 0);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify user no longer exists */
  result = config_runtime_authenticate_user("removetest", "removepass");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/* Test user password update with persistence queue */
static void test_user_password_update_with_persistence_queue(void** state) {
  (void)state;

  /* Add a user first */
  int result = config_runtime_add_user("updatetest", "oldpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify old password works */
  result = config_runtime_authenticate_user("updatetest", "oldpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Update password */
  result = config_runtime_update_user_password("updatetest", "newpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Check that persistence queue has entries */
  int queue_status = config_runtime_get_persistence_status();
  assert_true(queue_status > 0);

  /* Process persistence queue */
  result = config_runtime_process_persistence_queue();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify old password no longer works */
  result = config_runtime_authenticate_user("updatetest", "oldpass");
  assert_int_equal(result, ONVIF_ERROR_AUTHENTICATION_FAILED);

  /* Verify new password works */
  result = config_runtime_authenticate_user("updatetest", "newpass");
  assert_int_equal(result, ONVIF_SUCCESS);
}

/* Test user enumeration */
static void test_user_enumeration(void** state) {
  (void)state;

  /* Add multiple users */
  int result = config_runtime_add_user("user1", "pass1");
  assert_int_equal(result, ONVIF_SUCCESS);

  result = config_runtime_add_user("user2", "pass2");
  assert_int_equal(result, ONVIF_SUCCESS);

  result = config_runtime_add_user("user3", "pass3");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Enumerate users */
  char usernames[8][MAX_USERNAME_LENGTH + 1];
  int user_count = 0;

  result = config_runtime_enumerate_users(usernames, 8, &user_count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(user_count, 3);

  /* Check that all users are present */
  int user1_found = 0, user2_found = 0, user3_found = 0;
  for (int i = 0; i < user_count; i++) {
    if (strcmp(usernames[i], "user1") == 0)
      user1_found = 1;
    if (strcmp(usernames[i], "user2") == 0)
      user2_found = 1;
    if (strcmp(usernames[i], "user3") == 0)
      user3_found = 1;
  }

  assert_true(user1_found);
  assert_true(user2_found);
  assert_true(user3_found);
}

/* Test config file serialization includes user sections */
static void test_config_serialization_includes_users(void** state) {
  (void)state;

  /* Add a user */
  int result = config_runtime_add_user("serialtest", "serialpass");
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Save configuration to file */
  result = config_storage_save(TEST_CONFIG_FILE, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Read the file and check for user section */
  FILE* file = fopen(TEST_CONFIG_FILE, "r");
  assert_non_null(file);

  char line[256];
  int user_section_found = 0;
  int username_found = 0;
  int password_hash_found = 0;
  int active_found = 0;

  while (fgets(line, sizeof(line), file)) {
    if (strstr(line, "[user_1]") != NULL) {
      user_section_found = 1;
    }
    if (strstr(line, "username = serialtest") != NULL) {
      username_found = 1;
    }
    if (strstr(line, "password_hash = ") != NULL && strstr(line, "serialtest") == NULL) {
      password_hash_found = 1;
    }
    if (strstr(line, "active = 1") != NULL) {
      active_found = 1;
    }
  }

  fclose(file);

  assert_true(user_section_found);
  assert_true(username_found);
  assert_true(password_hash_found);
  assert_true(active_found);
}

/* Test config file deserialization loads user sections */
static void test_config_deserialization_loads_users(void** state) {
  (void)state;

  /* Create a test config file with user data */
  FILE* file = fopen(TEST_CONFIG_FILE, "w");
  assert_non_null(file);

  fprintf(file, "[user_1]\n");
  fprintf(file, "username = loadtest\n");
  fprintf(file, "password_hash = test_hash_12345\n");
  fprintf(file, "active = 1\n");

  fclose(file);

  /* Load configuration from file */
  int result = config_storage_load(TEST_CONFIG_FILE, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify user data was loaded */
  char username[64];
  result = config_runtime_get_string(CONFIG_SECTION_USER_1, "username", username, sizeof(username));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(username, "loadtest");

  char password_hash[128];
  result = config_runtime_get_string(CONFIG_SECTION_USER_1, "password_hash", password_hash, sizeof(password_hash));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(password_hash, "test_hash_12345");

  int active;
  result = config_runtime_get_int(CONFIG_SECTION_USER_1, "active", &active);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(active, 1);
}

/* Test error handling for invalid user operations */
static void test_user_error_handling(void** state) {
  (void)state;

  /* Test adding user with invalid username */
  int result = config_runtime_add_user("ab", "pass"); /* Too short */
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  /* Test adding user with NULL parameters */
  result = config_runtime_add_user(NULL, "pass");
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  result = config_runtime_add_user("user", NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  /* Test removing non-existent user */
  result = config_runtime_remove_user("nonexistent");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  /* Test updating password for non-existent user */
  result = config_runtime_update_user_password("nonexistent", "newpass");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  /* Test authentication with NULL parameters */
  result = config_runtime_authenticate_user(NULL, "pass");
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  result = config_runtime_authenticate_user("user", NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);
}

/* Test maximum user limit */
static void test_maximum_user_limit(void** state) {
  (void)state;

  /* Add maximum number of users */
  for (int i = 0; i < MAX_USERS; i++) {
    char username[32];
    char password[32];
    snprintf(username, sizeof(username), "user%d", i);
    snprintf(password, sizeof(password), "pass%d", i);

    int result = config_runtime_add_user(username, password);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  /* Try to add one more user (should fail) */
  int result = config_runtime_add_user("extratest", "extrapass");
  assert_int_equal(result, ONVIF_ERROR_OUT_OF_RESOURCES);

  /* Verify all users exist */
  char usernames[MAX_USERS][MAX_USERNAME_LENGTH + 1];
  int user_count = 0;

  result = config_runtime_enumerate_users(usernames, MAX_USERS, &user_count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(user_count, MAX_USERS);
}

/* Export functions for test suite integration */
const struct CMUnitTest* get_user_persistence_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_user_schema_entries_exist, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_field_pointer_handlers, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_add_with_persistence_queue, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_authentication_after_persistence, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_remove_with_persistence_queue, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_password_update_with_persistence_queue, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_enumeration, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_config_serialization_includes_users, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_config_deserialization_loads_users, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_user_error_handling, setup_test_config, teardown_test_config),
    cmocka_unit_test_setup_teardown(test_maximum_user_limit, setup_test_config, teardown_test_config),
  };
  if (count != NULL) {
    *count = sizeof(tests) / sizeof(tests[0]);
  }
  return tests;
}
