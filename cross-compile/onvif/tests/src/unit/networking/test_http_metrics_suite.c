/**
 * @file test_http_metrics_suite.c
 * @brief HTTP metrics test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_http_metrics.c and test_http_metrics_simple.c
void test_unit_http_metrics_init_cleanup(void** state);
void test_unit_http_metrics_recording_accuracy(void** state);
void test_unit_http_metrics_null_handling(void** state);
void test_unit_http_metrics_connection_updates(void** state);
void test_unit_http_metrics_concurrency(void** state);
void test_unit_http_metrics_cpu_overhead(void** state);
void test_unit_http_metrics_retrieval_performance(void** state);
void test_unit_http_metrics_realistic_patterns(void** state);

/**
 * @brief Get HTTP metrics unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_metrics_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_http_metrics_init_cleanup),
    cmocka_unit_test(test_unit_http_metrics_recording_accuracy),
    cmocka_unit_test(test_unit_http_metrics_null_handling),
    cmocka_unit_test(test_unit_http_metrics_connection_updates),
    cmocka_unit_test(test_unit_http_metrics_concurrency),
    cmocka_unit_test(test_unit_http_metrics_cpu_overhead),
    cmocka_unit_test(test_unit_http_metrics_retrieval_performance),
    cmocka_unit_test(test_unit_http_metrics_realistic_patterns),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
