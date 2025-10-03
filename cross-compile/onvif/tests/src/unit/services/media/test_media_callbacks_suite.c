/**
 * @file test_media_callbacks_suite.c
 * @brief Media callbacks test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_onvif_media_callbacks.c
void test_unit_media_callback_registration_success(void** state);
void test_unit_media_callback_registration_duplicate(void** state);
void test_unit_media_callback_registration_null_config(void** state);
void test_unit_media_callback_registration_dispatcher_failure(void** state);
void test_unit_media_callback_double_initialization(void** state);
void test_unit_media_callback_unregistration_success(void** state);
void test_unit_media_callback_unregistration_not_initialized(void** state);
void test_unit_media_callback_unregistration_failure(void** state);
void test_unit_media_callback_dispatch_success(void** state);
void test_unit_media_callback_dispatch_not_initialized(void** state);
void test_unit_media_callback_dispatch_null_params(void** state);

/**
 * @brief Get media callbacks unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_media_callbacks_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_media_callback_registration_success),
    cmocka_unit_test(test_unit_media_callback_registration_duplicate),
    cmocka_unit_test(test_unit_media_callback_registration_null_config),
    cmocka_unit_test(test_unit_media_callback_registration_dispatcher_failure),
    cmocka_unit_test(test_unit_media_callback_double_initialization),
    cmocka_unit_test(test_unit_media_callback_unregistration_success),
    cmocka_unit_test(test_unit_media_callback_unregistration_not_initialized),
    cmocka_unit_test(test_unit_media_callback_unregistration_failure),
    cmocka_unit_test(test_unit_media_callback_dispatch_success),
    cmocka_unit_test(test_unit_media_callback_dispatch_not_initialized),
    cmocka_unit_test(test_unit_media_callback_dispatch_null_params),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
