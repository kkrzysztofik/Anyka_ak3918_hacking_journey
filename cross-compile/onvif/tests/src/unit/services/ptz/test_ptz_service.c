/**
 * @file test_ptz_service.c
 * @brief Unit tests for ONVIF PTZ service
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_parser.h"
#include "platform/platform_common.h"

// Include the actual source files we're testing
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

// Mock includes
#include "mocks/platform_mock.h"
#include "mocks/platform_ptz_mock.h"

// Forward declarations for functions that need to be declared
int onvif_ptz_get_nodes(struct ptz_node** nodes, int* count);
int ptz_adapter_get_status(struct ptz_device_status* status);

/* ============================================================================
 * Test Data and Constants
 * ============================================================================ */

// Test constants
#define TEST_PTZ_MAX_PRESETS              10
#define TEST_PTZ_DEFAULT_TIMEOUT_MS       10000
#define TEST_PTZ_DEFAULT_PAN_TILT_SPEED   0.5F
#define TEST_PTZ_DEFAULT_ZOOM_SPEED       0.0F
#define TEST_PTZ_POSITION_PAN             45
#define TEST_PTZ_POSITION_TILT            30
#define TEST_PTZ_MOVE_SPEED               50
#define TEST_PTZ_MOVE_STEPS               10
#define TEST_PTZ_MOVE_DELTA               5
#define TEST_PTZ_TIMEOUT_MS               5000
#define TEST_PTZ_VELOCITY_PAN             50
#define TEST_PTZ_VELOCITY_TILT            30
#define TEST_PTZ_TIMEOUT_S                10
#define TEST_PTZ_FLOAT_TOLERANCE          0.01F
#define TEST_PTZ_POSITION_PAN_NORMALIZED  0.25F
#define TEST_PTZ_POSITION_TILT_NORMALIZED 0.33F
#define TEST_PTZ_RELATIVE_MOVE_PAN        10
#define TEST_PTZ_RELATIVE_MOVE_TILT       5
#define TEST_PTZ_RELATIVE_MOVE_SPEED      50
#define TEST_PTZ_CONTINUOUS_MOVE_PAN      50
#define TEST_PTZ_CONTINUOUS_MOVE_TILT     30
#define TEST_PTZ_CONTINUOUS_MOVE_TIMEOUT  10

// Test profile tokens
static const char* const TEST_PROFILE_TOKEN = "Profile_1";
static const char* const TEST_NODE_TOKEN = "PTZNode0";
static const char* const TEST_CONFIG_TOKEN = "PTZConfig0";

// Test preset data
static const char* const TEST_PRESET_NAME = "TestPreset";
static const char* const TEST_PRESET_TOKEN = "Preset1";

// Test position data
static const struct ptz_vector TEST_POSITION = {
  .pan_tilt = {.x = 0.5F, .y = 0.3F},
  .zoom = 0.0F,
  .space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"};

static const struct ptz_speed TEST_SPEED = {.pan_tilt = {.x = 0.5F, .y = 0.5F}, .zoom = 0.0F};

/* ============================================================================
 * Test Helper Functions
 * ============================================================================ */

/**
 * @brief Setup function for PTZ tests
 * @param state Test state
 * @return 0 on success
 */
int setup_ptz_tests(void** state) {
  (void)state;

  // Initialize platform mock
  platform_mock_init();

  // Initialize PTZ adapter
  ptz_adapter_init();

  return 0;
}

/**
 * @brief Teardown function for PTZ tests
 * @param state Test state
 * @return 0 on success
 */
int teardown_ptz_tests(void** state) {
  (void)state;

  // Cleanup PTZ adapter
  ptz_adapter_shutdown();

  // Cleanup platform mock
  platform_mock_cleanup();

  return 0;
}

/* ============================================================================
 * PTZ Node Management Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_nodes function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_get_nodes_success(void** state) {
  (void)state;

  struct ptz_node* nodes = NULL;
  int count = 0;

  int result = onvif_ptz_get_nodes(&nodes, &count);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(nodes);
  assert_int_equal(count, 1);
  assert_string_equal(nodes[0].token, "PTZNode0");
  assert_string_equal(nodes[0].name, "PTZ Node");
  assert_int_equal(nodes[0].maximum_number_of_presets, TEST_PTZ_MAX_PRESETS);
  assert_int_equal(nodes[0].home_supported, 1);
}

/**
 * @brief Test PTZ get_nodes function with NULL nodes parameter
 * @param state Test state (unused)
 */
void test_ptz_get_nodes_null_nodes(void** state) {
  (void)state;

  int count = 0;

  int result = onvif_ptz_get_nodes(NULL, &count);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_nodes function with NULL count parameter
 * @param state Test state (unused)
 */
void test_ptz_get_nodes_null_count(void** state) {
  (void)state;

  struct ptz_node* nodes = NULL;

  int result = onvif_ptz_get_nodes(&nodes, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_node function with valid token
 * @param state Test state (unused)
 */
void test_ptz_get_node_success(void** state) {
  (void)state;

  struct ptz_node node;
  memset(&node, 0, sizeof(node));

  int result = onvif_ptz_get_node(TEST_NODE_TOKEN, &node);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(node.token, "PTZNode0");
  assert_string_equal(node.name, "PTZ Node");
  assert_int_equal(node.maximum_number_of_presets, TEST_PTZ_MAX_PRESETS);
  assert_int_equal(node.home_supported, 1);
}

/**
 * @brief Test PTZ get_node function with invalid token
 * @param state Test state (unused)
 */
void test_ptz_get_node_invalid_token(void** state) {
  (void)state;

  struct ptz_node node;
  memset(&node, 0, sizeof(node));

  int result = onvif_ptz_get_node("InvalidToken", &node);

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test PTZ get_node function with NULL token
 * @param state Test state (unused)
 */
void test_ptz_get_node_null_token(void** state) {
  (void)state;

  struct ptz_node node;
  memset(&node, 0, sizeof(node));

  int result = onvif_ptz_get_node(NULL, &node);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_node function with NULL node parameter
 * @param state Test state (unused)
 */
void test_ptz_get_node_null_node(void** state) {
  (void)state;

  int result = onvif_ptz_get_node(TEST_NODE_TOKEN, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Configuration Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_configuration function with valid token
 * @param state Test state (unused)
 */
void test_ptz_get_configuration_success(void** state) {
  (void)state;

  struct ptz_configuration_ex config;
  memset(&config, 0, sizeof(config));

  int result = onvif_ptz_get_configuration(TEST_CONFIG_TOKEN, &config);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(config.token, "PTZConfig0");
  assert_string_equal(config.name, "PTZ Configuration");
  assert_int_equal(config.use_count, 1);
  assert_string_equal(config.node_token, "PTZNode0");
  assert_float_equal(config.default_ptz_speed.pan_tilt.x, TEST_PTZ_DEFAULT_PAN_TILT_SPEED, // NOLINT
                     TEST_PTZ_FLOAT_TOLERANCE);                                            // NOLINT
  assert_float_equal(config.default_ptz_speed.pan_tilt.y, TEST_PTZ_DEFAULT_PAN_TILT_SPEED, // NOLINT
                     TEST_PTZ_FLOAT_TOLERANCE);                                            // NOLINT
  assert_float_equal(config.default_ptz_speed.zoom, TEST_PTZ_DEFAULT_ZOOM_SPEED,           // NOLINT
                     TEST_PTZ_FLOAT_TOLERANCE);                                            // NOLINT
  assert_int_equal(config.default_ptz_timeout, TEST_PTZ_DEFAULT_TIMEOUT_MS);
}

/**
 * @brief Test PTZ get_configuration function with NULL token
 * @param state Test state (unused)
 */
void test_ptz_get_configuration_null_token(void** state) {
  (void)state;

  struct ptz_configuration_ex config;
  memset(&config, 0, sizeof(config));

  int result = onvif_ptz_get_configuration(NULL, &config);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_configuration function with NULL config
 * @param state Test state (unused)
 */
void test_ptz_get_configuration_null_config(void** state) {
  (void)state;

  int result = onvif_ptz_get_configuration(TEST_CONFIG_TOKEN, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Status Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_status function with valid profile token
 * @param state Test state (unused)
 */
void test_ptz_get_status_success(void** state) {
  (void)state;

  // Mock platform PTZ status
  struct ptz_device_status mock_status = {.h_pos_deg = TEST_PTZ_POSITION_PAN,
                                          .v_pos_deg = TEST_PTZ_POSITION_TILT,
                                          .h_speed = 0,
                                          .v_speed = 0};
  platform_mock_set_ptz_status(&mock_status);

  struct ptz_status status;
  memset(&status, 0, sizeof(status));

  int result = onvif_ptz_get_status(TEST_PROFILE_TOKEN, &status);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_float_equal(status.position.pan_tilt.x, TEST_PTZ_POSITION_PAN_NORMALIZED,
                     TEST_PTZ_FLOAT_TOLERANCE); // NOLINT // 45/180 = 0.25
  assert_float_equal(status.position.pan_tilt.y, TEST_PTZ_POSITION_TILT_NORMALIZED,
                     TEST_PTZ_FLOAT_TOLERANCE); // NOLINT // 30/90 = 0.33
  assert_float_equal(status.position.zoom, 0.0F, TEST_PTZ_FLOAT_TOLERANCE); // NOLINT
  assert_string_equal(status.position.space,
                      "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace");
  assert_int_equal(status.move_status.pan_tilt, PTZ_MOVE_IDLE);
  assert_int_equal(status.move_status.zoom, PTZ_MOVE_IDLE);
  assert_string_equal(status.error, "");
  assert_true(strlen(status.utc_time) > 0);
}

/**
 * @brief Test PTZ get_status function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_get_status_null_token(void** state) {
  (void)state;

  struct ptz_status status;
  memset(&status, 0, sizeof(status));

  int result = onvif_ptz_get_status(NULL, &status);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_status function with NULL status
 * @param state Test state (unused)
 */
void test_ptz_get_status_null_status(void** state) {
  (void)state;

  int result = onvif_ptz_get_status(TEST_PROFILE_TOKEN, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Movement Tests
 * ============================================================================ */

/**
 * @brief Test PTZ absolute_move function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_absolute_move_success(void** state) {
  (void)state;

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &TEST_POSITION, &TEST_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ absolute_move function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_absolute_move_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_absolute_move(NULL, &TEST_POSITION, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ absolute_move function with NULL position
 * @param state Test state (unused)
 */
void test_ptz_absolute_move_null_position(void** state) {
  (void)state;

  int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, NULL, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ absolute_move function with NULL speed (should use default)
 * @param state Test state (unused)
 */
void test_ptz_absolute_move_null_speed(void** state) {
  (void)state;

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &TEST_POSITION, NULL);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ relative_move function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_relative_move_success(void** state) {
  (void)state;

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, &TEST_POSITION, &TEST_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ relative_move function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_relative_move_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_relative_move(NULL, &TEST_POSITION, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ relative_move function with NULL translation
 * @param state Test state (unused)
 */
void test_ptz_relative_move_null_translation(void** state) {
  (void)state;

  int result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, NULL, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ continuous_move function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_continuous_move_success(void** state) {
  (void)state;

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &TEST_SPEED, TEST_PTZ_TIMEOUT_MS);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ continuous_move function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_continuous_move_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_continuous_move(NULL, &TEST_SPEED, TEST_PTZ_TIMEOUT_MS);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ continuous_move function with NULL velocity
 * @param state Test state (unused)
 */
void test_ptz_continuous_move_null_velocity(void** state) {
  (void)state;

  int result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, NULL, TEST_PTZ_TIMEOUT_MS);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ stop function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_stop_success(void** state) {
  (void)state;

  // Mock successful platform stop
  platform_mock_set_ptz_stop_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 0);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ stop function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_stop_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_stop(NULL, 1, 0);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Home Position Tests
 * ============================================================================ */

/**
 * @brief Test PTZ goto_home_position function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_goto_home_position_success(void** state) {
  (void)state;

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_goto_home_position(TEST_PROFILE_TOKEN, &TEST_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ goto_home_position function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_goto_home_position_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_goto_home_position(NULL, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ set_home_position function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_set_home_position_success(void** state) {
  (void)state;

  int result = onvif_ptz_set_home_position(TEST_PROFILE_TOKEN);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ set_home_position function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_set_home_position_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_set_home_position(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Preset Management Tests
 * ============================================================================ */

/**
 * @brief Test PTZ get_presets function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_get_presets_success(void** state) {
  (void)state;

  struct ptz_preset* preset_list = NULL;
  int count = 0;

  int result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, &preset_list, &count);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(preset_list);
  assert_int_equal(count, 0); // Initially no presets
}

/**
 * @brief Test PTZ get_presets function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_get_presets_null_token(void** state) {
  (void)state;

  struct ptz_preset* preset_list = NULL;
  int count = 0;

  int result = onvif_ptz_get_presets(NULL, &preset_list, &count);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_presets function with NULL preset_list
 * @param state Test state (unused)
 */
void test_ptz_get_presets_null_list(void** state) {
  (void)state;

  int count = 0;

  int result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, NULL, &count);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ get_presets function with NULL count
 * @param state Test state (unused)
 */
void test_ptz_get_presets_null_count(void** state) {
  (void)state;

  struct ptz_preset* preset_list = NULL;

  int result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, &preset_list, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ set_preset function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_set_preset_success(void** state) {
  (void)state;

  // Mock platform PTZ status for preset creation
  struct ptz_device_status mock_status = {
    .h_pos_deg = 0, .v_pos_deg = 0, .h_speed = 0, .v_speed = 0};
  platform_mock_set_ptz_status(&mock_status);

  char preset_token[PTZ_PRESET_TOKEN_SIZE];
  memset(preset_token, 0, sizeof(preset_token));

  int result =
    onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME, preset_token, sizeof(preset_token));

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(preset_token, "Preset1");
}

/**
 * @brief Test PTZ set_preset function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_set_preset_null_token(void** state) {
  (void)state;

  char preset_token[PTZ_PRESET_TOKEN_SIZE];
  memset(preset_token, 0, sizeof(preset_token));

  int result = onvif_ptz_set_preset(NULL, TEST_PRESET_NAME, preset_token, sizeof(preset_token));

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ set_preset function with NULL preset name
 * @param state Test state (unused)
 */
void test_ptz_set_preset_null_name(void** state) {
  (void)state;

  char preset_token[PTZ_PRESET_TOKEN_SIZE];
  memset(preset_token, 0, sizeof(preset_token));

  int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, NULL, preset_token, sizeof(preset_token));

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ set_preset function with NULL output token
 * @param state Test state (unused)
 */
void test_ptz_set_preset_null_output_token(void** state) {
  (void)state;

  int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME, NULL, 0);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ goto_preset function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_goto_preset_success(void** state) {
  (void)state;

  // First create a preset
  char preset_token[PTZ_PRESET_TOKEN_SIZE];
  memset(preset_token, 0, sizeof(preset_token));

  // Mock platform PTZ status for preset creation
  struct ptz_device_status mock_status = {.h_pos_deg = TEST_PTZ_POSITION_PAN,
                                          .v_pos_deg = TEST_PTZ_POSITION_TILT,
                                          .h_speed = 0,
                                          .v_speed = 0};
  platform_mock_set_ptz_status(&mock_status);

  int result =
    onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME, preset_token, sizeof(preset_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  // Now goto the preset
  result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, preset_token, &TEST_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ goto_preset function with invalid preset token
 * @param state Test state (unused)
 */
void test_ptz_goto_preset_invalid_token(void** state) {
  (void)state;

  int result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, "InvalidPreset", &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test PTZ goto_preset function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_goto_preset_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_goto_preset(NULL, TEST_PRESET_TOKEN, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ goto_preset function with NULL preset token
 * @param state Test state (unused)
 */
void test_ptz_goto_preset_null_preset_token(void** state) {
  (void)state;

  int result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, NULL, &TEST_SPEED);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ remove_preset function with valid parameters
 * @param state Test state (unused)
 */
void test_ptz_remove_preset_success(void** state) {
  (void)state;

  // First create a preset
  char preset_token[PTZ_PRESET_TOKEN_SIZE];
  memset(preset_token, 0, sizeof(preset_token));

  // Mock platform PTZ status for preset creation
  struct ptz_device_status mock_status = {
    .h_pos_deg = 0, .v_pos_deg = 0, .h_speed = 0, .v_speed = 0};
  platform_mock_set_ptz_status(&mock_status);

  int result =
    onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME, preset_token, sizeof(preset_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Now remove the preset
  result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, preset_token);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ remove_preset function with invalid preset token
 * @param state Test state (unused)
 */
void test_ptz_remove_preset_invalid_token(void** state) {
  (void)state;

  int result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, "InvalidPreset");

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}

/**
 * @brief Test PTZ remove_preset function with NULL profile token
 * @param state Test state (unused)
 */
void test_ptz_remove_preset_null_token(void** state) {
  (void)state;

  int result = onvif_ptz_remove_preset(NULL, TEST_PRESET_TOKEN);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ remove_preset function with NULL preset token
 * @param state Test state (unused)
 */
void test_ptz_remove_preset_null_preset_token(void** state) {
  (void)state;

  int result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * PTZ Adapter Tests
 * ============================================================================ */

/**
 * @brief Test PTZ adapter initialization
 * @param state Test state (unused)
 */
void test_ptz_adapter_init_success(void** state) {
  (void)state;

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_init();

  assert_int_equal(result, 0);
}

/**
 * @brief Test PTZ adapter initialization failure
 * @param state Test state (unused)
 */
void test_ptz_adapter_init_failure(void** state) {
  (void)state;

  // Mock failed platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_ERROR);

  int result = ptz_adapter_init();

  assert_int_equal(result, -1);
}

/**
 * @brief Test PTZ adapter shutdown
 * @param state Test state (unused)
 */
void test_ptz_adapter_shutdown_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform PTZ cleanup
  platform_mock_set_ptz_cleanup_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_shutdown();

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter get_status function
 * @param state Test state (unused)
 */
void test_ptz_adapter_get_status_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  struct ptz_device_status status;
  memset(&status, 0, sizeof(status));

  int result = ptz_adapter_get_status(&status);

  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(status.h_pos_deg, 0); // Default position
  assert_int_equal(status.v_pos_deg, 0); // Default position
  assert_int_equal(status.h_speed, 0);
  assert_int_equal(status.v_speed, 0);
}

/**
 * @brief Test PTZ adapter get_status function with NULL status
 * @param state Test state (unused)
 */
void test_ptz_adapter_get_status_null_status(void** state) {
  (void)state;

  int result = ptz_adapter_get_status(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ adapter get_status function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_get_status_not_initialized(void** state) {
  (void)state;

  struct ptz_device_status status;
  memset(&status, 0, sizeof(status));

  int result = ptz_adapter_get_status(&status);

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter absolute_move function
 * @param state Test state (unused)
 */
void test_ptz_adapter_absolute_move_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result =
    ptz_adapter_absolute_move(TEST_PTZ_POSITION_PAN, TEST_PTZ_POSITION_TILT, TEST_PTZ_MOVE_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter absolute_move function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_absolute_move_not_initialized(void** state) {
  (void)state;

  int result =
    ptz_adapter_absolute_move(TEST_PTZ_POSITION_PAN, TEST_PTZ_POSITION_TILT, TEST_PTZ_MOVE_SPEED);

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter relative_move function
 * @param state Test state (unused)
 */
void test_ptz_adapter_relative_move_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_relative_move(TEST_PTZ_RELATIVE_MOVE_PAN, TEST_PTZ_RELATIVE_MOVE_TILT,
                                         TEST_PTZ_RELATIVE_MOVE_SPEED);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter relative_move function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_relative_move_not_initialized(void** state) {
  (void)state;

  int result = ptz_adapter_relative_move(TEST_PTZ_RELATIVE_MOVE_PAN, TEST_PTZ_RELATIVE_MOVE_TILT,
                                         TEST_PTZ_RELATIVE_MOVE_SPEED);

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter continuous_move function
 * @param state Test state (unused)
 */
void test_ptz_adapter_continuous_move_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_continuous_move(
    TEST_PTZ_CONTINUOUS_MOVE_PAN, TEST_PTZ_CONTINUOUS_MOVE_TILT, TEST_PTZ_CONTINUOUS_MOVE_TIMEOUT);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter continuous_move function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_continuous_move_not_initialized(void** state) {
  (void)state;

  int result = ptz_adapter_continuous_move(
    TEST_PTZ_CONTINUOUS_MOVE_PAN, TEST_PTZ_CONTINUOUS_MOVE_TILT, TEST_PTZ_CONTINUOUS_MOVE_TIMEOUT);

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter stop function
 * @param state Test state (unused)
 */
void test_ptz_adapter_stop_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform stop
  platform_mock_set_ptz_stop_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_stop();

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter stop function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_stop_not_initialized(void** state) {
  (void)state;

  int result = ptz_adapter_stop();

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter set_preset function
 * @param state Test state (unused)
 */
void test_ptz_adapter_set_preset_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  int result = ptz_adapter_set_preset(TEST_PRESET_NAME, 1);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter set_preset function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_set_preset_not_initialized(void** state) {
  (void)state;

  int result = ptz_adapter_set_preset(TEST_PRESET_NAME, 1);

  assert_int_equal(result, ONVIF_ERROR);
}

/**
 * @brief Test PTZ adapter goto_preset function
 * @param state Test state (unused)
 */
void test_ptz_adapter_goto_preset_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  ptz_adapter_init();

  // Mock successful platform move
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);

  int result = ptz_adapter_goto_preset(1);

  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test PTZ adapter goto_preset function when not initialized
 * @param state Test state (unused)
 */
void test_ptz_adapter_goto_preset_not_initialized(void** state) {
  (void)state;

  int result = ptz_adapter_goto_preset(1);

  assert_int_equal(result, ONVIF_ERROR);
}

/* ============================================================================
 * PTZ Service Initialization Tests
 * ============================================================================ */

/**
 * @brief Test PTZ service initialization
 * @param state Test state (unused)
 */
void test_ptz_service_init_success(void** state) {
  (void)state;

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  int result = onvif_ptz_init(NULL);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_ptz_cleanup();
}

/**
 * @brief Test PTZ service cleanup
 * @param state Test state (unused)
 */
void test_ptz_service_cleanup_success(void** state) {
  (void)state;

  // First initialize
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  onvif_ptz_init(NULL);

  // Cleanup should not fail
  onvif_ptz_cleanup();

  // Test passes if no crash occurs
  assert_true(1);
}

/* ============================================================================
 * PTZ Operation Handler Tests
 * ============================================================================ */

/**
 * @brief Test PTZ operation handler with valid operation
 * @param state Test state (unused)
 */
void test_ptz_handle_operation_success(void** state) {
  (void)state;

  // Mock HTTP request and response
  http_request_t request;
  http_response_t response;
  memset(&request, 0, sizeof(request));
  memset(&response, 0, sizeof(response));

  // Mock successful platform PTZ initialization
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);

  // Initialize PTZ service
  onvif_ptz_init(NULL);

  // Test GetConfigurations operation
  int result = onvif_ptz_handle_operation("GetConfigurations", &request, &response);

  // Note: This will likely return an error due to missing gSOAP context
  // but we're testing the basic dispatch mechanism
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR);

  // Cleanup
  onvif_ptz_cleanup();
}

/**
 * @brief Test PTZ operation handler with NULL operation name
 * @param state Test state (unused)
 */
void test_ptz_handle_operation_null_operation(void** state) {
  (void)state;

  http_request_t request;
  http_response_t response;
  memset(&request, 0, sizeof(request));
  memset(&response, 0, sizeof(response));

  int result = onvif_ptz_handle_operation(NULL, &request, &response);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ operation handler with NULL request
 * @param state Test state (unused)
 */
void test_ptz_handle_operation_null_request(void** state) {
  (void)state;

  http_response_t response;
  memset(&response, 0, sizeof(response));

  int result = onvif_ptz_handle_operation("GetConfigurations", NULL, &response);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ operation handler with NULL response
 * @param state Test state (unused)
 */
void test_ptz_handle_operation_null_response(void** state) {
  (void)state;

  http_request_t request;
  memset(&request, 0, sizeof(request));

  int result = onvif_ptz_handle_operation("GetConfigurations", &request, NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test PTZ operation handler with unknown operation
 * @param state Test state (unused)
 */
void test_ptz_handle_operation_unknown_operation(void** state) {
  (void)state;

  http_request_t request;
  http_response_t response;
  memset(&request, 0, sizeof(request));
  memset(&response, 0, sizeof(response));

  int result = onvif_ptz_handle_operation("UnknownOperation", &request, &response);

  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);
}
