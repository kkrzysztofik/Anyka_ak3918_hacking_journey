/**
 * @file test_imaging_service.h
 * @brief Imaging service comprehensive unit tests - forward declarations
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_IMAGING_SERVICE_H
#define TEST_IMAGING_SERVICE_H

#include <stddef.h>

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

int setup_imaging_service_tests(void** state);
int teardown_imaging_service_tests(void** state);

/* ============================================================================
 * Section 1: Get/Set Settings Tests
 * ============================================================================ */

void test_unit_imaging_get_settings_success(void** state);
void test_unit_imaging_get_settings_null_params(void** state);
void test_unit_imaging_set_settings_success(void** state);
void test_unit_imaging_set_settings_null_params(void** state);
void test_unit_imaging_set_settings_invalid_brightness(void** state);
void test_unit_imaging_set_settings_invalid_contrast(void** state);
void test_unit_imaging_set_settings_not_initialized(void** state);

/* ============================================================================
 * Section 2: Day/Night Mode Tests
 * ============================================================================ */

void test_unit_imaging_set_day_night_mode_day(void** state);
void test_unit_imaging_set_day_night_mode_night(void** state);
void test_unit_imaging_set_day_night_mode_auto(void** state);
void test_unit_imaging_set_day_night_mode_not_initialized(void** state);
void test_unit_imaging_get_day_night_mode_success(void** state);
void test_unit_imaging_get_day_night_mode_not_initialized(void** state);

/* ============================================================================
 * Section 3: IR LED Tests
 * ============================================================================ */

void test_unit_imaging_set_irled_mode_on(void** state);
void test_unit_imaging_set_irled_mode_off(void** state);
void test_unit_imaging_set_irled_mode_auto(void** state);
void test_unit_imaging_get_irled_status_success(void** state);
void test_unit_imaging_get_irled_status_error(void** state);

/* ============================================================================
 * Section 4: Flip/Mirror Tests
 * ============================================================================ */

void test_unit_imaging_set_flip_mirror_success(void** state);
void test_unit_imaging_set_flip_mirror_not_initialized(void** state);

/* ============================================================================
 * Section 5: Auto Day/Night Config Tests
 * ============================================================================ */

void test_unit_imaging_set_auto_config_success(void** state);
void test_unit_imaging_set_auto_config_null_params(void** state);
void test_unit_imaging_get_auto_config_success(void** state);
void test_unit_imaging_get_auto_config_null_params(void** state);

/* ============================================================================
 * Section 6: VPSS Conversion Helper Tests
 * ============================================================================ */

void test_unit_imaging_convert_brightness_to_vpss(void** state);
void test_unit_imaging_convert_contrast_to_vpss(void** state);
void test_unit_imaging_convert_saturation_to_vpss(void** state);
void test_unit_imaging_convert_sharpness_to_vpss(void** state);
void test_unit_imaging_convert_hue_to_vpss(void** state);

/* ============================================================================
 * Section 7: Validation Helper Tests
 * ============================================================================ */

void test_unit_imaging_validate_settings_success(void** state);
void test_unit_imaging_validate_settings_invalid_brightness(void** state);
void test_unit_imaging_validate_settings_invalid_range(void** state);

/* ============================================================================
 * Section 8: Bulk Update Helper Tests
 * ============================================================================ */

void test_unit_imaging_bulk_update_validation_cache(void** state);
void test_unit_imaging_optimized_batch_update_no_changes(void** state);

/* ============================================================================
 * Section 9: Operation Handler Tests
 * ============================================================================ */

void test_unit_imaging_operation_handler_success(void** state);
void test_unit_imaging_operation_handler_null_operation(void** state);
void test_unit_imaging_operation_handler_null_request(void** state);
void test_unit_imaging_operation_handler_null_response(void** state);
void test_unit_imaging_operation_handler_unknown_operation(void** state);
void test_unit_imaging_operation_handler_not_initialized(void** state);
void test_unit_imaging_handle_get_imaging_settings(void** state);
void test_unit_imaging_handle_set_imaging_settings(void** state);

#endif /* TEST_IMAGING_SERVICE_H */
