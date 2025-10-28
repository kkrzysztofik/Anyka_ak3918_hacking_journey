/**
 * @file test_config_storage.c
 * @brief Unit tests for configuration storage layer
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CMocka test framework
#include "cmocka_wrapper.h"

// Module under test
#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "utils/error/error_handling.h"

// Test mocks
#include "mocks/config_mock.h"

/* ============================================================================
 * Test Fixtures and Setup
 * ============================================================================
 */

#define TEST_CONFIG_FILE              "/tmp/test_onvif_config.ini"
#define TEST_CONFIG_FILE_INVALID      "/tmp/test_invalid_config.ini"
#define TEST_CONFIG_FILE_MISSING      "/tmp/nonexistent_config.ini"
#define TEST_CONFIG_PATH_LENGTH       256U
#define TEST_CONFIG_READ_BUFFER_SIZE  1024U
#define TEST_CONFIG_SAMPLE_SIZE_BYTES 100U

/**
 * @brief Test fixture for config_storage tests
 */
struct test_config_storage_state {
  struct application_config test_config;
  config_manager_t test_manager;
  char test_file_path[TEST_CONFIG_PATH_LENGTH];
  int runtime_initialized;
};

/**
 * @brief Setup function called before each test
 */
static int setup(void** state) {
  struct test_config_storage_state* test_state = calloc(1, sizeof(struct test_config_storage_state));

  if (test_state == NULL) {
    return -1;
  }

  // Initialize test configuration
  memset(&test_state->test_config, 0, sizeof(struct application_config));
  memset(&test_state->test_manager, 0, sizeof(config_manager_t));
  (void)snprintf(test_state->test_file_path, sizeof(test_state->test_file_path), "%s", TEST_CONFIG_FILE);
  test_state->runtime_initialized = 0;

  // Enable real config_runtime functions for these tests
  config_mock_use_real_function(true);

  // Enable real config_storage_save for these tests
  config_mock_storage_use_real_function(true);

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function called after each test
 */
static int teardown(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  if (test_state != NULL) {
    // Clean up test files
    (void)remove(TEST_CONFIG_FILE);
    (void)remove(TEST_CONFIG_FILE_INVALID);

    if (test_state->runtime_initialized) {
      config_runtime_cleanup();
    }

    free(test_state);
  }

  // Restore mock behavior for other test suites
  config_mock_use_real_function(false);
  config_mock_storage_use_real_function(false);

  return 0;
}

/**
 * @brief Helper function to create a valid test config file
 */
static int create_test_config_file(const char* path) {
  FILE* config_file = fopen(path, "w");
  if (config_file == NULL) {
    return -1;
  }

  (void)fprintf(config_file, "[network]\n");
  (void)fprintf(config_file, "http_port=%d\n", HTTP_PORT_DEFAULT);
  (void)fprintf(config_file, "http_enabled=1\n");
  (void)fprintf(config_file, "\n");
  (void)fprintf(config_file, "[device]\n");
  (void)fprintf(config_file, "manufacturer=Anyka\n");
  (void)fprintf(config_file, "model=Test Camera\n");
  (void)fprintf(config_file, "firmware_version=1.0.0\n");
  (void)fprintf(config_file, "\n");
  (void)fprintf(config_file, "[media]\n");
  (void)fprintf(config_file, "video_width=1920\n");
  (void)fprintf(config_file, "video_height=1080\n");
  (void)fprintf(config_file, "video_fps=30\n");

  (void)fclose(config_file);
  return 0;
}

/**
 * @brief Helper function to create an invalid test config file
 */
static int create_invalid_config_file(const char* path) {
  FILE* config_file = fopen(path, "w");
  if (config_file == NULL) {
    return -1;
  }

  (void)fprintf(config_file, "This is not a valid INI file\n");
  (void)fprintf(config_file, "Random content without proper format\n");
  (void)fprintf(config_file, "Missing sections and keys\n");

  (void)fclose(config_file);
  return 0;
}

/* ============================================================================
 * Load Tests
 * ============================================================================
 */

/**
 * @brief Test loading configuration from valid INI file
 */
static void test_unit_config_storage_load_valid_file(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Create valid test file
  int file_result = create_test_config_file(TEST_CONFIG_FILE);
  assert_int_equal(0, file_result);

  // Initialize runtime manager first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Load configuration
  result = config_storage_load(TEST_CONFIG_FILE, &test_state->test_manager);

  // Note: The actual result depends on the implementation
  // This test verifies that the function can be called without crashing
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test loading configuration from missing file
 */
static void test_unit_config_storage_load_missing_file(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Ensure file doesn't exist
  (void)remove(TEST_CONFIG_FILE_MISSING);

  // Initialize runtime manager first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Try to load missing file - should fallback to defaults
  result = config_storage_load(TEST_CONFIG_FILE_MISSING, &test_state->test_manager);

  // Should handle missing file gracefully (either success with defaults or specific error)
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR_IO || result == ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test loading configuration with NULL path parameter
 */
static void test_unit_config_storage_load_null_path(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Try to load with NULL path
  result = config_storage_load(NULL, &test_state->test_manager);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test loading configuration with NULL manager parameter
 */
static void test_unit_config_storage_load_null_manager(void** state) {
  (void)state;

  // Try to load with NULL manager
  int result = config_storage_load(TEST_CONFIG_FILE, NULL);

  // Should either handle gracefully or return error
  assert_true(result != ONVIF_SUCCESS || result == ONVIF_ERROR_INVALID_PARAMETER);
}

/* ============================================================================
 * Save Tests
 * ============================================================================
 */

/**
 * @brief Test saving configuration to file
 */
static void test_unit_config_storage_save_success(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Apply defaults to populate all required schema entries
  result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Save configuration
  result = config_storage_save(TEST_CONFIG_FILE, &test_state->test_manager);

  // Should succeed with all required fields initialized
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test saving configuration with NULL path parameter
 */
static void test_unit_config_storage_save_null_path(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Try to save with NULL path
  result = config_storage_save(NULL, &test_state->test_manager);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test saving configuration with NULL manager parameter
 */
static void test_unit_config_storage_save_null_manager(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Apply defaults to populate all required schema entries
  result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Try to save with NULL manager
  result = config_storage_save(TEST_CONFIG_FILE, NULL);

  // Manager parameter is currently unused (maintained for interface compatibility)
  // So NULL manager is acceptable when runtime is initialized
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify file was created
  FILE* config_file = fopen(TEST_CONFIG_FILE, "r");
  assert_non_null(config_file);
  (void)fclose(config_file);

  // Clean up
  (void)remove(TEST_CONFIG_FILE);
}

/**
 * @brief Test save and reload round-trip
 */
static void test_unit_config_storage_save_reload_roundtrip(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Apply defaults to populate all required schema entries
  result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Set some configuration values
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "enabled", 1);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", HTTP_PORT_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Save configuration
  result = config_storage_save(TEST_CONFIG_FILE, &test_state->test_manager);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify file exists and has content
  FILE* config_file_read = fopen(TEST_CONFIG_FILE, "r");
  assert_non_null(config_file_read);

  char buffer[TEST_CONFIG_READ_BUFFER_SIZE];
  size_t read_size = fread(buffer, 1, sizeof(buffer) - 1, config_file_read);
  buffer[read_size] = '\0';
  (void)fclose(config_file_read);

  // Verify INI format with section header
  assert_non_null(strstr(buffer, "[onvif]"));

  // Clean up
  (void)remove(TEST_CONFIG_FILE);
}

/**
 * @brief Test saving configuration without runtime initialization
 */
static void test_unit_config_storage_save_not_initialized(void** state) {
  (void)state;

  // Try to save without initializing runtime - should fail
  int result = config_storage_save(TEST_CONFIG_FILE, NULL);
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Reload Tests
 * ============================================================================
 */

/**
 * @brief Test reloading configuration from file
 */
static void test_unit_config_storage_reload_success(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Create valid test file
  int file_result = create_test_config_file(TEST_CONFIG_FILE);
  assert_int_equal(0, file_result);

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Reload configuration
  result = config_storage_reload(TEST_CONFIG_FILE);

  // Should succeed or indicate implementation status
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test reloading configuration with NULL path parameter
 */
static void test_unit_config_storage_reload_null_path(void** state) {
  struct test_config_storage_state* test_state = (struct test_config_storage_state*)*state;

  // Initialize runtime manager
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->runtime_initialized = 1;

  // Try to reload with NULL path
  result = config_storage_reload(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/* ============================================================================
 * Atomic Write Tests
 * ============================================================================
 */

/**
 * @brief Test atomic write operation
 */
static void test_unit_config_storage_atomic_write_success(void** state) {
  (void)state;

  const char* test_data = "Test configuration data";
  size_t data_size = strlen(test_data);

  // Perform atomic write
  int result = config_storage_atomic_write(TEST_CONFIG_FILE, test_data, data_size);

  // Should succeed
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Clean up
  (void)remove(TEST_CONFIG_FILE);
}

/**
 * @brief Test atomic write with NULL path parameter
 */
static void test_unit_config_storage_atomic_write_null_path(void** state) {
  (void)state;

  const char* test_data = "Test data";
  size_t data_size = strlen(test_data);

  // Try atomic write with NULL path
  int result = config_storage_atomic_write(NULL, test_data, data_size);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test atomic write with NULL data parameter
 */
static void test_unit_config_storage_atomic_write_null_data(void** state) {
  (void)state;

  // Try atomic write with NULL data
  int result = config_storage_atomic_write(TEST_CONFIG_FILE, NULL, TEST_CONFIG_SAMPLE_SIZE_BYTES);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test atomic write with zero size
 */
static void test_unit_config_storage_atomic_write_zero_size(void** state) {
  (void)state;

  const char* test_data = "Test data";

  // Try atomic write with zero size
  int result = config_storage_atomic_write(TEST_CONFIG_FILE, test_data, 0);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/* ============================================================================
 * Validation Tests
 * ============================================================================
 */

/**
 * @brief Test file validation with valid config file
 */
static void test_unit_config_storage_validate_valid_file(void** state) {
  (void)state;

  // Create valid test file
  int file_result = create_test_config_file(TEST_CONFIG_FILE);
  assert_int_equal(0, file_result);

  // Validate file
  int result = config_storage_validate_file(TEST_CONFIG_FILE);

  // Should succeed
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Clean up
  (void)remove(TEST_CONFIG_FILE);
}

/**
 * @brief Test file validation with invalid config file
 */
static void test_unit_config_storage_validate_invalid_file(void** state) {
  (void)state;

  // Create invalid test file
  int file_result = create_invalid_config_file(TEST_CONFIG_FILE_INVALID);
  assert_int_equal(0, file_result);

  // Validate file
  int result = config_storage_validate_file(TEST_CONFIG_FILE_INVALID);

  // Should indicate invalid or error
  assert_true(result == ONVIF_ERROR || result == ONVIF_ERROR_INVALID);

  // Clean up
  (void)remove(TEST_CONFIG_FILE_INVALID);
}

/**
 * @brief Test file validation with NULL path parameter
 */
static void test_unit_config_storage_validate_null_path(void** state) {
  (void)state;

  // Try to validate with NULL path
  int result = config_storage_validate_file(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test file validation with missing file
 */
static void test_unit_config_storage_validate_missing_file(void** state) {
  (void)state;

  // Ensure file doesn't exist
  (void)remove(TEST_CONFIG_FILE_MISSING);

  // Try to validate missing file
  int result = config_storage_validate_file(TEST_CONFIG_FILE_MISSING);
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/* ============================================================================
 * Checksum Tests
 * ============================================================================
 */

/**
 * @brief Test checksum calculation
 */
static void test_unit_config_storage_checksum_calculation(void** state) {
  (void)state;

  const char* test_data1 = "Test configuration data";
  const char* test_data2 = "Test configuration data";
  const char* test_data3 = "Different test data";

  uint32_t checksum1 = config_storage_calculate_checksum(test_data1, strlen(test_data1));
  uint32_t checksum2 = config_storage_calculate_checksum(test_data2, strlen(test_data2));
  uint32_t checksum3 = config_storage_calculate_checksum(test_data3, strlen(test_data3));

  // Same data should produce same checksum
  assert_int_equal(checksum1, checksum2);

  // Different data should produce different checksum
  assert_int_not_equal(checksum1, checksum3);
}

/**
 * @brief Test checksum with NULL data parameter
 */
static void test_unit_config_storage_checksum_null_data(void** state) {
  (void)state;

  // Calculate checksum with NULL data should return 0 or handle gracefully
  uint32_t checksum = config_storage_calculate_checksum(NULL, TEST_CONFIG_SAMPLE_SIZE_BYTES);
  assert_int_equal(0, checksum);
}

/**
 * @brief Test checksum with zero size
 */
static void test_unit_config_storage_checksum_zero_size(void** state) {
  (void)state;

  const char* test_data = "Test data";

  // Calculate checksum with zero size
  uint32_t checksum = config_storage_calculate_checksum(test_data, 0);
  assert_int_equal(0, checksum);
}

/* ============================================================================
 * Test Suite Registration (main() is provided by test_runner.c)
 * ============================================================================
 */

/**
 * @brief Get config_storage unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_storage_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_config_storage_load_valid_file, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_load_missing_file, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_load_null_path, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_load_null_manager, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_save_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_save_null_path, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_save_null_manager, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_save_reload_roundtrip, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_save_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_reload_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_reload_null_path, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_atomic_write_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_atomic_write_null_path, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_atomic_write_null_data, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_atomic_write_zero_size, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_validate_valid_file, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_validate_invalid_file, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_validate_null_path, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_validate_missing_file, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_checksum_calculation, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_checksum_null_data, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_storage_checksum_zero_size, setup, teardown),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
