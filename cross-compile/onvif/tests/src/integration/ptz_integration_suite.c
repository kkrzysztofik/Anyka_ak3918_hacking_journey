/**
 * @file ptz_integration_suite.c
 * @brief PTZ integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from ptz_service_tests.c
void test_integration_ptz_relative_move_functionality(void** state);
void test_integration_ptz_continuous_move_functionality(void** state);
void test_integration_ptz_continuous_move_timeout_cleanup(void** state);
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
void test_integration_ptz_absolute_move_soap(void** state);
void test_integration_ptz_get_presets_soap(void** state);
void test_integration_ptz_set_preset_soap(void** state);
void test_integration_ptz_goto_preset_soap(void** state);
void test_integration_ptz_remove_preset_soap(void** state);

/**
 * @brief Get PTZ integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_ptz_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    // FAST TESTS FIRST (no preset creation or minimal presets)
    cmocka_unit_test(test_integration_ptz_relative_move_functionality),
    cmocka_unit_test(test_integration_ptz_continuous_move_functionality),
    cmocka_unit_test(test_integration_ptz_stop_functionality),

    // SOAP integration tests (create 1 preset each - within limit)
    cmocka_unit_test(test_integration_ptz_get_nodes_soap),
    cmocka_unit_test(test_integration_ptz_absolute_move_soap),
    cmocka_unit_test(test_integration_ptz_get_presets_soap),
    cmocka_unit_test(test_integration_ptz_set_preset_soap),
    cmocka_unit_test(test_integration_ptz_goto_preset_soap),
    cmocka_unit_test(test_integration_ptz_remove_preset_soap),

    // MODERATE TESTS (create few presets - within limit)
    cmocka_unit_test(test_integration_ptz_preset_memory_optimization),
    cmocka_unit_test(test_integration_ptz_buffer_pool_usage),
    cmocka_unit_test(test_integration_ptz_string_operations_optimization),
    cmocka_unit_test(test_integration_ptz_error_handling_robustness),

    // LONG TESTS LAST (create many presets - may exceed limit)
    cmocka_unit_test(test_integration_ptz_memory_usage_improvements),
    cmocka_unit_test(test_integration_ptz_stress_testing),
    cmocka_unit_test(test_integration_ptz_memory_leak_detection),
    cmocka_unit_test(test_integration_ptz_concurrent_operations),

    // LONGEST TEST LAST (timeout operations with cleanup)
    cmocka_unit_test(test_integration_ptz_continuous_move_timeout_cleanup),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
