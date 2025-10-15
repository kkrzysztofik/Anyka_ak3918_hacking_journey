/**
 * @file imaging_integration_suite.c
 * @brief Imaging integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from imaging_service_optimization_test.c
void test_integration_imaging_bulk_settings_validation(void** state);
void test_integration_imaging_batch_parameter_update_optimization(void** state);
void test_integration_imaging_parameter_cache_efficiency(void** state);
void test_integration_imaging_concurrent_access(void** state);
void test_integration_imaging_performance_regression(void** state);

// Forward declarations from test_imaging_integration.c
void test_integration_imaging_config_integration(void** state);

// Forward declarations for setup/teardown from test_imaging_integration.c
int imaging_service_setup(void** state);
int imaging_service_teardown(void** state);

/**
 * @brief Get imaging integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_imaging_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_integration_imaging_bulk_settings_validation),
    cmocka_unit_test(test_integration_imaging_batch_parameter_update_optimization),
    cmocka_unit_test(test_integration_imaging_parameter_cache_efficiency),
    cmocka_unit_test(test_integration_imaging_concurrent_access),
    cmocka_unit_test(test_integration_imaging_performance_regression),

    // T085: Configuration integration test with setup/teardown
    cmocka_unit_test_setup_teardown(test_integration_imaging_config_integration,
                                    imaging_service_setup, imaging_service_teardown),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
