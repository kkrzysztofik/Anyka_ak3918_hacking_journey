/**
 * @file test_device_service.h
 * @brief Unit tests for ONVIF Device service
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_DEVICE_SERVICE_H
#define TEST_DEVICE_SERVICE_H

#include "cmocka_wrapper.h"

// Test function declarations
void test_device_system_reboot_success(void** state);
void test_device_system_reboot_failure(void** state);
void test_device_init_success(void** state);
void test_device_init_null_config(void** state);
void test_device_init_already_initialized(void** state);
void test_device_init_gsoap_failure(void** state);
void test_device_init_buffer_pool_failure(void** state);
void test_device_cleanup_success(void** state);
void test_device_cleanup_not_initialized(void** state);
void test_device_cleanup_unregister_failure(void** state);
void test_device_handle_operation_get_device_information(void** state);
void test_device_handle_operation_get_capabilities(void** state);
void test_device_handle_operation_get_system_date_time(void** state);
void test_device_handle_operation_get_services(void** state);
void test_device_handle_operation_system_reboot(void** state);
void test_device_handle_operation_unknown_operation(void** state);
void test_device_handle_operation_null_operation(void** state);
void test_device_handle_operation_null_request(void** state);
void test_device_handle_operation_null_response(void** state);
void test_device_handle_operation_not_initialized(void** state);
void test_device_capabilities_handler_success(void** state);
void test_device_capabilities_handler_null_capability(void** state);
void test_device_capabilities_handler_unknown_capability(void** state);
void test_device_service_registration_success(void** state);
void test_device_service_registration_duplicate(void** state);
void test_device_service_registration_invalid_params(void** state);
void test_device_service_unregistration_success(void** state);
void test_device_service_unregistration_not_found(void** state);
void test_device_business_logic_get_device_information(void** state);
void test_device_business_logic_get_capabilities(void** state);
void test_device_business_logic_get_system_date_time(void** state);
void test_device_business_logic_get_services(void** state);
void test_device_business_logic_system_reboot(void** state);
void test_device_business_logic_null_callback_data(void** state);
void test_device_error_handling(void** state);
void test_device_memory_management(void** state);
void test_device_configuration_handling(void** state);

// Test constants
#define TEST_DEVICE_MANUFACTURER     "TestManufacturer"
#define TEST_DEVICE_MODEL            "TestModel"
#define TEST_DEVICE_FIRMWARE_VERSION "1.0.0"
#define TEST_DEVICE_SERIAL_NUMBER    "TEST123456"
#define TEST_DEVICE_HARDWARE_ID      "1.0"
#define TEST_HTTP_PORT               8080
#define TEST_OPERATION_NAME_LEN      64
#define TEST_SERVICE_NAME_LEN        32

// Test data structures
struct test_device_info {
  char manufacturer[64];
  char model[64];
  char firmware_version[32];
  char serial_number[64];
  char hardware_id[32];
};

struct test_capabilities {
  int has_analytics;
  int has_device;
  int has_events;
  int has_imaging;
  int has_media;
  int has_ptz;
};

struct test_system_datetime {
  struct tm* tm_info;
  int timezone_offset;
  int daylight_savings;
};

struct test_services {
  int include_capability;
  int service_count;
  char service_names[8][32];
};

struct test_system_reboot {
  char message[128];
  int reboot_initiated;
};

#endif // TEST_DEVICE_SERVICE_H
