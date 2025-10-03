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

/**
 * @brief Get service dispatcher unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_service_dispatcher_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_service_dispatcher_init),
    cmocka_unit_test(test_unit_service_dispatcher_cleanup),
    cmocka_unit_test(test_unit_service_dispatcher_register_service),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_null_params),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_invalid_params),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_duplicate),
    cmocka_unit_test(test_unit_service_dispatcher_register_service_registry_full),
    cmocka_unit_test(test_unit_service_dispatcher_unregister_service),
    cmocka_unit_test(test_unit_service_dispatcher_unregister_service_not_found),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch_invalid_params),
    cmocka_unit_test(test_unit_service_dispatcher_dispatch_service_not_found),
    cmocka_unit_test(test_unit_service_dispatcher_is_registered),
    cmocka_unit_test(test_unit_service_dispatcher_get_services),
    cmocka_unit_test(test_unit_service_dispatcher_init_cleanup_handlers),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
