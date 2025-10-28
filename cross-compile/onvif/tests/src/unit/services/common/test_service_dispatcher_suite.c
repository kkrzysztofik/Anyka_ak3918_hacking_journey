/**
 * @file test_service_dispatcher_suite.c
 * @brief Service dispatcher test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_service_dispatcher.c
void test_unit_service_dispatcher_init(void** state);
void test_unit_service_dispatcher_cleanup(void** state);
void test_unit_service_dispatcher_register_service(void** state);
void test_unit_service_dispatcher_register_service_null_params(void** state);
void test_unit_service_dispatcher_register_service_invalid_params(void** state);
void test_unit_service_dispatcher_register_service_duplicate(void** state);
void test_unit_service_dispatcher_register_service_registry_full(void** state);
void test_unit_service_dispatcher_unregister_service(void** state);
void test_unit_service_dispatcher_unregister_service_not_found(void** state);
void test_unit_service_dispatcher_dispatch(void** state);
void test_unit_service_dispatcher_dispatch_invalid_params(void** state);
void test_unit_service_dispatcher_dispatch_service_not_found(void** state);
void test_unit_service_dispatcher_is_registered(void** state);
void test_unit_service_dispatcher_get_services(void** state);
void test_unit_service_dispatcher_init_cleanup_handlers(void** state);
void test_unit_service_dispatcher_dispatch_with_registered_service(void** state);
void test_unit_service_dispatcher_dispatch_unknown_operation(void** state);
void test_unit_service_dispatcher_dispatch_null_service_name(void** state);
void test_unit_service_dispatcher_dispatch_null_operation_name(void** state);
void test_unit_service_dispatcher_dispatch_null_request_param(void** state);
void test_unit_service_dispatcher_dispatch_null_response_param(void** state);

// Setup/teardown functions
int setup_service_dispatcher_tests(void** state);
int teardown_service_dispatcher_tests(void** state);

/**
 * @brief Get service dispatcher unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_service_dispatcher_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_init, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_cleanup, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_null_params, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_invalid_params, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_duplicate, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_register_service_registry_full, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_unregister_service, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_unregister_service_not_found, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_invalid_params, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_service_not_found, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_is_registered, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_get_services, setup_service_dispatcher_tests, teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_init_cleanup_handlers, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    // Service Dispatch with Multiple Services Tests (moved from service callbacks)
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_with_registered_service, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_unknown_operation, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_service_name, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_operation_name, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_request_param, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
    cmocka_unit_test_setup_teardown(test_unit_service_dispatcher_dispatch_null_response_param, setup_service_dispatcher_tests,
                                    teardown_service_dispatcher_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
