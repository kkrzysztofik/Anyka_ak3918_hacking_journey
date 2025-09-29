/**
 * @file test_utils_runner.c
 * @brief Test runner for utility functions (memory, logging, basic tests)
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations for test_memory_utils.c
void test_unit_memory_manager_init(void** state);
void test_unit_memory_manager_alloc(void** state);
void test_unit_memory_manager_free(void** state);
void test_unit_smart_response_builder(void** state);
void test_unit_memory_manager_stats(void** state);
void test_unit_memory_manager_stress(void** state);
void test_unit_dynamic_buffer(void** state);

// Forward declarations for test_logging_utils.c
void test_unit_logging_init(void** state);
void test_unit_logging_cleanup(void** state);
void test_unit_log_level(void** state);
void test_unit_basic_logging(void** state);
void test_unit_service_logging(void** state);
void test_unit_platform_logging(void** state);

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
 * @brief Main test runner for utility tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Utility Tests\n");
  printf("===================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // Memory utility tests
    cmocka_unit_test(test_unit_memory_manager_init),
    cmocka_unit_test(test_unit_memory_manager_alloc),
    cmocka_unit_test(test_unit_memory_manager_free),
    cmocka_unit_test(test_unit_smart_response_builder),
    cmocka_unit_test(test_unit_memory_manager_stats),
    cmocka_unit_test(test_unit_memory_manager_stress),
    cmocka_unit_test(test_unit_dynamic_buffer),

    // Logging utility tests
    cmocka_unit_test(test_unit_logging_init),
    cmocka_unit_test(test_unit_logging_cleanup),
    cmocka_unit_test(test_unit_log_level),
    cmocka_unit_test(test_unit_basic_logging),
    cmocka_unit_test(test_unit_service_logging),
    cmocka_unit_test(test_unit_platform_logging),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nUtility Test Summary\n");
  printf("====================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}
