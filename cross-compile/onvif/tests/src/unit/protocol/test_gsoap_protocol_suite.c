/**
 * @file test_gsoap_protocol_suite.c
 * @brief gSOAP protocol test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>

#include "cmocka_wrapper.h"

// Forward declarations for suite setup/teardown
// gsoap_core_suite_setup and gsoap_core_suite_teardown are now in utils/test_gsoap_utils.h

// Forward declarations from test_onvif_gsoap_core.c
void test_unit_onvif_gsoap_init(void** state);
void test_unit_onvif_gsoap_init_null(void** state);
void test_unit_onvif_gsoap_cleanup(void** state);
void test_unit_onvif_gsoap_parse_invalid_xml(void** state);
void test_unit_onvif_gsoap_parse_invalid_namespace(void** state);
void test_unit_onvif_gsoap_parse_missing_required_param(void** state);
void test_unit_onvif_gsoap_parse_without_initialization(void** state);

// Context management tests (Task 83)
void test_unit_onvif_gsoap_reset_success(void** state);
void test_unit_onvif_gsoap_reset_null(void** state);
void test_unit_onvif_gsoap_reset_after_parsing(void** state);
void test_unit_onvif_gsoap_init_request_parsing_success(void** state);
void test_unit_onvif_gsoap_init_request_parsing_null_context(void** state);
void test_unit_onvif_gsoap_init_request_parsing_null_xml(void** state);
void test_unit_onvif_gsoap_init_request_parsing_zero_size(void** state);

// Error handling tests (Task 84)
void test_unit_onvif_gsoap_set_error_success(void** state);
void test_unit_onvif_gsoap_set_error_null_message(void** state);
void test_unit_onvif_gsoap_get_detailed_error_success(void** state);
void test_unit_onvif_gsoap_get_detailed_error_null_outputs(void** state);
void test_unit_onvif_gsoap_has_error_true(void** state);
void test_unit_onvif_gsoap_has_error_false(void** state);
void test_unit_onvif_gsoap_get_error_with_message(void** state);
void test_unit_onvif_gsoap_get_error_no_message(void** state);

// Parsing workflow tests (Task 85)
void test_unit_onvif_gsoap_validate_and_begin_parse_success(void** state);
void test_unit_onvif_gsoap_validate_and_begin_parse_null_context(void** state);
void test_unit_onvif_gsoap_validate_and_begin_parse_null_output(void** state);
void test_unit_onvif_gsoap_validate_and_begin_parse_not_initialized(void** state);
void test_unit_onvif_gsoap_finalize_parse_success(void** state);
void test_unit_onvif_gsoap_finalize_parse_null_context(void** state);

// Forward declarations from test_onvif_gsoap_media.c
void test_unit_onvif_gsoap_parse_get_profiles(void** state);
void test_unit_onvif_gsoap_parse_get_stream_uri(void** state);
void test_unit_onvif_gsoap_parse_create_profile(void** state);
void test_unit_onvif_gsoap_parse_delete_profile(void** state);
void test_unit_onvif_gsoap_parse_set_video_source_config(void** state);
void test_unit_onvif_gsoap_parse_set_video_encoder_config(void** state);

// Forward declarations from test_onvif_gsoap_ptz.c
void test_unit_onvif_gsoap_parse_get_nodes(void** state);
void test_unit_onvif_gsoap_parse_absolute_move(void** state);
void test_unit_onvif_gsoap_parse_absolute_move_no_speed(void** state);
void test_unit_onvif_gsoap_parse_get_presets(void** state);
void test_unit_onvif_gsoap_parse_set_preset(void** state);
void test_unit_onvif_gsoap_parse_goto_preset(void** state);
void test_unit_onvif_gsoap_parse_remove_preset(void** state);

// Forward declarations from test_onvif_gsoap_device.c
void test_unit_onvif_gsoap_parse_get_device_information(void** state);
void test_unit_onvif_gsoap_parse_get_capabilities(void** state);
void test_unit_onvif_gsoap_parse_get_system_date_and_time(void** state);
void test_unit_onvif_gsoap_parse_system_reboot(void** state);

// Forward declarations from test_onvif_gsoap_imaging.c
void test_unit_onvif_gsoap_parse_get_imaging_settings(void** state);
void test_unit_onvif_gsoap_parse_set_imaging_settings(void** state);

/* ============================================================================
 * Main Test Suite Getter
 * ============================================================================ */

/**
 * @brief Get gSOAP protocol unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 *
 * @note This function returns all gSOAP protocol tests in a single suite.
 *       Tests are organized by module but run as one cohesive test suite.
 */
const struct CMUnitTest* get_gsoap_protocol_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    // Core tests
    cmocka_unit_test(test_unit_onvif_gsoap_init),
    cmocka_unit_test(test_unit_onvif_gsoap_init_null),
    cmocka_unit_test(test_unit_onvif_gsoap_cleanup),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_invalid_xml),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_invalid_namespace),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_missing_required_param),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_without_initialization),

    // Context management tests (Task 83)
    cmocka_unit_test(test_unit_onvif_gsoap_reset_success),
    cmocka_unit_test(test_unit_onvif_gsoap_reset_null),
    cmocka_unit_test(test_unit_onvif_gsoap_reset_after_parsing),
    cmocka_unit_test(test_unit_onvif_gsoap_init_request_parsing_success),
    cmocka_unit_test(test_unit_onvif_gsoap_init_request_parsing_null_context),
    cmocka_unit_test(test_unit_onvif_gsoap_init_request_parsing_null_xml),
    cmocka_unit_test(test_unit_onvif_gsoap_init_request_parsing_zero_size),

    // Error handling tests (Task 84)
    cmocka_unit_test(test_unit_onvif_gsoap_set_error_success),
    cmocka_unit_test(test_unit_onvif_gsoap_set_error_null_message),
    cmocka_unit_test(test_unit_onvif_gsoap_get_detailed_error_success),
    cmocka_unit_test(test_unit_onvif_gsoap_get_detailed_error_null_outputs),
    cmocka_unit_test(test_unit_onvif_gsoap_has_error_true),
    cmocka_unit_test(test_unit_onvif_gsoap_has_error_false),
    cmocka_unit_test(test_unit_onvif_gsoap_get_error_with_message),
    cmocka_unit_test(test_unit_onvif_gsoap_get_error_no_message),

    // Parsing workflow tests (Task 85)
    cmocka_unit_test(test_unit_onvif_gsoap_validate_and_begin_parse_success),
    cmocka_unit_test(test_unit_onvif_gsoap_validate_and_begin_parse_null_context),
    cmocka_unit_test(test_unit_onvif_gsoap_validate_and_begin_parse_null_output),
    cmocka_unit_test(test_unit_onvif_gsoap_validate_and_begin_parse_not_initialized),
    cmocka_unit_test(test_unit_onvif_gsoap_finalize_parse_success),
    cmocka_unit_test(test_unit_onvif_gsoap_finalize_parse_null_context),

    // Media tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_profiles),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_stream_uri),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_create_profile),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_delete_profile),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_video_source_config),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_video_encoder_config),

    // PTZ tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_nodes),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_absolute_move),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_absolute_move_no_speed),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_presets),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_preset),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_goto_preset),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_remove_preset),

    // Device tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_device_information),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_capabilities),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_system_date_and_time),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_system_reboot),

    // Imaging tests
    cmocka_unit_test(test_unit_onvif_gsoap_parse_get_imaging_settings),
    cmocka_unit_test(test_unit_onvif_gsoap_parse_set_imaging_settings),
  };

  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
