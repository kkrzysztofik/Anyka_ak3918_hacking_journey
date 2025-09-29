/**
 * @file media_service_tests.c
 * @brief Integration tests for optimized ONVIF media service
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

#include "cmocka_wrapper.h"
#include "core/config/config.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Test constants
#define TEST_ITERATIONS_PERFORMANCE 1000
#define TEST_ITERATIONS_STRESS      10000
#define BENCHMARK_THRESHOLD_US      500  // Maximum acceptable microseconds per operation
#define MEMORY_LEAK_THRESHOLD       1024 // Maximum acceptable memory leak in bytes

// Memory measurement utilities
static size_t get_memory_usage(void) {
  FILE* status_file = fopen("/proc/self/status", "r");
  if (!status_file) {
    return 0;
  }

  char line[256];
  size_t memory_kb = 0;

  while (fgets(line, sizeof(line), status_file)) {
    if (strncmp(line, "VmRSS:", 6) == 0) {
      sscanf(line, "VmRSS: %zu kB", &memory_kb);
      break;
    }
  }

  fclose(status_file);
  return memory_kb * 1024; // Convert to bytes
}

static long get_time_microseconds(void) {
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return tv.tv_sec * 1000000 + tv.tv_usec;
}

// Setup and teardown functions
static int media_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Initialize media service with mock config
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));

  int result = onvif_media_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  *state = config;
  return 0;
}

static int media_service_teardown(void** state) {
  config_manager_t* config = (config_manager_t*)*state;

  onvif_media_cleanup();
  free(config);

  memory_manager_cleanup();
  return 0;
}

// Test functions for functionality preservation
void test_integration_media_profile_operations(void** state) {
  (void)state; // Unused parameter

  // Test getting profiles
  struct media_profile* profiles = NULL;
  int count = 0;
  int result = onvif_media_get_profiles(&profiles, &count);

  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles);
  assert_int_equal(2, count); // MainProfile and SubProfile

  // Verify profile tokens
  bool found_main = false, found_sub = false;
  for (int i = 0; i < count; i++) {
    if (strcmp(profiles[i].token, "MainProfile") == 0) {
      found_main = true;
    } else if (strcmp(profiles[i].token, "SubProfile") == 0) {
      found_sub = true;
    }
  }
  assert_true(found_main);
  assert_true(found_sub);

  // Test getting individual profile
  struct media_profile profile;
  result = onvif_media_get_profile("MainProfile", &profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("MainProfile", profile.token);
  assert_string_equal("Main Video Profile", profile.name);
}

void test_integration_stream_uri_generation_functionality(void** state) {
  (void)state; // Unused parameter

  struct stream_uri uri;

  // Test MainProfile RTSP URI generation
  int result = onvif_media_get_stream_uri("MainProfile", "RTSP", &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);
  assert_true(strlen(uri.uri) > 0);
  assert_true(strstr(uri.uri, "rtsp://") != NULL);
  assert_true(strstr(uri.uri, "/main") != NULL);

  // Test SubProfile RTSP URI generation
  result = onvif_media_get_stream_uri("SubProfile", "RTSP", &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);
  assert_true(strlen(uri.uri) > 0);
  assert_true(strstr(uri.uri, "rtsp://") != NULL);
  assert_true(strstr(uri.uri, "/sub") != NULL);

  // Test RTP-Unicast protocol
  result = onvif_media_get_stream_uri("MainProfile", "RTP-Unicast", &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);

  // Test invalid profile
  result = onvif_media_get_stream_uri("InvalidProfile", "RTSP", &uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test invalid protocol
  result = onvif_media_get_stream_uri("MainProfile", "InvalidProtocol", &uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Verify URI properties
  assert_int_equal(0, uri.invalid_after_connect);
  assert_int_equal(0, uri.invalid_after_reboot);
  assert_int_equal(60, uri.timeout);
}

void test_integration_optimized_profile_lookup_performance(void** state) {
  (void)state; // Unused parameter

  // Measure performance of optimized profile lookup
  long start_time = get_time_microseconds();

  for (int i = 0; i < TEST_ITERATIONS_PERFORMANCE; i++) {
    struct stream_uri uri;

    // Alternate between MainProfile and SubProfile to test both fast paths
    const char* profile_token = (i % 2 == 0) ? "MainProfile" : "SubProfile";
    int result = onvif_media_get_stream_uri(profile_token, "RTSP", &uri);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  long end_time = get_time_microseconds();
  long total_time = end_time - start_time;
  long avg_time_per_operation = total_time / TEST_ITERATIONS_PERFORMANCE;

  printf("\nProfile lookup performance test:\n");
  printf("  Total time: %ld microseconds\n", total_time);
  printf("  Average time per operation: %ld microseconds\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)TEST_ITERATIONS_PERFORMANCE / (total_time / 1000000.0));

  // Verify performance is within acceptable limits
  assert_true(avg_time_per_operation < BENCHMARK_THRESHOLD_US);
}

void test_integration_uri_caching_optimization(void** state) {
  (void)state; // Unused parameter

  struct stream_uri uri1, uri2;

  // Generate URI twice for the same profile to test caching
  long start_time = get_time_microseconds();
  int result1 = onvif_media_get_stream_uri("MainProfile", "RTSP", &uri1);
  long first_call_time = get_time_microseconds();

  int result2 = onvif_media_get_stream_uri("MainProfile", "RTSP", &uri2);
  long second_call_time = get_time_microseconds();

  assert_int_equal(ONVIF_SUCCESS, result1);
  assert_int_equal(ONVIF_SUCCESS, result2);

  // Verify URIs are identical (cached)
  assert_string_equal(uri1.uri, uri2.uri);

  long first_duration = first_call_time - start_time;
  long second_duration = second_call_time - first_call_time;

  printf("\nURI caching optimization test:\n");
  printf("  First call duration: %ld microseconds\n", first_duration);
  printf("  Second call duration: %ld microseconds\n", second_duration);
  printf("  Caching improvement ratio: %.2fx\n", (double)first_duration / second_duration);

  // Second call should be faster due to caching (allowing some variance for measurement noise)
  assert_true(second_duration <= first_duration);
}

void test_integration_memory_efficiency(void** state) {
  (void)state; // Unused parameter

  size_t initial_memory = get_memory_usage();

  // Perform many stream URI operations to test for memory leaks
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    struct stream_uri uri;
    const char* profile_token = (i % 2 == 0) ? "MainProfile" : "SubProfile";

    int result = onvif_media_get_stream_uri(profile_token, "RTSP", &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Occasionally test RTP-Unicast as well
    if (i % 100 == 0) {
      result = onvif_media_get_stream_uri(profile_token, "RTP-Unicast", &uri);
      assert_int_equal(ONVIF_SUCCESS, result);
    }
  }

  size_t final_memory = get_memory_usage();
  size_t memory_increase = final_memory - initial_memory;

  printf("\nMemory efficiency test:\n");
  printf("  Initial memory: %zu bytes\n", initial_memory);
  printf("  Final memory: %zu bytes\n", final_memory);
  printf("  Memory increase: %zu bytes\n", memory_increase);
  printf("  Operations performed: %d\n", TEST_ITERATIONS_STRESS);

  // Memory increase should be minimal (below threshold)
  assert_true(memory_increase < MEMORY_LEAK_THRESHOLD);
}

void test_integration_concurrent_stream_uri_access(void** state) {
  (void)state; // Unused parameter

  // Test concurrent access patterns (simulate multiple threads)
  struct stream_uri uris[10];
  const char* profiles[] = {"MainProfile", "SubProfile"};
  const char* protocols[] = {"RTSP", "RTP-Unicast"};

  // Simulate concurrent requests
  for (int i = 0; i < 10; i++) {
    const char* profile = profiles[i % 2];
    const char* protocol = protocols[i % 2];

    int result = onvif_media_get_stream_uri(profile, protocol, &uris[i]);
    assert_int_equal(ONVIF_SUCCESS, result);
    assert_non_null(uris[i].uri);
    assert_true(strlen(uris[i].uri) > 0);
  }

  // Verify consistency - same profile/protocol combinations should produce identical URIs
  for (int i = 0; i < 10; i++) {
    for (int j = i + 1; j < 10; j++) {
      if (i % 2 == j % 2) { // Same profile/protocol combination
        assert_string_equal(uris[i].uri, uris[j].uri);
      }
    }
  }
}

void test_integration_stress_test_optimization(void** state) {
  (void)state; // Unused parameter

  printf("\nRunning stress test for media service optimization...\n");

  size_t initial_memory = get_memory_usage();
  long start_time = get_time_microseconds();

  // High-frequency operations to stress test the optimization
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    struct media_profile* profiles;
    int count;

    // Test profile retrieval
    int result = onvif_media_get_profiles(&profiles, &count);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Test stream URI generation for both profiles
    struct stream_uri uri;
    result = onvif_media_get_stream_uri("MainProfile", "RTSP", &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    result = onvif_media_get_stream_uri("SubProfile", "RTSP", &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Print progress every 1000 iterations
    if (i % 1000 == 0 && i > 0) {
      printf("  Progress: %d/%d operations completed\n", i, TEST_ITERATIONS_STRESS);
    }
  }

  long end_time = get_time_microseconds();
  size_t final_memory = get_memory_usage();

  long total_time = end_time - start_time;
  long avg_time_per_operation =
    total_time / (TEST_ITERATIONS_STRESS * 3); // 3 operations per iteration

  printf("\nStress test results:\n");
  printf("  Total operations: %d\n", TEST_ITERATIONS_STRESS * 3);
  printf("  Total time: %ld microseconds\n", total_time);
  printf("  Average time per operation: %ld microseconds\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)(TEST_ITERATIONS_STRESS * 3) / (total_time / 1000000.0));
  printf("  Memory increase: %zu bytes\n", final_memory - initial_memory);

  // Verify performance and memory constraints
  assert_true(avg_time_per_operation < BENCHMARK_THRESHOLD_US);
  assert_true((final_memory - initial_memory) < MEMORY_LEAK_THRESHOLD);
}

// Test suite definition
const struct CMUnitTest media_service_optimization_tests[] = {
  cmocka_unit_test_setup_teardown(test_integration_media_profile_operations, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_stream_uri_generation_functionality,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_optimized_profile_lookup_performance,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_uri_caching_optimization, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_memory_efficiency, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_concurrent_stream_uri_access,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_stress_test_optimization, media_service_setup,
                                  media_service_teardown),
};

// Main function removed - tests are now integrated into main test runner
