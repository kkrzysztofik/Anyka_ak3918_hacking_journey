/**
 * @file test_runner.c
 * @brief Main test runner for ONVIF project unit tests
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>

#include "cmocka_wrapper.h"

// Forward declarations of test functions from test_simple.c
void test_basic_pass(void** state);
void test_basic_string(void** state);
void test_basic_memory(void** state);

// Forward declarations of test functions from test_memory_utils.c
void test_memory_manager_init(void** state);
void test_memory_manager_alloc(void** state);
void test_memory_manager_free(void** state);
void test_smart_response_builder(void** state);
void test_memory_manager_stats(void** state);
void test_memory_manager_stress(void** state);
void test_dynamic_buffer(void** state);

// Forward declarations of test functions from test_onvif_gsoap.c
void test_onvif_gsoap_init(void** state);
void test_onvif_gsoap_init_null(void** state);
void test_onvif_gsoap_cleanup(void** state);
void test_onvif_gsoap_reset(void** state);
void test_onvif_gsoap_generate_fault_response(void** state);
void test_onvif_gsoap_generate_device_info_response(void** state);
void test_onvif_gsoap_get_response_data(void** state);
void test_onvif_gsoap_get_response_length(void** state);
void test_onvif_gsoap_has_error(void** state);
void test_onvif_gsoap_get_error(void** state);
void test_onvif_gsoap_validate_response(void** state);
void test_onvif_gsoap_extract_operation_name(void** state);

// Forward declarations of test functions from test_logging_utils.c
void test_logging_init(void** state);
void test_logging_cleanup(void** state);
void test_log_level(void** state);
void test_basic_logging(void** state);
void test_service_logging(void** state);
void test_platform_logging(void** state);

// Forward declarations of test functions from test_http_auth.c
void test_http_auth_init_sets_defaults(void** state);
void test_http_auth_init_null(void** state);
void test_http_auth_verify_credentials_success(void** state);
void test_http_auth_verify_credentials_failure(void** state);
void test_http_auth_parse_basic_credentials_success(void** state);
void test_http_auth_parse_basic_credentials_invalid_scheme(void** state);
void test_http_auth_parse_basic_credentials_decode_failure(void** state);
void test_http_auth_parse_basic_credentials_missing_delimiter(void** state);
void test_http_auth_generate_challenge_success(void** state);
void test_http_auth_generate_challenge_invalid(void** state);
void test_http_auth_validate_basic_disabled(void** state);
void test_http_auth_validate_basic_missing_header(void** state);
void test_http_auth_validate_basic_invalid_credentials(void** state);
void test_http_auth_validate_basic_success(void** state);
void test_http_auth_validate_basic_parse_failure(void** state);
void test_http_auth_create_401_response(void** state);
void test_http_auth_create_401_response_invalid_realm(void** state);

// Forward declarations of test functions from test_service_dispatcher.c
void test_service_dispatcher_init(void** state);
void test_service_dispatcher_cleanup(void** state);
void test_service_dispatcher_register_service(void** state);
void test_service_dispatcher_register_service_null_params(void** state);
void test_service_dispatcher_register_service_invalid_params(void** state);
void test_service_dispatcher_register_service_duplicate(void** state);
void test_service_dispatcher_register_service_registry_full(void** state);
void test_service_dispatcher_unregister_service(void** state);
void test_service_dispatcher_unregister_service_not_found(void** state);
void test_service_dispatcher_dispatch(void** state);
void test_service_dispatcher_dispatch_invalid_params(void** state);
void test_service_dispatcher_dispatch_service_not_found(void** state);
void test_service_dispatcher_is_registered(void** state);
void test_service_dispatcher_get_services(void** state);
void test_service_dispatcher_init_cleanup_handlers(void** state);

/**
 * @brief Global test setup
 * @param state Test state
 * @return 0 on success
 */
static int setup_global_tests(void** state) {
  (void)state;
  printf("Setting up global test environment...\n");
  return 0;
}

/**
 * @brief Global test teardown
 * @param state Test state
 * @return 0 on success
 */
static int teardown_global_tests(void** state) {
  (void)state;
  printf("Cleaning up global test environment...\n");
  return 0;
}

/**
 * @brief Main test runner
 * @return Number of test failures
 */
int main(void) {
  printf("ONVIF Project Unit Test Suite\n");
  printf("=============================\n\n");

  // Collect working unit tests
  const struct CMUnitTest tests[] = {
    // Basic framework tests
    cmocka_unit_test(test_basic_pass),
    cmocka_unit_test(test_basic_string),
    cmocka_unit_test(test_basic_memory),

    // Memory utility tests
    cmocka_unit_test(test_memory_manager_init),
    cmocka_unit_test(test_memory_manager_alloc),
    cmocka_unit_test(test_memory_manager_free),
    cmocka_unit_test(test_smart_response_builder),
    cmocka_unit_test(test_memory_manager_stats),
    cmocka_unit_test(test_memory_manager_stress),
    cmocka_unit_test(test_dynamic_buffer),

    // Logging utility tests
    cmocka_unit_test(test_logging_init),
    cmocka_unit_test(test_logging_cleanup),
    cmocka_unit_test(test_log_level),
    cmocka_unit_test(test_basic_logging),
    cmocka_unit_test(test_service_logging),
    cmocka_unit_test(test_platform_logging),

    // HTTP authentication tests
    cmocka_unit_test(test_http_auth_init_sets_defaults),
    cmocka_unit_test(test_http_auth_init_null),
    cmocka_unit_test(test_http_auth_verify_credentials_success),
    cmocka_unit_test(test_http_auth_verify_credentials_failure),
    cmocka_unit_test(test_http_auth_parse_basic_credentials_success),
    cmocka_unit_test(test_http_auth_parse_basic_credentials_invalid_scheme),
    cmocka_unit_test(test_http_auth_parse_basic_credentials_decode_failure),
    cmocka_unit_test(test_http_auth_parse_basic_credentials_missing_delimiter),
    cmocka_unit_test(test_http_auth_generate_challenge_success),
    cmocka_unit_test(test_http_auth_generate_challenge_invalid),
    cmocka_unit_test(test_http_auth_validate_basic_disabled),
    cmocka_unit_test(test_http_auth_validate_basic_missing_header),
    cmocka_unit_test(test_http_auth_validate_basic_invalid_credentials),
    cmocka_unit_test(test_http_auth_validate_basic_success),
    cmocka_unit_test(test_http_auth_validate_basic_parse_failure),
    cmocka_unit_test(test_http_auth_create_401_response),
    cmocka_unit_test(test_http_auth_create_401_response_invalid_realm),

    // ONVIF gSOAP tests
    cmocka_unit_test(test_onvif_gsoap_init),
    cmocka_unit_test(test_onvif_gsoap_init_null),
    cmocka_unit_test(test_onvif_gsoap_cleanup),
    cmocka_unit_test(test_onvif_gsoap_reset),
    cmocka_unit_test(test_onvif_gsoap_generate_fault_response),
    cmocka_unit_test(test_onvif_gsoap_generate_device_info_response),
    cmocka_unit_test(test_onvif_gsoap_get_response_data),
    cmocka_unit_test(test_onvif_gsoap_get_response_length),
    cmocka_unit_test(test_onvif_gsoap_has_error),
    cmocka_unit_test(test_onvif_gsoap_get_error),
    cmocka_unit_test(test_onvif_gsoap_validate_response),
    cmocka_unit_test(test_onvif_gsoap_extract_operation_name),

    // Service dispatcher tests
    cmocka_unit_test(test_service_dispatcher_init),
    cmocka_unit_test(test_service_dispatcher_cleanup),
    cmocka_unit_test(test_service_dispatcher_register_service),
    cmocka_unit_test(test_service_dispatcher_register_service_null_params),
    cmocka_unit_test(test_service_dispatcher_register_service_invalid_params),
    cmocka_unit_test(test_service_dispatcher_register_service_duplicate),
    cmocka_unit_test(test_service_dispatcher_register_service_registry_full),
    cmocka_unit_test(test_service_dispatcher_unregister_service),
    cmocka_unit_test(test_service_dispatcher_unregister_service_not_found),
    cmocka_unit_test(test_service_dispatcher_dispatch),
    cmocka_unit_test(test_service_dispatcher_dispatch_invalid_params),
    cmocka_unit_test(test_service_dispatcher_dispatch_service_not_found),
    cmocka_unit_test(test_service_dispatcher_is_registered),
    cmocka_unit_test(test_service_dispatcher_get_services),
    cmocka_unit_test(test_service_dispatcher_init_cleanup_handlers),
  };

  int failures = 0;

  // Run all working unit tests
  printf("Running Unit Tests...\n");
  printf("--------------------\n");
  failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);
  printf("\n");

  // Print summary
  printf("Unit Test Summary\n");
  printf("=================\n");
  if (failures == 0) {
    printf("✅ All unit tests passed!\n");
  } else {
    printf("❌ %d unit test(s) failed!\n", failures);
  }

  return failures;
}
