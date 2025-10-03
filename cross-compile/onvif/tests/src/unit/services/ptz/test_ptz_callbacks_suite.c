/**
 * @file test_ptz_callbacks_suite.c
 * @brief PTZ callbacks test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_onvif_ptz_callbacks.c
void test_unit_ptz_service_registration_success(void** state);
void test_unit_ptz_service_registration_duplicate(void** state);
void test_unit_ptz_service_registration_invalid_params(void** state);
void test_unit_ptz_service_unregistration_success(void** state);
void test_unit_ptz_service_unregistration_not_found(void** state);
void test_unit_ptz_service_dispatch_success(void** state);
void test_unit_ptz_service_dispatch_unknown_operation(void** state);
void test_unit_ptz_service_dispatch_null_service(void** state);
void test_unit_ptz_service_dispatch_null_operation(void** state);
void test_unit_ptz_service_dispatch_null_request(void** state);
void test_unit_ptz_service_dispatch_null_response(void** state);
void test_unit_ptz_operation_handler_success(void** state);
void test_unit_ptz_operation_handler_null_operation(void** state);
void test_unit_ptz_operation_handler_null_request(void** state);
void test_unit_ptz_operation_handler_null_response(void** state);
void test_unit_ptz_operation_handler_unknown_operation(void** state);
void test_unit_ptz_service_registration_failure_handling(void** state);
void test_unit_ptz_service_dispatch_failure_handling(void** state);
void test_unit_ptz_service_unregistration_failure_handling(void** state);
void test_unit_ptz_service_callback_logging_success(void** state);
void test_unit_ptz_service_callback_logging_failure(void** state);

/**
 * @brief Get PTZ callbacks unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_ptz_callbacks_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_ptz_service_registration_success),
    cmocka_unit_test(test_unit_ptz_service_registration_duplicate),
    cmocka_unit_test(test_unit_ptz_service_registration_invalid_params),
    cmocka_unit_test(test_unit_ptz_service_unregistration_success),
    cmocka_unit_test(test_unit_ptz_service_unregistration_not_found),
    cmocka_unit_test(test_unit_ptz_service_dispatch_success),
    cmocka_unit_test(test_unit_ptz_service_dispatch_unknown_operation),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_service),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_operation),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_request),
    cmocka_unit_test(test_unit_ptz_service_dispatch_null_response),
    cmocka_unit_test(test_unit_ptz_operation_handler_success),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_operation),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_request),
    cmocka_unit_test(test_unit_ptz_operation_handler_null_response),
    cmocka_unit_test(test_unit_ptz_operation_handler_unknown_operation),
    cmocka_unit_test(test_unit_ptz_service_registration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_dispatch_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_unregistration_failure_handling),
    cmocka_unit_test(test_unit_ptz_service_callback_logging_success),
    cmocka_unit_test(test_unit_ptz_service_callback_logging_failure),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
