/**
 * @file device_integration_suite.c
 * @brief Device integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from device_service_tests.c
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
void test_integration_device_get_capabilities_soap(void** state);
void test_integration_device_get_system_date_time_soap(void** state);
void test_integration_device_get_services_soap(void** state);
void test_integration_device_system_reboot_soap(void** state);

/**
 * @brief Get device integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_device_integration_tests(size_t* count) {
  // Forward declare setup/teardown functions
  int device_service_setup(void** state);
  int device_service_teardown(void** state);

  static const struct CMUnitTest tests[] = {
    // GetDeviceInformation tests
    cmocka_unit_test_setup_teardown(test_integration_device_get_device_information_fields_validation, device_service_setup, device_service_teardown),

    // GetCapabilities tests
    cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_specific_category, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_multiple_categories, device_service_setup, device_service_teardown),

    // GetSystemDateAndTime tests
    cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_timezone, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_dst, device_service_setup, device_service_teardown),

    // GetServices tests
    cmocka_unit_test_setup_teardown(test_integration_device_get_services_namespaces, device_service_setup, device_service_teardown),

    // Error handling tests
    cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_null_params, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_handle_operation_invalid_operation, device_service_setup, device_service_teardown),
    cmocka_unit_test(test_integration_device_handle_operation_uninitialized),

    // Configuration integration test
    cmocka_unit_test_setup_teardown(test_integration_device_config_integration, device_service_setup, device_service_teardown),

    // SOAP integration tests (full HTTP/SOAP layer validation)
    cmocka_unit_test_setup_teardown(test_integration_device_get_device_info_soap, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_capabilities_soap, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_system_date_time_soap, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_get_services_soap, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_system_reboot_soap, device_service_setup, device_service_teardown),

    // Concurrent operations tests (may hang - placed at end)
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_device_information, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_get_capabilities, device_service_setup, device_service_teardown),
    cmocka_unit_test_setup_teardown(test_integration_device_concurrent_mixed_operations, device_service_setup, device_service_teardown),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
