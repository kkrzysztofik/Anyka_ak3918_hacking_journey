/**
 * @file test_networking_runner.c
 * @brief Test runner for networking tests (HTTP auth, metrics)
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations for test_http_auth.c
void test_unit_http_auth_init_sets_defaults(void** state);
void test_unit_http_auth_init_null(void** state);
void test_unit_http_auth_verify_credentials_success(void** state);
void test_unit_http_auth_verify_credentials_failure(void** state);
void test_unit_http_auth_parse_basic_credentials_success(void** state);
void test_unit_http_auth_parse_basic_credentials_invalid_scheme(void** state);
void test_unit_http_auth_parse_basic_credentials_decode_failure(void** state);
void test_unit_http_auth_parse_basic_credentials_missing_delimiter(void** state);
void test_unit_http_auth_generate_challenge_success(void** state);
void test_unit_http_auth_generate_challenge_invalid(void** state);
void test_unit_http_auth_validate_basic_disabled(void** state);
void test_unit_http_auth_validate_basic_missing_header(void** state);
void test_unit_http_auth_validate_basic_invalid_credentials(void** state);
void test_unit_http_auth_validate_basic_success(void** state);
void test_unit_http_auth_validate_basic_parse_failure(void** state);
void test_unit_http_auth_create_401_response(void** state);
void test_unit_http_auth_create_401_response_invalid_realm(void** state);

// Forward declarations for test_http_metrics.c
void test_unit_http_metrics_init_cleanup(void** state);
void test_unit_http_metrics_recording_accuracy(void** state);
void test_unit_http_metrics_null_handling(void** state);
void test_unit_http_metrics_connection_updates(void** state);
void test_unit_http_metrics_concurrency(void** state);
void test_unit_http_metrics_cpu_overhead(void** state);
void test_unit_http_metrics_retrieval_performance(void** state);
void test_unit_http_metrics_realistic_patterns(void** state);

/**
 * @brief Global test setup
 * @param state Test state
 * @return 0 on success
 */
static int setup_global_tests(void** state) {
  (void)state;
  return 0;
}

/**
 * @brief Global test teardown
 * @param state Test state
 * @return 0 on success
 */
static int teardown_global_tests(void** state) {
  (void)state;
  return 0;
}

/**
 * @brief Main test runner for networking tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Networking Tests\n");
  printf("======================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // HTTP authentication tests
    cmocka_unit_test(test_unit_http_auth_init_sets_defaults),
    cmocka_unit_test(test_unit_http_auth_init_null),
    cmocka_unit_test(test_unit_http_auth_verify_credentials_success),
    cmocka_unit_test(test_unit_http_auth_verify_credentials_failure),
    cmocka_unit_test(test_unit_http_auth_parse_basic_credentials_success),
    cmocka_unit_test(test_unit_http_auth_parse_basic_credentials_invalid_scheme),
    cmocka_unit_test(test_unit_http_auth_parse_basic_credentials_decode_failure),
    cmocka_unit_test(test_unit_http_auth_parse_basic_credentials_missing_delimiter),
    cmocka_unit_test(test_unit_http_auth_generate_challenge_success),
    cmocka_unit_test(test_unit_http_auth_generate_challenge_invalid),
    cmocka_unit_test(test_unit_http_auth_validate_basic_disabled),
    cmocka_unit_test(test_unit_http_auth_validate_basic_missing_header),
    cmocka_unit_test(test_unit_http_auth_validate_basic_invalid_credentials),
    cmocka_unit_test(test_unit_http_auth_validate_basic_success),
    cmocka_unit_test(test_unit_http_auth_validate_basic_parse_failure),
    cmocka_unit_test(test_unit_http_auth_create_401_response),
    cmocka_unit_test(test_unit_http_auth_create_401_response_invalid_realm),

    // HTTP metrics tests
    cmocka_unit_test(test_unit_http_metrics_init_cleanup),
    cmocka_unit_test(test_unit_http_metrics_recording_accuracy),
    cmocka_unit_test(test_unit_http_metrics_null_handling),
    cmocka_unit_test(test_unit_http_metrics_connection_updates),
    cmocka_unit_test(test_unit_http_metrics_concurrency),
    cmocka_unit_test(test_unit_http_metrics_cpu_overhead),
    cmocka_unit_test(test_unit_http_metrics_retrieval_performance),
    cmocka_unit_test(test_unit_http_metrics_realistic_patterns),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nNetworking Test Summary\n");
  printf("=======================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}