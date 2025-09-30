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

// TESTS
#include "cmocka_wrapper.h"
#include "common/time_utils.h"
#include "core/config/config.h"

// ONVIF project includes
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Test profile constants
#define TEST_PROFILE_MAIN      "MainProfile"
#define TEST_PROFILE_SUB       "SubProfile"
#define TEST_PROFILE_NAME_MAIN "Main Video Profile"
#define TEST_PROFILE_INVALID   "InvalidProfile"
#define TEST_PROFILE_COUNT     2

// Test protocol constants
#define TEST_PROTOCOL_RTSP        "RTSP"
#define TEST_PROTOCOL_RTP_UNICAST "RTP-Unicast"
#define TEST_PROTOCOL_INVALID     "InvalidProtocol"

// Test URI path constants
#define TEST_URI_PATH_MAIN "rtsp://"
#define TEST_URI_PATH_SUB  "/vs1"

// Test performance constants
#define TEST_ITERATIONS_PERFORMANCE 1000
#define TEST_ITERATIONS_STRESS      10000
#define BENCHMARK_THRESHOLD_US      500  // Maximum acceptable microseconds per operation
#define MEMORY_LEAK_THRESHOLD       1024 // Maximum acceptable memory leak in bytes

// Test timing constants
#define TEST_URI_TIMEOUT               60
#define TEST_URI_INVALID_AFTER_CONNECT 0
#define TEST_URI_INVALID_AFTER_REBOOT  0
#define MICROSECONDS_PER_SECOND        1000000.0

// Test iteration constants
#define TEST_ITERATIONS_CONCURRENT      10
#define TEST_PROGRESS_INTERVAL          1000
#define TEST_STRESS_OPERATIONS_PER_ITER 3

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
  assert_int_equal(TEST_PROFILE_COUNT, count);

  // Verify profile tokens
  bool found_main = false, found_sub = false;
  for (int i = 0; i < count; i++) {
    if (strcmp(profiles[i].token, TEST_PROFILE_MAIN) == 0) {
      found_main = true;
    } else if (strcmp(profiles[i].token, TEST_PROFILE_SUB) == 0) {
      found_sub = true;
    }
  }
  assert_true(found_main);
  assert_true(found_sub);

  // Test getting individual profile
  struct media_profile profile;
  result = onvif_media_get_profile(TEST_PROFILE_MAIN, &profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal(TEST_PROFILE_MAIN, profile.token);
  assert_string_equal(TEST_PROFILE_NAME_MAIN, profile.name);
}

void test_integration_media_stream_uri_generation_functionality(void** state) {
  (void)state; // Unused parameter

  struct stream_uri uri;

  // Test MainProfile RTSP URI generation
  int result = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_RTSP, &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);
  assert_true(strlen(uri.uri) > 0);
  assert_true(strstr(uri.uri, TEST_URI_PATH_MAIN) != NULL);
  assert_true(strstr(uri.uri, "/vs0") != NULL);

  // Test SubProfile RTSP URI generation
  result = onvif_media_get_stream_uri(TEST_PROFILE_SUB, TEST_PROTOCOL_RTSP, &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);
  assert_true(strlen(uri.uri) > 0);
  assert_true(strstr(uri.uri, TEST_URI_PATH_MAIN) != NULL);
  assert_true(strstr(uri.uri, TEST_URI_PATH_SUB) != NULL);

  // Test RTP-Unicast protocol
  result = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_RTP_UNICAST, &uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri.uri);

  // Test invalid profile
  result = onvif_media_get_stream_uri(TEST_PROFILE_INVALID, TEST_PROTOCOL_RTSP, &uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test invalid protocol
  result = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_INVALID, &uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Verify URI properties
  assert_int_equal(TEST_URI_INVALID_AFTER_CONNECT, uri.invalid_after_connect);
  assert_int_equal(TEST_URI_INVALID_AFTER_REBOOT, uri.invalid_after_reboot);
  assert_int_equal(TEST_URI_TIMEOUT, uri.timeout);
}

void test_integration_optimized_profile_lookup_performance(void** state) {
  (void)state; // Unused parameter

  // Measure performance of optimized profile lookup
  long start_time = test_get_time_microseconds();

  for (int i = 0; i < TEST_ITERATIONS_PERFORMANCE; i++) {
    struct stream_uri uri;

    // Alternate between MainProfile and SubProfile to test both fast paths
    const char* profile_token = (i % 2 == 0) ? TEST_PROFILE_MAIN : TEST_PROFILE_SUB;
    int result = onvif_media_get_stream_uri(profile_token, TEST_PROTOCOL_RTSP, &uri);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  long end_time = test_get_time_microseconds();
  long total_time = end_time - start_time;
  long avg_time_per_operation = total_time / TEST_ITERATIONS_PERFORMANCE;

  printf("\nProfile lookup performance test:\n");
  printf("  Total time: %ld microseconds\n", total_time);
  printf("  Average time per operation: %ld microseconds\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)TEST_ITERATIONS_PERFORMANCE / ((double)total_time / MICROSECONDS_PER_SECOND));

  // Verify performance is within acceptable limits
  assert_true(avg_time_per_operation < BENCHMARK_THRESHOLD_US);
}

void test_integration_uri_caching_optimization(void** state) {
  (void)state; // Unused parameter

  struct stream_uri uri1;
  struct stream_uri uri2;

  // Generate URI twice for the same profile to test caching
  long start_time = test_get_time_microseconds();
  int result1 = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_RTSP, &uri1);
  long first_call_time = test_get_time_microseconds();

  int result2 = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_RTSP, &uri2);
  long second_call_time = test_get_time_microseconds();

  assert_int_equal(ONVIF_SUCCESS, result1);
  assert_int_equal(ONVIF_SUCCESS, result2);

  // Verify URIs are identical (cached)
  assert_string_equal(uri1.uri, uri2.uri);

  long first_duration = first_call_time - start_time;
  long second_duration = second_call_time - first_call_time;

  printf("\nURI caching optimization test:\n");
  printf("  First call duration: %ld microseconds\n", first_duration);
  printf("  Second call duration: %ld microseconds\n", second_duration);
  printf("  Caching improvement ratio: %.2fx\n", (double)first_duration / (double)second_duration);

  // Second call should be faster due to caching (allowing some variance for measurement noise)
  assert_true(second_duration <= first_duration);
}

void test_integration_media_memory_efficiency(void** state) {
  (void)state; // Unused parameter

  size_t initial_memory = memory_manager_get_allocated_size();

  // Perform many stream URI operations to test for memory leaks
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    struct stream_uri uri;
    const char* profile_token = (i % 2 == 0) ? TEST_PROFILE_MAIN : TEST_PROFILE_SUB;

    int result = onvif_media_get_stream_uri(profile_token, TEST_PROTOCOL_RTSP, &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Occasionally test RTP-Unicast as well
    if (i % TEST_PROGRESS_INTERVAL == 0) {
      result = onvif_media_get_stream_uri(profile_token, TEST_PROTOCOL_RTP_UNICAST, &uri);
      assert_int_equal(ONVIF_SUCCESS, result);
    }
  }

  size_t final_memory = memory_manager_get_allocated_size();
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
  struct stream_uri uris[TEST_ITERATIONS_CONCURRENT];
  const char* profiles[] = {TEST_PROFILE_MAIN, TEST_PROFILE_SUB};
  const char* protocols[] = {TEST_PROTOCOL_RTSP, TEST_PROTOCOL_RTP_UNICAST};

  // Simulate concurrent requests
  for (int i = 0; i < TEST_ITERATIONS_CONCURRENT; i++) {
    const char* profile = profiles[i % 2];
    const char* protocol = protocols[i % 2];

    int result = onvif_media_get_stream_uri(profile, protocol, &uris[i]);
    assert_int_equal(ONVIF_SUCCESS, result);
    assert_non_null(uris[i].uri);
    assert_true(strlen(uris[i].uri) > 0);
  }

  // Verify consistency - same profile/protocol combinations should produce identical URIs
  for (int i = 0; i < TEST_ITERATIONS_CONCURRENT; i++) {
    for (int j = i + 1; j < TEST_ITERATIONS_CONCURRENT; j++) {
      if (i % 2 == j % 2) { // Same profile/protocol combination
        assert_string_equal(uris[i].uri, uris[j].uri);
      }
    }
  }
}

void test_integration_stress_test_optimization(void** state) {
  (void)state; // Unused parameter

  printf("\nRunning stress test for media service optimization...\n");

  size_t initial_memory = memory_manager_get_allocated_size();
  long start_time = test_get_time_microseconds();

  // High-frequency operations to stress test the optimization
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    struct media_profile* profiles = NULL;
    int count = 0;

    // Test profile retrieval
    int result = onvif_media_get_profiles(&profiles, &count);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Test stream URI generation for both profiles
    struct stream_uri uri;
    result = onvif_media_get_stream_uri(TEST_PROFILE_MAIN, TEST_PROTOCOL_RTSP, &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    result = onvif_media_get_stream_uri(TEST_PROFILE_SUB, TEST_PROTOCOL_RTSP, &uri);
    assert_int_equal(ONVIF_SUCCESS, result);

    // Print progress every TEST_PROGRESS_INTERVAL iterations
    if (i % TEST_PROGRESS_INTERVAL == 0 && i > 0) {
      printf("  Progress: %d/%d operations completed\n", i, TEST_ITERATIONS_STRESS);
    }
  }

  long end_time = test_get_time_microseconds();
  size_t final_memory = memory_manager_get_allocated_size();

  long total_time = end_time - start_time;
  long avg_time_per_operation =
    total_time /
    ((long)TEST_ITERATIONS_STRESS * TEST_STRESS_OPERATIONS_PER_ITER); // 3 operations per iteration

  printf("\nStress test results:\n");
  printf("  Total operations: %d\n", TEST_ITERATIONS_STRESS * TEST_STRESS_OPERATIONS_PER_ITER);
  printf("  Total time: %ld microseconds\n", total_time);
  printf("  Average time per operation: %ld microseconds\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)(TEST_ITERATIONS_STRESS * TEST_STRESS_OPERATIONS_PER_ITER) /
           ((double)total_time / MICROSECONDS_PER_SECOND));
  printf("  Memory increase: %zu bytes\n", final_memory - initial_memory);

  // Verify performance and memory constraints
  assert_true(avg_time_per_operation < BENCHMARK_THRESHOLD_US);
  assert_true((final_memory - initial_memory) < MEMORY_LEAK_THRESHOLD);
}

void test_integration_media_platform_integration(void** state) {
  (void)state; // Unused parameter

  // This test verifies that the media service properly integrates with
  // the platform abstraction layer for real device operations

  struct media_profile* profiles = NULL;
  int count = 0;

  // Test 1: Verify profile retrieval triggers proper platform calls
  int result = onvif_media_get_profiles(&profiles, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles);
  assert_int_equal(TEST_PROFILE_COUNT, count);

  // Test 2: Verify video source retrieval
  struct video_source* video_sources = NULL;
  int video_source_count = 0;
  result = onvif_media_get_video_sources(&video_sources, &video_source_count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(video_sources);
  assert_true(video_source_count > 0);

  // Test 3: Verify audio source retrieval
  struct audio_source* audio_sources = NULL;
  int audio_source_count = 0;
  result = onvif_media_get_audio_sources(&audio_sources, &audio_source_count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(audio_sources);
  assert_true(audio_source_count > 0);

  // Test 4: Verify video encoder configuration retrieval
  struct video_encoder_configuration* video_configs = NULL;
  int video_config_count = 0;
  result = onvif_media_get_video_encoder_configurations(&video_configs, &video_config_count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(video_configs);
  assert_true(video_config_count > 0);

  // Test 5: Verify snapshot URI generation (requires platform support)
  struct stream_uri snapshot_uri;
  result = onvif_media_get_snapshot_uri(TEST_PROFILE_MAIN, &snapshot_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(snapshot_uri.uri);
  assert_true(strlen(snapshot_uri.uri) > 0);

  printf("\nPlatform integration test results:\n");
  printf("  Profile count: %d\n", count);
  printf("  Video source count: %d\n", video_source_count);
  printf("  Audio source count: %d\n", audio_source_count);
  printf("  Video encoder config count: %d\n", video_config_count);
  printf("  Snapshot URI: %s\n", snapshot_uri.uri);
}

// Test suite definition
const struct CMUnitTest media_service_optimization_tests[] = {
  cmocka_unit_test_setup_teardown(test_integration_media_profile_operations, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_stream_uri_generation_functionality,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_optimized_profile_lookup_performance,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_uri_caching_optimization, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_memory_efficiency, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_concurrent_stream_uri_access,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_stress_test_optimization, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_platform_integration, media_service_setup,
                                  media_service_teardown),
};

// Main function removed - tests are now integrated into main test runner
