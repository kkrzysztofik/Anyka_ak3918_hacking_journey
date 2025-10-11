/**
 * @file test_config_runtime.c
 * @brief Unit tests for runtime configuration manager
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// CMocka test framework
#include "cmocka_wrapper.h"

// Module under test
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "services/common/onvif_types.h"
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
  result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "http_port", NULL);
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
  result = config_runtime_get_int(CONFIG_SECTION_NETWORK, NULL, &out_value);
  assert_int_equal(ONVIF_ERROR_INVALID_PARAMETER, result);
}

/**
 * @brief Test get integer when not initialized
 */
static void test_unit_config_runtime_get_int_not_initialized(void** state) {
  (void)state;
  int out_value = 0;

  int result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "http_port", &out_value);
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
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
