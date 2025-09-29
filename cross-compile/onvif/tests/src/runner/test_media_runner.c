/**
 * @file test_media_runner.c
 * @brief Test runner for Media service tests
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations from test_media_utils.c
void test_unit_media_profile_functions(void** state);
void test_unit_media_video_source_functions(void** state);
void test_unit_media_audio_source_functions(void** state);
void test_unit_media_video_configuration_functions(void** state);
void test_unit_media_audio_configuration_functions(void** state);
void test_unit_media_stream_uri_functions(void** state);
void test_unit_media_snapshot_uri_functions(void** state);
void test_unit_media_multicast_functions(void** state);
void test_unit_media_metadata_functions(void** state);

// Forward declarations from test_onvif_media_callbacks.c
void test_unit_media_callback_registration_success(void** state);
void test_unit_media_callback_registration_duplicate(void** state);
void test_unit_media_callback_registration_null_config(void** state);
void test_unit_media_callback_registration_dispatcher_failure(void** state);
void test_unit_media_callback_double_initialization(void** state);
void test_unit_media_callback_unregistration_success(void** state);
void test_unit_media_callback_unregistration_not_initialized(void** state);
void test_unit_media_callback_unregistration_failure(void** state);
void test_unit_media_callback_dispatch_success(void** state);
void test_unit_media_callback_dispatch_not_initialized(void** state);
void test_unit_media_callback_dispatch_null_params(void** state);

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
 * @brief Main test runner for Media tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Media Service Tests\n");
  printf("=========================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // Media utility tests
    cmocka_unit_test(test_unit_media_profile_functions),
    cmocka_unit_test(test_unit_media_video_source_functions),
    cmocka_unit_test(test_unit_media_audio_source_functions),
    cmocka_unit_test(test_unit_media_video_configuration_functions),
    cmocka_unit_test(test_unit_media_audio_configuration_functions),
    cmocka_unit_test(test_unit_media_stream_uri_functions),
    cmocka_unit_test(test_unit_media_snapshot_uri_functions),
    cmocka_unit_test(test_unit_media_multicast_functions),
    cmocka_unit_test(test_unit_media_metadata_functions),

    // Media callback tests
    cmocka_unit_test(test_unit_media_callback_registration_success),
    cmocka_unit_test(test_unit_media_callback_registration_duplicate),
    cmocka_unit_test(test_unit_media_callback_registration_null_config),
    cmocka_unit_test(test_unit_media_callback_registration_dispatcher_failure),
    cmocka_unit_test(test_unit_media_callback_double_initialization),
    cmocka_unit_test(test_unit_media_callback_unregistration_success),
    cmocka_unit_test(test_unit_media_callback_unregistration_not_initialized),
    cmocka_unit_test(test_unit_media_callback_unregistration_failure),
    cmocka_unit_test(test_unit_media_callback_dispatch_success),
    cmocka_unit_test(test_unit_media_callback_dispatch_not_initialized),
    cmocka_unit_test(test_unit_media_callback_dispatch_null_params),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nMedia Service Test Summary\n");
  printf("==========================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}