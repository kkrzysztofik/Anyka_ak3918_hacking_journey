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

// Forward declarations for PTZ setup/teardown
int ptz_service_setup(void** state);
int ptz_service_teardown(void** state);

// Forward declarations for Media integration tests
void test_integration_media_profile_operations(void** state);
void test_integration_media_stream_uri_generation_functionality(void** state);
void test_integration_optimized_profile_lookup_performance(void** state);
void test_integration_uri_caching_optimization(void** state);
void test_integration_media_memory_efficiency(void** state);
void test_integration_concurrent_stream_uri_access(void** state);
void test_integration_stress_test_optimization(void** state);

// Note: Imaging test declarations and setup/teardown are in imaging_service_optimization_tests.h

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
int main(int argc, char* argv[]) {
  (void)argc;
  (void)argv;

  printf("ONVIF Integration Tests\n");
  printf("=======================\n\n");
  printf("This test suite includes:\n");
  printf("• PTZ service integration tests\n");
  printf("• Media service integration tests\n");
  printf("• Imaging service optimization tests\n");
  printf("  - Parameter caching efficiency\n");
  printf("  - Bulk settings validation optimization\n");
  printf("  - Batch parameter update optimization\n");
  printf("  - Concurrent access handling\n");
  printf("  - Performance regression checking\n\n");

  clock_t start_time = clock();

  const struct CMUnitTest tests[] = {
    // PTZ integration tests with setup/teardown
    cmocka_unit_test_setup_teardown(test_integration_ptz_absolute_move_functionality,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_relative_move_functionality,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_continuous_move_functionality,
                                    ptz_service_setup, ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_stop_functionality, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_preset_creation, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_preset_retrieval, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_preset_goto, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_preset_removal, ptz_service_setup,
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
    cmocka_unit_test_setup_teardown(test_integration_ptz_concurrent_operations, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_stress_testing, ptz_service_setup,
                                    ptz_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_ptz_memory_leak_detection, ptz_service_setup,
                                    ptz_service_teardown),

    // Media integration tests
    cmocka_unit_test(test_integration_media_profile_operations),
    cmocka_unit_test(test_integration_media_stream_uri_generation_functionality),
    cmocka_unit_test(test_integration_optimized_profile_lookup_performance),
    cmocka_unit_test(test_integration_uri_caching_optimization),
    cmocka_unit_test(test_integration_media_memory_efficiency),
    cmocka_unit_test(test_integration_concurrent_stream_uri_access),
    cmocka_unit_test(test_integration_stress_test_optimization),

    // Imaging integration tests (with setup/teardown for proper initialization)
    cmocka_unit_test_setup_teardown(test_integration_imaging_parameter_cache_efficiency,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_bulk_settings_validation,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_batch_parameter_update_optimization,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_concurrent_access,
                                    setup_imaging_integration, teardown_imaging_integration),
    cmocka_unit_test_setup_teardown(test_integration_imaging_performance_regression,
                                    setup_imaging_integration, teardown_imaging_integration),
  };

  int test_count = sizeof(tests) / sizeof(tests[0]);
  int failures = cmocka_run_group_tests(tests, setup_global_tests, teardown_global_tests);

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  printf("\nIntegration Test Summary\n");
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
