/**
 * @file test_imaging_callbacks_suite.c
 * @brief Imaging callbacks test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_onvif_imaging_callbacks.c
int setup_imaging_unit_tests(void** state);
int teardown_imaging_unit_tests(void** state);
void test_unit_imaging_callback_registration_success(void** state);
void test_unit_imaging_callback_registration_duplicate(void** state);
void test_unit_imaging_callback_registration_null_config(void** state);
void test_unit_imaging_callback_registration_dispatcher_failure(void** state);
void test_unit_imaging_callback_double_initialization(void** state);
void test_unit_imaging_callback_unregistration_success(void** state);
void test_unit_imaging_callback_unregistration_not_initialized(void** state);
void test_unit_imaging_callback_unregistration_failure(void** state);

/**
 * @brief Get imaging callbacks unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_imaging_callbacks_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_success,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_duplicate,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_null_config,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_registration_dispatcher_failure,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_double_initialization,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_success,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_not_initialized,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_callback_unregistration_failure,
                                    setup_imaging_unit_tests, teardown_imaging_unit_tests),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
