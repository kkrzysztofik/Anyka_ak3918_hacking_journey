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

// Forward declarations for test_onvif_gsoap.c
void test_unit_onvif_gsoap_init(void** state);
void test_unit_onvif_gsoap_init_null(void** state);
void test_unit_onvif_gsoap_cleanup(void** state);
void test_unit_onvif_gsoap_reset(void** state);
void test_unit_onvif_gsoap_generate_fault_response(void** state);
void test_unit_onvif_gsoap_generate_device_info_response(void** state);
void test_unit_onvif_gsoap_get_response_data(void** state);
void test_unit_onvif_gsoap_get_response_length(void** state);
void test_unit_onvif_gsoap_has_error(void** state);
void test_unit_onvif_gsoap_get_error(void** state);
void test_unit_onvif_gsoap_validate_response(void** state);
void test_unit_onvif_gsoap_extract_operation_name(void** state);
void test_unit_onvif_gsoap_init_request_parsing(void** state);
void test_unit_onvif_gsoap_parse_profile_token(void** state);
void test_unit_onvif_gsoap_parse_configuration_token(void** state);
void test_unit_onvif_gsoap_parse_protocol(void** state);
void test_unit_onvif_gsoap_parse_value(void** state);
void test_unit_onvif_gsoap_parse_boolean(void** state);
void test_unit_onvif_gsoap_parse_integer(void** state);
void test_unit_onvif_gsoap_generate_response_with_callback(void** state);
void test_unit_onvif_gsoap_generate_profiles_response(void** state);
void test_unit_onvif_gsoap_generate_stream_uri_response(void** state);
void test_unit_onvif_gsoap_generate_create_profile_response(void** state);
void test_unit_onvif_gsoap_generate_delete_profile_response(void** state);
void test_unit_onvif_gsoap_generate_get_nodes_response(void** state);
void test_unit_onvif_gsoap_generate_absolute_move_response(void** state);
void test_unit_onvif_gsoap_generate_get_presets_response(void** state);
void test_unit_onvif_gsoap_generate_set_preset_response(void** state);
void test_unit_onvif_gsoap_generate_goto_preset_response(void** state);

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

  printf("ONVIF Protocol Tests (gSOAP)\n");
  printf("============================\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_onvif_gsoap_init),
    cmocka_unit_test(test_unit_onvif_gsoap_init_null),
    cmocka_unit_test(test_unit_onvif_gsoap_cleanup),
    cmocka_unit_test(test_unit_onvif_gsoap_reset),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_fault_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_device_info_response),
    cmocka_unit_test(test_unit_onvif_gsoap_get_response_data),
    cmocka_unit_test(test_unit_onvif_gsoap_get_response_length),
    cmocka_unit_test(test_unit_onvif_gsoap_has_error),
    cmocka_unit_test(test_unit_onvif_gsoap_get_error),
    cmocka_unit_test(test_unit_onvif_gsoap_validate_response),
    cmocka_unit_test(test_unit_onvif_gsoap_extract_operation_name),
    cmocka_unit_test(test_unit_onvif_gsoap_init_request_parsing),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_profile_token),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_configuration_token),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_protocol),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_value),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_boolean),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_integer),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_response_with_callback),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_profiles_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_stream_uri_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_create_profile_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_delete_profile_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_get_nodes_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_absolute_move_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_get_presets_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_set_preset_response),
    cmocka_unit_test(test_unit_onvif_gsoap_generate_goto_preset_response),
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