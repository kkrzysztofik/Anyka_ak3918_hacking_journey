/**
 * @file test_protocol_runner.c
 * @brief Test runner for protocol tests (gSOAP)
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cmocka_wrapper.h"

// Forward declarations for test_onvif_gsoap.c - Core context tests
void test_unit_onvif_gsoap_init(void** state);
void test_unit_onvif_gsoap_init_null(void** state);
void test_unit_onvif_gsoap_cleanup(void** state);

// Media service parsing tests
void test_unit_onvif_gsoap_parse_get_profiles(void** state);
void test_unit_onvif_gsoap_parse_get_stream_uri(void** state);
void test_unit_onvif_gsoap_parse_create_profile(void** state);
void test_unit_onvif_gsoap_parse_delete_profile(void** state);
void test_unit_onvif_gsoap_parse_set_video_source_config(void** state);
void test_unit_onvif_gsoap_parse_set_video_encoder_config(void** state);

// PTZ service parsing tests
void test_unit_onvif_gsoap_parse_get_nodes(void** state);
void test_unit_onvif_gsoap_parse_absolute_move(void** state);
void test_unit_onvif_gsoap_parse_absolute_move_no_speed(void** state);
void test_unit_onvif_gsoap_parse_get_presets(void** state);
void test_unit_onvif_gsoap_parse_set_preset(void** state);
void test_unit_onvif_gsoap_parse_goto_preset(void** state);
void test_unit_onvif_gsoap_parse_remove_preset(void** state);

// Device service parsing tests
void test_unit_onvif_gsoap_parse_get_device_information(void** state);
void test_unit_onvif_gsoap_parse_get_capabilities(void** state);
void test_unit_onvif_gsoap_parse_get_system_date_and_time(void** state);
void test_unit_onvif_gsoap_parse_system_reboot(void** state);

// Imaging service parsing tests
void test_unit_onvif_gsoap_parse_get_imaging_settings(void** state);
void test_unit_onvif_gsoap_parse_set_imaging_settings(void** state);

// Error handling tests
void test_unit_onvif_gsoap_parse_invalid_xml(void** state);
void test_unit_onvif_gsoap_parse_invalid_namespace(void** state);
void test_unit_onvif_gsoap_parse_missing_required_param(void** state);
void test_unit_onvif_gsoap_parse_without_initialization(void** state);

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
 * @brief Main test runner for protocol tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Protocol Tests (gSOAP Parsing)\n");
  printf("====================================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // Core context tests
    cmocka_unit_test(test_unit_onvif_gsoap_init),
    cmocka_unit_test(test_unit_onvif_gsoap_init_null),
    cmocka_unit_test(test_unit_onvif_gsoap_cleanup),

    // Media service parsing tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_profiles),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_stream_uri),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_create_profile),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_delete_profile),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_video_source_config),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_video_encoder_config),

    // PTZ service parsing tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_nodes),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_absolute_move),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_absolute_move_no_speed),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_presets),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_preset),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_goto_preset),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_remove_preset),

    // Device service parsing tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_device_information),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_capabilities),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_system_date_and_time),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_system_reboot),

    // Imaging service parsing tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_imaging_settings),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_imaging_settings),

    // Error handling tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_invalid_xml),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_invalid_namespace),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_missing_required_param),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_without_initialization),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nProtocol Test Summary\n");
  printf("=====================\n");
  printf("Tests Run: %d\n", test_count);
  printf("Duration: %.2f seconds\n", test_duration);

  if (failures == 0) {
    printf("✅ All %d test(s) passed!\n", test_count);
  } else {
    printf("❌ %d test(s) failed!\n", failures);
  }

  return failures;
}
