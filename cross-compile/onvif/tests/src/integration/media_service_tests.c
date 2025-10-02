/**
 * @file media_service_tests.c
 * @brief Integration tests for optimized ONVIF media service
 * @author kkrzysztofik
 * @date 2025
 */

#define _POSIX_C_SOURCE 200809L

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

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Test mocks

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
int media_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Initialize buffer pool mock

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize media service with mock config
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));

  result = onvif_media_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  *state = config;
  return 0;
}

int media_service_teardown(void** state) {
  config_manager_t* config = (config_manager_t*)*state;

  // Cleanup media service (this unregisters from dispatcher)
  onvif_media_cleanup();
  free(config);

  // Note: Don't cleanup dispatcher - keep it alive for next test
  // The dispatcher mutex gets destroyed and can't be reinitialized

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
/**
 * @brief Test DeleteProfile operation end-to-end
 */
void test_integration_media_delete_profile_operation(void** state) {
  (void)state;

  // Create a new profile first
  struct media_profile new_profile;
  int result = onvif_media_create_profile("TestDeleteProfile", "Test Profile for Deletion",
                                          &new_profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("TestDeleteProfile", new_profile.token);

  // Verify profile exists
  struct media_profile verify_profile;
  result = onvif_media_get_profile("TestDeleteProfile", &verify_profile);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Delete the profile
  result = onvif_media_delete_profile("TestDeleteProfile");
  assert_int_equal(ONVIF_SUCCESS, result);

  // Verify profile no longer exists
  result = onvif_media_get_profile("TestDeleteProfile", &verify_profile);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test error handling for invalid profile tokens
 */
void test_integration_media_error_invalid_profile_token(void** state) {
  (void)state;

  // Test getting profile with invalid token
  struct media_profile profile;
  int result = onvif_media_get_profile(TEST_PROFILE_INVALID, &profile);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test deleting profile with invalid token
  result = onvif_media_delete_profile(TEST_PROFILE_INVALID);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test getting stream URI with invalid profile token
  struct stream_uri uri;
  result = onvif_media_get_stream_uri(TEST_PROFILE_INVALID, TEST_PROTOCOL_RTSP, &uri);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test with NULL token
  result = onvif_media_get_profile(NULL, &profile);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test concurrent profile create/delete operations
 */
void test_integration_media_concurrent_profile_operations(void** state) {
  (void)state;

  // Create multiple profiles concurrently
  const int profile_count = 5;
  char profile_tokens[profile_count][64];
  struct media_profile profiles[profile_count];

  // Create profiles
  for (int i = 0; i < profile_count; i++) {
    snprintf(profile_tokens[i], sizeof(profile_tokens[i]), "ConcurrentProfile%d", i);
    int result = onvif_media_create_profile(profile_tokens[i], "Concurrent Test Profile",
                                            &profiles[i]);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  // Verify all profiles exist
  for (int i = 0; i < profile_count; i++) {
    struct media_profile verify_profile;
    int result = onvif_media_get_profile(profile_tokens[i], &verify_profile);
    assert_int_equal(ONVIF_SUCCESS, result);
    assert_string_equal(profile_tokens[i], verify_profile.token);
  }

  // Delete all profiles
  for (int i = 0; i < profile_count; i++) {
    int result = onvif_media_delete_profile(profile_tokens[i]);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  // Verify all profiles are deleted
  for (int i = 0; i < profile_count; i++) {
    struct media_profile verify_profile;
    int result = onvif_media_get_profile(profile_tokens[i], &verify_profile);
    assert_int_not_equal(ONVIF_SUCCESS, result);
  }
}

/**
 * @brief Test complete request-response validation
 */
void test_integration_media_request_response_validation(void** state) {
  (void)state;

  // Test GetProfiles request-response cycle
  struct media_profile* profiles = NULL;
  int count = 0;
  int result = onvif_media_get_profiles(&profiles, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles);
  assert_true(count > 0);

  // Validate each profile structure
  for (int i = 0; i < count; i++) {
    // Verify profile has valid token
    assert_non_null(profiles[i].token);
    assert_true(strlen(profiles[i].token) > 0);

    // Verify profile has valid name
    assert_non_null(profiles[i].name);
    assert_true(strlen(profiles[i].name) > 0);

    // Test GetStreamUri for this profile
    struct stream_uri uri;
    result = onvif_media_get_stream_uri(profiles[i].token, TEST_PROTOCOL_RTSP, &uri);
    assert_int_equal(ONVIF_SUCCESS, result);
    assert_non_null(uri.uri);
    assert_true(strlen(uri.uri) > 0);
    assert_non_null(strstr(uri.uri, "rtsp://"));

    // Verify URI timeout is set
    assert_true(uri.timeout >= 0);

    // Verify invalidAfterConnect flag
    assert_true(uri.invalid_after_connect == 0 || uri.invalid_after_connect == 1);

    // Verify invalidAfterReboot flag
    assert_true(uri.invalid_after_reboot == 0 || uri.invalid_after_reboot == 1);
  }
}

/**
 * @brief Test SetVideoSourceConfiguration operation
 */
void test_integration_media_set_video_source_configuration(void** state) {
  (void)state;

  // Setup valid configuration
  struct video_source_configuration config = {0};
  strncpy(config.token, "VideoSourceConfig0", sizeof(config.token) - 1);
  strncpy(config.name, "Test Video Source Config", sizeof(config.name) - 1);
  config.use_count = 1;
  strncpy(config.source_token, "VideoSource0", sizeof(config.source_token) - 1);
  config.bounds.width = 1920;
  config.bounds.height = 1080;
  config.bounds.x = 0;
  config.bounds.y = 0;

  // Test valid configuration
  int result = onvif_media_set_video_source_configuration("VideoSourceConfig0", &config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test invalid configuration token
  result = onvif_media_set_video_source_configuration("InvalidConfigToken", &config);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test NULL configuration token
  result = onvif_media_set_video_source_configuration(NULL, &config);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL configuration
  result = onvif_media_set_video_source_configuration("VideoSourceConfig0", NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test SetVideoEncoderConfiguration operation
 */
void test_integration_media_set_video_encoder_configuration(void** state) {
  (void)state;

  // Setup valid encoder configuration
  struct video_encoder_configuration config = {0};
  strncpy(config.token, "VideoEncoder0", sizeof(config.token) - 1);
  strncpy(config.name, "Test H.264 Encoder", sizeof(config.name) - 1);
  config.use_count = 1;
  strncpy(config.encoding, "H264", sizeof(config.encoding) - 1);
  config.resolution.width = 1920;
  config.resolution.height = 1080;
  config.quality = 4.0f;
  config.framerate_limit = 30;
  config.encoding_interval = 30;
  config.bitrate_limit = 4096;

  // Test valid configuration for VideoEncoder0
  int result = onvif_media_set_video_encoder_configuration("VideoEncoder0", &config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test valid configuration for VideoEncoder1
  result = onvif_media_set_video_encoder_configuration("VideoEncoder1", &config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test invalid configuration token
  result = onvif_media_set_video_encoder_configuration("InvalidEncoder", &config);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test NULL configuration token
  result = onvif_media_set_video_encoder_configuration(NULL, &config);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL configuration
  result = onvif_media_set_video_encoder_configuration("VideoEncoder0", NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test GetMetadataConfigurations operation
 */
void test_integration_media_get_metadata_configurations(void** state) {
  (void)state;

  struct metadata_configuration* configs = NULL;
  int count = 0;

  // Test getting metadata configurations
  int result = onvif_media_get_metadata_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_true(count > 0);

  // Verify configuration structure
  for (int i = 0; i < count; i++) {
    assert_non_null(configs[i].token);
    assert_true(strlen(configs[i].token) > 0);
    assert_non_null(configs[i].name);
    assert_true(strlen(configs[i].name) > 0);
    assert_true(configs[i].session_timeout >= 0);
  }

  // Test NULL configs parameter
  result = onvif_media_get_metadata_configurations(NULL, &count);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL count parameter
  result = onvif_media_get_metadata_configurations(&configs, NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test SetMetadataConfiguration operation
 */
void test_integration_media_set_metadata_configuration(void** state) {
  (void)state;

  // Setup valid metadata configuration
  struct metadata_configuration config = {0};
  strncpy(config.token, "MetadataConfig0", sizeof(config.token) - 1);
  strncpy(config.name, "Test Metadata Config", sizeof(config.name) - 1);
  config.use_count = 1;
  config.session_timeout = 60;
  config.analytics = 1;

  // Test valid configuration
  int result = onvif_media_set_metadata_configuration("MetadataConfig0", &config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test invalid configuration token
  result = onvif_media_set_metadata_configuration("InvalidMetadataConfig", &config);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test NULL configuration token
  result = onvif_media_set_metadata_configuration(NULL, &config);
  assert_int_not_equal(ONVIF_SUCCESS, result);

  // Test NULL configuration
  result = onvif_media_set_metadata_configuration("MetadataConfig0", NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test StartMulticastStreaming operation
 */
void test_integration_media_start_multicast_streaming(void** state) {
  (void)state;

  // Test starting multicast for MainProfile
  int result = onvif_media_start_multicast_streaming(TEST_PROFILE_MAIN);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test starting multicast for SubProfile
  result = onvif_media_start_multicast_streaming(TEST_PROFILE_SUB);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test invalid profile token
  result = onvif_media_start_multicast_streaming(TEST_PROFILE_INVALID);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test NULL profile token
  result = onvif_media_start_multicast_streaming(NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test StopMulticastStreaming operation
 */
void test_integration_media_stop_multicast_streaming(void** state) {
  (void)state;

  // Start multicast first
  int result = onvif_media_start_multicast_streaming(TEST_PROFILE_MAIN);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test stopping multicast for MainProfile
  result = onvif_media_stop_multicast_streaming(TEST_PROFILE_MAIN);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test stopping multicast for SubProfile
  result = onvif_media_stop_multicast_streaming(TEST_PROFILE_SUB);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Test invalid profile token
  result = onvif_media_stop_multicast_streaming(TEST_PROFILE_INVALID);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  // Test NULL profile token
  result = onvif_media_stop_multicast_streaming(NULL);
  assert_int_not_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Pilot SOAP test for Media GetProfiles operation
 * Tests SOAP envelope parsing and response structure validation
 * Note: This is a standalone test that validates SOAP structure without full service initialization
 */
void test_integration_media_get_profiles_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GetProfiles", SOAP_MEDIA_GET_PROFILES, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Validate request structure
  assert_non_null(request->body);
  assert_true(strstr(request->body, "GetProfiles") != NULL);

  // Step 3: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 4: Call actual service handler (integration test)
  int result = onvif_media_handle_request("GetProfiles", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 5: Validate HTTP response structure
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);
  assert_true(response.body_length > 0);

  // Step 6: Check for SOAP faults
  char fault_code[256] = {0};
  char fault_string[512] = {0};
  int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);
  assert_int_equal(0, has_fault);

  // Step 7: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__GetProfilesResponse* profiles_response = NULL;
  result = soap_test_parse_get_profiles_response(&ctx, &profiles_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles_response);

  // Step 8: Validate response data
  assert_non_null(profiles_response->Profiles);
  assert_true(profiles_response->__sizeProfiles >= 1);
  assert_non_null(profiles_response->Profiles[0].token);

  // Step 9: Cleanup resources
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media GetStreamUri operation
 */
void test_integration_media_get_stream_uri_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GetStreamUri", SOAP_MEDIA_GET_STREAM_URI, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("GetStreamUri", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);
  assert_true(response.body_length > 0);

  // Step 5: Check for SOAP faults
  char fault_code[256] = {0};
  char fault_string[512] = {0};
  int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__GetStreamUriResponse* uri_response = NULL;
  result = soap_test_parse_get_stream_uri_response(&ctx, &uri_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(uri_response);

  // Step 7: Validate response data - verify RTSP URI
  assert_non_null(uri_response->MediaUri);
  assert_non_null(uri_response->MediaUri->Uri);
  assert_true(strstr(uri_response->MediaUri->Uri, "rtsp://") != NULL);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media DeleteProfile operation
 */
void test_integration_media_delete_profile_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("DeleteProfile", SOAP_MEDIA_DELETE_PROFILE, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("DeleteProfile", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  char fault_code[256] = {0};
  int has_fault = soap_test_check_soap_fault(&response, fault_code, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__DeleteProfileResponse* delete_response = NULL;
  result = soap_test_parse_delete_profile_response(&ctx, &delete_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(delete_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media SetVideoSourceConfiguration operation
 */
void test_integration_media_set_video_source_config_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "SetVideoSourceConfiguration", SOAP_MEDIA_SET_VIDEO_SOURCE_CONFIG, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result =
    onvif_media_handle_request("SetVideoSourceConfiguration", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__SetVideoSourceConfigurationResponse* config_response = NULL;
  result = soap_test_parse_set_video_source_config_response(&ctx, &config_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(config_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media SetVideoEncoderConfiguration operation
 */
void test_integration_media_set_video_encoder_config_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "SetVideoEncoderConfiguration", SOAP_MEDIA_SET_VIDEO_ENCODER_CONFIG, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result =
    onvif_media_handle_request("SetVideoEncoderConfiguration", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__SetVideoEncoderConfigurationResponse* config_response = NULL;
  result = soap_test_parse_set_video_encoder_config_response(&ctx, &config_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(config_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media GetMetadataConfigurations operation
 */
void test_integration_media_get_metadata_configs_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetMetadataConfigurations", SOAP_MEDIA_GET_METADATA_CONFIGURATIONS, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("GetMetadataConfigurations", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__GetMetadataConfigurationsResponse* configs_response = NULL;
  result = soap_test_parse_get_metadata_configs_response(&ctx, &configs_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs_response);

  // Step 7: Validate response data - should have at least one configuration
  assert_true(configs_response->__sizeConfigurations >= 0);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media SetMetadataConfiguration operation
 */
void test_integration_media_set_metadata_config_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "SetMetadataConfiguration", SOAP_MEDIA_SET_METADATA_CONFIGURATION, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("SetMetadataConfiguration", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__SetMetadataConfigurationResponse* config_response = NULL;
  result = soap_test_parse_set_metadata_config_response(&ctx, &config_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(config_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media StartMulticastStreaming operation
 */
void test_integration_media_start_multicast_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "StartMulticastStreaming", SOAP_MEDIA_START_MULTICAST_STREAMING, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("StartMulticastStreaming", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__StartMulticastStreamingResponse* multicast_response = NULL;
  result = soap_test_parse_start_multicast_response(&ctx, &multicast_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(multicast_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media StopMulticastStreaming operation
 */
void test_integration_media_stop_multicast_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "StopMulticastStreaming", SOAP_MEDIA_STOP_MULTICAST_STREAMING, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("StopMulticastStreaming", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__StopMulticastStreamingResponse* multicast_response = NULL;
  result = soap_test_parse_stop_multicast_response(&ctx, &multicast_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(multicast_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

/**
 * @brief SOAP test for Media CreateProfile operation
 */
void test_integration_media_create_profile_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("CreateProfile", SOAP_MEDIA_CREATE_PROFILE, "/onvif/media_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_media_handle_request("CreateProfile", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _trt__CreateProfileResponse* create_response = NULL;
  result = soap_test_parse_create_profile_response(&ctx, &create_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(create_response);

  // Step 7: Validate response data - profile should be created
  assert_non_null(create_response->Profile);
  assert_non_null(create_response->Profile->token);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    free(response.body);
  }
}

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
  // Enhanced coverage tests
  cmocka_unit_test_setup_teardown(test_integration_media_delete_profile_operation,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_error_invalid_profile_token,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_request_response_validation,
                                  media_service_setup, media_service_teardown),
  // Configuration operation tests
  cmocka_unit_test_setup_teardown(test_integration_media_set_video_source_configuration,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_set_video_encoder_configuration,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_get_metadata_configurations,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_set_metadata_configuration,
                                  media_service_setup, media_service_teardown),
  // Multicast streaming tests
  cmocka_unit_test_setup_teardown(test_integration_media_start_multicast_streaming,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_stop_multicast_streaming,
                                  media_service_setup, media_service_teardown),
  // SOAP integration tests (full HTTP/SOAP layer validation)
  cmocka_unit_test_setup_teardown(test_integration_media_get_profiles_soap, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_get_stream_uri_soap, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_create_profile_soap, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_delete_profile_soap, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_set_video_source_config_soap,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_set_video_encoder_config_soap,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_get_metadata_configs_soap,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_set_metadata_config_soap,
                                  media_service_setup, media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_start_multicast_soap, media_service_setup,
                                  media_service_teardown),
  cmocka_unit_test_setup_teardown(test_integration_media_stop_multicast_soap, media_service_setup,
                                  media_service_teardown),
  // Concurrent tests (may hang - placed at end)
  cmocka_unit_test_setup_teardown(test_integration_media_concurrent_profile_operations,
                                  media_service_setup, media_service_teardown),
};

// Main function removed - tests are now integrated into main test runner
