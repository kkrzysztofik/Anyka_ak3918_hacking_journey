/**
 * @file network_integration_suite.c
 * @brief Network layer integration test suite wrapper (T087)
 * @author Claude Code
 * @date 2025-10-15
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_network_integration.c
void test_integration_network_onvif_config(void** state);
void test_integration_network_service_ports(void** state);
void test_integration_network_http_server_config(void** state);
void test_integration_network_logging_config(void** state);
void test_integration_network_runtime_updates(void** state);

// Forward declarations for setup/teardown from test_network_integration.c
int network_service_setup(void** state);
int network_service_teardown(void** state);

/**
 * @brief Get network layer integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_network_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    // T087: Network layer configuration integration tests
    cmocka_unit_test_setup_teardown(test_integration_network_onvif_config, network_service_setup, network_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_network_service_ports, network_service_setup, network_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_network_http_server_config, network_service_setup, network_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_network_logging_config, network_service_setup, network_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_network_runtime_updates, network_service_setup, network_service_teardown),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
