/**
 * @file test_media_callbacks_suite.c
 * @brief Media callbacks test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_onvif_media_callbacks.c
int setup_media_callback_tests(void** state);
int teardown_media_callback_tests(void** state);
void test_unit_media_callback_registration_success(void** state);
void test_unit_media_callback_registration_duplicate(void** state);
void test_unit_media_callback_registration_null_config(void** state);
void test_unit_media_callback_registration_dispatcher_failure(void** state);
void test_unit_media_callback_double_initialization(void** state);
void test_unit_media_callback_unregistration_success(void** state);
void test_unit_media_callback_unregistration_not_initialized(void** state);
void test_unit_media_callback_unregistration_failure(void** state);

/**
 * @brief Get media callbacks unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 *
 * @note Dispatch tests (dispatch_success, dispatch_not_initialized, dispatch_null_params)
 *       have been moved to test_service_dispatcher.c where they belong, as they test
 *       dispatcher functionality rather than media-specific callback behavior.
 */
const struct CMUnitTest* get_media_callbacks_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_success,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_duplicate,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_null_config,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_registration_dispatcher_failure,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_double_initialization,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_success,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_not_initialized,
                                    setup_media_callback_tests, teardown_media_callback_tests),
    cmocka_unit_test_setup_teardown(test_unit_media_callback_unregistration_failure,
                                    setup_media_callback_tests, teardown_media_callback_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
