/**
 * @file test_ptz_runner.c
 * @brief Test runner for PTZ service tests
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations from test_ptz_service.c
void test_unit_ptz_get_nodes_null_params(void** state);
void test_unit_ptz_get_node_null_params(void** state);
void test_unit_ptz_get_configuration_null_params(void** state);
void test_unit_ptz_get_status_null_params(void** state);
void test_unit_ptz_absolute_move_null_params(void** state);
void test_unit_ptz_get_presets_null_params(void** state);
void test_unit_ptz_get_nodes_success(void** state);
void test_unit_ptz_get_node_success(void** state);

// Forward declarations from test_onvif_ptz_callbacks.c
void test_unit_ptz_service_registration_success(void** state);
void test_unit_ptz_service_registration_duplicate(void** state);
void test_unit_ptz_service_registration_invalid_params(void** state);
void test_unit_ptz_service_registration_dispatcher_failure(void** state);
void test_unit_ptz_service_unregistration_success(void** state);
void test_unit_ptz_service_unregistration_not_found(void** state);
void test_unit_ptz_operation_handler_success(void** state);
void test_unit_ptz_operation_handler_null_operation(void** state);
void test_unit_ptz_operation_handler_null_request(void** state);
void test_unit_ptz_operation_handler_null_response(void** state);
void test_unit_ptz_operation_handler_unknown_operation(void** state);
void test_unit_ptz_service_registration_failure_handling(void** state);
void test_unit_ptz_service_unregistration_failure_handling(void** state);
void test_unit_ptz_service_callback_logging_failure(void** state);

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
 * @brief Main test runner for PTZ tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF PTZ Service Tests\n");
  printf("=======================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // PTZ service unit tests (refactored with common patterns)
    cmocka_unit_test(test_unit_ptz_get_nodes_null_params),
    cmocka_unit_test(test_unit_ptz_get_node_null_params),
    cmocka_unit_test(test_unit_ptz_get_configuration_null_params),
    cmocka_unit_test(test_unit_ptz_get_status_null_params),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_params),
    cmocka_unit_test(test_unit_ptz_get_presets_null_params),
    cmocka_unit_test(test_unit_ptz_get_nodes_success),
    cmocka_unit_test(test_unit_ptz_get_node_success),

    // PTZ callback tests
    cmocka_unit_test(test_unit_ptz_service_registration_success),
    cmocka_unit_test(test_unit_ptz_service_registration_duplicate),
    cmocka_unit_test(test_unit_ptz_service_registration_invalid_params),
    cmocka_unit_test(test_unit_ptz_service_registration_dispatcher_failure),
    cmocka_unit_test(test_unit_ptz_service_unregistration_success),
    cmocka_unit_test(test_unit_ptz_service_unregistration_not_found),
    cmocka_unit_test(test_unit_ptz_operation_handler_success),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_operation),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_request),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_response),
    cmocka_unit_test(test_unit_ptz_operation_handler_unknown_operation),
    cmocka_unit_test(test_unit_ptz_service_registration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_unregistration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_callback_logging_failure),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nPTZ Service Test Summary\n");
  printf("========================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}
