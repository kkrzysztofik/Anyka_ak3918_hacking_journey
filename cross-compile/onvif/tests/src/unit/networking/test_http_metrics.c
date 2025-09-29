/**
 * @file test_http_metrics.c
 * @brief Unit tests for HTTP performance metrics module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <pthread.h>
#include <unistd.h>
#include <sys/time.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_server.h"
#include "utils/error/error_handling.h"

// Test constants
#define TEST_METRICS_THREAD_COUNT 10
#define TEST_METRICS_REQUEST_COUNT 100
#define TEST_METRICS_TIMEOUT_MS 5000
#define TEST_CPU_OVERHEAD_THRESHOLD 5.0  // 5% CPU overhead threshold

// Global test state
static int g_test_metrics_initialized = 0;
static pthread_mutex_t g_test_metrics_mutex = PTHREAD_MUTEX_INITIALIZER;

/* ==================== Test Setup/Teardown ==================== */

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

/* ==================== Helper Functions ==================== */

/**
 * @brief Get current time in milliseconds
 * @return Current time in milliseconds
 */
static uint64_t get_current_time_ms(void) {
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return (uint64_t)(tv.tv_sec * 1000 + tv.tv_usec / 1000);
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
    sleep(1);  // Simplified simulation for testing
  }

  // Record metrics
  http_metrics_record_request(latency_ms, response_size, status_code);
}

/* ==================== Unit Tests ==================== */

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
  http_metrics_record_request(10, 1024, 200);  // 10ms, 1KB, success
  http_metrics_record_request(20, 2048, 200);  // 20ms, 2KB, success
  http_metrics_record_request(5, 512, 400);    // 5ms, 512B, client error
  http_metrics_record_request(30, 4096, 500);  // 30ms, 4KB, server error

  // Get current metrics
  assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);

  // Verify accuracy
  assert_int_equal(metrics.total_requests, 4);
  assert_int_equal(metrics.successful_requests, 2);
  assert_int_equal(metrics.client_errors, 1);
  assert_int_equal(metrics.server_errors, 1);
  assert_int_equal(metrics.total_response_bytes, 1024 + 2048 + 512 + 4096);
  assert_int_equal(metrics.total_latency_ms, 10 + 20 + 5 + 30);
  assert_int_equal(metrics.min_latency_ms, 5);
  assert_int_equal(metrics.max_latency_ms, 30);
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

/* ==================== Concurrency Tests ==================== */

/**
 * @brief Thread function for concurrent metrics testing
 * @param arg Thread argument (unused)
 * @return NULL
 */
static void* metrics_thread_function(void* arg) {
  (void)arg;

  // Simulate multiple requests from this thread
  for (int i = 0; i < TEST_METRICS_REQUEST_COUNT; i++) {
    uint64_t latency = (i % 50) + 1;  // 1-50ms latency
    size_t response_size = (i % 1024) + 100;  // 100-1123 bytes
    int status_code = (i % 4 == 0) ? 400 : 200;  // 25% error rate

    simulate_http_request(latency, response_size, status_code);
  }

  return NULL;
}

/**
 * @brief Test concurrent metrics collection thread safety
 */
static void test_http_metrics_concurrency(void** state) {
  (void)state;

  pthread_t threads[TEST_METRICS_THREAD_COUNT];
  http_performance_metrics_t metrics_before, metrics_after;

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
  uint64_t expected_requests = metrics_before.total_requests +
                              (TEST_METRICS_THREAD_COUNT * TEST_METRICS_REQUEST_COUNT);
  assert_int_equal(metrics_after.total_requests, expected_requests);

  // Verify no data corruption (basic sanity checks)
  assert_true(metrics_after.total_requests >= metrics_before.total_requests);
  assert_true(metrics_after.total_response_bytes >= metrics_before.total_response_bytes);
  assert_true(metrics_after.total_latency_ms >= metrics_before.total_latency_ms);
}

/* ==================== Performance Tests ==================== */

/**
 * @brief Test CPU overhead threshold compliance
 */
static void test_http_metrics_cpu_overhead(void** state) {
  (void)state;

  uint64_t start_time = get_current_time_ms();
  uint64_t start_cpu_time = clock();

  // Simulate high request load
  for (int i = 0; i < 1000; i++) {
    uint64_t latency = (i % 10) + 1;  // 1-10ms latency
    size_t response_size = (i % 512) + 100;  // 100-611 bytes
    int status_code = (i % 10 == 0) ? 400 : 200;  // 10% error rate

    http_metrics_record_request(latency, response_size, status_code);
  }

  uint64_t end_time = get_current_time_ms();
  uint64_t end_cpu_time = clock();

  // Calculate CPU overhead
  uint64_t total_time_ms = end_time - start_time;
  uint64_t cpu_time_ms = (end_cpu_time - start_cpu_time) / (CLOCKS_PER_SEC / 1000);

  double cpu_overhead_percent = (double)cpu_time_ms / total_time_ms * 100.0;

  // Verify overhead is below threshold
  assert_true(cpu_overhead_percent < TEST_CPU_OVERHEAD_THRESHOLD);
}

/**
 * @brief Test metrics retrieval performance (non-blocking)
 */
static void test_http_metrics_retrieval_performance(void** state) {
  (void)state;

  http_performance_metrics_t metrics;
  uint64_t start_time = get_current_time_ms();

  // Perform many metrics retrievals
  for (int i = 0; i < 1000; i++) {
    assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);
  }

  uint64_t end_time = get_current_time_ms();
  uint64_t total_time_ms = end_time - start_time;

  // Verify retrieval is fast (should be < 1ms for 1000 retrievals)
  assert_true(total_time_ms < 1000);  // Less than 1ms per retrieval on average
}

/* ==================== Integration Tests ==================== */

/**
 * @brief Test metrics with realistic HTTP request patterns
 */
static void test_http_metrics_realistic_patterns(void** state) {
  (void)state;

  http_performance_metrics_t metrics;

  // Simulate realistic ONVIF request patterns
  // Device service requests (fast, small responses)
  for (int i = 0; i < 50; i++) {
    http_metrics_record_request(5, 512, 200);
  }

  // Media service requests (medium latency, larger responses)
  for (int i = 0; i < 30; i++) {
    http_metrics_record_request(15, 2048, 200);
  }

  // PTZ service requests (variable latency)
  for (int i = 0; i < 20; i++) {
    uint64_t latency = (i % 20) + 10;  // 10-29ms
    http_metrics_record_request(latency, 1024, 200);
  }

  // Some error cases
  for (int i = 0; i < 10; i++) {
    http_metrics_record_request(8, 256, 400);
  }

  // Get final metrics
  assert_int_equal(http_metrics_get_current(&metrics), ONVIF_SUCCESS);

  // Verify realistic patterns
  assert_int_equal(metrics.total_requests, 110);
  assert_true(metrics.successful_requests >= 100);  // Most requests successful
  assert_true(metrics.client_errors >= 10);  // Some client errors
  assert_true(metrics.total_response_bytes > 50000);  // Reasonable response size
  // Calculate average latency
  uint64_t avg_latency = (metrics.total_requests > 0) ?
                        (metrics.total_latency_ms / metrics.total_requests) : 0;
  assert_true(avg_latency > 0);  // Average latency calculated
}

/* ==================== Test Suite Definition ==================== */

/**
 * @brief HTTP metrics test suite
 */
static const struct CMUnitTest http_metrics_tests[] = {
  // Unit tests
  cmocka_unit_test_setup_teardown(test_http_metrics_init_cleanup, setup_http_metrics_tests, teardown_http_metrics_tests),
  cmocka_unit_test_setup_teardown(test_http_metrics_recording_accuracy, setup_http_metrics_tests, teardown_http_metrics_tests),
  cmocka_unit_test_setup_teardown(test_http_metrics_null_handling, setup_http_metrics_tests, teardown_http_metrics_tests),
  cmocka_unit_test_setup_teardown(test_http_metrics_connection_updates, setup_http_metrics_tests, teardown_http_metrics_tests),

  // Concurrency tests
  cmocka_unit_test_setup_teardown(test_http_metrics_concurrency, setup_http_metrics_tests, teardown_http_metrics_tests),

  // Performance tests
  cmocka_unit_test_setup_teardown(test_http_metrics_cpu_overhead, setup_http_metrics_tests, teardown_http_metrics_tests),
  cmocka_unit_test_setup_teardown(test_http_metrics_retrieval_performance, setup_http_metrics_tests, teardown_http_metrics_tests),

  // Integration tests
  cmocka_unit_test_setup_teardown(test_http_metrics_realistic_patterns, setup_http_metrics_tests, teardown_http_metrics_tests),
};
