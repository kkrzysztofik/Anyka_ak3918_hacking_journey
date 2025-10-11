/**
 * @file test_config_runtime.c
 * @brief Unit tests for runtime configuration manager
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CMocka test framework
#include "cmocka_wrapper.h"

// Module under test
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "utils/error/error_handling.h"


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
  struct test_config_runtime_state* test_state =
    calloc(1, sizeof(struct test_config_runtime_state));

  if (test_state == NULL) {
    return -1;
  }

  // Initialize test configuration with default values
  memset(&test_state->test_config, 0, sizeof(struct application_config));

  // Allocate pointer members
  test_state->test_config.network = calloc(1, sizeof(struct network_settings));
  test_state->test_config.device = calloc(1, sizeof(struct device_info));
  test_state->test_config.logging = calloc(1, sizeof(struct logging_settings));
  test_state->test_config.server = calloc(1, sizeof(struct server_settings));

  if (!test_state->test_config.network || !test_state->test_config.device ||
      !test_state->test_config.logging || !test_state->test_config.server) {
    // Clean up on allocation failure
    free(test_state->test_config.network);
    free(test_state->test_config.device);
    free(test_state->test_config.logging);
    free(test_state->test_config.server);
    free(test_state);
    return -1;
  }

  test_state->initialized = 0;

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
      config_runtime_shutdown();
    }
    // Free allocated pointer members
    free(test_state->test_config.network);
    free(test_state->test_config.device);
    free(test_state->test_config.logging);
    free(test_state->test_config.server);
    free(test_state);
  }

  return 0;
}

/* ============================================================================
 * Bootstrap Tests
 * ============================================================================
 */

/**
 * @brief Test successful bootstrap with valid config
 */
static void test_unit_config_runtime_bootstrap_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Verify generation counter starts at 0
  uint32_t generation = config_runtime_get_generation();
  assert_int_equal(0, generation);
}

/**
 * @brief Test bootstrap with NULL config parameter
 */
static void test_unit_config_runtime_bootstrap_null_param(void** state) {
  (void)state;

  int result = config_runtime_bootstrap(NULL);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test bootstrap when already initialized
 */
static void test_unit_config_runtime_bootstrap_already_initialized(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // First bootstrap
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Second bootstrap should fail
  result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_ERROR_ALREADY_EXISTS, result);
}

/* ============================================================================
 * Shutdown Tests
 * ============================================================================
 */

/**
 * @brief Test successful shutdown after initialization
 */
static void test_unit_config_runtime_shutdown_success(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize first
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Shutdown
  result = config_runtime_shutdown();
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 0;
}

/**
 * @brief Test shutdown when not initialized
 */
static void test_unit_config_runtime_shutdown_not_initialized(void** state) {
  (void)state;

  int result = config_runtime_shutdown();
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Try to get with NULL output
  result = config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", NULL, 64);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get string with zero buffer size
 */
static void test_unit_config_runtime_get_string_zero_buffer(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;
  char out_value[64] = {0};

  // Initialize first
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  char out_value[64] = {0};

  int result =
    config_runtime_get_string(CONFIG_SECTION_DEVICE, "manufacturer", out_value, sizeof(out_value));
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set HTTP port to 70000 (above maximum of 65535)
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 70000);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test schema validation rejects out-of-bounds string (T026)
 * String fields must respect max_length constraints
 */
static void test_unit_config_runtime_validation_bounds_string_too_long(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Create a string that exceeds maximum length (assume 64 chars for manufacturer)
  char too_long_string[256];
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Attempt to set non-existent key
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "nonexistent_key", 12345);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

/**
 * @brief Test config_runtime_set_int with validation (T028)
 * Successful set within valid range
 */
static void test_unit_config_runtime_set_int_valid(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set valid HTTP port value
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the value was set correctly
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(8080, out_value);

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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set valid manufacturer value
  const char* expected_value = "Anyka Test";
  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", expected_value);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify the value was set correctly
  char out_value[64] = {0};
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
  int result = config_runtime_bootstrap(&test_state->test_config);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Set value and verify immediate availability
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 9090);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Read back immediately - should reflect new value
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(9090, out_value);
}

/**
 * @brief Test persistence queue is populated on config update (T038)
 * Verify that updates are added to persistence queue
 */
static void test_unit_config_runtime_persistence_queue_populated(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Check initial queue status (should be empty)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(0, queue_status);

  // Perform config update
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 9091);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Perform multiple rapid updates to same key
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8001);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8002);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8003);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Queue should have only 1 entry (coalesced)
  int queue_status = config_runtime_get_persistence_status();
  assert_int_equal(1, queue_status);

  // Verify final value is the latest update
  int out_value = 0;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &out_value);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(8003, out_value);
}

/**
 * @brief Test persistence queue processes successfully (T042)
 * Verify queue processing empties the queue
 */
static void test_unit_config_runtime_persistence_queue_process(void** state) {
  struct test_config_runtime_state* test_state = (struct test_config_runtime_state*)*state;

  // Initialize
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Add updates to queue
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Multiple sequential operations (simulating concurrent access)
  for (int i = 0; i < 10; i++) {
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
  int result = config_runtime_bootstrap(&test_state->test_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  test_state->initialized = 1;

  // Update different types
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
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
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_bootstrap_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_bootstrap_null_param, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_bootstrap_already_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_shutdown_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_shutdown_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_apply_defaults_success, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_apply_defaults_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_null_output, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_null_key, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_int_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_null_output, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_zero_buffer, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_string_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_bool_null_output, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_get_bool_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_snapshot_success, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_snapshot_not_initialized, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_generation_increment, setup, teardown),

    /* Schema Validation Tests (User Story 2) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_type_mismatch_string_to_int,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_type_mismatch_int_to_string,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_integer_too_low,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_integer_too_high,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_bounds_string_too_long,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_missing_required_key_get,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_validation_missing_required_key_set,
                                    setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_int_valid, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_string_valid, setup, teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_bool_valid, setup, teardown),

    /* Async Persistence Queue Tests (User Story 3) */
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_set_int_immediate_update, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_populated, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_coalescing, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_process, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_thread_safe, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_unit_config_runtime_persistence_queue_mixed_types, setup,
                                    teardown),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
