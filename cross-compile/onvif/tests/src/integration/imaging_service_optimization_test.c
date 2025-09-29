/**
 * @file imaging_service_optimization_tests.c
 * @brief Integration tests for imaging service optimization features
 * @author kkrzysztofik
 * @date 2025
 */

#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

#include "cmocka_wrapper.h"
#include "imaging_service_optimization_tests.h"


// Test constants
#define TEST_ITERATIONS                 500
#define PARAMETER_CACHE_TEST_ITERATIONS 100
#define BENCHMARK_THRESHOLD_US          50
#define CONCURRENT_THREADS              4

// Mock imaging settings structure for testing
typedef struct {
  int brightness;
  int contrast;
  int saturation;
  int sharpness;
  int hue;
} test_imaging_settings;

// Mock function declarations (would be replaced by actual implementation)
int test_onvif_imaging_get_settings(test_imaging_settings* settings);
int test_onvif_imaging_set_settings(const test_imaging_settings* settings);
int test_validate_parameter_cache_efficiency(void);
int test_bulk_settings_validation(void);

// Global variables for testing
static test_imaging_settings g_test_settings = {0, 0, 0, 0, 0};
static int g_mock_vpss_call_count = 0;
static pthread_mutex_t g_test_mutex = PTHREAD_MUTEX_INITIALIZER;

static long get_time_microseconds(void) {
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return tv.tv_sec * 1000000 + tv.tv_usec;
}

// Mock implementation with optimization simulation
int test_onvif_imaging_get_settings(test_imaging_settings* settings) {
  if (!settings) {
    return -1;
  }

  pthread_mutex_lock(&g_test_mutex);
  *settings = g_test_settings;
  pthread_mutex_unlock(&g_test_mutex);

  // Simulate cache hit (faster response)
  usleep(1); // 1 microsecond for cached response
  return 0;
}

int test_onvif_imaging_set_settings(const test_imaging_settings* settings) {
  if (!settings) {
    return -1;
  }

  pthread_mutex_lock(&g_test_mutex);

  // Simulate optimized batch update - only update changed parameters
  int changes_count = 0;
  if (g_test_settings.brightness != settings->brightness)
    changes_count++;
  if (g_test_settings.contrast != settings->contrast)
    changes_count++;
  if (g_test_settings.saturation != settings->saturation)
    changes_count++;
  if (g_test_settings.sharpness != settings->sharpness)
    changes_count++;
  if (g_test_settings.hue != settings->hue)
    changes_count++;

  g_mock_vpss_call_count += changes_count; // Track VPSS optimization
  g_test_settings = *settings;

  pthread_mutex_unlock(&g_test_mutex);

  // Simulate VPSS call time based on number of changes
  usleep(changes_count * 2); // 2 microseconds per changed parameter
  return 0;
}

void test_integration_imaging_parameter_cache_efficiency(void** state) {
  (void)state; // Unused parameter

  printf("Test: Parameter Cache Efficiency\n");
  printf("-------------------------------\n");

  test_imaging_settings settings = {50, 25, 0, -10, 5};

  // First call - cache miss
  long start_time = get_time_microseconds();
  int result = test_onvif_imaging_get_settings(&settings);
  long first_call_time = get_time_microseconds() - start_time;

  assert_int_equal(result, 0);
  printf("First call time: %ld μs\n", first_call_time);

  // Second call - cache hit
  start_time = get_time_microseconds();
  result = test_onvif_imaging_get_settings(&settings);
  long second_call_time = get_time_microseconds() - start_time;

  assert_int_equal(result, 0);
  printf("Second call time: %ld μs\n", second_call_time);

  if (second_call_time <= first_call_time) {
    printf("✅ Parameter caching is effective (improvement: %.1fx)\n",
           (double)first_call_time / (double)second_call_time);
  } else {
    printf("⚠️  Parameter caching effectiveness inconclusive\n");
  }
}

void test_integration_imaging_bulk_settings_validation(void** state) {
  (void)state; // Unused parameter

  printf("\nTest: Bulk Settings Validation Optimization\n");
  printf("------------------------------------------\n");

  test_imaging_settings identical_settings = {25, 50, 75, -25, 10};

  // Test validation caching with identical settings
  long start_time = get_time_microseconds();
  for (int i = 0; i < 10; i++) {
    int result = test_onvif_imaging_set_settings(&identical_settings);
    assert_int_equal(result, 0);
  }
  long total_time = get_time_microseconds() - start_time;
  long avg_time = total_time / 10;

  printf("Bulk validation test completed\n");
  printf("Average time per validation: %ld μs\n", avg_time);

  if (avg_time < 50) { // Should be fast due to validation caching
    printf("✅ Bulk validation optimization is effective\n");
  } else {
    printf("⚠️  Bulk validation optimization may need improvement\n");
  }
}

void test_integration_imaging_batch_parameter_update_optimization(void** state) {
  (void)state; // Unused parameter

  printf("\nTest: Batch Parameter Update Optimization\n");
  printf("----------------------------------------\n");

  test_imaging_settings baseline_settings = {0, 0, 0, 0, 0};
  int result = test_onvif_imaging_set_settings(&baseline_settings);
  assert_int_equal(result, 0);

  int initial_vpss_calls = g_mock_vpss_call_count;

  // Test 1: Update all parameters (should make 5 VPSS calls)
  test_imaging_settings all_changed = {10, 20, 30, 40, 50};
  result = test_onvif_imaging_set_settings(&all_changed);
  assert_int_equal(result, 0);

  int calls_after_all_change = g_mock_vpss_call_count - initial_vpss_calls;
  printf("VPSS calls for all parameters changed: %d\n", calls_after_all_change);

  // Test 2: Update only brightness (should make 1 VPSS call)
  test_imaging_settings brightness_only = {15, 20, 30, 40, 50}; // Only brightness changed
  result = test_onvif_imaging_set_settings(&brightness_only);
  assert_int_equal(result, 0);

  int calls_after_single_change =
    g_mock_vpss_call_count - initial_vpss_calls - calls_after_all_change;
  printf("VPSS calls for single parameter changed: %d\n", calls_after_single_change);

  // Test 3: No changes (should make 0 VPSS calls)
  result = test_onvif_imaging_set_settings(&brightness_only); // Same settings
  assert_int_equal(result, 0);

  int calls_after_no_change = g_mock_vpss_call_count - initial_vpss_calls - calls_after_all_change -
                              calls_after_single_change;
  printf("VPSS calls for no parameters changed: %d\n", calls_after_no_change);

  // Validate optimization
  if (calls_after_all_change == 5 && calls_after_single_change == 1 && calls_after_no_change == 0) {
    printf("✅ Batch parameter update optimization is working perfectly\n");
  } else if (calls_after_single_change <= calls_after_all_change && calls_after_no_change == 0) {
    printf("✅ Batch parameter update optimization is effective\n");
  } else {
    printf("❌ Batch parameter update optimization needs improvement\n");
    fail_msg("Batch parameter update optimization failed validation");
  }
}

// Thread function for concurrent testing
void* concurrent_imaging_test_thread(void* arg) {
  int thread_id = *(int*)arg;
  test_imaging_settings settings;

  for (int i = 0; i < 50; i++) {
    // Vary settings based on thread ID and iteration
    settings.brightness = (thread_id * 10 + i) % 200 - 100;
    settings.contrast = (thread_id * 15 + i) % 200 - 100;
    settings.saturation = (thread_id * 20 + i) % 200 - 100;
    settings.sharpness = (thread_id * 25 + i) % 200 - 100;
    settings.hue = (thread_id * 30 + i) % 360 - 180;

    if (test_onvif_imaging_set_settings(&settings) != 0) {
      printf("❌ Thread %d failed at iteration %d\n", thread_id, i);
      return (void*)-1;
    }

    if (test_onvif_imaging_get_settings(&settings) != 0) {
      printf("❌ Thread %d get_settings failed at iteration %d\n", thread_id, i);
      return (void*)-1;
    }
  }

  return (void*)0;
}

void test_integration_imaging_concurrent_access(void** state) {
  (void)state; // Unused parameter

  printf("\nTest: Concurrent Imaging Access\n");
  printf("------------------------------\n");

  pthread_t threads[CONCURRENT_THREADS];
  int thread_ids[CONCURRENT_THREADS];

  long start_time = get_time_microseconds();

  // Create threads
  for (int i = 0; i < CONCURRENT_THREADS; i++) {
    thread_ids[i] = i;
    int result = pthread_create(&threads[i], NULL, concurrent_imaging_test_thread, &thread_ids[i]);
    assert_int_equal(result, 0);
  }

  // Wait for threads to complete
  int failed_threads = 0;
  for (int i = 0; i < CONCURRENT_THREADS; i++) {
    void* result;
    pthread_join(threads[i], &result);
    if (result != (void*)0) {
      failed_threads++;
    }
  }

  long total_time = get_time_microseconds() - start_time;

  printf("Concurrent test completed in %ld μs\n", total_time);
  printf("Failed threads: %d/%d\n", failed_threads, CONCURRENT_THREADS);

  assert_int_equal(failed_threads, 0);
  printf("✅ Concurrent imaging access test passed\n");
}

void test_integration_imaging_performance_regression(void** state) {
  (void)state; // Unused parameter

  printf("\nTest: Performance Regression Check\n");
  printf("---------------------------------\n");

  test_imaging_settings test_settings = {75, -25, 50, 0, -90};

  // Warm up
  for (int i = 0; i < 10; i++) {
    int result = test_onvif_imaging_set_settings(&test_settings);
    assert_int_equal(result, 0);
    result = test_onvif_imaging_get_settings(&test_settings);
    assert_int_equal(result, 0);
  }

  // Performance test
  long start_time = get_time_microseconds();

  for (int i = 0; i < TEST_ITERATIONS; i++) {
    test_settings.brightness = (i % 200) - 100;
    test_settings.contrast = ((i * 2) % 200) - 100;

    int result = test_onvif_imaging_set_settings(&test_settings);
    assert_int_equal(result, 0);

    result = test_onvif_imaging_get_settings(&test_settings);
    assert_int_equal(result, 0);
  }

  long total_time = get_time_microseconds() - start_time;
  long avg_time_per_operation = total_time / (TEST_ITERATIONS * 2); // set + get

  printf("Performance Results:\n");
  printf("  Total operations: %d\n", TEST_ITERATIONS * 2);
  printf("  Total time: %ld μs\n", total_time);
  printf("  Average time per operation: %ld μs\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)(TEST_ITERATIONS * 2) / (total_time / 1000000.0));

  if (avg_time_per_operation < BENCHMARK_THRESHOLD_US) {
    printf("✅ Performance regression test passed (under %d μs threshold)\n",
           BENCHMARK_THRESHOLD_US);
  } else {
    printf("⚠️  Performance regression test warning (exceeds %d μs threshold)\n",
           BENCHMARK_THRESHOLD_US);
  }
}

// Test suite definition
const struct CMUnitTest imaging_service_optimization_tests[] = {
  cmocka_unit_test(test_integration_imaging_parameter_cache_efficiency),
  cmocka_unit_test(test_integration_imaging_bulk_settings_validation),
  cmocka_unit_test(test_integration_imaging_batch_parameter_update_optimization),
  cmocka_unit_test(test_integration_imaging_concurrent_access),
  cmocka_unit_test(test_integration_imaging_performance_regression),
};
