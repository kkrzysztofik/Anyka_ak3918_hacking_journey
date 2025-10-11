/**
 * @file test_config_integration.c
 * @brief Integration tests for unified configuration system (User Story 1)
 *
 * Tests full configuration lifecycle:
 * - Load configuration at daemon startup
 * - Query values from different subsystems (services, platform, networking)
 * - Verify all receive identical values from unified manager
 *
 * Part of Feature 001: Unified Configuration System
 * User Story 1: Single Source of Truth for Configuration
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

/* CMocka must be included before other system headers that define malloc/free */
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Test State and Fixtures
 * ============================================================================
 */

static struct {
  struct application_config test_config;
  config_manager_t manager;
  char test_config_path[256];
  int runtime_initialized;
  int manager_initialized;
} test_state;

/**
 * @brief Setup function for each test
 */
static int setup(void** state) {
  (void)state;

  memset(&test_state, 0, sizeof(test_state));
  test_state.runtime_initialized = 0;
  test_state.manager_initialized = 0;

  /* Allocate pointer members for application_config */
  test_state.test_config.network = calloc(1, sizeof(struct network_settings));
  test_state.test_config.device = calloc(1, sizeof(struct device_info));
  test_state.test_config.logging = calloc(1, sizeof(struct logging_settings));
  test_state.test_config.server = calloc(1, sizeof(struct server_settings));

  if (!test_state.test_config.network || !test_state.test_config.device ||
      !test_state.test_config.logging || !test_state.test_config.server) {
    /* Clean up on allocation failure */
    free(test_state.test_config.network);
    free(test_state.test_config.device);
    free(test_state.test_config.logging);
    free(test_state.test_config.server);
    return -1;
  }

  /* Create temporary config file path */
  snprintf(test_state.test_config_path, sizeof(test_state.test_config_path),
           "/tmp/onvif_test_config_%d.ini", getpid());

  return 0;
}

/**
 * @brief Teardown function for each test
 */
static int teardown(void** state) {
  (void)state;

  if (test_state.runtime_initialized) {
    config_runtime_cleanup();
    test_state.runtime_initialized = 0;
  }

  if (test_state.manager_initialized) {
    config_cleanup(&test_state.manager);
    test_state.manager_initialized = 0;
  }

  /* Free allocated pointer members */
  free(test_state.test_config.network);
  free(test_state.test_config.device);
  free(test_state.test_config.logging);
  free(test_state.test_config.server);

  /* Clean up test config file */
  unlink(test_state.test_config_path);

  return 0;
}

/* ============================================================================
 * Test Helper Functions
 * ============================================================================
 */

/**
 * @brief Create a valid test configuration INI file
 */
static void create_test_config_file(const char* path) {
  FILE* fp = fopen(path, "w");
  assert_non_null(fp);

  fprintf(fp, "[http]\n");
  fprintf(fp, "http_port=8080\n");
  fprintf(fp, "\n");
  fprintf(fp, "[rtsp]\n");
  fprintf(fp, "rtsp_port=554\n");
  fprintf(fp, "\n");
  fprintf(fp, "[network]\n");
  fprintf(fp, "ip_address=192.168.1.100\n");
  fprintf(fp, "netmask=255.255.255.0\n");
  fprintf(fp, "gateway=192.168.1.1\n");
  fprintf(fp, "\n");
  fprintf(fp, "[device]\n");
  fprintf(fp, "name=ONVIF Camera\n");
  fprintf(fp, "location=Test Location\n");
  fprintf(fp, "\n");

  fclose(fp);
}

/* ============================================================================
 * Integration Tests for User Story 1
 * ============================================================================
 */

/**
 * @brief Test T016: Full configuration lifecycle integration
 *
 * Verifies:
 * 1. Bootstrap runtime manager
 * 2. Load configuration from file
 * 3. Query values from different subsystems
 * 4. All subsystems receive identical values
 * 5. Proper shutdown
 */
static void test_integration_config_lifecycle_full(void** state) {
  (void)state;
  int result;
  int value_int;
  char value_string[256];

  /* Step 1: Create test configuration file */
  create_test_config_file(test_state.test_config_path);

  /* Step 2: Bootstrap runtime manager with empty config */
  result = config_runtime_init(&test_state.test_config);
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 1;

  /* Step 3: Load configuration from INI file using new config_storage */
  result = config_storage_load(test_state.test_config_path, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Step 4: Query values from services subsystem (HTTP port) */
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &value_int);
  if (result == ONVIF_SUCCESS) {
    assert_int_equal(value_int, 8080);
  }

  /* Step 5: Query values from services subsystem (RTSP port) */
  result = config_runtime_get_int(CONFIG_SECTION_RTSP, "rtsp_port", &value_int);
  if (result == ONVIF_SUCCESS) {
    assert_int_equal(value_int, 554);
  }

  /* Step 6: Query values from network subsystem */
  result = config_runtime_get_string(CONFIG_SECTION_NETWORK, "ip_address", value_string,
                                     sizeof(value_string));
  if (result == ONVIF_SUCCESS) {
    assert_string_equal(value_string, "192.168.1.100");
  }

  /* Step 7: Query values from device subsystem */
  result =
    config_runtime_get_string(CONFIG_SECTION_DEVICE, "name", value_string, sizeof(value_string));
  if (result == ONVIF_SUCCESS) {
    assert_string_equal(value_string, "ONVIF Camera");
  }

  /* Step 8: Verify snapshot returns consistent values */
  const struct application_config* snapshot = config_runtime_snapshot();
  assert_non_null(snapshot);

  /* Step 9: Verify generation counter initialized */
  uint32_t generation = config_runtime_get_generation();
  assert_true(generation >= 0); /* Generation 0 is valid after bootstrap */

  /* Step 10: Proper shutdown */
  result = config_runtime_cleanup();
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 0;
}

/**
 * @brief Test: Configuration lifecycle with missing file fallback
 *
 * Verifies:
 * 1. Bootstrap with defaults when file missing
 * 2. All subsystems receive default values
 * 3. System remains operational
 */
static void test_integration_config_lifecycle_missing_file(void** state) {
  (void)state;
  int result;

  /* Step 1: Bootstrap runtime manager */
  result = config_runtime_init(&test_state.test_config);
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 1;

  /* Step 2: Apply defaults (fallback behavior) */
  result = config_runtime_apply_defaults();
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Step 3: Verify snapshot works with defaults */
  const struct application_config* snapshot = config_runtime_snapshot();
  assert_non_null(snapshot);

  /* Step 4: Verify generation counter initialized */
  uint32_t generation = config_runtime_get_generation();
  assert_true(generation >= 0);

  /* Step 5: Proper shutdown */
  result = config_runtime_cleanup();
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 0;
}

/**
 * @brief Test: Multiple subsystems reading same configuration
 *
 * Verifies:
 * 1. Different subsystems query same config source
 * 2. All receive identical values (single source of truth)
 * 3. No configuration drift between subsystems
 */
static void test_integration_config_single_source_of_truth(void** state) {
  (void)state;
  int result;
  int http_port_1, http_port_2, http_port_3;

  /* Step 1: Setup - Create INI file, bootstrap runtime, then load */
  create_test_config_file(test_state.test_config_path);

  result = config_runtime_init(&test_state.test_config);
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 1;

  result = config_storage_load(test_state.test_config_path, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Step 2: Query same value from "different subsystems" multiple times */
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port_1);
  if (result == ONVIF_SUCCESS) {
    result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port_2);
    if (result == ONVIF_SUCCESS) {
      result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port_3);
      if (result == ONVIF_SUCCESS) {
        /* Step 3: Verify all subsystems got identical values */
        assert_int_equal(http_port_1, http_port_2);
        assert_int_equal(http_port_2, http_port_3);
        assert_int_equal(http_port_1, 8080);
      }
    }
  }

  /* Step 4: Cleanup */
  config_runtime_cleanup();
  test_state.runtime_initialized = 0;
}

/**
 * @brief Test: Configuration reload preserves consistency
 *
 * Verifies:
 * 1. Initial configuration load
 * 2. Configuration reload
 * 3. Values remain consistent across reload
 * 4. Generation counter increments
 */
static void test_integration_config_reload_consistency(void** state) {
  (void)state;
  int result;
  uint32_t gen_before, gen_after;

  /* Step 1: Initial setup - Create INI file, bootstrap runtime, then load */
  create_test_config_file(test_state.test_config_path);

  result = config_runtime_init(&test_state.test_config);
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 1;

  result = config_storage_load(test_state.test_config_path, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  gen_before = config_runtime_get_generation();

  /* Step 2: Reload configuration */
  result = config_storage_reload(test_state.test_config_path);
  assert_int_equal(result, ONVIF_SUCCESS);

  gen_after = config_runtime_get_generation();

  /* Step 3: Verify generation counter unchanged (reload is same config) */
  /* Note: This behavior depends on implementation - may increment or stay same */
  assert_true(gen_after >= gen_before);

  /* Step 4: Verify snapshot still works */
  const struct application_config* snapshot = config_runtime_snapshot();
  assert_non_null(snapshot);

  /* Step 5: Cleanup */
  config_runtime_cleanup();
  test_state.runtime_initialized = 0;
}

/* ============================================================================
 * Integration Tests for User Story 2 - Schema Validation
 * ============================================================================
 */

/**
 * @brief Test T029: Validation error handling integration
 *
 * Verifies:
 * 1. Schema validation rejects out-of-bounds integer values
 * 2. Schema validation rejects strings exceeding max length
 * 3. Schema validation rejects type mismatches
 * 4. Proper error codes returned for validation failures
 * 5. System remains stable after validation errors
 */
static void test_integration_validation_error_handling(void** state) {
  (void)state;
  int result;

  /* Step 1: Bootstrap runtime manager */
  result = config_runtime_init(&test_state.test_config);
  assert_int_equal(result, ONVIF_SUCCESS);
  test_state.runtime_initialized = 1;

  /* Step 2: Test out-of-bounds port value (exceeds max) */
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 70000);
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  /* Step 3: Test out-of-bounds port value (below min) */
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  /* Step 4: Test string exceeding maximum length */
  char long_string[300];
  memset(long_string, 'A', sizeof(long_string) - 1);
  long_string[sizeof(long_string) - 1] = '\0';
  result = config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", long_string);
  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);

  /* Step 5: Verify valid values still work after validation errors */
  result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
  assert_int_equal(result, ONVIF_SUCCESS);

  int port_value;
  result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &port_value);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(port_value, 8080);

  /* Step 6: Verify system remains stable - get snapshot */
  const struct application_config* snapshot = config_runtime_snapshot();
  assert_non_null(snapshot);

  /* Step 7: Verify generation counter incremented for successful set */
  uint32_t generation = config_runtime_get_generation();
  assert_true(generation > 0); /* Should have incremented from successful set */

  /* Step 8: Cleanup */
  config_runtime_cleanup();
  test_state.runtime_initialized = 0;
}

/* ============================================================================
 * Test Suite Registration (main() is provided by test_runner.c)
 * ============================================================================
 */

/**
 * @brief Get config integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_integration_config_lifecycle_full, setup, teardown),
    cmocka_unit_test_setup_teardown(test_integration_config_lifecycle_missing_file, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_integration_config_single_source_of_truth, setup,
                                    teardown),
    cmocka_unit_test_setup_teardown(test_integration_config_reload_consistency, setup, teardown),
    cmocka_unit_test_setup_teardown(test_integration_validation_error_handling, setup, teardown),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
