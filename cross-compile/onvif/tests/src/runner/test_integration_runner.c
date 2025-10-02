/**
 * @file test_integration_runner.c
 * @brief Test runner for integration tests (PTZ, Media, and Imaging)
 * @author kkrzysztofik
 * @date 2025
 */
#include <stdio.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "integration/imaging_service_optimization_tests.h"

// Forward declarations for PTZ integration tests
// Service dispatcher include
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"

void test_integration_ptz_relative_move_functionality(void** state);
void test_integration_ptz_continuous_move_functionality(void** state);
void test_integration_ptz_stop_functionality(void** state);
void test_integration_ptz_preset_memory_optimization(void** state);
void test_integration_ptz_memory_usage_improvements(void** state);
void test_integration_ptz_buffer_pool_usage(void** state);
void test_integration_ptz_string_operations_optimization(void** state);
void test_integration_ptz_error_handling_robustness(void** state);
void test_integration_ptz_concurrent_operations(void** state);
void test_integration_ptz_stress_testing(void** state);
void test_integration_ptz_memory_leak_detection(void** state);
void test_integration_ptz_get_nodes_soap(void** state);

// Forward declarations for PTZ setup/teardown
int ptz_service_setup(void** state);
int ptz_service_teardown(void** state);

// Forward declarations for Media integration tests
void test_integration_optimized_profile_lookup_performance(void** state);
void test_integration_uri_caching_optimization(void** state);
void test_integration_media_memory_efficiency(void** state);
void test_integration_concurrent_stream_uri_access(void** state);
void test_integration_stress_test_optimization(void** state);
void test_integration_media_get_profiles_soap(void** state);

// Forward declarations for Media setup/teardown
int media_service_setup(void** state);
int media_service_teardown(void** state);

// Forward declarations for Device integration tests
void test_integration_device_init_cleanup_lifecycle(void** state);
void test_integration_device_get_device_information_fields_validation(void** state);
void test_integration_device_get_capabilities_specific_category(void** state);
void test_integration_device_get_capabilities_multiple_categories(void** state);
void test_integration_device_get_system_date_time_timezone(void** state);
void test_integration_device_get_system_date_time_dst(void** state);
void test_integration_device_get_services_namespaces(void** state);
void test_integration_device_handle_operation_null_params(void** state);
void test_integration_device_handle_operation_invalid_operation(void** state);
void test_integration_device_handle_operation_uninitialized(void** state);
void test_integration_device_concurrent_get_device_information(void** state);
void test_integration_device_concurrent_get_capabilities(void** state);
void test_integration_device_concurrent_mixed_operations(void** state);
void test_integration_device_config_integration(void** state);
void test_integration_device_get_device_info_soap(void** state);

// Forward declarations for Device setup/teardown
int device_service_setup(void** state);
int device_service_teardown(void** state);

// Note: Imaging test declarations and setup/teardown are in imaging_service_optimization_tests.h

// Forward declaration for Imaging SOAP test
void test_integration_imaging_get_settings_soap(void** state);

// Forward declarations for SOAP error tests
void test_integration_soap_error_invalid_xml(void** state);
void test_integration_soap_error_missing_param(void** state);
void test_integration_soap_error_wrong_operation(void** state);
void test_integration_soap_error_malformed_envelope(void** state);

// Forward declarations for SOAP error setup/teardown
int soap_error_tests_setup(void** state);
int soap_error_tests_teardown(void** state);

/**
 * @brief Global test setup
 * @param state Test state
 * @return 0 on success
 */
static int setup_global_tests(void** state) {
  (void)state;

  // Initialize service dispatcher once for all tests
  int result = onvif_service_dispatcher_init();
  if (result != ONVIF_SUCCESS) {
    printf("Failed to initialize service dispatcher: %d\n", result);
    return -1;
  }

  return 0;
}

/**
 * @brief Global test teardown
 * @param state Test state
 * @return 0 on success
 */
static int teardown_global_tests(void** state) {
  (void)state;

  // Cleanup service dispatcher once after all tests
  onvif_service_dispatcher_cleanup();

  return 0;
}

/**
 * @brief Main test runner for integration tests
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(void) {
  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // Device integration tests with setup/teardown
    cmocka_unit_test(test_integration_device_init_cleanup_lifecycle),
    cmocka_unit_test_setup_teardown(test_integration_device_get_device_information_fields_validation,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_specific_category,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_multiple_categories,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_timezone,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_dst,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_services_namespaces,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_null_params,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_invalid_operation,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test(test_integration_device_handle_operation_uninitialized),
    cmocka_unit_test_setup_teardown(test_integration_device_config_integration, device_service_setup,
                                    device_service_teardown),
    // SOAP pilot test with proper service initialization - placed before concurrent tests
    cmocka_unit_test_setup_teardown(test_integration_device_get_device_info_soap, device_service_setup,
                                    device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_device_information,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_capabilities,
                                    device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_mixed_operations,
                                    device_service_setup, device_service_teardown),

    // PTZ integration tests with setup/teardown
    cmocka_unit_test_setup_teardown(test_integration_ptz_relative_move_functionality,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_continuous_move_functionality,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_stop_functionality, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_preset_memory_optimization,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_memory_usage_improvements,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_buffer_pool_usage, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_string_operations_optimization,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_error_handling_robustness,
                                    ptz_service_setup, ptz_service_teardown),
    // SOAP pilot test with proper service initialization - placed before concurrent tests
    cmocka_unit_test_setup_teardown(test_integration_ptz_get_nodes_soap, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_concurrent_operations, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_stress_testing, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_memory_leak_detection, ptz_service_setup,
                                    ptz_service_teardown),

    // Media integration tests
    cmocka_unit_test_setup_teardown(test_integration_optimized_profile_lookup_performance,
                                    media_service_setup, media_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_uri_caching_optimization, media_service_setup,
                                    media_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_media_memory_efficiency, media_service_setup,
                                    media_service_teardown),
    // SOAP pilot test with proper service initialization - placed before concurrent tests
    cmocka_unit_test_setup_teardown(test_integration_media_get_profiles_soap, media_service_setup,
                                    media_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_concurrent_stream_uri_access,
                                    media_service_setup, media_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_stress_test_optimization, media_service_setup,
                                    media_service_teardown),

    // Imaging integration tests (with setup/teardown for proper initialization)
    cmocka_unit_test_setup_teardown(test_integration_imaging_parameter_cache_efficiency,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_bulk_settings_validation,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_batch_parameter_update_optimization,
                                    setup_imaging_integration, teardown_imaging_integration),
    // SOAP pilot test with proper service initialization - placed before concurrent tests
    cmocka_unit_test_setup_teardown(test_integration_imaging_get_settings_soap,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_concurrent_access,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_performance_regression,
                                    setup_imaging_integration, teardown_imaging_integration),

    // SOAP error handling tests
    cmocka_unit_test_setup_teardown(test_integration_soap_error_invalid_xml, soap_error_tests_setup,
                                    soap_error_tests_teardown),
    cmocka_unit_test_setup_teardown(test_integration_soap_error_missing_param,
                                    soap_error_tests_setup, soap_error_tests_teardown),
    cmocka_unit_test_setup_teardown(test_integration_soap_error_wrong_operation,
                                    soap_error_tests_setup, soap_error_tests_teardown),
    cmocka_unit_test_setup_teardown(test_integration_soap_error_malformed_envelope,
                                    soap_error_tests_setup, soap_error_tests_teardown),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, NULL, NULL);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nIntegration Test Summary\n");
  printf("========================\n");
  printf("Total tests: %d\n", test_count);
  printf("Test duration: %.2f seconds\n", test_duration);

  return failures;
}
