/**
 * @file test_runner.c
 * @brief Main test runner for ONVIF project unit tests
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// #include "../integration/media_service_tests.h"
// #include "../integration/ptz_service_tests.h"
#include "cmocka_wrapper.h"

// Test mode enumeration
typedef enum { TEST_MODE_ALL, TEST_MODE_UNIT_ONLY, TEST_MODE_INTEGRATION_ONLY } test_mode_t;

// Test options structure
typedef struct {
  test_mode_t mode;
  int verbose;
  int help;
} test_options_t;

// Forward declarations of test functions from test_simple.c
void test_unit_basic_pass(void** state);
void test_unit_basic_string(void** state);
void test_unit_basic_memory(void** state);

// Forward declarations of test functions from test_memory_utils.c
void test_unit_memory_manager_init(void** state);
void test_unit_memory_manager_alloc(void** state);
void test_unit_memory_manager_free(void** state);
void test_unit_smart_response_builder(void** state);
void test_unit_memory_manager_stats(void** state);
void test_unit_memory_manager_stress(void** state);
void test_unit_dynamic_buffer(void** state);

// Forward declarations of test functions from test_onvif_gsoap.c
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

// Forward declarations of test functions from test_logging_utils.c
void test_unit_logging_init(void** state);
void test_unit_logging_cleanup(void** state);
void test_unit_log_level(void** state);
void test_unit_basic_logging(void** state);
void test_unit_service_logging(void** state);
void test_unit_platform_logging(void** state);

// Forward declarations of test functions from test_http_auth.c
void test_unit_http_auth_init_sets_defaults(void** state);
void test_unit_http_auth_init_null(void** state);
void test_unit_http_auth_verify_credentials_success(void** state);

// Forward declarations of test functions from test_http_metrics.c
void test_unit_http_metrics_init_cleanup(void** state);
void test_unit_http_metrics_recording_accuracy(void** state);
void test_unit_http_metrics_null_handling(void** state);
void test_unit_http_metrics_connection_updates(void** state);
void test_unit_http_metrics_concurrency(void** state);
void test_unit_http_metrics_cpu_overhead(void** state);
void test_unit_http_metrics_retrieval_performance(void** state);
void test_unit_http_metrics_realistic_patterns(void** state);
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

// Forward declarations of test functions from test_service_dispatcher.c
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

// Forward declarations of test functions from test_ptz_service.c
void test_unit_ptz_get_nodes_success(void** state);
void test_unit_ptz_get_nodes_null_nodes(void** state);
void test_unit_ptz_get_nodes_null_count(void** state);
void test_unit_ptz_get_node_success(void** state);
void test_unit_ptz_get_node_invalid_token(void** state);
void test_unit_ptz_get_node_null_token(void** state);
void test_unit_ptz_get_node_null_node(void** state);
void test_unit_ptz_get_configuration_success(void** state);
void test_unit_ptz_get_configuration_null_token(void** state);
void test_unit_ptz_get_configuration_null_config(void** state);
void test_unit_ptz_get_status_success(void** state);
void test_unit_ptz_get_status_null_token(void** state);
void test_unit_ptz_get_status_null_status(void** state);
void test_unit_ptz_absolute_move_success(void** state);
void test_unit_ptz_absolute_move_null_token(void** state);
void test_unit_ptz_absolute_move_null_position(void** state);
void test_unit_ptz_absolute_move_null_speed(void** state);
void test_unit_ptz_relative_move_success(void** state);
void test_unit_ptz_relative_move_null_token(void** state);
void test_unit_ptz_relative_move_null_translation(void** state);
void test_unit_ptz_continuous_move_success(void** state);
void test_unit_ptz_continuous_move_null_token(void** state);
void test_unit_ptz_continuous_move_null_velocity(void** state);
void test_unit_ptz_stop_success(void** state);
void test_unit_ptz_stop_null_token(void** state);
void test_unit_ptz_goto_home_position_success(void** state);
void test_unit_ptz_goto_home_position_null_token(void** state);
void test_unit_ptz_set_home_position_success(void** state);
void test_unit_ptz_set_home_position_null_token(void** state);
void test_unit_ptz_get_presets_success(void** state);
void test_unit_ptz_get_presets_null_token(void** state);
void test_unit_ptz_get_presets_null_list(void** state);
void test_unit_ptz_get_presets_null_count(void** state);
void test_unit_ptz_set_preset_success(void** state);
void test_unit_ptz_set_preset_null_token(void** state);
void test_unit_ptz_set_preset_null_name(void** state);
void test_unit_ptz_set_preset_null_output_token(void** state);
void test_unit_ptz_goto_preset_success(void** state);
void test_unit_ptz_goto_preset_invalid_token(void** state);
void test_unit_ptz_goto_preset_null_token(void** state);
void test_unit_ptz_goto_preset_null_preset_token(void** state);
void test_unit_ptz_remove_preset_success(void** state);
void test_unit_ptz_remove_preset_invalid_token(void** state);
void test_unit_ptz_remove_preset_null_token(void** state);
void test_unit_ptz_remove_preset_null_preset_token(void** state);
void test_unit_ptz_adapter_init_success(void** state);
void test_unit_ptz_adapter_init_failure(void** state);
void test_unit_ptz_adapter_shutdown_success(void** state);
void test_unit_ptz_adapter_get_status_success(void** state);
void test_unit_ptz_adapter_get_status_null_status(void** state);
void test_unit_ptz_adapter_get_status_not_initialized(void** state);
void test_unit_ptz_adapter_absolute_move_success(void** state);
void test_unit_ptz_adapter_absolute_move_not_initialized(void** state);
void test_unit_ptz_adapter_relative_move_success(void** state);
void test_unit_ptz_adapter_relative_move_not_initialized(void** state);
void test_unit_ptz_adapter_continuous_move_success(void** state);
void test_unit_ptz_adapter_continuous_move_not_initialized(void** state);
void test_unit_ptz_adapter_stop_success(void** state);
void test_unit_ptz_adapter_stop_not_initialized(void** state);
void test_unit_ptz_adapter_set_preset_success(void** state);
void test_unit_ptz_adapter_set_preset_not_initialized(void** state);
void test_unit_ptz_adapter_goto_preset_success(void** state);
void test_unit_ptz_adapter_goto_preset_not_initialized(void** state);
void test_unit_ptz_service_init_success(void** state);
void test_unit_ptz_service_cleanup_success(void** state);
void test_unit_ptz_handle_operation_success(void** state);
void test_unit_ptz_handle_operation_null_operation(void** state);
void test_unit_ptz_handle_operation_null_request(void** state);
void test_unit_ptz_handle_operation_null_response(void** state);
void test_unit_ptz_handle_operation_unknown_operation(void** state);

// Forward declarations of test functions from test_media_utils.c
void test_unit_media_profile_functions(void** state);
void test_unit_media_video_source_functions(void** state);
void test_unit_media_audio_source_functions(void** state);
void test_unit_media_video_configuration_functions(void** state);
void test_unit_media_audio_configuration_functions(void** state);
void test_unit_media_stream_uri_functions(void** state);
void test_unit_media_snapshot_uri_functions(void** state);
void test_unit_media_multicast_functions(void** state);
void test_unit_media_metadata_functions(void** state);

// Forward declarations of test functions from test_onvif_media_callbacks.c
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

// Forward declarations of test functions from test_onvif_imaging_callbacks.c
void test_unit_imaging_callback_registration_success(void** state);
void test_unit_imaging_callback_registration_duplicate(void** state);
void test_unit_imaging_callback_registration_null_config(void** state);
void test_unit_imaging_callback_registration_dispatcher_failure(void** state);
void test_unit_imaging_callback_double_initialization(void** state);
void test_unit_imaging_callback_unregistration_success(void** state);
void test_unit_imaging_callback_unregistration_not_initialized(void** state);
void test_unit_imaging_callback_unregistration_failure(void** state);
void test_unit_imaging_callback_dispatch_success(void** state);

// Forward declarations of test functions from test_onvif_ptz_callbacks.c
void test_unit_ptz_service_registration_success(void** state);
void test_unit_ptz_service_registration_duplicate(void** state);
void test_unit_ptz_service_registration_invalid_params(void** state);
void test_unit_ptz_service_unregistration_success(void** state);
void test_unit_ptz_service_unregistration_not_found(void** state);
void test_unit_ptz_service_dispatch_success(void** state);
void test_unit_ptz_service_dispatch_unknown_operation(void** state);
void test_unit_ptz_service_dispatch_null_service(void** state);
void test_unit_ptz_service_dispatch_null_operation(void** state);
void test_unit_ptz_service_dispatch_null_request(void** state);
void test_unit_ptz_service_dispatch_null_response(void** state);
void test_unit_ptz_operation_handler_success(void** state);
void test_unit_ptz_operation_handler_null_operation(void** state);
void test_unit_ptz_operation_handler_null_request(void** state);
void test_unit_ptz_operation_handler_null_response(void** state);
void test_unit_ptz_operation_handler_unknown_operation(void** state);
void test_unit_ptz_service_registration_failure_handling(void** state);
void test_unit_ptz_service_dispatch_failure_handling(void** state);
void test_unit_ptz_service_unregistration_failure_handling(void** state);
void test_unit_ptz_service_callback_logging_success(void** state);
void test_unit_ptz_service_callback_logging_failure(void** state);

// Command-line parsing functions
static void print_help(const char* program_name);
static void parse_arguments(int argc, char* argv[], test_options_t* options);

// Forward declarations of test functions from integration tests
// PTZ service optimization tests
void test_integration_ptz_absolute_move_functionality(void** state);
void test_integration_ptz_relative_move_functionality(void** state);
void test_integration_ptz_continuous_move_functionality(void** state);
void test_integration_ptz_stop_functionality(void** state);
void test_integration_ptz_preset_creation(void** state);
void test_integration_ptz_preset_retrieval(void** state);
void test_integration_ptz_preset_goto(void** state);
void test_integration_ptz_preset_removal(void** state);
void test_integration_ptz_preset_memory_optimization(void** state);
void test_integration_ptz_memory_usage_improvements(void** state);
void test_integration_ptz_buffer_pool_usage(void** state);
void test_integration_ptz_string_operations_optimization(void** state);
void test_integration_ptz_error_handling_robustness(void** state);
void test_integration_ptz_concurrent_operations(void** state);
void test_integration_ptz_stress_testing(void** state);
void test_integration_ptz_memory_leak_detection(void** state);

// Media service optimization tests
void test_integration_media_profile_operations(void** state);
void test_integration_stream_uri_generation_functionality(void** state);
void test_integration_optimized_profile_lookup_performance(void** state);
void test_integration_uri_caching_optimization(void** state);
void test_integration_memory_efficiency(void** state);
void test_integration_concurrent_stream_uri_access(void** state);
void test_integration_stress_test_optimization(void** state);

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
 * @brief Print help information
 * @param program_name Name of the program
 */
static void print_help(const char* program_name) {
  printf("ONVIF Project Comprehensive Test Suite\n");
  printf("======================================\n\n");
  printf("Usage: %s [OPTIONS]\n\n", program_name);
  printf("Options:\n");
  printf("  --unit-only          Run only unit tests\n");
  printf("  --integration-only   Run only integration tests\n");
  printf("  --verbose            Enable verbose output\n");
  printf("  --help, -h           Show this help message\n");
  printf("\nExamples:\n");
  printf("  %s                    # Run all tests (default)\n", program_name);
  printf("  %s --unit-only        # Run only unit tests\n", program_name);
  printf("  %s --integration-only # Run only integration tests\n", program_name);
  printf("  %s --verbose          # Run all tests with verbose output\n", program_name);
  printf("\nTest Categories:\n");
  printf("  Unit Tests: Functions following test_unit_* naming pattern\n");
  printf("  Integration Tests: Functions following test_integration_* naming pattern\n");
  printf("\nNaming Convention:\n");
  printf("  - Unit tests: test_unit_<module>_<functionality>\n");
  printf("  - Integration tests: test_integration_<service>_<functionality>\n");
}

/**
 * @brief Parse command-line arguments
 * @param argc Argument count
 * @param argv Argument vector
 * @param options Test options structure to populate
 */
static void parse_arguments(int argc, char* argv[], test_options_t* options) {
  // Initialize with default values
  memset(options, 0, sizeof(test_options_t));
  options->mode = TEST_MODE_ALL; // Run all tests by default

  for (int i = 1; i < argc; i++) {
    if (strcmp(argv[i], "--help") == 0 || strcmp(argv[i], "-h") == 0) {
      options->help = 1;
    } else if (strcmp(argv[i], "--verbose") == 0) {
      options->verbose = 1;
    } else if (strcmp(argv[i], "--unit-only") == 0) {
      options->mode = TEST_MODE_UNIT_ONLY;
    } else if (strcmp(argv[i], "--integration-only") == 0) {
      options->mode = TEST_MODE_INTEGRATION_ONLY;
    } else {
      printf("Unknown option: %s\n", argv[i]);
      printf("Use --help for usage information.\n");
      exit(1);
    }
  }
}

/**
 * @brief Main test runner
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  test_options_t options;
  parse_arguments(argc, argv, &options);

  // Show help if requested
  if (options.help) {
    print_help(argv[0]);
    return 0;
  }

  printf("ONVIF Project Comprehensive Test Suite\n");
  printf("======================================\n\n");
  printf("This test suite validates:\n");
  printf("• Unit tests for utility functions and core components\n");
  printf("• PTZ service optimization and functionality\n");
  printf("• Media service optimization and performance\n");
  printf("• Memory management and leak detection\n");
  printf("• Error handling robustness and edge cases\n");
  printf("• Concurrent operations and stress testing\n");
  printf("• Integration testing across service boundaries\n");
  printf("\nStarting comprehensive test execution...\n\n");

  if (options.verbose) {
    printf("Verbose mode enabled\n");
    switch (options.mode) {
    case TEST_MODE_ALL:
      printf("Running all test categories\n");
      break;
    case TEST_MODE_UNIT_ONLY:
      printf("Running unit tests only\n");
      break;
    case TEST_MODE_INTEGRATION_ONLY:
      printf("Running integration tests only\n");
      break;
    }
    printf("\n");
  }

  clock_t start_time = clock();

  // Define unit tests array
  const struct CMUnitTest unit_tests[] = {
    // Basic framework tests
    cmocka_unit_test(test_unit_basic_pass),
    cmocka_unit_test(test_unit_basic_string),
    cmocka_unit_test(test_unit_basic_memory),

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

    // ONVIF gSOAP tests
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

    // Service dispatcher tests
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

    // PTZ service unit tests
    cmocka_unit_test(test_unit_ptz_get_nodes_success),
    cmocka_unit_test(test_unit_ptz_get_nodes_null_nodes),
    cmocka_unit_test(test_unit_ptz_get_nodes_null_count),
    cmocka_unit_test(test_unit_ptz_get_node_success),
    cmocka_unit_test(test_unit_ptz_get_node_invalid_token),
    cmocka_unit_test(test_unit_ptz_get_node_null_token),
    cmocka_unit_test(test_unit_ptz_get_node_null_node),
    cmocka_unit_test(test_unit_ptz_get_configuration_success),
    cmocka_unit_test(test_unit_ptz_get_configuration_null_token),
    cmocka_unit_test(test_unit_ptz_get_configuration_null_config),
    cmocka_unit_test(test_unit_ptz_get_status_success),
    cmocka_unit_test(test_unit_ptz_get_status_null_token),
    cmocka_unit_test(test_unit_ptz_get_status_null_status),
    cmocka_unit_test(test_unit_ptz_absolute_move_success),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_token),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_position),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_speed),
    cmocka_unit_test(test_unit_ptz_relative_move_success),
    cmocka_unit_test(test_unit_ptz_relative_move_null_token),
    cmocka_unit_test(test_unit_ptz_relative_move_null_translation),
    cmocka_unit_test(test_unit_ptz_continuous_move_success),
    cmocka_unit_test(test_unit_ptz_continuous_move_null_token),
    cmocka_unit_test(test_unit_ptz_continuous_move_null_velocity),
    cmocka_unit_test(test_unit_ptz_stop_success),
    cmocka_unit_test(test_unit_ptz_stop_null_token),
    cmocka_unit_test(test_unit_ptz_goto_home_position_success),
    cmocka_unit_test(test_unit_ptz_goto_home_position_null_token),
    cmocka_unit_test(test_unit_ptz_set_home_position_success),
    cmocka_unit_test(test_unit_ptz_set_home_position_null_token),
    cmocka_unit_test(test_unit_ptz_get_presets_success),
    cmocka_unit_test(test_unit_ptz_get_presets_null_token),
    cmocka_unit_test(test_unit_ptz_get_presets_null_list),
    cmocka_unit_test(test_unit_ptz_get_presets_null_count),
    cmocka_unit_test(test_unit_ptz_set_preset_success),
    cmocka_unit_test(test_unit_ptz_set_preset_null_token),
    cmocka_unit_test(test_unit_ptz_set_preset_null_name),
    cmocka_unit_test(test_unit_ptz_set_preset_null_output_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_success),
    cmocka_unit_test(test_unit_ptz_goto_preset_invalid_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_null_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_null_preset_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_success),
    cmocka_unit_test(test_unit_ptz_remove_preset_invalid_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_null_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_null_preset_token),
    cmocka_unit_test(test_unit_ptz_adapter_init_success),
    cmocka_unit_test(test_unit_ptz_adapter_init_failure),
    cmocka_unit_test(test_unit_ptz_adapter_shutdown_success),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_success),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_null_status),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_absolute_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_absolute_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_relative_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_relative_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_continuous_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_continuous_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_stop_success),
    cmocka_unit_test(test_unit_ptz_adapter_stop_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_set_preset_success),
    cmocka_unit_test(test_unit_ptz_adapter_set_preset_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_goto_preset_success),
    cmocka_unit_test(test_unit_ptz_adapter_goto_preset_not_initialized),
    cmocka_unit_test(test_unit_ptz_service_init_success),
    cmocka_unit_test(test_unit_ptz_service_cleanup_success),
    cmocka_unit_test(test_unit_ptz_handle_operation_success),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_operation),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_request),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_response),
    cmocka_unit_test(test_unit_ptz_handle_operation_unknown_operation),

    // Media service unit tests
    cmocka_unit_test(test_unit_media_profile_functions),
    cmocka_unit_test(test_unit_media_video_source_functions),
    cmocka_unit_test(test_unit_media_audio_source_functions),
    cmocka_unit_test(test_unit_media_video_configuration_functions),
    cmocka_unit_test(test_unit_media_audio_configuration_functions),
    cmocka_unit_test(test_unit_media_stream_uri_functions),
    cmocka_unit_test(test_unit_media_snapshot_uri_functions),
    cmocka_unit_test(test_unit_media_multicast_functions),
    cmocka_unit_test(test_unit_media_metadata_functions),

    // Media service callback unit tests
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

    // Imaging service callback unit tests
    cmocka_unit_test(test_unit_imaging_callback_registration_success),
    cmocka_unit_test(test_unit_imaging_callback_registration_duplicate),
    cmocka_unit_test(test_unit_imaging_callback_registration_null_config),
    cmocka_unit_test(test_unit_imaging_callback_registration_dispatcher_failure),
    cmocka_unit_test(test_unit_imaging_callback_double_initialization),
    cmocka_unit_test(test_unit_imaging_callback_unregistration_success),
    cmocka_unit_test(test_unit_imaging_callback_unregistration_not_initialized),
    cmocka_unit_test(test_unit_imaging_callback_unregistration_failure),
    cmocka_unit_test(test_unit_imaging_callback_dispatch_success),

    // PTZ service registration and dispatch unit tests
    cmocka_unit_test(test_unit_ptz_service_registration_success),
    cmocka_unit_test(test_unit_ptz_service_registration_duplicate),
    cmocka_unit_test(test_unit_ptz_service_registration_invalid_params),
    cmocka_unit_test(test_unit_ptz_service_unregistration_success),
    cmocka_unit_test(test_unit_ptz_service_unregistration_not_found),
    cmocka_unit_test(test_unit_ptz_service_dispatch_success),
    cmocka_unit_test(test_unit_ptz_service_dispatch_unknown_operation),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_service),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_operation),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_request),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_response),
    cmocka_unit_test(test_unit_ptz_operation_handler_success),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_operation),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_request),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_response),
    cmocka_unit_test(test_unit_ptz_operation_handler_unknown_operation),
    cmocka_unit_test(test_unit_ptz_service_registration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_dispatch_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_unregistration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_callback_logging_success),
    cmocka_unit_test(test_unit_ptz_service_callback_logging_failure),
  };

  // Define integration tests array
  const struct CMUnitTest integration_tests[] = {
    // PTZ service integration tests
    cmocka_unit_test(test_integration_ptz_absolute_move_functionality),
    cmocka_unit_test(test_integration_ptz_relative_move_functionality),
    cmocka_unit_test(test_integration_ptz_continuous_move_functionality),
    cmocka_unit_test(test_integration_ptz_stop_functionality),
    cmocka_unit_test(test_integration_ptz_preset_creation),
    cmocka_unit_test(test_integration_ptz_preset_retrieval),
    cmocka_unit_test(test_integration_ptz_preset_goto),
    cmocka_unit_test(test_integration_ptz_preset_removal),
    cmocka_unit_test(test_integration_ptz_preset_memory_optimization),
    cmocka_unit_test(test_integration_ptz_memory_usage_improvements),
    cmocka_unit_test(test_integration_ptz_buffer_pool_usage),
    cmocka_unit_test(test_integration_ptz_string_operations_optimization),
    cmocka_unit_test(test_integration_ptz_error_handling_robustness),
    cmocka_unit_test(test_integration_ptz_concurrent_operations),
    cmocka_unit_test(test_integration_ptz_stress_testing),
    cmocka_unit_test(test_integration_ptz_memory_leak_detection),

    // Media service integration tests
    cmocka_unit_test(test_integration_media_profile_operations),
    cmocka_unit_test(test_integration_stream_uri_generation_functionality),
    cmocka_unit_test(test_integration_optimized_profile_lookup_performance),
    cmocka_unit_test(test_integration_uri_caching_optimization),
    cmocka_unit_test(test_integration_memory_efficiency),
    cmocka_unit_test(test_integration_concurrent_stream_uri_access),
    cmocka_unit_test(test_integration_stress_test_optimization),
  };

  int unit_test_count = sizeof(unit_tests) / sizeof(unit_tests[0]);
  int integration_test_count = sizeof(integration_tests) / sizeof(integration_tests[0]);
  int total_failures = 0;

  // Run tests based on mode
  if (options.mode == TEST_MODE_UNIT_ONLY || options.mode == TEST_MODE_ALL) {
    if (options.verbose) {
      printf("Running %d unit test(s)...\n", unit_test_count);
    }
    printf("Running Unit Tests...\n");
    printf("--------------------\n");
    int unit_failures =
      cmocka_run_group_tests(unit_tests, setup_global_tests, teardown_global_tests);
    total_failures += unit_failures;

    if (options.verbose) {
      printf("Unit tests completed: %d passed, %d failed\n\n", unit_test_count - unit_failures,
             unit_failures);
    }
  }

  if (options.mode == TEST_MODE_INTEGRATION_ONLY || options.mode == TEST_MODE_ALL) {
    if (options.verbose) {
      printf("Running %d integration test(s)...\n", integration_test_count);
    }
    printf("Running Integration Tests...\n");
    printf("----------------------------\n");
    int integration_failures =
      cmocka_run_group_tests(integration_tests, setup_global_tests, teardown_global_tests);
    total_failures += integration_failures;

    if (options.verbose) {
      printf("Integration tests completed: %d passed, %d failed\n\n",
             integration_test_count - integration_failures, integration_failures);
    }
  }

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  // Calculate total tests run
  int total_tests_run = 0;
  if (options.mode == TEST_MODE_UNIT_ONLY || options.mode == TEST_MODE_ALL) {
    total_tests_run += unit_test_count;
  }
  if (options.mode == TEST_MODE_INTEGRATION_ONLY || options.mode == TEST_MODE_ALL) {
    total_tests_run += integration_test_count;
  }

  printf("\n");
  printf("Comprehensive Test Summary\n");
  printf("==========================\n");
  printf("Tests Run: %d\n", total_tests_run);
  printf("Test Duration: %.2f seconds\n", test_duration);

  if (total_failures == 0) {
    printf("✅ All %d test(s) passed!\n", total_tests_run);
    switch (options.mode) {
    case TEST_MODE_ALL:
      printf("✅ Unit tests validated\n");
      printf("✅ Integration tests validated\n");
      printf("✅ All test categories passed\n");
      break;
    case TEST_MODE_UNIT_ONLY:
      printf("✅ Unit tests validated\n");
      break;
    case TEST_MODE_INTEGRATION_ONLY:
      printf("✅ Integration tests validated\n");
      break;
    }
    printf("\n🎉 Test suite completed successfully!\n");
  } else {
    printf("❌ %d test(s) failed!\n", total_failures);
    printf("\n");
    printf("Failed Test Categories:\n");
    printf("• Unit Tests: Core functionality validation\n");
    printf("• Integration Tests: Cross-service functionality\n");
    printf("• Memory Management: Leak detection and optimization\n");
    printf("• Error Handling: Robustness and edge cases\n");
    printf("• Performance Tests: Optimization and stress testing\n");
    printf("• HTTP Services: Authentication, metrics, gSOAP integration\n");
    printf("• Service Dispatcher: Registration, dispatch, lifecycle\n");
    printf("• PTZ Service: Movement operations, preset management, optimization\n");
    printf("• Media Service: Profile operations, stream URIs, performance\n");
    printf("• Integration Tests: Cross-service functionality, stress testing\n");
    printf("• Memory Management: Leak detection, buffer pools, optimization\n");
  }

  return total_failures;
}
