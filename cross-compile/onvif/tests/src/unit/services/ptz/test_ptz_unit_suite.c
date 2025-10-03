/**
 * @file test_ptz_unit_suite.c
 * @brief PTZ service unit test suite wrapper (consolidates all PTZ unit tests)
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_ptz_service.c
void test_unit_ptz_get_nodes_success(void** state);
void test_unit_ptz_get_nodes_null_nodes(void** state);
void test_unit_ptz_get_nodes_null_count(void** state);
void test_unit_ptz_get_node_success(void** state);
void test_unit_ptz_get_node_invalid_token(void** state);
void test_unit_ptz_get_node_null_token(void** state);
void test_unit_ptz_get_node_null_node(void** state);
void test_unit_ptz_get_configuration_success(void** state);
void test_unit_ptz_get_configuration_null_token(void** state);
void test_unit_ptz_get_configuration_null_config(void** state);
void test_unit_ptz_get_status_success(void** state);
void test_unit_ptz_get_status_null_token(void** state);
void test_unit_ptz_get_status_null_status(void** state);
void test_unit_ptz_absolute_move_success(void** state);
void test_unit_ptz_absolute_move_null_token(void** state);
void test_unit_ptz_absolute_move_null_position(void** state);
void test_unit_ptz_absolute_move_null_speed(void** state);
void test_unit_ptz_relative_move_success(void** state);
void test_unit_ptz_relative_move_null_token(void** state);
void test_unit_ptz_relative_move_null_translation(void** state);
void test_unit_ptz_continuous_move_success(void** state);
void test_unit_ptz_continuous_move_null_token(void** state);
void test_unit_ptz_continuous_move_null_velocity(void** state);
void test_unit_ptz_stop_success(void** state);
void test_unit_ptz_stop_null_token(void** state);
void test_unit_ptz_goto_home_position_success(void** state);
void test_unit_ptz_goto_home_position_null_token(void** state);
void test_unit_ptz_set_home_position_success(void** state);
void test_unit_ptz_set_home_position_null_token(void** state);
void test_unit_ptz_get_presets_success(void** state);
void test_unit_ptz_get_presets_null_token(void** state);
void test_unit_ptz_get_presets_null_list(void** state);
void test_unit_ptz_get_presets_null_count(void** state);
void test_unit_ptz_set_preset_success(void** state);
void test_unit_ptz_set_preset_null_token(void** state);
void test_unit_ptz_set_preset_null_name(void** state);
void test_unit_ptz_set_preset_null_output_token(void** state);
void test_unit_ptz_goto_preset_success(void** state);
void test_unit_ptz_goto_preset_invalid_token(void** state);
void test_unit_ptz_goto_preset_null_token(void** state);
void test_unit_ptz_goto_preset_null_preset_token(void** state);
void test_unit_ptz_remove_preset_success(void** state);
void test_unit_ptz_remove_preset_invalid_token(void** state);
void test_unit_ptz_remove_preset_null_token(void** state);
void test_unit_ptz_remove_preset_null_preset_token(void** state);
void test_unit_ptz_adapter_init_success(void** state);
void test_unit_ptz_adapter_init_failure(void** state);
void test_unit_ptz_adapter_shutdown_success(void** state);
void test_unit_ptz_adapter_get_status_success(void** state);
void test_unit_ptz_adapter_get_status_null_status(void** state);
void test_unit_ptz_adapter_get_status_not_initialized(void** state);
void test_unit_ptz_adapter_absolute_move_success(void** state);
void test_unit_ptz_adapter_absolute_move_not_initialized(void** state);
void test_unit_ptz_adapter_relative_move_success(void** state);
void test_unit_ptz_adapter_relative_move_not_initialized(void** state);
void test_unit_ptz_adapter_continuous_move_success(void** state);
void test_unit_ptz_adapter_continuous_move_not_initialized(void** state);
void test_unit_ptz_adapter_stop_success(void** state);
void test_unit_ptz_adapter_stop_not_initialized(void** state);
void test_unit_ptz_adapter_set_preset_success(void** state);
void test_unit_ptz_adapter_set_preset_not_initialized(void** state);
void test_unit_ptz_adapter_goto_preset_success(void** state);
void test_unit_ptz_adapter_goto_preset_not_initialized(void** state);
void test_unit_ptz_service_init_success(void** state);
void test_unit_ptz_service_cleanup_success(void** state);
void test_unit_ptz_handle_operation_success(void** state);
void test_unit_ptz_handle_operation_null_operation(void** state);
void test_unit_ptz_handle_operation_null_request(void** state);
void test_unit_ptz_handle_operation_null_response(void** state);
void test_unit_ptz_handle_operation_unknown_operation(void** state);

/**
 * @brief Get PTZ service unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_ptz_service_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_ptz_get_nodes_success),
    cmocka_unit_test(test_unit_ptz_get_nodes_null_nodes),
    cmocka_unit_test(test_unit_ptz_get_nodes_null_count),
    cmocka_unit_test(test_unit_ptz_get_node_success),
    cmocka_unit_test(test_unit_ptz_get_node_invalid_token),
    cmocka_unit_test(test_unit_ptz_get_node_null_token),
    cmocka_unit_test(test_unit_ptz_get_node_null_node),
    cmocka_unit_test(test_unit_ptz_get_configuration_success),
    cmocka_unit_test(test_unit_ptz_get_configuration_null_token),
    cmocka_unit_test(test_unit_ptz_get_configuration_null_config),
    cmocka_unit_test(test_unit_ptz_get_status_success),
    cmocka_unit_test(test_unit_ptz_get_status_null_token),
    cmocka_unit_test(test_unit_ptz_get_status_null_status),
    cmocka_unit_test(test_unit_ptz_absolute_move_success),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_token),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_position),
    cmocka_unit_test(test_unit_ptz_absolute_move_null_speed),
    cmocka_unit_test(test_unit_ptz_relative_move_success),
    cmocka_unit_test(test_unit_ptz_relative_move_null_token),
    cmocka_unit_test(test_unit_ptz_relative_move_null_translation),
    cmocka_unit_test(test_unit_ptz_continuous_move_success),
    cmocka_unit_test(test_unit_ptz_continuous_move_null_token),
    cmocka_unit_test(test_unit_ptz_continuous_move_null_velocity),
    cmocka_unit_test(test_unit_ptz_stop_success),
    cmocka_unit_test(test_unit_ptz_stop_null_token),
    cmocka_unit_test(test_unit_ptz_goto_home_position_success),
    cmocka_unit_test(test_unit_ptz_goto_home_position_null_token),
    cmocka_unit_test(test_unit_ptz_set_home_position_success),
    cmocka_unit_test(test_unit_ptz_set_home_position_null_token),
    cmocka_unit_test(test_unit_ptz_get_presets_success),
    cmocka_unit_test(test_unit_ptz_get_presets_null_token),
    cmocka_unit_test(test_unit_ptz_get_presets_null_list),
    cmocka_unit_test(test_unit_ptz_get_presets_null_count),
    cmocka_unit_test(test_unit_ptz_set_preset_success),
    cmocka_unit_test(test_unit_ptz_set_preset_null_token),
    cmocka_unit_test(test_unit_ptz_set_preset_null_name),
    cmocka_unit_test(test_unit_ptz_set_preset_null_output_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_success),
    cmocka_unit_test(test_unit_ptz_goto_preset_invalid_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_null_token),
    cmocka_unit_test(test_unit_ptz_goto_preset_null_preset_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_success),
    cmocka_unit_test(test_unit_ptz_remove_preset_invalid_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_null_token),
    cmocka_unit_test(test_unit_ptz_remove_preset_null_preset_token),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}

/**
 * @brief Get PTZ adapter unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_ptz_adapter_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_ptz_adapter_init_success),
    cmocka_unit_test(test_unit_ptz_adapter_init_failure),
    cmocka_unit_test(test_unit_ptz_adapter_shutdown_success),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_success),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_null_status),
    cmocka_unit_test(test_unit_ptz_adapter_get_status_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_absolute_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_absolute_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_relative_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_relative_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_continuous_move_success),
    cmocka_unit_test(test_unit_ptz_adapter_continuous_move_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_stop_success),
    cmocka_unit_test(test_unit_ptz_adapter_stop_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_set_preset_success),
    cmocka_unit_test(test_unit_ptz_adapter_set_preset_not_initialized),
    cmocka_unit_test(test_unit_ptz_adapter_goto_preset_success),
    cmocka_unit_test(test_unit_ptz_adapter_goto_preset_not_initialized),
    cmocka_unit_test(test_unit_ptz_service_init_success),
    cmocka_unit_test(test_unit_ptz_service_cleanup_success),
    cmocka_unit_test(test_unit_ptz_handle_operation_success),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_operation),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_request),
    cmocka_unit_test(test_unit_ptz_handle_operation_null_response),
    cmocka_unit_test(test_unit_ptz_handle_operation_unknown_operation),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
