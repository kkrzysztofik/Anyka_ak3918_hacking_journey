/**
 * @file device_integration_suite.c
 * @brief Device integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from device_service_tests.c
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
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_integration_device_init_cleanup_lifecycle),
    cmocka_unit_test(test_integration_device_get_device_information_fields_validation),
    cmocka_unit_test(test_integration_device_get_capabilities_specific_category),
    cmocka_unit_test(test_integration_device_get_capabilities_multiple_categories),
    cmocka_unit_test(test_integration_device_get_system_date_time_timezone),
    cmocka_unit_test(test_integration_device_get_system_date_time_dst),
    cmocka_unit_test(test_integration_device_get_services_namespaces),
    cmocka_unit_test(test_integration_device_handle_operation_null_params),
    cmocka_unit_test(test_integration_device_handle_operation_invalid_operation),
    cmocka_unit_test(test_integration_device_handle_operation_uninitialized),
    cmocka_unit_test(test_integration_device_concurrent_get_device_information),
    cmocka_unit_test(test_integration_device_concurrent_get_capabilities),
    cmocka_unit_test(test_integration_device_concurrent_mixed_operations),
    cmocka_unit_test(test_integration_device_config_integration),
    cmocka_unit_test(test_integration_device_get_device_info_soap),
    cmocka_unit_test(test_integration_device_get_capabilities_soap),
    cmocka_unit_test(test_integration_device_get_system_date_time_soap),
    cmocka_unit_test(test_integration_device_get_services_soap),
    cmocka_unit_test(test_integration_device_system_reboot_soap),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
