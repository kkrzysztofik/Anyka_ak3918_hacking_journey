/**
 * @file test_service_dispatcher_runner.c
 * @brief Test runner for service dispatcher tests
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations for test_service_dispatcher.c
void test_unit_service_dispatcher_init(void** state);
void test_unit_service_dispatcher_cleanup(void** state);
void test_unit_service_dispatcher_register_service(void** state);
void test_unit_service_dispatcher_register_service_null_params(void** state);
void test_unit_service_dispatcher_register_service_invalid_params(void** state);
void test_unit_service_dispatcher_register_service_duplicate(void** state);
void test_unit_service_dispatcher_register_service_registry_full(void** state);
void test_unit_service_dispatcher_unregister_service(void** state);
void test_unit_service_dispatcher_unregister_service_not_found(void** state);
void test_unit_service_dispatcher_dispatch(void** state);
void test_unit_service_dispatcher_dispatch_invalid_params(void** state);
void test_unit_service_dispatcher_dispatch_service_not_found(void** state);
void test_unit_service_dispatcher_is_registered(void** state);
void test_unit_service_dispatcher_get_services(void** state);
void test_unit_service_dispatcher_init_cleanup_handlers(void** state);

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
 * @brief Main test runner for service dispatcher tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Service Dispatcher Tests\n");
  printf("===============================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_service_dispatcher_init),
    cmocka_unit_test(test_unit_service_dispatcher_cleanup),
    cmocka_unit_test(test_unit_service_dispatcher_register_service),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_null_params),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_invalid_params),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_duplicate),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_registry_full),
    cmocka_unit_test(test_unit_service_dispatcher_unregister_service),
    cmocka_unit_test(test_unit_service_dispatcher_unregister_service_not_found),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch_invalid_params),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch_service_not_found),
    cmocka_unit_test(test_unit_service_dispatcher_is_registered),
    cmocka_unit_test(test_unit_service_dispatcher_get_services),
    cmocka_unit_test(test_unit_service_dispatcher_init_cleanup_handlers),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nService Dispatcher Test Summary\n");
  printf("================================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}