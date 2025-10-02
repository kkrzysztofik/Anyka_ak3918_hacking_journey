/**
 * @file device_service_tests.h
 * @brief Integration tests header for ONVIF Device service
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef DEVICE_SERVICE_TESTS_H
#define DEVICE_SERVICE_TESTS_H

#include "cmocka_wrapper.h"

// Test function declarations for Device service lifecycle
void test_integration_device_init_cleanup_lifecycle(void** state);

// Test function declarations for GetDeviceInformation operation
void test_integration_device_get_device_information(void** state);
void test_integration_device_get_device_information_fields_validation(void** state);

// Test function declarations for GetCapabilities operation
void test_integration_device_get_capabilities_all_services(void** state);
void test_integration_device_get_capabilities_specific_category(void** state);
void test_integration_device_get_capabilities_multiple_categories(void** state);

// Test function declarations for GetSystemDateTime operation
void test_integration_device_get_system_date_time(void** state);
void test_integration_device_get_system_date_time_timezone(void** state);
void test_integration_device_get_system_date_time_dst(void** state);

// Test function declarations for GetServices operation
void test_integration_device_get_services_all(void** state);
void test_integration_device_get_services_namespaces(void** state);

// Test function declarations for SystemReboot operation
void test_integration_device_system_reboot(void** state);

// Test function declarations for error handling
void test_integration_device_handle_operation_null_params(void** state);
void test_integration_device_handle_operation_invalid_operation(void** state);
void test_integration_device_handle_operation_uninitialized(void** state);

// Test function declarations for concurrent operations
void test_integration_device_concurrent_get_device_information(void** state);
void test_integration_device_concurrent_get_capabilities(void** state);
void test_integration_device_concurrent_mixed_operations(void** state);

// Test function declarations for configuration integration
void test_integration_device_config_integration(void** state);

// Test suite
extern const struct CMUnitTest device_service_tests[];

#endif // DEVICE_SERVICE_TESTS_H
