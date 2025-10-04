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

// Forward declarations for setup/teardown from test_http_metrics_simple.c
int setup_http_metrics_tests(void** state);
int teardown_http_metrics_tests(void** state);

/**
 * @brief Get HTTP metrics unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_metrics_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_http_metrics_init_cleanup),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_recording_accuracy,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_null_handling,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_connection_updates,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_concurrency,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_cpu_overhead,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_retrieval_performance,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
    cmocka_unit_test_setup_teardown(test_unit_http_metrics_realistic_patterns,
                                     setup_http_metrics_tests, teardown_http_metrics_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
