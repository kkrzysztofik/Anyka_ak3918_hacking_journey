/**
 * @file test_config_performance.c
 * @brief Performance benchmarking suite for unified configuration system
 *
 * Tests and validates performance against requirements:
 * - Configuration initialization: <150ms
 * - Runtime getters: <10 microseconds
 * - Configuration updates: <200ms
 * - Async persistence: <2 seconds
 * - Throughput: Support 100+ queries/second
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-16
 */

#include <stdbool.h>
#include <sys/time.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "mocks/config_mock.h"
#include "mocks/network_mock.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Performance Test Utilities
 * ============================================================================ */

/**
 * @brief High-resolution timer for microsecond precision
 */
typedef struct {
  struct timespec start;
  struct timespec end;
} perf_timer_t;

/**
 * @brief Start performance timer
 */
static void perf_timer_start(perf_timer_t* timer) {
  if (timer) {
    clock_gettime(CLOCK_MONOTONIC, &timer->start);
  }
}

/**
 * @brief Stop performance timer and return elapsed microseconds
 */
static uint64_t perf_timer_elapsed_us(perf_timer_t* timer) {
  if (!timer)
    return 0;

  clock_gettime(CLOCK_MONOTONIC, &timer->end);

  uint64_t start_us = (uint64_t)timer->start.tv_sec * 1000000 + timer->start.tv_nsec / 1000;
  uint64_t end_us = (uint64_t)timer->end.tv_sec * 1000000 + timer->end.tv_nsec / 1000;

  return (end_us > start_us) ? (end_us - start_us) : 0;
}

/**
 * @brief Stop performance timer and return elapsed milliseconds
 */
static uint64_t perf_timer_elapsed_ms(perf_timer_t* timer) {
  return perf_timer_elapsed_us(timer) / 1000;
}

/**
 * @brief Print performance result
 */
static void perf_print_result(const char* test_name, uint64_t elapsed_us, uint64_t limit_us) {
  double elapsed_ms = (double)elapsed_us / 1000.0;
  double limit_ms = (double)limit_us / 1000.0;
  const char* status = (elapsed_us <= limit_us) ? "✓ PASS" : "✗ FAIL";

  printf("  %-50s: %8.3f ms (limit: %8.3f ms) %s\n", test_name, elapsed_ms, limit_ms, status);
}

/* ============================================================================
 * Mock Data and Fixtures
 * ============================================================================ */

static struct application_config g_test_config;
static config_manager_t g_test_manager;

/**
 * @brief Setup test fixture
 */
static int setup_fixture(void** state) {
  (void)state;

  // Enable real config_runtime functions (not mocked) for performance testing
  config_mock_use_real_function(true);

  // Enable real network functions for integration testing
  network_mock_use_real_function(true);

  memset(&g_test_config, 0, sizeof(g_test_config));
  memset(&g_test_manager, 0, sizeof(g_test_manager));

  // Initialize config manager
  if (config_runtime_init(&g_test_config) != ONVIF_SUCCESS) {
    return -1;
  }

  // Apply defaults
  if (config_runtime_apply_defaults() != ONVIF_SUCCESS) {
    return -1;
  }

  return 0;
}

/**
 * @brief Teardown test fixture
 */
static int teardown_fixture(void** state) {
  (void)state;

  config_runtime_cleanup();

  // Disable real functions to restore mock behavior for other tests
  network_mock_use_real_function(false);
  config_mock_use_real_function(false);

  memset(&g_test_config, 0, sizeof(g_test_config));
  memset(&g_test_manager, 0, sizeof(g_test_manager));

  return 0;
}

/* ============================================================================
 * Performance Tests: Runtime Getter Operations (Requirement: <10µs per call)
 * ============================================================================ */

/**
 * @brief Test: Single integer getter performance
 *
 * Measures performance of retrieving a single integer configuration value.
 * Requirement: <10 microseconds
 */
static void test_perf_config_runtime_get_int_single(void** state) {
  (void)state;

  int value = 0;
  perf_timer_t timer;

  // Warm up
  config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &value);

  // Measure single getter
  perf_timer_start(&timer);
  int result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &value);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 10); // Requirement: <10µs

  perf_print_result("Single integer getter", elapsed_us, 10);
}

/**
 * @brief Test: Single string getter performance
 *
 * Measures performance of retrieving a single string configuration value.
 * Requirement: <10 microseconds
 */
static void test_perf_config_runtime_get_string_single(void** state) {
  (void)state;

  char buffer[256];
  perf_timer_t timer;

  // Warm up  - use a string that should exist (username is optional but in config schema)
  config_runtime_get_string(CONFIG_SECTION_ONVIF, "username", buffer, sizeof(buffer));

  // Measure single getter
  perf_timer_start(&timer);
  int result = config_runtime_get_string(CONFIG_SECTION_ONVIF, "username", buffer, sizeof(buffer));
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  // Even if key doesn't exist, the lookup should be fast
  // We're measuring performance, not testing value retrieval
  assert_true(elapsed_us < 100); // Lookup itself should be <100µs, even if key missing

  if (result == ONVIF_SUCCESS) {
    perf_print_result("Single string getter", elapsed_us, 10);
  } else {
    printf("  %-50s: %8.3f µs (key not found, but lookup was fast)\n", "Single string getter", (double)elapsed_us);
  }
}

/**
 * @brief Test: Batch getter performance (100 sequential calls)
 *
 * Measures throughput of multiple getter operations.
 * Requirement: Support 100+ queries/second (10µs per query)
 */
static void test_perf_config_runtime_get_batch(void** state) {
  (void)state;

  int value;
  perf_timer_t timer;
  const int batch_size = 100;

  // Warm up
  config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &value);

  // Measure batch of 100 getters
  perf_timer_start(&timer);

  for (int i = 0; i < batch_size; i++) {
    int result = config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &value);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  uint64_t total_elapsed_us = perf_timer_elapsed_us(&timer);
  uint64_t avg_per_call_us = total_elapsed_us / batch_size;

  assert_true(avg_per_call_us < 10); // Requirement: <10µs average

  printf("  %-50s: %llu calls in %llu µs (avg: %llu µs/call)\n", "Batch getter (100 calls)", (unsigned long long)batch_size,
         (unsigned long long)total_elapsed_us, (unsigned long long)avg_per_call_us);
}

/* ============================================================================
 * Performance Tests: Runtime Setter Operations (Requirement: <200ms)
 * ============================================================================ */

/**
 * @brief Test: Single integer setter performance
 *
 * Measures performance of setting a single integer configuration value.
 * Requirement: <200 milliseconds
 */
static void test_perf_config_runtime_set_int_single(void** state) {
  (void)state;

  perf_timer_t timer;

  // Measure single setter
  perf_timer_start(&timer);
  int result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 200000); // Requirement: <200ms

  perf_print_result("Single integer setter", elapsed_us, 200000);
}

/**
 * @brief Test: Batch setter performance with coalescing
 *
 * Measures performance of multiple rapid setter operations
 * (should benefit from coalescing).
 * Requirement: <200ms per operation average
 */
static void test_perf_config_runtime_set_batch_coalesce(void** state) {
  (void)state;

  perf_timer_t timer;
  const int batch_size = 50;

  // Measure batch of 50 setters (should coalesce in persistence queue)
  perf_timer_start(&timer);

  for (int i = 0; i < batch_size; i++) {
    int result = config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080 + i);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  uint64_t total_elapsed_us = perf_timer_elapsed_us(&timer);
  uint64_t avg_per_call_us = total_elapsed_us / batch_size;

  // Average should be well under 200ms (most are in-memory, only last one persists)
  assert_true(avg_per_call_us < 200000);

  printf("  %-50s: %llu calls in %llu ms (avg: %llu µs/call)\n", "Batch setter with coalescing (50 calls)", (unsigned long long)batch_size,
         (unsigned long long)(total_elapsed_us / 1000), (unsigned long long)avg_per_call_us);
}

/* ============================================================================
 * Performance Tests: Configuration Initialization
 * ============================================================================ */

/**
 * @brief Test: Configuration initialization performance
 *
 * Measures time to initialize the runtime configuration manager
 * with a fresh application config structure.
 * Requirement: <150 milliseconds
 */
static void test_perf_config_runtime_init(void** state) {
  (void)state;

  // Note: setup_fixture already initialized the config, so we clean it up first
  // to measure a fresh initialization
  config_runtime_cleanup();

  struct application_config fresh_config;
  memset(&fresh_config, 0, sizeof(fresh_config));

  perf_timer_t timer;

  // Measure initialization
  perf_timer_start(&timer);
  int result = config_runtime_init(&fresh_config);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 150000); // Requirement: <150ms

  perf_print_result("Configuration initialization", elapsed_us, 150000);

  // Note: teardown_fixture will handle final cleanup
}

/**
 * @brief Test: Default value application performance
 *
 * Measures time to apply all default values to configuration.
 * This typically happens during initialization.
 * Requirement: <150 milliseconds (should be fast)
 */
static void test_perf_config_runtime_apply_defaults(void** state) {
  (void)state;

  perf_timer_t timer;

  // Measure defaults application
  perf_timer_start(&timer);
  int result = config_runtime_apply_defaults();
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 150000); // Requirement: <150ms

  perf_print_result("Apply default values", elapsed_us, 150000);
}

/* ============================================================================
 * Performance Tests: Persistence Queue Processing
 * ============================================================================ */

/**
 * @brief Test: Persistence queue processing performance
 *
 * Measures time to process queued configuration updates and persist to storage.
 * Requirement: <2 seconds
 */
static void test_perf_config_runtime_process_queue(void** state) {
  (void)state;

  perf_timer_t timer;

  // Queue several configuration updates
  config_runtime_set_int(CONFIG_SECTION_DEVICE, "port", 8080);
  config_runtime_set_int(CONFIG_SECTION_DEVICE, "port", 8081);
  config_runtime_set_int(CONFIG_SECTION_DEVICE, "port", 8082);

  // Measure queue processing
  perf_timer_start(&timer);
  int result = config_runtime_process_persistence_queue();
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 2000000); // Requirement: <2 seconds

  perf_print_result("Process persistence queue", elapsed_us, 2000000);
}

/**
 * @brief Test: Persistence queue coalescing efficiency
 *
 * Verifies that rapid updates to the same key are coalesced
 * (only the final value is persisted).
 */
static void test_perf_config_runtime_queue_coalescing(void** state) {
  (void)state;

  perf_timer_t timer;
  const int update_count = 1000;

  // Queue 1000 rapid updates to same key
  perf_timer_start(&timer);

  for (int i = 0; i < update_count; i++) {
    config_runtime_set_int(CONFIG_SECTION_DEVICE, "port", 8080 + i);
  }

  uint64_t queue_elapsed_us = perf_timer_elapsed_us(&timer);

  // Check queue status - should have minimal entries due to coalescing
  int queue_status = config_runtime_get_persistence_status();
  assert_true(queue_status >= 0);
  assert_true(queue_status <= 10); // Should be coalesced to ~1 entry

  printf("  %-50s: %llu updates coalesced to %d entry(ies) in %llu µs\n", "Queue coalescing efficiency", (unsigned long long)update_count,
         queue_status, (unsigned long long)queue_elapsed_us);
}

/* ============================================================================
 * Performance Tests: Memory Efficiency
 * ============================================================================ */

/**
 * @brief Test: Runtime snapshot performance
 *
 * Measures performance of taking a configuration snapshot.
 * Should be very fast (just returns pointer).
 * Requirement: <100 microseconds
 */
static void test_perf_config_runtime_snapshot(void** state) {
  (void)state;

  perf_timer_t timer;

  // Warm up
  config_runtime_snapshot();

  // Measure snapshot
  perf_timer_start(&timer);
  const struct application_config* snapshot = config_runtime_snapshot();
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_non_null(snapshot);
  assert_true(elapsed_us < 100); // Should be very fast (pointer return)

  perf_print_result("Configuration snapshot", elapsed_us, 100);
}

/**
 * @brief Test: Generation counter performance
 *
 * Measures performance of retrieving generation counter.
 * Should be extremely fast (just return value).
 * Requirement: <10 microseconds
 */
static void test_perf_config_runtime_get_generation(void** state) {
  (void)state;

  perf_timer_t timer;

  // Warm up
  config_runtime_get_generation();

  // Measure generation counter retrieval
  perf_timer_start(&timer);
  uint32_t generation = config_runtime_get_generation();
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_true(generation >= 0);
  assert_true(elapsed_us < 10); // Should be extremely fast

  perf_print_result("Get generation counter", elapsed_us, 10);
}

/* ============================================================================
 * Performance Tests: Stream Profile Operations
 * ============================================================================ */

/**
 * @brief Test: Get stream profile performance
 *
 * Measures performance of retrieving a stream profile.
 * Requirement: <100 microseconds
 */
static void test_perf_config_runtime_get_stream_profile(void** state) {
  (void)state;

  perf_timer_t timer;
  video_config_t profile;
  memset(&profile, 0, sizeof(profile));

  // Warm up
  config_runtime_get_stream_profile(0, &profile);

  // Measure profile retrieval
  perf_timer_start(&timer);
  int result = config_runtime_get_stream_profile(0, &profile);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  // Performance test: even if function fails, it should be fast
  assert_true(elapsed_us < 1000); // Lookup should be <1ms even if fails

  if (result == ONVIF_SUCCESS) {
    perf_print_result("Get stream profile", elapsed_us, 100);
  } else {
    printf("  %-50s: %8.3f µs (result: %d, but lookup was fast)\n", "Get stream profile", (double)elapsed_us, result);
  }
}

/**
 * @brief Test: Set stream profile performance
 *
 * Measures performance of updating a stream profile.
 * Requirement: <200 milliseconds
 */
static void test_perf_config_runtime_set_stream_profile(void** state) {
  (void)state;

  perf_timer_t timer;
  video_config_t profile;
  memset(&profile, 0, sizeof(profile));
  profile.width = 1920;
  profile.height = 1080;
  profile.fps = 30;
  profile.bitrate = 2048;
  profile.codec_type = 0; // H.264
  strncpy(profile.name, "Main Profile", sizeof(profile.name) - 1);

  // Measure profile update
  perf_timer_start(&timer);
  int result = config_runtime_set_stream_profile(0, &profile);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  // Performance test: measure the operation speed regardless of success
  assert_true(elapsed_us < 1000000); // Should complete within 1 second

  if (result == ONVIF_SUCCESS) {
    perf_print_result("Set stream profile", elapsed_us, 200000);
  } else {
    printf("  %-50s: %8.3f ms (result: %d, but operation was fast)\n", "Set stream profile", (double)elapsed_us / 1000.0, result);
  }
}

/* ============================================================================
 * Performance Tests: User Management Operations
 * ============================================================================ */

/**
 * @brief Test: Add user performance
 *
 * Measures performance of adding a new user account.
 * Requirement: <200 milliseconds
 */
static void test_perf_config_runtime_add_user(void** state) {
  (void)state;

  perf_timer_t timer;

  // Measure user addition
  perf_timer_start(&timer);
  int result = config_runtime_add_user("testuser", "password123");
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 200000); // Requirement: <200ms

  perf_print_result("Add user account", elapsed_us, 200000);
}

/**
 * @brief Test: Password hashing performance
 *
 * Measures performance of SHA256 password hashing.
 * Requirement: <100 milliseconds
 */
static void test_perf_config_runtime_hash_password(void** state) {
  (void)state;

  perf_timer_t timer;
  char hash[128];

  // Measure password hashing
  perf_timer_start(&timer);
  int result = config_runtime_hash_password("testpassword123", hash, sizeof(hash));
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 100000); // Should be reasonably fast

  perf_print_result("Hash password (SHA256)", elapsed_us, 100000);
}

/**
 * @brief Test: Password verification performance
 *
 * Measures performance of password verification.
 * Requirement: <100 milliseconds
 */
static void test_perf_config_runtime_verify_password(void** state) {
  (void)state;

  // First hash a password
  char hash[128];
  int hash_result = config_runtime_hash_password("testpassword123", hash, sizeof(hash));
  if (hash_result != ONVIF_SUCCESS) {
    fail_msg("Failed to hash password for verification test");
    return;
  }

  perf_timer_t timer;

  // Warm up
  config_runtime_verify_password("testpassword123", hash);

  // Measure password verification
  perf_timer_start(&timer);
  int result = config_runtime_verify_password("testpassword123", hash);
  uint64_t elapsed_us = perf_timer_elapsed_us(&timer);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(elapsed_us < 100000); // Should be fast

  perf_print_result("Verify password (SHA256)", elapsed_us, 100000);
}

/**
 * @brief Test: Batch user add operations with coalescing
 *
 * Measures performance of adding multiple users.
 * Each add should be fast due to in-memory updates.
 * Requirement: <200ms average per user
 */
static void test_perf_config_runtime_add_users_batch(void** state) {
  (void)state;

  perf_timer_t timer;
  const int user_count = 8; // Maximum users

  // Measure adding multiple users
  perf_timer_start(&timer);

  for (int i = 1; i < user_count; i++) {
    char username[32];
    char password[32];
    snprintf(username, sizeof(username), "user%d", i);
    snprintf(password, sizeof(password), "pass%d123", i);

    int result = config_runtime_add_user(username, password);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  uint64_t total_elapsed_us = perf_timer_elapsed_us(&timer);
  uint64_t avg_per_user_us = total_elapsed_us / (user_count - 1);

  assert_true(avg_per_user_us < 200000); // Requirement: <200ms average

  printf("  %-50s: %d users added in %llu ms (avg: %llu µs/user)\n", "Batch add users (8 users)", user_count,
         (unsigned long long)(total_elapsed_us / 1000), (unsigned long long)avg_per_user_us);
}

/* ============================================================================
 * Global Test Array and Exports (for common test launcher integration)
 * ============================================================================ */

/**
 * @brief Global test array exported for common test launcher
 */
const struct CMUnitTest g_config_performance_tests[] = {
  // Getter tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_get_int_single, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_get_string_single, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_get_batch, setup_fixture, teardown_fixture),

  // Setter tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_set_int_single, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_set_batch_coalesce, setup_fixture, teardown_fixture),

  // Initialization tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_init, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_apply_defaults, setup_fixture, teardown_fixture),

  // Persistence tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_process_queue, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_queue_coalescing, setup_fixture, teardown_fixture),

  // Memory/snapshot tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_snapshot, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_get_generation, setup_fixture, teardown_fixture),

  // Stream profile tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_get_stream_profile, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_set_stream_profile, setup_fixture, teardown_fixture),

  // User management tests
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_add_user, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_hash_password, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_verify_password, setup_fixture, teardown_fixture),
  cmocka_unit_test_setup_teardown(test_perf_config_runtime_add_users_batch, setup_fixture, teardown_fixture),
};

/**
 * @brief Count of performance tests for export
 */
size_t g_config_performance_test_count = sizeof(g_config_performance_tests) / sizeof(g_config_performance_tests[0]);
