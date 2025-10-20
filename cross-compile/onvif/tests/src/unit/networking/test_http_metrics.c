/**
 * @file test_http_metrics.c
 * @brief Unit tests for HTTP performance metrics module
 * @author kkrzysztofik
 * @date 2025
 */

#include <bits/pthreadtypes.h>
#include <bits/types/struct_timeval.h>
#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_server.h"
#include "utils/error/error_handling.h"

// Test constants
#define TEST_METRICS_THREAD_COUNT   10
#define TEST_METRICS_REQUEST_COUNT  100
#define TEST_METRICS_TIMEOUT_MS     5000
#define TEST_CPU_OVERHEAD_THRESHOLD 5.0 // 5% CPU overhead threshold

// HTTP status codes
#define TEST_HTTP_STATUS_OK           200
#define TEST_HTTP_STATUS_BAD_REQUEST  400
#define TEST_HTTP_STATUS_SERVER_ERROR 500

// Test data sizes
#define TEST_RESPONSE_SIZE_SMALL  512
#define TEST_RESPONSE_SIZE_MEDIUM 1024
#define TEST_RESPONSE_SIZE_LARGE  2048
#define TEST_RESPONSE_SIZE_XLARGE 4096
#define TEST_RESPONSE_SIZE_MIN    100
#define TEST_RESPONSE_SIZE_MAX    1123

// Test latencies
#define TEST_LATENCY_MIN_MS    1
#define TEST_LATENCY_MAX_MS    50
#define TEST_LATENCY_SHORT_MS  5
#define TEST_LATENCY_MEDIUM_MS 10
#define TEST_LATENCY_LONG_MS   20
#define TEST_LATENCY_XLONG_MS  30

// Test iteration counts
#define TEST_ITERATIONS_SMALL       5
#define TEST_ITERATIONS_MEDIUM      10
#define TEST_ITERATIONS_LARGE       20
#define TEST_ITERATIONS_XLARGE      30
#define TEST_ITERATIONS_PERFORMANCE 50
#define TEST_ITERATIONS_STRESS      1000

// Test thresholds
#define TEST_CPU_OVERHEAD_PERCENT 5.0
#define TEST_RETRIEVAL_TIME_MS    1000

// Test response size constants
#define TEST_RESPONSE_SIZE_TINY      256
#define TEST_RESPONSE_SIZE_MINIMUM   100
#define TEST_RESPONSE_SIZE_THRESHOLD 50000

// Test request counts
#define TEST_TOTAL_REQUESTS_EXPECTED 110
#define TEST_SUCCESSFUL_REQUESTS_MIN 100

// Time conversion constants
#define MILLISECONDS_PER_SECOND      1000
#define MICROSECONDS_PER_MILLISECOND 1000
#define CLOCKS_PER_MILLISECOND       (CLOCKS_PER_SEC / MILLISECONDS_PER_SECOND)

// Global test state
// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
static int g_test_metrics_initialized = 0;
// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
static pthread_mutex_t g_test_metrics_mutex = PTHREAD_MUTEX_INITIALIZER;

/**
 * @brief Setup function for metrics tests
 * @param state Test state (unused)
 * @return 0 on success, -1 on failure
 */
static int setup_http_metrics_tests(void** state) {
  (void)state;

  pthread_mutex_lock(&g_test_metrics_mutex);

  if (!g_test_metrics_initialized) {
    // Initialize HTTP server metrics
    if (http_metrics_init() != ONVIF_SUCCESS) {
      pthread_mutex_unlock(&g_test_metrics_mutex);
      return -1;
    }
    g_test_metrics_initialized = 1;
  }

  pthread_mutex_unlock(&g_test_metrics_mutex);
  return 0;
}

/**
 * @brief Teardown function for metrics tests
 * @param state Test state (unused)
 * @return 0 on success, -1 on failure
 */
static int teardown_http_metrics_tests(void** state) {
  (void)state;

  pthread_mutex_lock(&g_test_metrics_mutex);

  if (g_test_metrics_initialized) {
    http_metrics_cleanup();
    g_test_metrics_initialized = 0;
  }

  pthread_mutex_unlock(&g_test_metrics_mutex);
  return 0;
}

/**
 * @brief Get current time in milliseconds
 * @return Current time in milliseconds
 */
static uint64_t get_current_time_ms(void) {
  struct timeval time_val;
  gettimeofday(&time_val, NULL);
  return (uint64_t)(time_val.tv_sec * MILLISECONDS_PER_SECOND + time_val.tv_usec / MICROSECONDS_PER_MILLISECOND);
}

/**
 * @brief Simulate HTTP request processing with metrics recording
 * @param latency_ms Simulated latency in milliseconds
 * @param response_size Simulated response size in bytes
 * @param status_code HTTP status code
 */
static void simulate_http_request(uint64_t latency_ms, size_t response_size, int status_code) {
  // Simulate request processing time
  // Simulate processing time (use sleep for portability)
  if (latency_ms > 0) {
    sleep(1); // Simplified simulation for testing
  }

  // Record metrics
  http_metrics_record_request(latency_ms, response_size, status_code);
}

/**
 * @brief Test metrics initialization and cleanup
 */
static void test_http_metrics_init_cleanup(void** state) {
  (void)state;

  // Test initialization
  assert_int_equal(http_metrics_init(), ONVIF_SUCCESS);

  // Test cleanup
  assert_int_equal(http_metrics_cleanup(), ONVIF_SUCCESS);
}

/**
 * @brief Test metrics recording accuracy
 */
static void test_http_metrics_recording_accuracy(void** state) {
  (void)state;

  http_performance_metrics_t metrics;

  // Record some test requests
  http_metrics_record_request(TEST_LATENCY_MEDIUM_MS, TEST_RESPONSE_SIZE_MEDIUM, TEST_HTTP_STATUS_OK);
  http_metrics_record_request(TEST_LATENCY_LONG_MS, TEST_RESPONSE_SIZE_LARGE, TEST_HTTP_STATUS_OK);
  http_metrics_record_request(TEST_LATENCY_SHORT_MS, TEST_RESPONSE_SIZE_SMALL, TEST_HTTP_STATUS_BAD_REQUEST);
  http_metrics_record_request(TEST_LATENCY_XLONG_MS, TEST_RESPONSE_SIZE_XLARGE, TEST_HTTP_STATUS_SERVER_ERROR);

  // Get current metrics
  assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);

  // Verify accuracy
  assert_int_equal(metrics.total_requests, 4);
  assert_int_equal(metrics.successful_requests, 2);
  assert_int_equal(metrics.client_errors, 1);
  assert_int_equal(metrics.server_errors, 1);
  assert_int_equal(metrics.total_response_bytes,
                   TEST_RESPONSE_SIZE_MEDIUM + TEST_RESPONSE_SIZE_LARGE + TEST_RESPONSE_SIZE_SMALL + TEST_RESPONSE_SIZE_XLARGE);
  assert_int_equal(metrics.total_latency_ms, TEST_LATENCY_MEDIUM_MS + TEST_LATENCY_LONG_MS + TEST_LATENCY_SHORT_MS + TEST_LATENCY_XLONG_MS);
  assert_int_equal(metrics.min_latency_ms, TEST_LATENCY_SHORT_MS);
  assert_int_equal(metrics.max_latency_ms, TEST_LATENCY_XLONG_MS);
}

/**
 * @brief Test metrics with null pointer handling
 */
static void test_http_metrics_null_handling(void** state) {
  (void)state;

  // Test null metrics pointer
  assert_int_equal(http_metrics_get_current(NULL), ONVIF_ERROR_NULL);
}

/**
 * @brief Test connection count updates
 */
static void test_http_metrics_connection_updates(void** state) {
  (void)state;

  http_performance_metrics_t metrics;

  // Update connection count
  assert_int_equal(http_metrics_update_connections(1), ONVIF_SUCCESS);
  assert_int_equal(http_metrics_update_connections(1), ONVIF_SUCCESS);
  assert_int_equal(http_metrics_update_connections(-1), ONVIF_SUCCESS);

  // Get current metrics
  assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);

  // Verify connection count
  assert_int_equal(metrics.current_connections, 1);
}

/**
 * @brief Thread function for concurrent metrics testing
 * @param arg Thread argument (unused)
 * @return NULL
 */
static void* metrics_thread_function(void* arg) {
  (void)arg;

  // Simulate multiple requests from this thread
  for (int i = 0; i < TEST_METRICS_REQUEST_COUNT; i++) {
    uint64_t latency = (i % TEST_LATENCY_MAX_MS) + TEST_LATENCY_MIN_MS;
    size_t response_size = (i % TEST_RESPONSE_SIZE_MEDIUM) + TEST_RESPONSE_SIZE_MIN;
    int status_code = (i % 4 == 0) ? TEST_HTTP_STATUS_BAD_REQUEST : TEST_HTTP_STATUS_OK;

    simulate_http_request(latency, response_size, status_code);
  }

  return NULL;
}

/**
 * @brief Test concurrent metrics collection thread safety
 */
// NOLINTNEXTLINE(readability-isolate-declaration)
static void test_http_metrics_concurrency(void** state) {
  (void)state;

  pthread_t threads[TEST_METRICS_THREAD_COUNT];
  http_performance_metrics_t metrics_before;
  http_performance_metrics_t metrics_after;

  // Get initial metrics
  assert_int_equal(http_metrics_get_current(&metrics_before), ONVIF_SUCCESS);

  // Create and start threads
  for (int i = 0; i < TEST_METRICS_THREAD_COUNT; i++) {
    assert_int_equal(pthread_create(&threads[i], NULL, metrics_thread_function, NULL), 0);
  }

  // Wait for all threads to complete
  for (int i = 0; i < TEST_METRICS_THREAD_COUNT; i++) {
    assert_int_equal(pthread_join(threads[i], NULL), 0);
  }

  // Get final metrics
  assert_int_equal(http_metrics_get_current(&metrics_after), ONVIF_SUCCESS);

  // Verify thread safety - total requests should equal expected count
  uint64_t expected_requests = metrics_before.total_requests + ((uint64_t)TEST_METRICS_THREAD_COUNT * TEST_METRICS_REQUEST_COUNT);
  assert_int_equal(metrics_after.total_requests, expected_requests);

  // Verify no data corruption (basic sanity checks)
  assert_true(metrics_after.total_requests >= metrics_before.total_requests);
  assert_true(metrics_after.total_response_bytes >= metrics_before.total_response_bytes);
  assert_true(metrics_after.total_latency_ms >= metrics_before.total_latency_ms);
}

/**
 * @brief Test CPU overhead threshold compliance
 */
static void test_http_metrics_cpu_overhead(void** state) {
  (void)state;

  uint64_t start_time = get_current_time_ms();
  uint64_t start_cpu_time = clock();

  // Simulate high request load
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    uint64_t latency = (i % TEST_LATENCY_MEDIUM_MS) + TEST_LATENCY_MIN_MS;
    size_t response_size = (i % TEST_RESPONSE_SIZE_SMALL) + TEST_RESPONSE_SIZE_MIN;
    int status_code = (i % TEST_LATENCY_MEDIUM_MS == 0) ? TEST_HTTP_STATUS_BAD_REQUEST : TEST_HTTP_STATUS_OK;

    http_metrics_record_request(latency, response_size, status_code);
  }

  uint64_t end_time = get_current_time_ms();
  uint64_t end_cpu_time = clock();

  // Calculate CPU overhead
  uint64_t total_time_ms = end_time - start_time;
  uint64_t cpu_time_ms = (end_cpu_time - start_cpu_time) / CLOCKS_PER_MILLISECOND;

  double cpu_overhead_percent = (double)cpu_time_ms / (double)total_time_ms * 100.0;

  // Verify overhead is below threshold
  const double cpu_threshold = TEST_CPU_OVERHEAD_PERCENT;
  assert_true(cpu_overhead_percent < cpu_threshold);
}

/**
 * @brief Test metrics retrieval performance (non-blocking)
 */
static void test_http_metrics_retrieval_performance(void** state) {
  (void)state;

  http_performance_metrics_t metrics;
  uint64_t start_time = get_current_time_ms();

  // Perform many metrics retrievals
  for (int i = 0; i < TEST_ITERATIONS_STRESS; i++) {
    assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);
  }

  uint64_t end_time = get_current_time_ms();
  uint64_t total_time_ms = end_time - start_time;

  // Verify retrieval is fast (should be < 1ms for 1000 retrievals)
  assert_true(total_time_ms < TEST_RETRIEVAL_TIME_MS);
}

/**
 * @brief Test metrics with realistic HTTP request patterns
 */
static void test_http_metrics_realistic_patterns(void** state) {
  (void)state;

  http_performance_metrics_t metrics;

  // Simulate realistic ONVIF request patterns
  // Device service requests (fast, small responses)
  for (int i = 0; i < TEST_ITERATIONS_PERFORMANCE; i++) {
    http_metrics_record_request(TEST_LATENCY_SHORT_MS, TEST_RESPONSE_SIZE_SMALL, TEST_HTTP_STATUS_OK);
  }

  // Media service requests (medium latency, larger responses)
  for (int i = 0; i < TEST_ITERATIONS_XLARGE; i++) {
    http_metrics_record_request(TEST_LATENCY_XLONG_MS, TEST_RESPONSE_SIZE_LARGE, TEST_HTTP_STATUS_OK);
  }

  // PTZ service requests (variable latency)
  for (int i = 0; i < TEST_ITERATIONS_LARGE; i++) {
    uint64_t latency = (i % TEST_ITERATIONS_LARGE) + TEST_LATENCY_MEDIUM_MS;
    http_metrics_record_request(latency, TEST_RESPONSE_SIZE_MEDIUM, TEST_HTTP_STATUS_OK);
  }

  // Some error cases
  for (int i = 0; i < TEST_LATENCY_MEDIUM_MS; i++) {
    http_metrics_record_request(TEST_LATENCY_MEDIUM_MS, TEST_RESPONSE_SIZE_TINY, TEST_HTTP_STATUS_BAD_REQUEST);
  }

  // Get final metrics
  assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);

  // Verify realistic patterns
  assert_int_equal(metrics.total_requests, TEST_TOTAL_REQUESTS_EXPECTED);
  assert_true(metrics.successful_requests >= TEST_SUCCESSFUL_REQUESTS_MIN); // Most requests successful
  assert_true(metrics.client_errors >= TEST_LATENCY_MEDIUM_MS);             // Some client errors
  assert_true(metrics.total_response_bytes > TEST_RESPONSE_SIZE_THRESHOLD); // Reasonable response size
  // Calculate average latency
  uint64_t avg_latency = (metrics.total_requests > 0) ? (metrics.total_latency_ms / metrics.total_requests) : 0;
  assert_true(avg_latency > 0); // Average latency calculated
}
