/**
 * @file snapshot_integration_suite.c
 * @brief Snapshot integration test suite wrapper (T086)
 * @author Claude Code
 * @date 2025-10-15
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_snapshot_integration.c
void test_integration_snapshot_config_integration(void** state);
void test_integration_snapshot_bounds_validation(void** state);
void test_integration_snapshot_format_parameter(void** state);

// Forward declarations for setup/teardown from test_snapshot_integration.c
int snapshot_service_setup(void** state);
int snapshot_service_teardown(void** state);

/**
 * @brief Get snapshot integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_snapshot_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    // T086: Snapshot configuration integration tests
    cmocka_unit_test_setup_teardown(test_integration_snapshot_config_integration,
                                    snapshot_service_setup, snapshot_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_snapshot_bounds_validation,
                                    snapshot_service_setup, snapshot_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_snapshot_format_parameter,
                                    snapshot_service_setup, snapshot_service_teardown),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
