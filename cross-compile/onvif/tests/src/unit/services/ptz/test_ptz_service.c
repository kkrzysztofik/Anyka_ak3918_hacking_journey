/**
 * @file test_ptz_service.c
 * @brief PTZ service tests with common pattern elimination
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "mocks/config_mock.h"
#include "mocks/platform_mock.h"
#include "platform/platform_common.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

// Test constants
#define TEST_PTZ_MAX_PRESETS            10
#define TEST_PTZ_DEFAULT_TIMEOUT_MS     10000
#define TEST_PTZ_DEFAULT_PAN_TILT_SPEED 0.5F
#define TEST_PTZ_DEFAULT_ZOOM_SPEED     0.0F
#define TEST_PTZ_POSITION_PAN           45
#define TEST_PTZ_POSITION_TILT          30
#define TEST_PTZ_FLOAT_TOLERANCE        0.001F
#define TEST_PTZ_TOKEN_BUFFER_SIZE      64

// Test data
static const char* const TEST_PROFILE_TOKEN = "Profile1";
static const char* const TEST_NODE_TOKEN = "PTZNode0";
static const char* const TEST_CONFIG_TOKEN = "PTZConfig0";

static const struct ptz_vector TEST_POSITION = {
  .pan_tilt = {.x = 0.5F, .y = 0.3F},
  .zoom = 0.0F,
  .space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"};

static const struct ptz_speed TEST_SPEED = {.pan_tilt = {.x = 0.5F, .y = 0.5F}, .zoom = 0.0F};

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

/**
 * @brief Setup function for PTZ tests
 * @note CMocka handles state management automatically via will_return() and expect_*()
 */
static int setup_ptz_tests(void** state) {
  (void)state;
  config_mock_use_real_function(false);
  return 0;
}

/**
 * @brief Helper function to set up common mock expectations for load_ptz_presets_from_config
 */
static void setup_load_presets_mock_expectations(void) {
  // Mock load_ptz_presets_from_config calls for all 4 profiles
  expect_function_call(__wrap_config_runtime_get_ptz_profile_presets);
  expect_value(__wrap_config_runtime_get_ptz_profile_presets, profile_index, 0);
  expect_any(__wrap_config_runtime_get_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_get_ptz_profile_presets, ONVIF_SUCCESS);

  expect_function_call(__wrap_config_runtime_get_ptz_profile_presets);
  expect_value(__wrap_config_runtime_get_ptz_profile_presets, profile_index, 1);
  expect_any(__wrap_config_runtime_get_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_get_ptz_profile_presets, ONVIF_SUCCESS);

  expect_function_call(__wrap_config_runtime_get_ptz_profile_presets);
  expect_value(__wrap_config_runtime_get_ptz_profile_presets, profile_index, 2);
  expect_any(__wrap_config_runtime_get_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_get_ptz_profile_presets, ONVIF_SUCCESS);

  expect_function_call(__wrap_config_runtime_get_ptz_profile_presets);
  expect_value(__wrap_config_runtime_get_ptz_profile_presets, profile_index, 3);
  expect_any(__wrap_config_runtime_get_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_get_ptz_profile_presets, ONVIF_SUCCESS);
}

/**
 * @brief Teardown function for PTZ tests
 * @note CMocka handles cleanup automatically
 */
static int teardown_ptz_tests(void** state) {
  (void)state;
  // Reset preset state so each test starts fresh
  onvif_ptz_reset_presets();
  return 0;
}

/* ============================================================================
 * NULL Parameter Test Wrappers
 * ============================================================================ */

/**
 * @brief Wrapper for onvif_ptz_get_nodes NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_get_nodes_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct ptz_node* nodes = NULL;
  int count = 0;
  int result;

  switch (test_config->param_index) {
  case 0: // NULL nodes parameter
    result = onvif_ptz_get_nodes(NULL, &count);
    break;
  case 1: // NULL count parameter
    result = onvif_ptz_get_nodes(&nodes, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_get_node NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_get_node_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct ptz_node node;
  memset(&node, 0, sizeof(node));
  int result;

  switch (test_config->param_index) {
  case 0: // NULL token parameter
    result = onvif_ptz_get_node(NULL, &node);
    break;
  case 1: // NULL node parameter
    result = onvif_ptz_get_node(TEST_NODE_TOKEN, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_get_configuration NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_get_configuration_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct ptz_configuration_ex config;
  memset(&config, 0, sizeof(config));
  int result;

  switch (test_config->param_index) {
  case 0: // NULL token parameter
    result = onvif_ptz_get_configuration(NULL, &config);
    break;
  case 1: // NULL config parameter
    result = onvif_ptz_get_configuration(TEST_CONFIG_TOKEN, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_get_status NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_get_status_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct ptz_status status;
  memset(&status, 0, sizeof(status));
  int result;

  switch (test_config->param_index) {
  case 0: // NULL token parameter
    result = onvif_ptz_get_status(NULL, &status);
    break;
  case 1: // NULL status parameter
    result = onvif_ptz_get_status(TEST_PROFILE_TOKEN, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_absolute_move NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_absolute_move_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  int result;

  switch (test_config->param_index) {
  case 0: // NULL token parameter
    result = onvif_ptz_absolute_move(NULL, &TEST_POSITION, &TEST_SPEED);
    break;
  case 1: // NULL position parameter
    result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, NULL, &TEST_SPEED);
    break;
  case 2: // NULL speed parameter (should succeed with default speed)
    // When speed is NULL, the function uses default speed and calls the adapter
    // PTZ implementation converts: pan = x * 180, tilt = y * 90
    expect_function_call(__wrap_ptz_adapter_absolute_move);
    expect_value(__wrap_ptz_adapter_absolute_move, pan_degrees, 90);  // 0.5 * 180 = 90
    expect_value(__wrap_ptz_adapter_absolute_move, tilt_degrees, 27); // 0.3 * 90 = 27
    expect_value(__wrap_ptz_adapter_absolute_move, move_speed, 50);   // Default speed
    will_return(__wrap_ptz_adapter_absolute_move, PLATFORM_SUCCESS);
    result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &TEST_POSITION, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_get_presets NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_get_presets_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  struct ptz_preset* preset_list = NULL;
  int count = 0;
  int result;

  switch (test_config->param_index) {
  case 0: // NULL token parameter
    result = onvif_ptz_get_presets(NULL, &preset_list, &count);
    break;
  case 1: // NULL preset_list parameter
    result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, NULL, &count);
    break;
  case 2: // NULL count parameter
    result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, &preset_list, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_set_preset NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_set_preset_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  char output_token[TEST_PTZ_TOKEN_BUFFER_SIZE] = {0};
  int result;

  // Set up mocks for case 3 (valid call with NULL input_preset_token)
  if (test_config->param_index == 3) {
    setup_load_presets_mock_expectations();

    // Mock ptz_adapter_get_status (used by onvif_ptz_get_status) - returns error so no need for
    // position values
    expect_function_call(__wrap_ptz_adapter_get_status);
    expect_any(__wrap_ptz_adapter_get_status, status);
    will_return(__wrap_ptz_adapter_get_status, 0);              // h_pos_deg
    will_return(__wrap_ptz_adapter_get_status, 0);              // v_pos_deg
    will_return(__wrap_ptz_adapter_get_status, 0);              // h_speed
    will_return(__wrap_ptz_adapter_get_status, 0);              // v_speed
    will_return(__wrap_ptz_adapter_get_status, PLATFORM_ERROR); // function return

    // Mock config_runtime_set_ptz_profile_presets (called first)
    expect_function_call(__wrap_config_runtime_set_ptz_profile_presets);
    expect_any(__wrap_config_runtime_set_ptz_profile_presets, profile_index);
    expect_any(__wrap_config_runtime_set_ptz_profile_presets, presets);
    will_return(__wrap_config_runtime_set_ptz_profile_presets, ONVIF_SUCCESS);

    // Mock ptz_adapter_set_preset (called after config save)
    expect_function_call(__wrap_ptz_adapter_set_preset);
    expect_any(__wrap_ptz_adapter_set_preset, name);
    expect_any(__wrap_ptz_adapter_set_preset, preset_id);
    will_return(__wrap_ptz_adapter_set_preset, PLATFORM_SUCCESS);
  }

  switch (test_config->param_index) {
  case 0: // NULL profile_token parameter
    result = onvif_ptz_set_preset(NULL, "TestPreset", NULL, output_token, sizeof(output_token));
    break;
  case 1: // NULL preset_name parameter
    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, NULL, NULL, output_token, sizeof(output_token));
    break;
  case 2: // NULL output_preset_token parameter
    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, "TestPreset", NULL, NULL, sizeof(output_token));
    break;
  case 3: // NULL input_preset_token parameter (should succeed)
    result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, "TestPreset", NULL, output_token,
                                  sizeof(output_token));
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_remove_preset NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_remove_preset_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  int result;

  switch (test_config->param_index) {
  case 0: // NULL profile_token parameter
    result = onvif_ptz_remove_preset(NULL, "TestPresetToken");
    break;
  case 1: // NULL preset_token parameter
    result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/**
 * @brief Wrapper for onvif_ptz_goto_preset NULL parameter testing
 * @param state Test state (unused)
 * @param test_config Configuration for which parameter to test
 */
void test_ptz_goto_preset_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;

  int result;

  // Set up mocks for case 2 (valid call with NULL speed)
  if (test_config->param_index == 2) {
    setup_load_presets_mock_expectations();
    // Preset won't exist, so no need to mock ptz_adapter_absolute_move
  }

  switch (test_config->param_index) {
  case 0: // NULL profile_token parameter
    result = onvif_ptz_goto_preset(NULL, "TestPresetToken", &TEST_SPEED);
    break;
  case 1: // NULL preset_token parameter
    result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, NULL, &TEST_SPEED);
    break;
  case 2: // NULL speed parameter (should succeed with default)
    result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, "TestPresetToken", NULL);
    break;
  default:
    fail_msg("Invalid parameter index: %d", test_config->param_index);
    return;
  }

  assert_int_equal(result, test_config->expected_result);
}

/* ============================================================================
 * Refactored NULL Parameter Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_nodes function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_nodes_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("nodes parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("count parameter", 1, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_get_nodes", test_ptz_get_nodes_with_null, tests, 2);
}

/**
 * @brief Test PTZ get_node function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_node_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("node parameter", 1, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_get_node", test_ptz_get_node_with_null, tests, 2);
}

/**
 * @brief Test PTZ get_configuration function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_configuration_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("config parameter", 1, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_get_configuration",
                              test_ptz_get_configuration_with_null, tests, 2);
}

/**
 * @brief Test PTZ get_status function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_status_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("status parameter", 1, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_get_status", test_ptz_get_status_with_null, tests,
                              2);
}

/**
 * @brief Test PTZ absolute_move function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_absolute_move_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("position parameter", 1, ONVIF_ERROR_NULL),
    test_helper_create_null_test("speed parameter (uses default)", 2, ONVIF_SUCCESS),
  };

  test_helper_null_parameters(state, "onvif_ptz_absolute_move", test_ptz_absolute_move_with_null,
                              tests, 3);
}

/**
 * @brief Test PTZ get_presets function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_presets_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("preset_list parameter", 1, ONVIF_ERROR_NULL),
    test_helper_create_null_test("count parameter", 2, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_get_presets", test_ptz_get_presets_with_null, tests,
                              3);
}

/**
 * @brief Test PTZ set_preset function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_set_preset_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("profile_token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("preset_name parameter", 1, ONVIF_ERROR_NULL),
    test_helper_create_null_test("output_preset_token parameter", 2, ONVIF_ERROR_NULL),
    test_helper_create_null_test("input_preset_token parameter (optional)", 3, ONVIF_SUCCESS),
  };

  test_helper_null_parameters(state, "onvif_ptz_set_preset", test_ptz_set_preset_with_null, tests,
                              4);
}

/**
 * @brief Test PTZ remove_preset function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_remove_preset_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("profile_token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("preset_token parameter", 1, ONVIF_ERROR_NULL),
  };

  test_helper_null_parameters(state, "onvif_ptz_remove_preset", test_ptz_remove_preset_with_null,
                              tests, 2);
}

/**
 * @brief Test PTZ goto_preset function with NULL parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_goto_preset_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("profile_token parameter", 0, ONVIF_ERROR_NULL),
    test_helper_create_null_test("preset_token parameter", 1, ONVIF_ERROR_NULL),
    test_helper_create_null_test("speed parameter (optional)", 2,
                                 ONVIF_ERROR_NOT_FOUND), // Preset doesn't exist
  };

  test_helper_null_parameters(state, "onvif_ptz_goto_preset", test_ptz_goto_preset_with_null, tests,
                              3);
}

/* ============================================================================
 * Success Case Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_nodes function with valid parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_get_nodes_success(void** state) {
  (void)state;

  struct ptz_node* nodes = NULL;
  int count = 0;

  int result = onvif_ptz_get_nodes(&nodes, &count);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(nodes);
  assert_int_equal(count, 1);
  assert_string_equal(nodes[0].token, "PTZNode0");
  assert_string_equal(nodes[0].name, "PTZ Node");
  assert_int_equal(nodes[0].maximum_number_of_presets, 4); // PTZ_MAX_PRESETS
  assert_int_equal(nodes[0].home_supported, 1);
}

/**
 * @brief Test PTZ get_node function with valid token
 * @param state Test state (unused)
 */
void test_unit_ptz_get_node_success(void** state) {
  (void)state;

  struct ptz_node node;
  memset(&node, 0, sizeof(node));

  int result = onvif_ptz_get_node(TEST_NODE_TOKEN, &node);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(node.token, "PTZNode0");
  assert_string_equal(node.name, "PTZ Node");
  assert_int_equal(node.maximum_number_of_presets, 4); // PTZ_MAX_PRESETS
  assert_int_equal(node.home_supported, 1);
}

/**
 * @brief Test PTZ set_preset function with valid parameters (auto-generate token)
 * @param state Test state (unused)
 */
void test_unit_ptz_set_preset_success(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  // Mock ptz_adapter_get_status (used by onvif_ptz_get_status) - returns error so no need for
  // position values
  expect_function_call(__wrap_ptz_adapter_get_status);
  expect_any(__wrap_ptz_adapter_get_status, status);
  will_return(__wrap_ptz_adapter_get_status, 0);              // h_pos_deg
  will_return(__wrap_ptz_adapter_get_status, 0);              // v_pos_deg
  will_return(__wrap_ptz_adapter_get_status, 0);              // h_speed
  will_return(__wrap_ptz_adapter_get_status, 0);              // v_speed
  will_return(__wrap_ptz_adapter_get_status, PLATFORM_ERROR); // function return

  // Mock config_runtime_set_ptz_profile_presets (called first)
  expect_function_call(__wrap_config_runtime_set_ptz_profile_presets);
  expect_any(__wrap_config_runtime_set_ptz_profile_presets, profile_index);
  expect_any(__wrap_config_runtime_set_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_set_ptz_profile_presets, ONVIF_SUCCESS);

  // Mock ptz_adapter_set_preset (called after config save)
  expect_function_call(__wrap_ptz_adapter_set_preset);
  expect_any(__wrap_ptz_adapter_set_preset, name);
  expect_any(__wrap_ptz_adapter_set_preset, preset_id);
  will_return(__wrap_ptz_adapter_set_preset, PLATFORM_SUCCESS);

  char output_token[TEST_PTZ_TOKEN_BUFFER_SIZE] = {0};
  int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, "TestPreset", NULL, output_token,
                                    sizeof(output_token));

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(output_token, ""); // Token should be populated
}

/**
 * @brief Test PTZ set_preset function with specific input token (new preset)
 * @param state Test state (unused)
 */
void test_unit_ptz_set_preset_with_input_token_new(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  // Mock ptz_adapter_get_status (used by onvif_ptz_get_status) - returns error so no need for
  // position values
  expect_function_call(__wrap_ptz_adapter_get_status);
  expect_any(__wrap_ptz_adapter_get_status, status);
  will_return(__wrap_ptz_adapter_get_status, 0);              // h_pos_deg
  will_return(__wrap_ptz_adapter_get_status, 0);              // v_pos_deg
  will_return(__wrap_ptz_adapter_get_status, 0);              // h_speed
  will_return(__wrap_ptz_adapter_get_status, 0);              // v_speed
  will_return(__wrap_ptz_adapter_get_status, PLATFORM_ERROR); // function return

  // Mock config_runtime_set_ptz_profile_presets (called first)
  expect_function_call(__wrap_config_runtime_set_ptz_profile_presets);
  expect_any(__wrap_config_runtime_set_ptz_profile_presets, profile_index);
  expect_any(__wrap_config_runtime_set_ptz_profile_presets, presets);
  will_return(__wrap_config_runtime_set_ptz_profile_presets, ONVIF_SUCCESS);

  // Mock ptz_adapter_set_preset (called after config save)
  expect_function_call(__wrap_ptz_adapter_set_preset);
  expect_any(__wrap_ptz_adapter_set_preset, name);
  expect_any(__wrap_ptz_adapter_set_preset, preset_id);
  will_return(__wrap_ptz_adapter_set_preset, PLATFORM_SUCCESS);

  char output_token[TEST_PTZ_TOKEN_BUFFER_SIZE] = {0};
  int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, "TestPreset", "CustomToken123",
                                    output_token, sizeof(output_token));

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(output_token, "CustomToken123");
}

/**
 * @brief Test PTZ remove_preset function with valid parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_remove_preset_success(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  // Note: Without populating presets, this will return NOT_FOUND
  // This test verifies the function doesn't crash with valid parameters
  int result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, "TestPresetToken");

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test PTZ goto_preset function with valid parameters
 * @param state Test state (unused)
 */
void test_unit_ptz_goto_preset_success(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  // Note: Without populating presets, this will return NOT_FOUND
  // This test verifies the function doesn't crash with valid parameters
  int result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, "TestPresetToken", &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/* ============================================================================
 * Error Condition Tests
 * ============================================================================ */

/**
 * @brief Test PTZ set_preset function with invalid profile token
 * @param state Test state (unused)
 */
void test_unit_ptz_set_preset_invalid_profile(void** state) {
  (void)state;

  char output_token[TEST_PTZ_TOKEN_BUFFER_SIZE] = {0};
  int result =
    onvif_ptz_set_preset("InvalidProfile", "TestPreset", NULL, output_token, sizeof(output_token));

  assert_int_equal(result, ONVIF_ERROR_INVALID_PARAMETER);
}

/**
 * @brief Test PTZ remove_preset function with preset not found
 * @param state Test state (unused)
 */
void test_unit_ptz_remove_preset_not_found(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  int result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, "NonExistentToken");

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test PTZ goto_preset function with preset not found
 * @param state Test state (unused)
 */
void test_unit_ptz_goto_preset_not_found(void** state) {
  (void)state;

  // Set up common mock expectations for load_ptz_presets_from_config
  setup_load_presets_mock_expectations();

  int result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, "NonExistentToken", &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest ptz_tests[] = {
  // NULL Parameter Tests (Refactored)
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_nodes_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_node_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_configuration_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_status_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_absolute_move_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_presets_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_set_preset_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_remove_preset_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_goto_preset_null_params, setup_ptz_tests,
                                  teardown_ptz_tests),

  // Success Case Tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_nodes_success, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_get_node_success, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_set_preset_success, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_set_preset_with_input_token_new, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_remove_preset_success, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_goto_preset_success, setup_ptz_tests,
                                  teardown_ptz_tests),

  // Error Condition Tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_set_preset_invalid_profile, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_remove_preset_not_found, setup_ptz_tests,
                                  teardown_ptz_tests),
  cmocka_unit_test_setup_teardown(test_unit_ptz_goto_preset_not_found, setup_ptz_tests,
                                  teardown_ptz_tests),
};

/**
 * @brief Get unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_ptz_service_unit_tests(size_t* count) {
  *count = sizeof(ptz_tests) / sizeof(ptz_tests[0]);
  return ptz_tests;
}
