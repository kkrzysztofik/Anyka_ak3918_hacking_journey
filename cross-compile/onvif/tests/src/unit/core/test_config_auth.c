/**
 * @file test_config_auth.c
 * @brief Unit tests for authentication configuration functionality
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "core/config/config.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

static int setup_config_auth_tests(void** state) {
  (void)state;
  return 0;
}

static int teardown_config_auth_tests(void** state) {
  (void)state;
  return 0;
}

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

/**
 * @brief Initialize a test application configuration with auth_enabled set
 * @param app_config Pointer to application_config structure to initialize
 * @param auth_enabled Value for auth_enabled field
 */
static void init_test_app_config(struct application_config* app_config, int auth_enabled) {
  memset(app_config, 0, sizeof(struct application_config));

  // Set ONVIF configuration
  app_config->onvif.enabled = 1;
  app_config->onvif.http_port = 8080;
  app_config->onvif.auth_enabled = auth_enabled;
  strcpy(app_config->onvif.username, "admin");
  strcpy(app_config->onvif.password, "admin");
}

/* ============================================================================
 * Configuration Structure Tests
 * ============================================================================ */

/**
 * @brief Test that auth_enabled field exists in onvif_settings structure
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_field_exists(void** state) {
  (void)state;

  struct onvif_settings settings;
  memset(&settings, 0, sizeof(settings));

  // Test that we can set and get the auth_enabled field
  settings.auth_enabled = 1;
  assert_int_equal(settings.auth_enabled, 1);

  settings.auth_enabled = 0;
  assert_int_equal(settings.auth_enabled, 0);
}

/**
 * @brief Test that auth_enabled field is properly positioned in structure
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_field_position(void** state) {
  (void)state;

  struct onvif_settings settings;
  memset(&settings, 0, sizeof(settings));

  // Set values to verify field positions
  settings.enabled = 1;
  settings.http_port = 8080;
  settings.auth_enabled = 1;
  strcpy(settings.username, "testuser");
  strcpy(settings.password, "testpass");

  // Verify all fields are set correctly
  assert_int_equal(settings.enabled, 1);
  assert_int_equal(settings.http_port, 8080);
  assert_int_equal(settings.auth_enabled, 1);
  assert_string_equal(settings.username, "testuser");
  assert_string_equal(settings.password, "testpass");
}

/* ============================================================================
 * Configuration Parameter Tests
 * ============================================================================ */

/**
 * @brief Test that auth_enabled parameter is properly defined in config.c
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_parameter_definition(void** state) {
  (void)state;

  // This test verifies that the auth_enabled parameter is properly defined
  // in the onvif_parameters_init array in config.c
  // We can't directly access the static array, but we can test through
  // the configuration system

  struct application_config app_config;
  init_test_app_config(&app_config, 1);

  config_manager_t config;
  memset(&config, 0, sizeof(config));

  // Test that we can initialize the configuration system
  int result = config_init(&config, &app_config);
  assert_int_equal(result, 0);

  // Test that we can get the auth_enabled value
  int auth_enabled_value;
  result = config_get_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &auth_enabled_value,
                            CONFIG_TYPE_BOOL);
  assert_int_equal(result, 0);
  assert_int_equal(auth_enabled_value, 1);

  // Test that we can set the auth_enabled value
  int new_auth_enabled = 0;
  result = config_set_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &new_auth_enabled,
                            CONFIG_TYPE_BOOL);
  assert_int_equal(result, 0);

  // Verify the value was set
  int retrieved_value;
  result = config_get_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &retrieved_value,
                            CONFIG_TYPE_BOOL);
  assert_int_equal(result, 0);
  assert_int_equal(retrieved_value, 0);

  config_cleanup(&config);
}

/**
 * @brief Test auth_enabled parameter validation
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_parameter_validation(void** state) {
  (void)state;

  struct application_config app_config;
  init_test_app_config(&app_config, 1);

  config_manager_t config;
  memset(&config, 0, sizeof(config));
  int result = config_init(&config, &app_config);
  assert_int_equal(result, 0);

  // Test valid values (0 and 1)
  int valid_values[] = {0, 1};
  for (int i = 0; i < 2; i++) {
    result = config_set_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &valid_values[i],
                              CONFIG_TYPE_BOOL);
    assert_int_equal(result, 0);
  }

  // Test invalid values (should be rejected by validation)
  int invalid_values[] = {-1, 2, 999};
  for (int i = 0; i < 3; i++) {
    result = config_set_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &invalid_values[i],
                              CONFIG_TYPE_BOOL);
    // The validation should reject invalid values
    assert_int_not_equal(result, 0);
  }

  config_cleanup(&config);
}

/* ============================================================================
 * Configuration Loading Tests
 * ============================================================================ */

/**
 * @brief Test loading auth_enabled from INI file
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_ini_loading(void** state) {
  (void)state;

  // Create a temporary INI file with auth_enabled=0
  const char* test_ini_content = "[onvif]\n"
                                 "enabled=1\n"
                                 "http_port=8080\n"
                                 "auth_enabled=0\n"
                                 "username=admin\n"
                                 "password=admin\n";

  FILE* temp_file = tmpfile();
  assert_non_null(temp_file);

  fwrite(test_ini_content, 1, strlen(test_ini_content), temp_file);
  fflush(temp_file);
  rewind(temp_file);

  // Get the temporary file path (this is a bit tricky with tmpfile)
  // For this test, we'll use a different approach
  fclose(temp_file);

  // Create a real temporary file
  char temp_filename[] = "/tmp/test_config_auth_XXXXXX";
  int fd = mkstemp(temp_filename);
  assert_true(fd >= 0);
  expect_any(__wrap_close, fd);
  will_return(__wrap_close, 0);
  close(fd);

  FILE* file = fopen(temp_filename, "w");
  assert_non_null(file);
  fwrite(test_ini_content, 1, strlen(test_ini_content), file);
  fclose(file);

  // Test loading the configuration
  struct application_config app_config;
  init_test_app_config(&app_config, 1);

  config_manager_t config;
  memset(&config, 0, sizeof(config));
  int result = config_init(&config, &app_config);
  assert_int_equal(result, 0);

  // Load from the temporary file
  result = config_load(&config, temp_filename);
  assert_int_equal(result, 0);

  // Verify auth_enabled was loaded as 0
  int auth_enabled_value;
  result = config_get_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &auth_enabled_value,
                            CONFIG_TYPE_BOOL);
  assert_int_equal(result, 0);
  assert_int_equal(auth_enabled_value, 0);

  config_cleanup(&config);

  // Clean up temporary file
  unlink(temp_filename);
}

/**
 * @brief Test default value for auth_enabled when not specified in INI
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_default_value(void** state) {
  (void)state;

  // Create a temporary INI file without auth_enabled
  const char* test_ini_content = "[onvif]\n"
                                 "enabled=1\n"
                                 "http_port=8080\n"
                                 "username=admin\n"
                                 "password=admin\n";

  char temp_filename[] = "/tmp/test_config_auth_default_XXXXXX";
  int fd = mkstemp(temp_filename);
  assert_true(fd >= 0);
  expect_any(__wrap_close, fd);
  will_return(__wrap_close, 0);
  close(fd);

  FILE* file = fopen(temp_filename, "w");
  assert_non_null(file);
  fwrite(test_ini_content, 1, strlen(test_ini_content), file);
  fclose(file);

  // Test loading the configuration
  struct application_config app_config;
  init_test_app_config(&app_config, 1);

  config_manager_t config;
  memset(&config, 0, sizeof(config));
  int result = config_init(&config, &app_config);
  assert_int_equal(result, 0);

  // Load from the temporary file
  result = config_load(&config, temp_filename);
  assert_int_equal(result, 0);

  // Verify auth_enabled has the default value (1)
  int auth_enabled_value;
  result = config_get_value(&config, CONFIG_SECTION_ONVIF, "auth_enabled", &auth_enabled_value,
                            CONFIG_TYPE_BOOL);
  assert_int_equal(result, 0);
  assert_int_equal(auth_enabled_value, 1); // Default should be 1

  config_cleanup(&config);

  // Clean up temporary file
  unlink(temp_filename);
}

/* ============================================================================
 * Configuration Summary Tests
 * ============================================================================ */

/**
 * @brief Test that auth_enabled appears in configuration summary
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_summary(void** state) {
  (void)state;

  struct application_config app_config;
  init_test_app_config(&app_config, 0);

  config_manager_t config;
  memset(&config, 0, sizeof(config));
  int result = config_init(&config, &app_config);
  assert_int_equal(result, 0);

  // Get configuration summary
  char summary[1024];
  result = config_get_summary(&config, summary, sizeof(summary));
  assert_int_equal(result, 0);

  // Verify auth_enabled appears in the summary
  assert_true(strstr(summary, "auth_enabled") != NULL);
  assert_true(strstr(summary, "0") != NULL); // Should show disabled

  config_cleanup(&config);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest config_auth_tests[] = {
  // Configuration Structure Tests
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_field_exists,
                                  setup_config_auth_tests, teardown_config_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_field_position,
                                  setup_config_auth_tests, teardown_config_auth_tests),

  // Configuration Parameter Tests
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_parameter_definition,
                                  setup_config_auth_tests, teardown_config_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_parameter_validation,
                                  setup_config_auth_tests, teardown_config_auth_tests),

  // Configuration Loading Tests
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_ini_loading,
                                  setup_config_auth_tests, teardown_config_auth_tests),
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_default_value,
                                  setup_config_auth_tests, teardown_config_auth_tests),

  // Configuration Summary Tests
  cmocka_unit_test_setup_teardown(test_unit_config_auth_enabled_summary, setup_config_auth_tests,
                                  teardown_config_auth_tests),
};

/**
 * @brief Get config auth unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_auth_unit_tests(size_t* count) {
  *count = sizeof(config_auth_tests) / sizeof(config_auth_tests[0]);
  return config_auth_tests;
}
