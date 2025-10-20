/**
 * @file test_config_runtime.c
 * @brief Unit tests for runtime configuration manager
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
#include "services/common/video_config_types.h"
#include "utils/error/error_handling.h"

// Test mocks
#include "mocks/config_mock.h"

#define TEST_TMP_PATH_PREFIX            "/tmp/"
#define TEST_TMP_PATH_PREFIX_LENGTH     (sizeof(TEST_TMP_PATH_PREFIX) - 1U)
#define TEST_HTTP_PORT_INVALID_HIGH     ((int)UINT16_MAX + 1)
#define TEST_STRING_BUFFER_LENGTH       CONFIG_STRING_MEDIUM_LEN
#define TEST_USERNAME_OVERFLOW_LENGTH   (MAX_USERNAME_LENGTH + 2)
#define TEST_HASH_BUFFER_LENGTH         (MAX_PASSWORD_HASH_LENGTH + 1)
#define TEST_HASH_INVALID_BUFFER_LENGTH 64U
#define TEST_INVALID_FPS_HIGH           200
#define TEST_INVALID_BITRATE_LOW        10
#define TEST_INVALID_BITRATE_HIGH       100000
#define TEST_INVALID_WIDTH_HIGH         10000
#define TEST_INVALID_HEIGHT_HIGH        10000
#define TEST_STREAM_WIDTH_MAIN_DEFAULT  1280
#define TEST_STREAM_WIDTH_SUB_DEFAULT   640
#define TEST_STREAM_WIDTH_TERTIARY      320
#define TEST_STREAM_DIMENSION_MIN       10
#define TEST_STREAM_WIDTH_1080P         1920
#define TEST_STREAM_HEIGHT_1080P        1080
#define TEST_STREAM_FPS_STANDARD        30
#define TEST_STREAM_BITRATE_MAIN_KBPS   4000
#define TEST_HTTP_PORT_IMMEDIATE        9090
#define TEST_HTTP_PORT_QUEUE_INITIAL    9091
#define TEST_HTTP_PORT_QUEUE_FIRST      8001
#define TEST_HTTP_PORT_QUEUE_SECOND     8002
#define TEST_HTTP_PORT_QUEUE_THIRD      8003
#define TEST_SERVER_ITERATION_COUNT     10
#define TEST_INVALID_KEY_VALUE          12345
#define TEST_OVERSIZED_STRING_LENGTH    256U

/* ============================================================================
 * Test Fixtures and Setup
 * ============================================================================
 */

/**
 * @brief Test fixture for config_runtime tests
 */
struct test_config_runtime_state {
  struct application_config test_config;
  int initialized;
};

/**
 * @brief Setup function called before each test
 */
static int setup(void** state) {
  struct test_config_runtime_state* test_state = calloc(1, sizeof(struct test_config_runtime_state));

  if (test_state == NULL) {
    return -1;
  }

  // Initialize test configuration with default values
  memset(&test_state->test_config, 0, sizeof(struct application_config));

  test_state->initialized = 0;

  // Enable real config_runtime functions for these tests
  config_mock_use_real_function(true);

  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function called after each test
 */
static int teardown(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  if (test_state != NULL) {
    if (test_state->initialized) {
      config_runtime_cleanup();
    }
    // No memory to free since all fields are now direct struct members
    free(test_state);
  }

  // Restore mock behavior for other test suites
  config_mock_use_real_function(false);

  return 0;
}

/* ============================================================================
 * Bootstrap Tests
 * ============================================================================
 */

/**
 * @brief Test successful bootstrap with valid config
 */
static void test_unit_config_runtime_init_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Verify generation counter starts at 0
  uint32_t generation = config_runtime_get_generation();
  assert_int_equal(0, generation);
}

/**
 * @brief Test bootstrap with NULL config parameter
 */
static void test_unit_config_runtime_init_null_param(void** state) {
  (void)state;

  int result = config_runtime_init(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test bootstrap when already initialized
 */
static void test_unit_config_runtime_init_already_initialized(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // First bootstrap
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Second bootstrap should fail
  result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_ERROR_ALREADY_EXISTS, result);
}

/**
 * @brief Test config_runtime_is_initialized when not initialized
 */
static void test_unit_config_runtime_is_initialized_false(void** state) {
  (void)state;

  // Should return 0 when not initialized
  int initialized = config_runtime_is_initialized();
  assert_int_equal(0, initialized);
}

/**
 * @brief Test config_runtime_is_initialized when initialized
 */
static void test_unit_config_runtime_is_initialized_true(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize config_runtime
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Should return 1 when initialized
  int initialized = config_runtime_is_initialized();
  assert_int_equal(1, initialized);
}

/**
 * @brief Test config_runtime_is_initialized after cleanup
 */
static void test_unit_config_runtime_is_initialized_after_cleanup(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize config_runtime
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Verify it's initialized
  int initialized = config_runtime_is_initialized();
  assert_int_equal(1, initialized);

  // Cleanup
  result = config_runtime_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 0;

  // Should return 0 after cleanup
  initialized = config_runtime_is_initialized();
  assert_int_equal(0, initialized);
}

/* ============================================================================
 * Shutdown Tests
 * ============================================================================
 */

/**
 * @brief Test successful shutdown after initialization
 */
static void test_unit_config_runtime_cleanup_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Shutdown
  result = config_runtime_cleanup();
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 0;
}

/**
 * @brief Test shutdown when not initialized
 */
static void test_unit_config_runtime_cleanup_not_initialized(void** state) {
  (void)state;

  int result = config_runtime_cleanup();
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Apply Defaults Tests
 * ============================================================================
 */

/**
 * @brief Test apply defaults with valid initialization
 */
static void test_unit_config_runtime_apply_defaults_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Apply defaults
  result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify generation counter incremented
  uint32_t generation = config_runtime_get_generation();
  assert_int_equal(1, generation);
}

/**
 * @brief Test apply defaults when not initialized
 */
static void test_unit_config_runtime_apply_defaults_not_initialized(void** state) {
  (void)state;

  int result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Get Integer Tests
 * ============================================================================
 */

/**
 * @brief Test get integer with NULL output parameter
 */
static void test_unit_config_runtime_get_int_null_output(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with NULL output
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get integer with NULL key parameter
 */
static void test_unit_config_runtime_get_int_null_key(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;
  int out_value = 0;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with NULL key
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, NULL, &out_value);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get integer when not initialized
 */
static void test_unit_config_runtime_get_int_not_initialized(void** state) {
  (void)state;
  int out_value = 0;

  int result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Get String Tests
 * ============================================================================
 */

/**
 * @brief Test get string with NULL output parameter
 */
static void test_unit_config_runtime_get_string_null_output(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with NULL output
  result = config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", NULL, TEST_STRING_BUFFER_LENGTH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get string with zero buffer size
 */
static void test_unit_config_runtime_get_string_zero_buffer(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;
  char out_value[TEST_STRING_BUFFER_LENGTH] = {0};

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with zero buffer size
  result = config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", out_value, 0);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get string when not initialized
 */
static void test_unit_config_runtime_get_string_not_initialized(void** state) {
  (void)state;
  char out_value[TEST_STRING_BUFFER_LENGTH] = {0};

  int result = config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", out_value, sizeof(out_value));
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Get Boolean Tests
 * ============================================================================
 */

/**
 * @brief Test get boolean with NULL output parameter
 */
static void test_unit_config_runtime_get_bool_null_output(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with NULL output
  result = config_runtime_get_bool(CONFIG_SECTION_DEVICE, "enabled", NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get boolean when not initialized
 */
static void test_unit_config_runtime_get_bool_not_initialized(void** state) {
  (void)state;
  int out_value = 0;

  int result = config_runtime_get_bool(CONFIG_SECTION_DEVICE, "enabled", &out_value);
  assert_int_equal(ONVIF_ERROR_NOT_INITIALIZED, result);
}

/* ============================================================================
 * Snapshot Tests
 * ============================================================================
 */

/**
 * @brief Test snapshot returns valid pointer when initialized
 */
static void test_unit_config_runtime_snapshot_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Get snapshot
  const struct application_config* snapshot = config_runtime_snapshot();
  assert_non_null(snapshot);

  // Verify it points to our test config
  assert_ptr_equal(snapshot, &test_state->test_config);
}

/**
 * @brief Test snapshot returns NULL when not initialized
 */
static void test_unit_config_runtime_snapshot_not_initialized(void** state) {
  (void)state;

  const struct application_config* snapshot = config_runtime_snapshot();
  assert_null(snapshot);
}

/* ============================================================================
 * Generation Counter Tests
 * ============================================================================
 */

/**
 * @brief Test generation counter increments on updates
 */
static void test_unit_config_runtime_generation_increment(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  uint32_t gen1 = config_runtime_get_generation();

  // Apply defaults should increment generation
  result = config_runtime_apply_defaults();
  assert_int_equal(ONVIF_SUCCESS, result);

  uint32_t gen2 = config_runtime_get_generation();
  assert_true(gen2 > gen1);
}

/* ============================================================================
 * Schema Validation Tests (User Story 2)
 * ============================================================================
 */

/**
 * @brief Test schema validation rejects type mismatch (T025)
 * Attempt to set string value on integer field should fail
 */
static void test_unit_config_runtime_validation_type_mismatch_string_to_int(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set string value on integer field (http_port)
  // This should fail with type mismatch error
  result = config_runtime_set_string(CONFIG_SECTION_ONVIF, "http_port", "not_a_number");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects type mismatch (T025)
 * Attempt to set integer value on string field should fail
 */
static void test_unit_config_runtime_validation_type_mismatch_int_to_string(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to get string field as integer (manufacturer)
  // This should fail with type mismatch error
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_DEVICE, "manufacturer", &out_value);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects out-of-bounds integer (T026)
 * HTTP port must be within valid range (1-65535)
 */
static void test_unit_config_runtime_validation_bounds_integer_too_low(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set HTTP port to 0 (below minimum of 1)
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 0);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects out-of-bounds integer (T026)
 * HTTP port must be within valid range (1-65535)
 */
static void test_unit_config_runtime_validation_bounds_integer_too_high(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set HTTP port above maximum of 65535
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_INVALID_HIGH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects out-of-bounds string (T026)
 * String fields must respect max_length constraints
 */
static void test_unit_config_runtime_validation_bounds_string_too_long(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Create a string that exceeds maximum length (assume 64 chars for manufacturer)
  char too_long_string[TEST_OVERSIZED_STRING_LENGTH];
  memset(too_long_string, 'A', sizeof(too_long_string) - 1);
  too_long_string[sizeof(too_long_string) - 1] = '\0';

  // Attempt to set manufacturer to excessively long string
  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", too_long_string);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects missing required key (T027)
 * Attempt to access non-existent configuration key
 */
static void test_unit_config_runtime_validation_missing_required_key_get(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to get non-existent key
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "nonexistent_key", &out_value);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test schema validation rejects missing required key (T027)
 * Attempt to set non-existent configuration key
 */
static void test_unit_config_runtime_validation_missing_required_key_set(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set non-existent key
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "nonexistent_key", TEST_INVALID_KEY_VALUE);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test config_runtime_set_int with validation (T028)
 * Successful set within valid range
 */
static void test_unit_config_runtime_set_int_valid(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set valid HTTP port value
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", HTTP_PORT_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the value was set correctly
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(HTTP_PORT_DEFAULT, out_value);

  // Verify generation counter incremented
  uint32_t generation = config_runtime_get_generation();
  assert_true(generation > 0);
}

/**
 * @brief Test config_runtime_set_string with validation (T028)
 * Successful set within valid length constraints
 */
static void test_unit_config_runtime_set_string_valid(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set valid manufacturer value
  const char* expected_value = "Anyka Test";
  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", expected_value);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the value was set correctly
  char out_value[TEST_STRING_BUFFER_LENGTH] = {0};
  result = config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", out_value, sizeof(out_value));
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal(expected_value, out_value);
}

/**
 * @brief Test config_runtime_set_bool with validation (T028)
 * Successful set with boolean validation
 */
static void test_unit_config_runtime_set_bool_valid(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set boolean value to true (using logging->enabled)
  result = config_runtime_set_bool(CONFIG_SECTION_LOGGING, "enabled", 1);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the value was set correctly
  int out_value = 0;
  result = config_runtime_get_bool(CONFIG_SECTION_LOGGING, "enabled", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1, out_value);
}

/* ============================================================================
 * Async Persistence Queue Tests (User Story 3)
 * ============================================================================
 */

/**
 * @brief Test config_runtime_set_int triggers immediate in-memory update (T037)
 * Verify that value is immediately available after set
 */
static void test_unit_config_runtime_set_int_immediate_update(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set value and verify immediate availability
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_IMMEDIATE);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Read back immediately - should reflect new value
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(TEST_HTTP_PORT_IMMEDIATE, out_value);
}

/**
 * @brief Test persistence queue is populated on config update (T038)
 * Verify that updates are added to persistence queue
 */
static void test_unit_config_runtime_persistence_queue_populated(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Check initial queue status (should be empty)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(0, queue_status);

  // Perform config update
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_QUEUE_INITIAL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify queue now has 1 pending operation
  queue_status = config_runtime_get_persistence_status();
  assert_int_equal(1, queue_status);
}

/**
 * @brief Test persistence queue coalescing for rapid updates (T039)
 * Multiple updates to same key should coalesce to single persistence operation
 */
static void test_unit_config_runtime_persistence_queue_coalescing(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Perform multiple rapid updates to same key
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_QUEUE_FIRST);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_QUEUE_SECOND);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", TEST_HTTP_PORT_QUEUE_THIRD);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Queue should have only 1 entry (coalesced)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(1, queue_status);

  // Verify final value is the latest update
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(TEST_HTTP_PORT_QUEUE_THIRD, out_value);
}

/**
 * @brief Test persistence queue processes successfully (T042)
 * Verify queue processing empties the queue
 */
static void test_unit_config_runtime_persistence_queue_process(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Add updates to queue
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", HTTP_PORT_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", "Anyka");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify queue has pending operations
  int queue_status = config_runtime_get_persistence_status();
  assert_true(queue_status > 0);

  // Process the queue
  result = config_runtime_process_persistence_queue();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify queue is now empty
  queue_status = config_runtime_get_persistence_status();
  assert_int_equal(0, queue_status);
}

/**
 * @brief Test queue operations are thread-safe (T038)
 * Verify no race conditions in queue management
 */
static void test_unit_config_runtime_persistence_queue_thread_safe(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Multiple sequential operations (simulating concurrent access)
  for (int i = 0; i < TEST_SERVER_ITERATION_COUNT; i++) {
    result = config_runtime_set_int(CONFIG_SECTION_SERVER, "worker_threads", i + 1);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  // Verify final value is correct
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_SERVER, "worker_threads", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(10, out_value);

  // Queue should have 1 entry (coalesced from 10 updates)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(1, queue_status);
}

/**
 * @brief Test mixed type updates in persistence queue (T038)
 * Verify queue handles int, string, and bool updates
 */
static void test_unit_config_runtime_persistence_queue_mixed_types(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Update different types
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", HTTP_PORT_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", "Test");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_bool(CONFIG_SECTION_LOGGING, "enabled", 1);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Queue should have 3 entries (different sections/keys)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(3, queue_status);

  // Process queue
  result = config_runtime_process_persistence_queue();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify queue is empty
  queue_status = config_runtime_get_persistence_status();
  assert_int_equal(0, queue_status);
}

/* ============================================================================
 * Stream Profile Configuration Tests (User Story 4)
 * ============================================================================
 */

/**
 * @brief Test stream profile schema validation - valid parameters (T051)
 * Verify that valid stream profile parameters are accepted
 */
static void test_unit_config_runtime_stream_profile_validation_valid(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set valid stream profile parameters for profile 1
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", TEST_STREAM_WIDTH_1080P);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "height", TEST_STREAM_HEIGHT_1080P);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "fps", TEST_STREAM_FPS_STANDARD);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "bitrate", TEST_STREAM_BITRATE_MAIN_KBPS);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify values were set correctly
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1920, out_value);
}

/**
 * @brief Test stream profile limit enforcement - max 4 profiles (T052)
 * Verify that only 4 stream profiles can be configured
 */
static void test_unit_config_runtime_stream_profile_limit_enforcement(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Configure all 4 valid profiles
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", TEST_STREAM_WIDTH_1080P);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_2, "width", TEST_STREAM_WIDTH_MAIN_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_3, "width", TEST_STREAM_WIDTH_SUB_DEFAULT);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_4, "width", TEST_STREAM_WIDTH_TERTIARY);
  assert_int_equal(ONVIF_SUCCESS, result);

  // All 4 profiles should be configurable
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1920, out_value);
}

/**
 * @brief Test stream profile parameter validation - invalid width (T053)
 * Verify that invalid width values are rejected
 */
static void test_unit_config_runtime_stream_profile_invalid_width(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to set invalid width (too small)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", TEST_STREAM_DIMENSION_MIN);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // Try to set invalid width (too large)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "width", TEST_INVALID_WIDTH_HIGH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test stream profile parameter validation - invalid height (T053)
 * Verify that invalid height values are rejected
 */
static void test_unit_config_runtime_stream_profile_invalid_height(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to set invalid height (too small)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "height", TEST_STREAM_DIMENSION_MIN);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // Try to set invalid height (too large)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "height", TEST_INVALID_HEIGHT_HIGH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test stream profile parameter validation - invalid FPS (T053)
 * Verify that invalid FPS values are rejected
 */
static void test_unit_config_runtime_stream_profile_invalid_fps(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to set invalid FPS (zero)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "fps", 0);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // Try to set invalid FPS (too high)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "fps", TEST_INVALID_FPS_HIGH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test stream profile parameter validation - invalid bitrate (T053)
 * Verify that invalid bitrate values are rejected
 */
static void test_unit_config_runtime_stream_profile_invalid_bitrate(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to set invalid bitrate (too low)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "bitrate", TEST_INVALID_BITRATE_LOW);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // Try to set invalid bitrate (too high)
  result = config_runtime_set_int(CONFIG_SECTION_STREAM_PROFILE_1, "bitrate", TEST_INVALID_BITRATE_HIGH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/* ============================================================================
 * User Credential Management Tests (User Story 5)
 * ============================================================================
 */

/**
 * @brief Test user credential schema validation - valid username (T065)
 * Verify that valid usernames are accepted
 */
static void test_unit_config_runtime_user_validation_valid_username(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Valid usernames: 3-32 alphanumeric characters
  result = config_runtime_add_user("user1", "password123");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("admin", "adminpass");
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test user credential schema validation - invalid username too short (T065)
 * Usernames must be at least 3 characters
 */
static void test_unit_config_runtime_user_validation_username_too_short(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to add user with username too short (less than 3 chars)
  result = config_runtime_add_user("ab", "password123");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test user credential schema validation - invalid username too long (T065)
 * Usernames must be at most 32 characters
 */
static void test_unit_config_runtime_user_validation_username_too_long(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to add user with username too long (more than 32 chars)
  char long_username[TEST_USERNAME_OVERFLOW_LENGTH];
  memset(long_username, 'a', MAX_USERNAME_LENGTH + 1);
  long_username[MAX_USERNAME_LENGTH + 1] = '\0';

  result = config_runtime_add_user(long_username, "password123");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test user credential schema validation - invalid username characters (T065)
 * Usernames must contain only alphanumeric characters
 */
static void test_unit_config_runtime_user_validation_username_invalid_chars(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to add user with invalid characters (spaces, special chars)
  result = config_runtime_add_user("user name", "password123");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  result = config_runtime_add_user("user@name", "password123");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test user limit enforcement - maximum 8 users (T066)
 * Verify that only 8 users can be created
 */
static void test_unit_config_runtime_user_limit_enforcement(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Add 8 valid users
  result = config_runtime_add_user("user1", "pass1");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user2", "pass2");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user3", "pass3");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user4", "pass4");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user5", "pass5");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user6", "pass6");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user7", "pass7");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_add_user("user8", "pass8");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Try to add 9th user - should fail
  result = config_runtime_add_user("user9", "pass9");
  assert_int_equal(ONVIF_ERROR_OUT_OF_RESOURCES, result);
}

/**
 * @brief Test password hashing with salted SHA256 (T067)
 * Verify that passwords are properly hashed using salt$hash format
 */
static void test_unit_config_runtime_hash_password_success(void** state) {
  (void)state;

  char hash_output[TEST_HASH_BUFFER_LENGTH] = {0};
  int result = config_runtime_hash_password("testpassword", hash_output, sizeof(hash_output));
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify hash format: salt$hash (32 hex + $ + 64 hex = 97 chars)
  size_t hash_len = strlen(hash_output);
  assert_true(hash_len >= 97); // At least 97 characters

  // Verify presence of $ separator
  char* separator = strchr(hash_output, '$');
  assert_non_null(separator);

  // Verify salt is 32 hex characters
  size_t salt_len = separator - hash_output;
  assert_int_equal(32, salt_len);

  // Verify hash is 64 hex characters after separator
  size_t hash_part_len = strlen(separator + 1);
  assert_int_equal(64, hash_part_len);

  // Verify salt contains only hex digits
  for (size_t i = 0; i < salt_len; i++) {
    assert_true((hash_output[i] >= '0' && hash_output[i] <= '9') || (hash_output[i] >= 'a' && hash_output[i] <= 'f') ||
                (hash_output[i] >= 'A' && hash_output[i] <= 'F'));
  }

  // Verify hash part contains only hex digits
  const char* hash_part = separator + 1;
  for (size_t i = 0; i < hash_part_len; i++) {
    assert_true((hash_part[i] >= '0' && hash_part[i] <= '9') || (hash_part[i] >= 'a' && hash_part[i] <= 'f') ||
                (hash_part[i] >= 'A' && hash_part[i] <= 'F'));
  }
}

/**
 * @brief Test password hashing with NULL parameters (T067)
 */
static void test_unit_config_runtime_hash_password_null_params(void** state) {
  (void)state;

  char hash_output[TEST_HASH_BUFFER_LENGTH] = {0};

  // NULL password
  int result = config_runtime_hash_password(NULL, hash_output, sizeof(hash_output));
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // NULL output buffer
  result = config_runtime_hash_password("password", NULL, TEST_HASH_BUFFER_LENGTH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // Output buffer too small (less than ONVIF_PASSWORD_HASH_SIZE = 128)
  result = config_runtime_hash_password("password", hash_output, TEST_HASH_INVALID_BUFFER_LENGTH);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test password hashing with random salt (T067)
 * Same password should produce DIFFERENT hashes due to random salt
 */
static void test_unit_config_runtime_hash_password_consistency(void** state) {
  (void)state;

  char hash1[TEST_HASH_BUFFER_LENGTH] = {0};
  char hash2[TEST_HASH_BUFFER_LENGTH] = {0};

  int result = config_runtime_hash_password("testpassword", hash1, sizeof(hash1));
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_hash_password("testpassword", hash2, sizeof(hash2));
  assert_int_equal(ONVIF_SUCCESS, result);

  // Same password should produce DIFFERENT hashes due to random salt
  assert_string_not_equal(hash1, hash2);

  // But both should be valid salted hash format (salt$hash)
  char* sep1 = strchr(hash1, '$');
  char* sep2 = strchr(hash2, '$');
  assert_non_null(sep1);
  assert_non_null(sep2);

  // Both should have 32-char salt and 64-char hash
  assert_int_equal(32, sep1 - hash1);
  assert_int_equal(32, sep2 - hash2);
  assert_int_equal(64, strlen(sep1 + 1));
  assert_int_equal(64, strlen(sep2 + 1));
}

/**
 * @brief Test password verification - successful match (T068)
 */
static void test_unit_config_runtime_verify_password_success(void** state) {
  (void)state;

  // Hash a password with salt
  char hash[TEST_HASH_BUFFER_LENGTH] = {0};
  int result = config_runtime_hash_password("mypassword", hash, sizeof(hash));
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify correct password
  result = config_runtime_verify_password("mypassword", hash);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test password verification - failed match (T068)
 */
static void test_unit_config_runtime_verify_password_failure(void** state) {
  (void)state;

  // Hash a password with salt
  char hash[TEST_HASH_BUFFER_LENGTH] = {0};
  int result = config_runtime_hash_password("mypassword", hash, sizeof(hash));
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify incorrect password
  result = config_runtime_verify_password("wrongpassword", hash);
  assert_int_equal(ONVIF_ERROR_AUTHENTICATION_FAILED, result);
}

/**
 * @brief Test password verification with NULL parameters (T068)
 */
static void test_unit_config_runtime_verify_password_null_params(void** state) {
  (void)state;

  char hash[TEST_HASH_BUFFER_LENGTH] = {0};
  config_runtime_hash_password("password", hash, sizeof(hash));

  // NULL password
  int result = config_runtime_verify_password(NULL, hash);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // NULL hash
  result = config_runtime_verify_password("password", NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test user management - add and remove user (T069)
 */
static void test_unit_config_runtime_user_management_add_remove(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Add a user
  result = config_runtime_add_user("testuser", "testpass");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Try to add duplicate user - should fail
  result = config_runtime_add_user("testuser", "otherpass");
  assert_int_equal(ONVIF_ERROR_ALREADY_EXISTS, result);

  // Remove the user
  result = config_runtime_remove_user("testuser");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Try to remove non-existent user - should fail
  result = config_runtime_remove_user("testuser");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test user management - update password (T069)
 */
static void test_unit_config_runtime_user_management_update_password(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Add a user
  result = config_runtime_add_user("testuser", "oldpass");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Update password
  result = config_runtime_update_user_password("testuser", "newpass");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Try to update password for non-existent user - should fail
  result = config_runtime_update_user_password("nonexistent", "somepass");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test user management with NULL parameters (T069)
 */
static void test_unit_config_runtime_user_management_null_params(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_init(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // NULL username in add_user
  result = config_runtime_add_user(NULL, "password");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // NULL password in add_user
  result = config_runtime_add_user("username", NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // NULL username in remove_user
  result = config_runtime_remove_user(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  // NULL parameters in update_user_password
  result = config_runtime_update_user_password(NULL, "newpass");
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);

  result = config_runtime_update_user_password("username", NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/* ============================================================================
 * Test Suite Registration (main() is provided by test_runner.c)
 * ============================================================================
 */

/**
 * @brief Get config_runtime unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_runtime_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_init_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_init_null_param, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_init_already_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_is_initialized_false, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_is_initialized_true, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_is_initialized_after_cleanup, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_cleanup_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_cleanup_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_apply_defaults_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_apply_defaults_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_null_output, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_null_key, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_null_output, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_zero_buffer, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_bool_null_output, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_bool_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_snapshot_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_snapshot_not_initialized, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_generation_increment, setup, teardown),

    /* Schema Validation Tests (User Story 2) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_type_mismatch_string_to_int, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_type_mismatch_int_to_string, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_integer_too_low, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_integer_too_high, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_string_too_long, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_missing_required_key_get, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_missing_required_key_set, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_int_valid, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_string_valid, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_bool_valid, setup, teardown),

    /* Async Persistence Queue Tests (User Story 3) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_int_immediate_update, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_populated, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_coalescing, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_process, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_thread_safe, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_mixed_types, setup, teardown),

    /* Stream Profile Configuration Tests (User Story 4) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_validation_valid, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_limit_enforcement, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_invalid_width, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_invalid_height, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_invalid_fps, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_stream_profile_invalid_bitrate, setup, teardown),

    /* User Credential Management Tests (User Story 5) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_validation_valid_username, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_validation_username_too_short, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_validation_username_too_long, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_validation_username_invalid_chars, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_limit_enforcement, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_hash_password_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_hash_password_null_params, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_hash_password_consistency, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_verify_password_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_verify_password_failure, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_verify_password_null_params, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_management_add_remove, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_management_update_password, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_user_management_null_params, setup, teardown),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
