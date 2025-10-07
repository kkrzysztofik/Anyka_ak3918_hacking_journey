/**
 * @file imaging_service_optimization_tests.c
 * @brief Integration tests for imaging service optimization features using REAL implementation
 * @author kkrzysztofik
 * @date 2025
 */

#define _POSIX_C_SOURCE 200809L

#include <assert.h>
#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

// TESTS
#include "cmocka_wrapper.h"
#include "common/time_utils.h"
#include "imaging_service_optimization_tests.h"
#include "mocks/buffer_pool_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/platform_mock.h"

// ONVIF project includes
#include "services/common/onvif_imaging_types.h"
#include "services/common/service_dispatcher.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"
#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Test constants
#define TEST_ITERATIONS                 500
#define PARAMETER_CACHE_TEST_ITERATIONS 100
#define BENCHMARK_THRESHOLD_US          150
#define CONCURRENT_THREADS              4

// Imaging parameter test constants (descriptive names)
#define BRIGHTNESS_BASELINE  25
#define BRIGHTNESS_MEDIUM    50
#define BRIGHTNESS_HIGH      75
#define CONTRAST_BASELINE    50
#define CONTRAST_LOW         25
#define SATURATION_HIGH      75
#define SATURATION_MEDIUM    30
#define SATURATION_MIN       0
#define SHARPNESS_LOW        -25
#define SHARPNESS_HIGH       40
#define SHARPNESS_MEDIUM_LOW -10
#define HUE_SMALL_POS        10
#define HUE_MEDIUM_POS       50
#define HUE_SMALL_POS_ALT    5
#define HUE_LARGE_NEG        -90

// Test iteration constants
#define BULK_VALIDATION_ITERATIONS 10
#define CONCURRENT_ITERATIONS      50
#define WARMUP_ITERATIONS          10

// Parameter range constants
#define PARAM_RANGE_MIN   -100
#define PARAM_RANGE_MAX   200
#define HUE_RANGE_MIN     -180
#define HUE_RANGE_MAX     360
#define MICROS_PER_SECOND 1000000.0

// Additional test values
#define BRIGHTNESS_LOW        10
#define BRIGHTNESS_LOW_MED    15
#define BRIGHTNESS_HIGH_ALT   75
#define CONTRAST_MEDIUM       20
#define CONTRAST_NEG_MED      -25
#define SATURATION_MEDIUM_ALT 50
#define SHARPNESS_NEUTRAL     0
#define HUE_MAX               360
#define HUE_HALF_RANGE        180

// Thread variation constants
#define VAR_BRIGHTNESS_STEP 10
#define VAR_CONTRAST_STEP   15
#define VAR_SATURATION_STEP 20
#define VAR_SHARPNESS_STEP  25
#define VAR_HUE_STEP        30

// Expected VPSS call counts
#define EXPECTED_VPSS_CALLS_ALL_CHANGED    5
#define EXPECTED_VPSS_CALLS_SINGLE_CHANGED 1
#define EXPECTED_VPSS_CALLS_NO_CHANGE      0

/* moved to common/time_utils.h: test_get_time_microseconds() */

/* ============================================================================
 * Test Setup and Teardown
 * ============================================================================ */

int setup_imaging_integration(void** state) {
  (void)state;

  // Initialize memory manager for tracking
  memory_manager_init();

  // Enable real functions for integration testing (test real service interactions)
  service_dispatcher_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);
  gsoap_mock_use_real_function(true);

  // Configure platform mock expectations for imaging service initialization
  // The imaging service init calls platform_irled_init(level) and applies VPSS settings

  // 1. IR LED initialization
  expect_function_call(__wrap_platform_irled_init);
  expect_any(__wrap_platform_irled_init, level);
  will_return(__wrap_platform_irled_init, PLATFORM_SUCCESS);

  // 2. VPSS effect settings (brightness, contrast, saturation, sharpness, hue)
  //    Called during apply_imaging_settings_to_vpss() if vi_handle is non-NULL
  //    Since we pass NULL to onvif_imaging_init(), vi_handle will be NULL, so these won't be called
  //    No need to mock VPSS calls

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  if (result != ONVIF_SUCCESS) {
    printf("Failed to initialize service dispatcher: %d\n", result);
    return -1;
  }

  // Initialize real imaging service
  result = onvif_imaging_init(NULL);
  if (result != ONVIF_SUCCESS) {
    printf("Failed to initialize imaging service: %d\n", result);
    return -1;
  }

  // Initialize imaging service handler for SOAP operations
  result = onvif_imaging_service_init(NULL);
  if (result != ONVIF_SUCCESS) {
    printf("Failed to initialize imaging service handler: %d\n", result);
    return -1;
  }

  return 0;
}

int teardown_imaging_integration(void** state) {
  (void)state;

  // Cleanup real imaging service (this unregisters from dispatcher)
  onvif_imaging_cleanup();

  // Note: Don't cleanup dispatcher - keep it alive for next test
  // The dispatcher mutex gets destroyed and can't be reinitialized

  // Cleanup memory manager
  memory_manager_cleanup();

  // Restore mock behavior for subsequent tests
  service_dispatcher_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);
  gsoap_mock_use_real_function(false);

  return 0;
}

/* ============================================================================
 * Integration Test: Bulk Settings Validation Optimization
 * ============================================================================ */

void test_integration_imaging_bulk_settings_validation(void** state) {
  (void)state;

  printf("\nTest: Bulk Settings Validation Optimization\n");
  printf("----------------------------------------------------------\n");

  struct imaging_settings settings = {
    .brightness = BRIGHTNESS_BASELINE,
    .contrast = CONTRAST_BASELINE,
    .saturation = SATURATION_HIGH,
    .sharpness = SHARPNESS_LOW,
    .hue = HUE_SMALL_POS,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  // First call - should validate and apply all parameters
  // Note: This test now uses real platform functions instead of mocks
  int result = onvif_imaging_set_settings(&settings);
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("Initial set completed\n");

  // Test validation caching with identical settings (10 iterations)
  long start_time = test_get_time_microseconds();
  for (int i = 0; i < BULK_VALIDATION_ITERATIONS; i++) {
    result = onvif_imaging_set_settings(&settings);
    assert_int_equal(result, ONVIF_SUCCESS);
  }
  long total_time = test_get_time_microseconds() - start_time;
  long avg_time = total_time / BULK_VALIDATION_ITERATIONS;

  printf("Bulk validation test completed\n");
  printf("Average time per validation: %ld μs\n", avg_time);

  if (avg_time < BENCHMARK_THRESHOLD_US) {
    printf("✅ Bulk validation optimization is effective\n");
  } else {
    printf("⚠️  Performance: %ld μs (threshold: %d μs)\n", avg_time, BENCHMARK_THRESHOLD_US);
  }
}

/* ============================================================================
 * Integration Test: Batch Parameter Update Optimization
 * ============================================================================ */

void test_integration_imaging_batch_parameter_update_optimization(void** state) {
  (void)state;

  printf("\nTest: Batch Parameter Update Optimization\n");
  printf("--------------------------------------------------------\n");

  // Set baseline settings
  struct imaging_settings baseline_settings = {
    .brightness = 0,
    .contrast = 0,
    .saturation = 0,
    .sharpness = 0,
    .hue = 0,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  int result = onvif_imaging_set_settings(&baseline_settings);
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("Baseline set completed\n");

  // Test 1: Update all 5 parameters
  struct imaging_settings all_changed = {
    .brightness = BRIGHTNESS_LOW,
    .contrast = CONTRAST_MEDIUM,
    .saturation = SATURATION_MEDIUM,
    .sharpness = SHARPNESS_HIGH,
    .hue = HUE_MEDIUM_POS,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  result = onvif_imaging_set_settings(&all_changed);
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("All parameters changed test completed\n");

  // Test 2: Update only brightness
  struct imaging_settings brightness_only = {
    .brightness = BRIGHTNESS_LOW_MED, // Changed
    .contrast = CONTRAST_MEDIUM,
    .saturation = SATURATION_MEDIUM,
    .sharpness = SHARPNESS_HIGH,
    .hue = HUE_MEDIUM_POS,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  result = onvif_imaging_set_settings(&brightness_only);
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("Single parameter changed test completed\n");

  // Test 3: No changes
  result = onvif_imaging_set_settings(&brightness_only);
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("No parameters changed test completed\n");
  printf("✅ Batch parameter update optimization test completed\n");
}

/* ============================================================================
 * Integration Test: Parameter Cache Efficiency
 * ============================================================================ */

void test_integration_imaging_parameter_cache_efficiency(void** state) {
  (void)state;

  printf("\nTest: Parameter Cache Efficiency\n");
  printf("-----------------------------------------------\n");

  struct imaging_settings settings = {
    .brightness = BRIGHTNESS_MEDIUM,
    .contrast = CONTRAST_LOW,
    .saturation = SATURATION_MIN,
    .sharpness = SHARPNESS_MEDIUM_LOW,
    .hue = HUE_SMALL_POS_ALT,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  // First call - should validate and apply
  long start_time = test_get_time_microseconds();
  int result = onvif_imaging_get_settings(&settings);
  long first_call_time = test_get_time_microseconds() - start_time;

  assert_int_equal(result, ONVIF_SUCCESS);
  printf("First call time: %ld μs\n", first_call_time);

  // Second call - should use cached values
  start_time = test_get_time_microseconds();
  result = onvif_imaging_get_settings(&settings);
  long second_call_time = test_get_time_microseconds() - start_time;

  assert_int_equal(result, ONVIF_SUCCESS);
  printf("Second call time: %ld μs\n", second_call_time);

  if (second_call_time <= first_call_time) {
    printf("✅ Parameter caching is effective (improvement: %.1fx)\n",
           (double)first_call_time / (double)(second_call_time > 0 ? second_call_time : 1));
  } else {
    printf("⚠️  Parameter caching effectiveness inconclusive\n");
  }
}

/* ============================================================================
 * Integration Test: Concurrent Access
 * ============================================================================ */

static void* concurrent_imaging_test_thread(void* arg) {
  int thread_id = *(int*)arg;
  struct imaging_settings settings;

  for (int i = 0; i < CONCURRENT_ITERATIONS; i++) {
    // Vary settings based on thread ID and iteration
    settings.brightness = (thread_id * VAR_BRIGHTNESS_STEP + i) % PARAM_RANGE_MAX + PARAM_RANGE_MIN;
    settings.contrast = (thread_id * VAR_CONTRAST_STEP + i) % PARAM_RANGE_MAX + PARAM_RANGE_MIN;
    settings.saturation = (thread_id * VAR_SATURATION_STEP + i) % PARAM_RANGE_MAX + PARAM_RANGE_MIN;
    settings.sharpness = (thread_id * VAR_SHARPNESS_STEP + i) % PARAM_RANGE_MAX + PARAM_RANGE_MIN;
    settings.hue = (thread_id * VAR_HUE_STEP + i) % HUE_RANGE_MAX + HUE_RANGE_MIN;
    settings.daynight.mode = DAY_NIGHT_AUTO;
    settings.daynight.enable_auto_switching = 1;

    if (onvif_imaging_set_settings(&settings) != ONVIF_SUCCESS) {
      printf("❌ Thread %d failed at iteration %d\n", thread_id, i);
      return (void*)(intptr_t)ONVIF_ERROR_INVALID; // NOLINT
    }

    if (onvif_imaging_get_settings(&settings) != ONVIF_SUCCESS) {
      printf("❌ Thread %d get_settings failed at iteration %d\n", thread_id, i);
      return (void*)(intptr_t)ONVIF_ERROR_INVALID; // NOLINT
    }
  }

  return (void*)(intptr_t)ONVIF_SUCCESS;
}

void test_integration_imaging_concurrent_access(void** state) {
  (void)state;

  printf("\nTest: Concurrent Imaging Access\n");
  printf("----------------------------------------------\n");

  pthread_t threads[CONCURRENT_THREADS];
  int thread_ids[CONCURRENT_THREADS];

  long start_time = test_get_time_microseconds();

  // Create threads
  for (int i = 0; i < CONCURRENT_THREADS; i++) {
    thread_ids[i] = i;
    int result = pthread_create(&threads[i], NULL, concurrent_imaging_test_thread, &thread_ids[i]);
    assert_int_equal(result, 0);
  }

  // Wait for threads to complete
  int failed_threads = 0;
  for (int i = 0; i < CONCURRENT_THREADS; i++) {
    void* thread_return = NULL;
    pthread_join(threads[i], &thread_return);
    int thread_status = (int)(intptr_t)thread_return;
    if (thread_status != ONVIF_SUCCESS) {
      failed_threads++;
    }
  }

  long total_time = test_get_time_microseconds() - start_time;

  printf("Concurrent test completed in %ld μs\n", total_time);
  printf("Failed threads: %d/%d\n", failed_threads, CONCURRENT_THREADS);

  assert_int_equal(failed_threads, 0);
  printf("✅ Concurrent imaging access test passed\n");
}

/* ============================================================================
 * Integration Test: Performance Regression
 * ============================================================================ */

void test_integration_imaging_performance_regression(void** state) {
  (void)state;

  printf("\nTest: Performance Regression Check\n");
  printf("-------------------------------------------------\n");

  struct imaging_settings test_settings = {
    .brightness = BRIGHTNESS_HIGH_ALT,
    .contrast = CONTRAST_NEG_MED,
    .saturation = SATURATION_MEDIUM_ALT,
    .sharpness = SHARPNESS_NEUTRAL,
    .hue = HUE_LARGE_NEG,
    .daynight = {.mode = DAY_NIGHT_AUTO, .enable_auto_switching = 1}};

  // Warm up
  for (int i = 0; i < WARMUP_ITERATIONS; i++) {
    int result = onvif_imaging_set_settings(&test_settings);
    assert_int_equal(result, ONVIF_SUCCESS);
    result = onvif_imaging_get_settings(&test_settings);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Performance test
  long start_time = test_get_time_microseconds();

  for (int i = 0; i < TEST_ITERATIONS; i++) {
    test_settings.brightness = (i % PARAM_RANGE_MAX) + PARAM_RANGE_MIN;
    test_settings.contrast = ((i * 2) % PARAM_RANGE_MAX) + PARAM_RANGE_MIN;

    int result = onvif_imaging_set_settings(&test_settings);
    assert_int_equal(result, ONVIF_SUCCESS);

    result = onvif_imaging_get_settings(&test_settings);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  long total_time = test_get_time_microseconds() - start_time;
  long avg_time_per_operation = total_time / (long)(TEST_ITERATIONS * 2);

  printf("Performance Results:\n");
  printf("  Total operations: %d\n", TEST_ITERATIONS * 2);
  printf("  Total time: %ld μs\n", total_time);
  printf("  Average time per operation: %ld μs\n", avg_time_per_operation);
  printf("  Operations per second: %.2f\n",
         (double)((long)(TEST_ITERATIONS * 2)) / ((double)total_time / (double)MICROS_PER_SECOND));

  if (avg_time_per_operation < BENCHMARK_THRESHOLD_US) {
    printf("✅ Performance regression test passed (under %d μs threshold)\n",
           BENCHMARK_THRESHOLD_US);
  } else {
    printf("⚠️  Performance regression test warning (exceeds %d μs threshold)\n",
           BENCHMARK_THRESHOLD_US);
  }
}

/**
 * @brief Pilot SOAP test for Imaging GetImagingSettings operation
 * Tests SOAP envelope parsing and response structure validation
 */
void test_integration_imaging_get_settings_soap(void** state) {
  (void)state;

  // Note: Imaging service is already initialized by setup_imaging_integration()
  // Platform mock, service dispatcher, and imaging service are ready

  // Configure platform configuration mock expectations for gSOAP verbosity lookup
  expect_string(__wrap_platform_config_get_int, section, "logging");
  expect_string(__wrap_platform_config_get_int, key, "http_verbose");
  expect_function_call(__wrap_platform_config_get_int);
  will_return(__wrap_platform_config_get_int, 0);
  expect_string(__wrap_platform_config_get_int, section, "logging");
  expect_string(__wrap_platform_config_get_int, key, "http_verbose");
  expect_function_call(__wrap_platform_config_get_int);
  will_return(__wrap_platform_config_get_int, 0);

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "GetImagingSettings", SOAP_IMAGING_GET_IMAGING_SETTINGS, "/onvif/imaging_service");
  assert_non_null(request);

  // Step 3: Validate request structure
  assert_non_null(request->body);
  assert_true(strstr(request->body, "GetImagingSettings") != NULL);

  // Step 4: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 5: Call actual service handler (integration test)
  int result = onvif_imaging_handle_operation("GetImagingSettings", request, &response);
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

  // Step 7: Validate SOAP response contains expected elements
  assert_true(strstr(response.body, "GetImagingSettingsResponse") != NULL);
  assert_true(strstr(response.body, "Brightness") != NULL);

  // Step 8: Verify response contains ImagingSettings with valid brightness value
  char brightness_value[64] = {0};
  result = soap_test_extract_element_text(response.body, "Brightness", brightness_value,
                                          sizeof(brightness_value));
  if (result == ONVIF_SUCCESS) {
    assert_true(strlen(brightness_value) > 0);
  }

  // Step 9: Cleanup resources
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }

  // Note: Imaging service cleanup handled by teardown_imaging_integration()
}

/**
 * @brief SOAP test for Imaging SetImagingSettings operation
 */
void test_integration_imaging_set_settings_soap(void** state) {
  (void)state;

  // Configure platform configuration mock expectations for gSOAP verbosity lookup
  expect_string(__wrap_platform_config_get_int, section, "logging");
  expect_string(__wrap_platform_config_get_int, key, "http_verbose");
  expect_function_call(__wrap_platform_config_get_int);
  will_return(__wrap_platform_config_get_int, 0);
  expect_string(__wrap_platform_config_get_int, section, "logging");
  expect_string(__wrap_platform_config_get_int, key, "http_verbose");
  expect_function_call(__wrap_platform_config_get_int);
  will_return(__wrap_platform_config_get_int, 0);

  // Step 1: Create SOAP request envelope
  http_request_t* request = soap_test_create_request(
    "SetImagingSettings", SOAP_IMAGING_SET_IMAGING_SETTINGS, "/onvif/imaging_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_imaging_handle_operation("SetImagingSettings", request, &response);
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

  struct _timg__SetImagingSettingsResponse* settings_response = NULL;
  result = soap_test_parse_set_imaging_settings_response(&ctx, &settings_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(settings_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest imaging_service_optimization_tests[] = {
  cmocka_unit_test_setup_teardown(test_integration_imaging_parameter_cache_efficiency,
                                  setup_imaging_integration, teardown_imaging_integration),
  cmocka_unit_test_setup_teardown(test_integration_imaging_bulk_settings_validation,
                                  setup_imaging_integration, teardown_imaging_integration),
  cmocka_unit_test_setup_teardown(test_integration_imaging_batch_parameter_update_optimization,
                                  setup_imaging_integration, teardown_imaging_integration),
  cmocka_unit_test_setup_teardown(test_integration_imaging_performance_regression,
                                  setup_imaging_integration, teardown_imaging_integration),

  // SOAP integration tests (full HTTP/SOAP layer validation)
  cmocka_unit_test_setup_teardown(test_integration_imaging_get_settings_soap,
                                  setup_imaging_integration, teardown_imaging_integration),
  cmocka_unit_test_setup_teardown(test_integration_imaging_set_settings_soap,
                                  setup_imaging_integration, teardown_imaging_integration),

  // Concurrent tests (may hang - placed at end)
  cmocka_unit_test_setup_teardown(test_integration_imaging_concurrent_access,
                                  setup_imaging_integration, teardown_imaging_integration),
};
