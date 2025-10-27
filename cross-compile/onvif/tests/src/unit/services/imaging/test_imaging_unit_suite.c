/**
 * @file test_imaging_unit_suite.c
 * @brief Imaging service unit test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_imaging_unit_suite.h"

#include "test_imaging_service.h"

/**
 * @brief Get imaging service unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_imaging_service_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    /* Section 1: Get/Set Settings Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_settings_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_settings_null_params, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_settings_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_settings_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_settings_null_params, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_settings_invalid_brightness, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_settings_invalid_contrast, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_settings_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 2: Day/Night Mode Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_day_night_mode_day, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_day_night_mode_night, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_day_night_mode_auto, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_day_night_mode_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_day_night_mode_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_day_night_mode_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 3: IR LED Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_irled_mode_on, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_irled_mode_off, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_irled_mode_auto, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_irled_status_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_irled_status_error, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 4: Flip/Mirror Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_flip_mirror_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_flip_mirror_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 5: Auto Day/Night Config Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_auto_config_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_set_auto_config_null_params, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_auto_config_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_get_auto_config_null_params, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 6: VPSS Conversion Helper Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_convert_brightness_to_vpss, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_convert_contrast_to_vpss, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_convert_saturation_to_vpss, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_convert_sharpness_to_vpss, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_convert_hue_to_vpss, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 7: Validation Helper Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_validate_settings_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_validate_settings_invalid_brightness, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_validate_settings_invalid_range, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 8: Bulk Update Helper Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_bulk_update_validation_cache, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_optimized_batch_update_no_changes, setup_imaging_service_tests, teardown_imaging_service_tests),

    /* Section 9: Operation Handler Tests */
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_success, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_null_operation, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_null_request, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_null_response, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_unknown_operation, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_operation_handler_not_initialized, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_handle_get_imaging_settings, setup_imaging_service_tests, teardown_imaging_service_tests),
    cmocka_unit_test_setup_teardown(test_unit_imaging_handle_set_imaging_settings, setup_imaging_service_tests, teardown_imaging_service_tests),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
