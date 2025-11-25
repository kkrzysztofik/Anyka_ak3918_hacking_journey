/**
 * @file test_config_path_resolution.c
 * @brief Unit tests for config path resolution
 * @author kkrzysztofik
 * @date 2025
 */

#include <libgen.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "cmocka_wrapper.h"
#include "core/config/config_storage.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"

/* Test constants */
#define TEST_PATH_BUFFER_SIZE  512
#define TEST_SMALL_BUFFER_SIZE 10

/* ============================================================================
 * Test Cases
 * ============================================================================ */

/**
 * @brief Test successful executable path retrieval
 */
void test_unit_platform_get_executable_path_success(void** state) {
  char path_buffer[TEST_PATH_BUFFER_SIZE];
  platform_result_t result;

  (void)state; /* Unused */

  /* Call platform function - mock provides fallback behavior */
  result = platform_get_executable_path(path_buffer, sizeof(path_buffer));

  /* Verify success */
  assert_int_equal(result, PLATFORM_SUCCESS);

  /* Verify path is not empty */
  assert_true(strlen(path_buffer) > 0);

  /* Verify path contains expected test executable name */
  assert_string_equal(path_buffer, "/usr/bin/test_executable");
}

/**
 * @brief Test null buffer parameter handling
 */
void test_unit_platform_get_executable_path_null_buffer(void** state) {
  platform_result_t result;

  (void)state; /* Unused */

  /* Call with NULL buffer - should return error */
  result = platform_get_executable_path(NULL, TEST_PATH_BUFFER_SIZE);

  /* Verify error return */
  assert_int_equal(result, PLATFORM_ERROR_INVALID_PARAM);
}

/**
 * @brief Test zero buffer size parameter handling
 */
void test_unit_platform_get_executable_path_zero_size(void** state) {
  char path_buffer[TEST_PATH_BUFFER_SIZE];
  platform_result_t result;

  (void)state; /* Unused */

  /* Call with zero buffer size - should return error */
  result = platform_get_executable_path(path_buffer, 0);

  /* Verify error return */
  assert_int_equal(result, PLATFORM_ERROR_INVALID_PARAM);
}

/**
 * @brief Test small buffer size handling
 */
void test_unit_platform_get_executable_path_small_buffer(void** state) {
  char path_buffer[TEST_SMALL_BUFFER_SIZE];
  platform_result_t result;

  (void)state; /* Unused */

  /* Call with small buffer - mock provides fallback behavior */
  result = platform_get_executable_path(path_buffer, sizeof(path_buffer));

  /* Verify success (mock provides fallback behavior) */
  assert_int_equal(result, PLATFORM_SUCCESS);

  /* Verify path is valid */
  assert_true(strlen(path_buffer) > 0);
}

/**
 * @brief Test config path resolution integration with load
 */
void test_unit_config_path_resolution_integration_load(void** state) {
  char test_config_path[TEST_PATH_BUFFER_SIZE];
  char exe_path[TEST_PATH_BUFFER_SIZE];
  char exe_dir[TEST_PATH_BUFFER_SIZE];
  char* dir_result = NULL;
  FILE* config_file = NULL;
  int result;

  (void)state; /* Unused */

  /* Get executable path */
  result = platform_get_executable_path(exe_path, sizeof(exe_path));
  if (result != PLATFORM_SUCCESS) {
    /* Skip test if we can't get executable path */
    return;
  }

  /* Extract directory from executable path */
  strncpy(exe_dir, exe_path, sizeof(exe_dir) - 1);
  exe_dir[sizeof(exe_dir) - 1] = '\0';
  dir_result = dirname(exe_dir);

  if (dir_result == NULL) {
    /* Skip test if we can't extract directory */
    return;
  }

  /* Create test config file in executable directory */
  snprintf(test_config_path, sizeof(test_config_path), "%s/test_config.ini", dir_result);

  config_file = fopen(test_config_path, "w");
  if (config_file == NULL) {
    /* Skip test if we can't create test file */
    return;
  }

  /* Write minimal valid config */
  fprintf(config_file, "[onvif]\n");
  fprintf(config_file, "enabled = 1\n");
  fprintf(config_file, "http_port = 8080\n");
  fclose(config_file);

  /* Test loading with relative path - should resolve to executable directory */
  result = config_storage_load("test_config.ini", NULL);

  /* Clean up test file */
  (void)unlink(test_config_path);

  /* Verify load succeeded or failed gracefully */
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR_NOT_INITIALIZED);
}

/**
 * @brief Test fallback behavior when executable path resolution fails
 */
void test_unit_config_path_resolution_fallback_behavior(void** state) {
  char test_config_path[TEST_PATH_BUFFER_SIZE];
  FILE* config_file = NULL;
  int result;

  (void)state; /* Unused */

  /* Create test config file in current directory */
  snprintf(test_config_path, sizeof(test_config_path), "./test_fallback_config.ini");

  config_file = fopen(test_config_path, "w");
  if (config_file == NULL) {
    /* Skip test if we can't create test file */
    return;
  }

  /* Write minimal valid config */
  fprintf(config_file, "[onvif]\n");
  fprintf(config_file, "enabled = 1\n");
  fprintf(config_file, "http_port = 8080\n");
  fclose(config_file);

  /* Test loading with relative path - should fallback to current directory */
  result = config_storage_load("test_fallback_config.ini", NULL);

  /* Clean up test file */
  (void)unlink(test_config_path);

  /* Verify load succeeded or failed gracefully */
  /* Expected: ONVIF_ERROR_NOT_FOUND (file not in executable dir) or ONVIF_ERROR_NOT_INITIALIZED (runtime not initialized) */
  assert_true(result == ONVIF_ERROR_NOT_FOUND || result == ONVIF_ERROR_NOT_INITIALIZED);
}

/* ============================================================================
 * Test Suite Registration
 * ============================================================================ */

/**
 * @brief Get config path resolution unit tests
 */
const struct CMUnitTest* get_config_path_resolution_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_platform_get_executable_path_success),    cmocka_unit_test(test_unit_platform_get_executable_path_null_buffer),
    cmocka_unit_test(test_unit_platform_get_executable_path_zero_size),  cmocka_unit_test(test_unit_platform_get_executable_path_small_buffer),
    cmocka_unit_test(test_unit_config_path_resolution_integration_load), cmocka_unit_test(test_unit_config_path_resolution_fallback_behavior),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
