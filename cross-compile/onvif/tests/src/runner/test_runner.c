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

// Forward declarations of test functions from test_ptz_service.c
void test_ptz_get_nodes_success(void** state);
void test_ptz_get_nodes_null_nodes(void** state);
void test_ptz_get_nodes_null_count(void** state);
void test_ptz_get_node_success(void** state);
void test_ptz_get_node_invalid_token(void** state);
void test_ptz_get_node_null_token(void** state);
void test_ptz_get_node_null_node(void** state);
void test_ptz_get_configuration_success(void** state);
void test_ptz_get_configuration_null_token(void** state);
void test_ptz_get_configuration_null_config(void** state);
void test_ptz_get_status_success(void** state);
void test_ptz_get_status_null_token(void** state);
void test_ptz_get_status_null_status(void** state);
void test_ptz_absolute_move_success(void** state);
void test_ptz_absolute_move_null_token(void** state);
void test_ptz_absolute_move_null_position(void** state);
void test_ptz_absolute_move_null_speed(void** state);
void test_ptz_relative_move_success(void** state);
void test_ptz_relative_move_null_token(void** state);
void test_ptz_relative_move_null_translation(void** state);
void test_ptz_continuous_move_success(void** state);
void test_ptz_continuous_move_null_token(void** state);
void test_ptz_continuous_move_null_velocity(void** state);
void test_ptz_stop_success(void** state);
void test_ptz_stop_null_token(void** state);
void test_ptz_goto_home_position_success(void** state);
void test_ptz_goto_home_position_null_token(void** state);
void test_ptz_set_home_position_success(void** state);
void test_ptz_set_home_position_null_token(void** state);
void test_ptz_get_presets_success(void** state);
void test_ptz_get_presets_null_token(void** state);
void test_ptz_get_presets_null_list(void** state);
void test_ptz_get_presets_null_count(void** state);
void test_ptz_set_preset_success(void** state);
void test_ptz_set_preset_null_token(void** state);
void test_ptz_set_preset_null_name(void** state);
void test_ptz_set_preset_null_output_token(void** state);
void test_ptz_goto_preset_success(void** state);
void test_ptz_goto_preset_invalid_token(void** state);
void test_ptz_goto_preset_null_token(void** state);
void test_ptz_goto_preset_null_preset_token(void** state);
void test_ptz_remove_preset_success(void** state);
void test_ptz_remove_preset_invalid_token(void** state);
void test_ptz_remove_preset_null_token(void** state);
void test_ptz_remove_preset_null_preset_token(void** state);
void test_ptz_adapter_init_success(void** state);
void test_ptz_adapter_init_failure(void** state);
void test_ptz_adapter_shutdown_success(void** state);
void test_ptz_adapter_get_status_success(void** state);
void test_ptz_adapter_get_status_null_status(void** state);
void test_ptz_adapter_get_status_not_initialized(void** state);
void test_ptz_adapter_absolute_move_success(void** state);
void test_ptz_adapter_absolute_move_not_initialized(void** state);
void test_ptz_adapter_relative_move_success(void** state);
void test_ptz_adapter_relative_move_not_initialized(void** state);
void test_ptz_adapter_continuous_move_success(void** state);
void test_ptz_adapter_continuous_move_not_initialized(void** state);
void test_ptz_adapter_stop_success(void** state);
void test_ptz_adapter_stop_not_initialized(void** state);
void test_ptz_adapter_set_preset_success(void** state);
void test_ptz_adapter_set_preset_not_initialized(void** state);
void test_ptz_adapter_goto_preset_success(void** state);
void test_ptz_adapter_goto_preset_not_initialized(void** state);
void test_ptz_service_init_success(void** state);
void test_ptz_service_cleanup_success(void** state);
void test_ptz_handle_operation_success(void** state);
void test_ptz_handle_operation_null_operation(void** state);
void test_ptz_handle_operation_null_request(void** state);
void test_ptz_handle_operation_null_response(void** state);
void test_ptz_handle_operation_unknown_operation(void** state);

// Forward declarations of test functions from test_media_utils.c
void test_media_basic_functions(void** state);
void test_media_video_sources(void** state);
void test_media_audio_sources(void** state);
void test_media_video_configurations(void** state);
void test_media_audio_configurations(void** state);
void test_media_metadata_configurations(void** state);
void test_media_error_handling(void** state);
void test_media_initialization(void** state);

// Forward declarations of test functions from test_onvif_media_callbacks.c
void test_media_callback_registration_success(void** state);
void test_media_callback_registration_duplicate(void** state);
void test_media_callback_registration_null_config(void** state);
void test_media_callback_registration_dispatcher_failure(void** state);
void test_media_callback_double_initialization(void** state);
void test_media_callback_unregistration_success(void** state);
void test_media_callback_unregistration_not_initialized(void** state);
void test_media_callback_unregistration_failure(void** state);
void test_media_callback_dispatch_success(void** state);
void test_media_callback_dispatch_not_initialized(void** state);
void test_media_callback_dispatch_null_params(void** state);

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

    // PTZ service tests (commented out - tests not implemented yet)
    // All PTZ tests are commented out until the test file is created

    // Media service tests
    cmocka_unit_test(test_media_basic_functions),
    cmocka_unit_test(test_media_video_sources),
    cmocka_unit_test(test_media_audio_sources),
    cmocka_unit_test(test_media_video_configurations),
    cmocka_unit_test(test_media_audio_configurations),
    cmocka_unit_test(test_media_metadata_configurations),
    cmocka_unit_test(test_media_error_handling),
    cmocka_unit_test(test_media_initialization),

    // Media service callback tests
    cmocka_unit_test(test_media_callback_registration_success),
    cmocka_unit_test(test_media_callback_registration_duplicate),
    cmocka_unit_test(test_media_callback_registration_null_config),
    cmocka_unit_test(test_media_callback_registration_dispatcher_failure),
    cmocka_unit_test(test_media_callback_double_initialization),
    cmocka_unit_test(test_media_callback_unregistration_success),
    cmocka_unit_test(test_media_callback_unregistration_not_initialized),
    cmocka_unit_test(test_media_callback_unregistration_failure),
    cmocka_unit_test(test_media_callback_dispatch_success),
    cmocka_unit_test(test_media_callback_dispatch_not_initialized),
    cmocka_unit_test(test_media_callback_dispatch_null_params),
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
