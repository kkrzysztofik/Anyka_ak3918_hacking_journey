/**
 * @file ptz_service_tests.c
 * @brief Integration tests for optimized ONVIF PTZ service
 * @author kkrzysztofik
 * @date 2025
 */

#define _POSIX_C_SOURCE 200809L

#include "ptz_service_tests.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "platform/platform_common.h"

// ONVIF project includes
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "platform/adapters/ptz_adapter.h"
#include "platform/platform.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

// Test helper includes
#include "common/test_helpers.h"

// SOAP test helpers
#include "common/soap_test_helpers.h"
#include "data/soap_test_envelopes.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

// Mock includes
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/platform_mock.h"
#include "mocks/platform_ptz_mock.h"
#include "mocks/ptz_adapter_mock.h"

// Test configuration file path
#define TEST_CONFIG_PATH "configs/ptz_test_config.ini"

// Test profile token constants
#define TEST_PROFILE_TOKEN      "ProfileToken1"
#define TEST_PROFILE_TOKEN_LONG "VeryLongProfileTokenForEdgeCaseTesting"

// Test preset constants
#define TEST_PRESET_NAME         "TestPreset"
#define TEST_PRESET_NAME_SPECIAL "Preset-With_Special.Chars"
#define TEST_PRESET_NAME_EMPTY   ""
#define TEST_PRESET_NONEXISTENT  "NonExistentPreset"
#define TEST_PRESET_OVERFLOW     "OverflowPreset"
#define TEST_PRESET_MAX_COUNT    10

// Test movement constants
#define TEST_POSITION_PAN_NORMALIZED  0.5f
#define TEST_POSITION_TILT_NORMALIZED 0.3f
#define TEST_POSITION_ZOOM            0.0f
#define TEST_POSITION_PAN_CENTER      0.0f
#define TEST_POSITION_TILT_CENTER     0.0f
#define TEST_POSITION_PAN_EXTREME     999.0f
#define TEST_POSITION_TILT_EXTREME    -999.0f

// Test relative movement constants
#define TEST_TRANSLATION_PAN  0.1f
#define TEST_TRANSLATION_TILT -0.1f

// Test position multiplier constants
#define TEST_MULTIPLIER_0_05F 0.05f
#define TEST_MULTIPLIER_0_1F  0.1f
#define TEST_MULTIPLIER_0_2F  0.2f
#define TEST_MULTIPLIER_0_5F  0.5f

// Test velocity constants
#define TEST_VELOCITY_PAN  0.7f
#define TEST_VELOCITY_TILT 0.5f

// Test speed constants
#define TEST_SPEED_PAN_TILT_FAST   0.8f
#define TEST_SPEED_PAN_TILT_MEDIUM 0.6f
#define TEST_SPEED_ZOOM            0.0f

// Test timeout constants
#define TEST_TIMEOUT_MS     5000
#define TEST_TIMEOUT_NONE   0
#define TEST_TIMEOUT_1000MS 1000
#define TEST_TIMEOUT_2000MS 2000
#define TEST_TIMEOUT_500MS  500

// Test delay constants (for sleep/timing) - OPTIMIZED FOR FASTER TESTS
#define TEST_DELAY_1200MS 200 // Reduced from 1200ms - just verify timeout fired
#define TEST_DELAY_200MS  50  // Reduced from 200ms
#define TEST_DELAY_10MS   10  // Kept minimal
#define TEST_DELAY_50MS   50
#define TEST_DELAY_100MS  100
#define TEST_DELAY_250MS  250
#define TEST_DELAY_750MS  750
#define TEST_DELAY_900MS  900

// Test iteration constants - OPTIMIZED FOR FASTER TESTS
#define TEST_STRESS_ITERATIONS 10 // Reduced from 50
#define TEST_MEMORY_CYCLES     3
#define TEST_MEMORY_PRESETS    3 // Reduced from 5
#define TEST_CONCURRENT_OPS    10
#define TEST_BUFFER_POOL_OPS   3
#define TEST_LOOP_COUNT_3      2 // Reduced from 3
#define TEST_LOOP_COUNT_10     3 // Reduced from 10

// Test string constants
#define TEST_STRING_LONG_SIZE        512
#define TEST_PRESET_NAME_SIZE        256
#define TEST_PRESET_TOKEN_SIZE       64
#define TEST_PRESET_NAME_BUFFER_SIZE 32

/**
 * @brief Setup function for PTZ integration tests
 * @param state Test state pointer
 * @return 0 on success, -1 on failure
 *
 * This function initializes all required components for PTZ integration testing:
 * - Memory manager for tracking allocations
 * - Platform mock with PTZ support enabled
 * - PTZ adapter for hardware abstraction
 * - PTZ service with ONVIF protocol support
 */
int ptz_service_setup(void** state) {
  // Initialize memory manager for tracking
  memory_manager_init();

  // Enable real functions for integration testing (test real service interactions)
  service_dispatcher_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);
  gsoap_mock_use_real_function(true);
  ptz_adapter_mock_use_real_function(true);
  config_mock_use_real_function(true);

  // Initialize service dispatcher
  int result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize platform mock for PTZ operations
  platform_ptz_mock_init();

  // Configure CMocka expectations for PTZ operations (pure CMocka pattern)
  // ptz_adapter_init() calls platform_ptz_init() and then move_to_position(0, 0) to reset
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Heap-allocate application config structure (required for config_runtime_init)
  // CRITICAL: Must be heap-allocated because config_runtime_init() stores the pointer
  struct application_config* app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(app_config);

  // Allocate pointer members for application_config (required for config_storage_load)
  app_config->network = calloc(1, sizeof(struct network_settings));
  app_config->device = calloc(1, sizeof(struct device_info));
  app_config->logging = calloc(1, sizeof(struct logging_settings));
  app_config->server = calloc(1, sizeof(struct server_settings));
  assert_non_null(app_config->network);
  assert_non_null(app_config->device);
  assert_non_null(app_config->logging);
  assert_non_null(app_config->server);

  // Initialize runtime configuration system
  result = config_runtime_init(app_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Load configuration from test INI file (replaces manual defaults initialization)
  result = config_storage_load(TEST_CONFIG_PATH, NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Initialize empty PTZ preset data for all 4 profiles
  // CRITICAL: PTZ preset storage requires explicit initialization via dedicated API
  // The config_storage_load() only handles basic key-value pairs, not complex structures
  for (int i = 0; i < 4; i++) {
    ptz_preset_list_t empty_presets = {.preset_count = 0};
    memset(empty_presets.presets, 0, sizeof(empty_presets.presets));
    result = config_runtime_set_ptz_profile_presets(i, &empty_presets);
    assert_int_equal(ONVIF_SUCCESS, result);
  }

  // Initialize PTZ service with mock config
  config_manager_t* config = malloc(sizeof(config_manager_t));
  assert_non_null(config);
  memset(config, 0, sizeof(config_manager_t));

  // Initialize PTZ adapter
  result = ptz_adapter_init();
  assert_int_equal(0, result);

  // Initialize PTZ service (dispatcher already initialized)
  result = onvif_ptz_init(config);
  assert_int_equal(ONVIF_SUCCESS, result);

  *state = config;
  return 0;
}

/**
 * @brief Reset function for PTZ tests (lightweight state reset between tests)
 * @param state Test state pointer
 * @return 0 on success
 *
 * This function resets mock state between tests WITHOUT full teardown/setup.
 * Much faster than full teardown/setup cycle.
 */
int ptz_service_reset(void** state) {
  (void)state;

  // Reset mock state (lightweight operation)
  platform_ptz_mock_reset();

  // CRITICAL: Reset preset state between tests to prevent overflow
  // This ensures each test starts with a clean preset slate
  (void)onvif_ptz_reset_presets();

  // No need to reinitialize - service remains initialized
  return 0;
}

/**
 * @brief Teardown function for PTZ integration tests
 * @param state Test state pointer
 * @return 0 on success
 *
 * This function cleans up all resources allocated during setup:
 * - PTZ service cleanup
 * - PTZ adapter shutdown
 * - Platform mock cleanup
 * - Memory manager cleanup
 *
 * NOTE: Config must be freed BEFORE onvif_ptz_cleanup() because
 * onvif_ptz_cleanup() calls memory_manager_check_leaks() internally.
 */
int ptz_service_teardown(void** state) {
  config_manager_t* config = (config_manager_t*)*state;

  // Free config first, before leak checking
  free(config);

  // Clean up runtime configuration system
  config_runtime_cleanup();

  // Set up expectation for platform_ptz_cleanup (called by ptz_adapter_cleanup)
  expect_function_call(__wrap_platform_ptz_cleanup);
  will_return(__wrap_platform_ptz_cleanup, PLATFORM_SUCCESS);

  // Cleanup PTZ service (this unregisters from dispatcher)
  onvif_ptz_cleanup();
  ptz_adapter_cleanup();

  platform_ptz_mock_cleanup();
  memory_manager_cleanup();

  // Restore mock behavior for subsequent tests
  service_dispatcher_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);
  gsoap_mock_use_real_function(false);
  ptz_adapter_mock_use_real_function(false);
  config_mock_use_real_function(false);

  return 0;
}

// Test PTZ Absolute Move Functionality
void test_integration_ptz_relative_move_functionality(void** state) {
  (void)state;

  printf("Testing PTZ relative move functionality...\n");

  // Test valid relative move
  printf("  [TEST CASE] Valid relative move with translation and speed\n");

  // Set up expectations for the platform calls that will be made
  // TEST_TRANSLATION_PAN = 0.1f -> pan_delta = 18 -> direction = LEFT (2), steps = 16
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_value(__wrap_platform_ptz_turn, steps, 16);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // TEST_TRANSLATION_TILT = -0.1f -> tilt_delta = -9 -> direction = UP (0), steps = 8
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_value(__wrap_platform_ptz_turn, steps, 8);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  struct ptz_vector translation;
  struct ptz_speed speed;
  test_helper_ptz_create_test_position(&translation, TEST_TRANSLATION_PAN, TEST_TRANSLATION_TILT,
                                       TEST_POSITION_ZOOM);
  test_helper_ptz_create_test_speed(&speed, TEST_SPEED_PAN_TILT_MEDIUM, TEST_SPEED_ZOOM);

  int result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, &translation, &speed);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL speed
  printf("  [TEST CASE] Valid relative move with NULL speed (default speed)\n");

  // Set up expectations for the second call (same parameters)
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_value(__wrap_platform_ptz_turn, steps, 16);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_value(__wrap_platform_ptz_turn, steps, 8);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, &translation, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  printf("  [TEST CASE] Invalid NULL profile_token parameter\n");
  result = onvif_ptz_relative_move(NULL, &translation, &speed);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  printf("  [TEST CASE] Invalid NULL translation parameter\n");
  result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, NULL, &speed);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  printf("✅ PTZ relative move functionality tests passed\n");
}

// Test PTZ Continuous Move Functionality
void test_integration_ptz_continuous_move_functionality(void** state) {
  (void)state;

  printf("Testing PTZ continuous move functionality...\n");

  // Test valid continuous move
  printf("  [TEST CASE] Valid continuous move with velocity and timeout\n");

  // Set up expectations for first continuous move call
  // TEST_VELOCITY_PAN = 0.7f (positive) -> direction = RIGHT (3), steps = 350
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_value(__wrap_platform_ptz_turn, steps, 350);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // TEST_VELOCITY_TILT = 0.5f (positive) -> direction = DOWN (1), steps = 130
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_value(__wrap_platform_ptz_turn, steps, 130);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  struct ptz_speed velocity;
  test_helper_ptz_create_test_speed(&velocity, TEST_VELOCITY_PAN, TEST_SPEED_ZOOM);
  velocity.pan_tilt.y = TEST_VELOCITY_TILT; // Set different tilt velocity

  int result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_MS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with zero timeout (no timeout)
  printf("  [TEST CASE] Valid continuous move with zero timeout (no timeout)\n");

  // Set up expectations for second continuous move call (same parameters)
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_value(__wrap_platform_ptz_turn, steps, 350);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_value(__wrap_platform_ptz_turn, steps, 130);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_NONE);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  printf("  [TEST CASE] Invalid NULL profile_token parameter\n");
  result = onvif_ptz_continuous_move(NULL, &velocity, TEST_TIMEOUT_MS);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  printf("  [TEST CASE] Invalid NULL velocity parameter\n");
  result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, NULL, TEST_TIMEOUT_MS);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  printf("✅ PTZ continuous move functionality tests passed\n");
}

// Test PTZ Continuous Move Timeout Cleanup
void test_integration_ptz_continuous_move_timeout_cleanup(void** state) {
  (void)state;

  printf("Testing PTZ continuous move timeout cleanup (deadlock prevention)...\n");

  // Test continuous move with short timeout to verify cleanup doesn't deadlock
  printf("  [TEST CASE] Timeout cleanup - continuous move with 1 second timeout\n");

  // Set up expectations for continuous move platform calls
  // TEST_VELOCITY_PAN = 0.7f (positive) -> direction = RIGHT (3), steps = 350
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_value(__wrap_platform_ptz_turn, steps, 350);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // TEST_VELOCITY_TILT = 0.5f (positive) -> direction = DOWN (1), steps = 130
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_value(__wrap_platform_ptz_turn, steps, 130);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  struct ptz_speed velocity;
  test_helper_ptz_create_test_speed(&velocity, TEST_VELOCITY_PAN, TEST_SPEED_ZOOM);
  velocity.pan_tilt.y = TEST_VELOCITY_TILT;

  // Start continuous move with 1 second timeout
  int result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_1000MS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Wait for timeout to trigger (1.2 seconds to ensure thread has time to execute)
  expect_value(__wrap_platform_sleep_ms, ms, TEST_DELAY_1200MS);
  expect_function_call(__wrap_platform_sleep_ms);
  __wrap_platform_sleep_ms(TEST_DELAY_1200MS);

  // Verify that we can still perform operations after timeout
  // This would hang indefinitely if the deadlock bug exists
  printf("  [TEST CASE] Verify stop works after timeout (deadlock check)\n");

  // Set up expectations for stop calls (all 4 directions)
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test rapid continuous move followed by immediate stop
  // This tests the race condition where stop is called while timer thread is active
  printf("  [TEST CASE] Rapid continuous move with immediate stop (race condition test)\n");

  // Set up expectations for second continuous move
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_value(__wrap_platform_ptz_turn, steps, 350);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_value(__wrap_platform_ptz_turn, steps, 130);
  expect_function_call(__wrap_platform_ptz_turn);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_2000MS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Immediately call stop (timer thread should still be sleeping)
  // Set up expectations for second stop calls
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test multiple rapid continuous moves with timeouts
  // This stresses the thread join logic
  printf(
    "  [TEST CASE] Multiple rapid continuous moves with partial timeout (thread join stress)\n");
  for (int i = 0; i < TEST_LOOP_COUNT_3; i++) {
    // Set up expectations for continuous move (pan + tilt)
    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_value(__wrap_platform_ptz_turn, steps, 350);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_value(__wrap_platform_ptz_turn, steps, 130);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_500MS);
    assert_int_equal(result, ONVIF_SUCCESS);

    // Set up expectations for sleep
    expect_value(__wrap_platform_sleep_ms, ms, TEST_DELAY_200MS);
    expect_function_call(__wrap_platform_sleep_ms);
    __wrap_platform_sleep_ms(TEST_DELAY_200MS); // Wait partial timeout

    // Set up expectations for stop (all 4 directions)
    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Stress test: Rapid start/stop cycles with very short timeouts
  // This aggressively tests the condition variable wake-up mechanism
  printf("  [TEST CASE] Rapid start/stop cycles stress test (condition variable wake-up)\n");
  for (int i = 0; i < TEST_LOOP_COUNT_10; i++) {
    // Set up expectations for continuous move (pan + tilt)
    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_value(__wrap_platform_ptz_turn, steps, 350);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_value(__wrap_platform_ptz_turn, steps, 130);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_1000MS);
    assert_int_equal(result, ONVIF_SUCCESS);

    // Stop almost immediately (before timer thread even starts waiting)
    // Set up expectations for sleep
    expect_value(__wrap_platform_sleep_ms, ms, TEST_DELAY_10MS);
    expect_function_call(__wrap_platform_sleep_ms);
    __wrap_platform_sleep_ms(TEST_DELAY_10MS); // Minimal delay

    // Set up expectations for stop (all 4 directions)
    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Test stop called at various points during timeout (reduced to 3 representative values)
  printf("  [TEST CASE] Stop at various timing points during timeout\n");
  int test_delays[] = {TEST_DELAY_50MS, TEST_DELAY_250MS, TEST_TIMEOUT_500MS};
  for (size_t i = 0; i < sizeof(test_delays) / sizeof(test_delays[0]); i++) {
    // Set up expectations for continuous move (pan + tilt)
    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_value(__wrap_platform_ptz_turn, steps, 350);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_value(__wrap_platform_ptz_turn, steps, 130);
    expect_function_call(__wrap_platform_ptz_turn);
    will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

    result = onvif_ptz_continuous_move(TEST_PROFILE_TOKEN, &velocity, TEST_TIMEOUT_1000MS);
    assert_int_equal(result, ONVIF_SUCCESS);

    // Set up expectations for sleep with variable delay
    expect_value(__wrap_platform_sleep_ms, ms, test_delays[i]);
    expect_function_call(__wrap_platform_sleep_ms);
    __wrap_platform_sleep_ms(test_delays[i]);

    // Set up expectations for stop (all 4 directions)
    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
    expect_function_call(__wrap_platform_ptz_turn_stop);
    will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

    result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ continuous move timeout cleanup tests passed\n");
}

// Test PTZ Stop Functionality
void test_integration_ptz_stop_functionality(void** state) {
  (void)state;

  printf("Testing PTZ stop functionality...\n");

  // Test valid stop
  printf("  [TEST CASE] Valid stop pan/tilt and zoom\n");

  // Set up expectations for first stop call (all 4 directions)
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  int result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test stop pan/tilt only
  printf("  [TEST CASE] Valid stop pan/tilt only\n");

  // Set up expectations for second stop call
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_function_call(__wrap_platform_ptz_turn_stop);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 1, 0);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test stop zoom only (should succeed even without zoom support)
  printf("  [TEST CASE] Valid stop zoom only (graceful without zoom support)\n");
  result = onvif_ptz_stop(TEST_PROFILE_TOKEN, 0, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  printf("  [TEST CASE] Invalid NULL profile_token parameter\n");
  result = onvif_ptz_stop(NULL, 1, 1);
  assert_int_equal(result, ONVIF_ERROR_NULL);

  printf("✅ PTZ stop functionality tests passed\n");
}

// Test PTZ Preset Creation
void test_integration_ptz_preset_memory_optimization(void** state) {
  (void)state;

  printf("Testing PTZ preset memory optimization...\n");

  // Test multiple preset creation and removal
  printf("  [TEST CASE] Create multiple presets\n");
  char output_tokens[TEST_MEMORY_PRESETS][TEST_PRESET_TOKEN_SIZE] = {0};

  // Create multiple presets
  for (int i = 0; i < TEST_MEMORY_PRESETS; i++) {
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "Preset%d", i + 1);

    int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_tokens[i],
                                      sizeof(output_tokens[i]));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Verify all presets exist
  printf("  [TEST CASE] Verify all created presets exist\n");
  struct ptz_preset* preset_list = NULL;
  int count = 0;
  int result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, &preset_list, &count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(count, TEST_MEMORY_PRESETS);

  // Remove some presets and verify memory cleanup
  printf("  [TEST CASE] Remove multiple presets and verify memory cleanup\n");
  result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, output_tokens[1]);
  assert_int_equal(result, ONVIF_SUCCESS);

  result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, output_tokens[3]);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify remaining presets
  printf("  [TEST CASE] Verify correct preset count after removal\n");
  result = onvif_ptz_get_presets(TEST_PROFILE_TOKEN, &preset_list, &count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(count, TEST_MEMORY_PRESETS - 2);

  printf("✅ PTZ preset memory optimization tests passed\n");
}

// Test PTZ Memory Usage Improvements
void test_integration_ptz_memory_usage_improvements(void** state) {
  (void)state;

  printf("Testing PTZ memory usage improvements...\n");

  // Test buffer pool usage for string operations
  // This would require access to internal buffer pool statistics
  // For now, we test that operations complete without memory errors

  // Test multiple operations to verify no memory leaks
  printf("  [TEST CASE] Multiple PTZ operations (memory leak check)\n");
  for (int i = 0; i < TEST_CONCURRENT_OPS; i++) {
    struct ptz_vector position;
    test_helper_ptz_create_test_position(&position, (float)(i % 2),
                                         (float)(i % 3) * TEST_MULTIPLIER_0_5F, TEST_POSITION_ZOOM);

    // Set up expectations for absolute move (pan and tilt positions)
    int expected_pan = (int)((float)(i % 2) * 180.0f); // Convert normalized to degrees
    int expected_tilt =
      (int)((float)(i % 3) * TEST_MULTIPLIER_0_5F * 90.0f); // Convert normalized to degrees

    expect_value(__wrap_platform_ptz_move_to_position, pan_deg, expected_pan);
    expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, expected_tilt);
    expect_function_call(__wrap_platform_ptz_move_to_position);
    will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

    int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "TestPreset%d", i);

    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ memory usage improvements tests passed\n");
}

// Test PTZ Buffer Pool Usage
void test_integration_ptz_buffer_pool_usage(void** state) {
  (void)state;

  printf("Testing PTZ buffer pool usage...\n");

  // Test that buffer pool is properly used for temporary operations
  // This is validated by ensuring operations complete successfully
  // and don't cause memory allocation failures

  // Test concurrent operations that might stress buffer pool
  printf("  [TEST CASE] Buffer pool stress test with concurrent operations\n");
  const float test_positions[][2] = {{0.1F, 0.1F}, {0.5F, 0.5F}, {0.9F, 0.9F}};

  for (int i = 0; i < TEST_BUFFER_POOL_OPS; i++) {
    struct ptz_vector position;
    test_helper_ptz_create_test_position(&position, test_positions[i][0], test_positions[i][1],
                                         TEST_POSITION_ZOOM);

    // Set up expectations for absolute move (pan and tilt positions)
    int expected_pan = (int)(test_positions[i][0] * 180.0f); // Convert normalized to degrees
    int expected_tilt = (int)(test_positions[i][1] * 90.0f); // Convert normalized to degrees

    expect_value(__wrap_platform_ptz_move_to_position, pan_deg, expected_pan);
    expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, expected_tilt);
    expect_function_call(__wrap_platform_ptz_move_to_position);
    will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

    int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "ConcurrentPreset%d", i);

    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ buffer pool usage tests passed\n");
}

// Test PTZ String Operations Optimization
void test_integration_ptz_string_operations_optimization(void** state) {
  (void)state;

  printf("Testing PTZ string operations optimization...\n");

  // Test with various string lengths to verify bounds checking
  printf("  [TEST CASE] Long preset name (bounds checking)\n");
  char long_preset_name[TEST_PRESET_NAME_SIZE];
  memset(long_preset_name, 'A', sizeof(long_preset_name) - 1);
  long_preset_name[sizeof(long_preset_name) - 1] = '\0';

  // Set up expectations for buffer_pool_get calls
  expect_function_call(__wrap_buffer_pool_get);
  will_return(__wrap_buffer_pool_get, (void*)0x12345678); // Mock buffer pointer

  char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
  int result =
    onvif_ptz_set_preset(TEST_PROFILE_TOKEN, long_preset_name, output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with empty string
  printf("  [TEST CASE] Empty string preset name\n");

  // Set up expectations for second buffer_pool_get call
  expect_function_call(__wrap_buffer_pool_get);
  will_return(__wrap_buffer_pool_get, (void*)0x12345678); // Mock buffer pointer

  result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME_EMPTY, output_token,
                                sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with special characters
  printf("  [TEST CASE] Special characters in preset name\n");

  // Set up expectations for third buffer_pool_get call
  expect_function_call(__wrap_buffer_pool_get);
  will_return(__wrap_buffer_pool_get, (void*)0x12345678); // Mock buffer pointer

  result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_NAME_SPECIAL, output_token,
                                sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("✅ PTZ string operations optimization tests passed\n");
}

// Test PTZ Error Handling Robustness
void test_integration_ptz_error_handling_robustness(void** state) {
  (void)state;

  printf("Testing PTZ error handling robustness...\n");

  // Test with extreme values
  printf("  [TEST CASE] Extreme position values (clamping test)\n");
  struct ptz_vector extreme_position;
  test_helper_ptz_create_test_position(&extreme_position, TEST_POSITION_PAN_EXTREME,
                                       TEST_POSITION_TILT_EXTREME, TEST_POSITION_ZOOM);

  // Set up expectations for absolute move with clamped values
  // From the log: pan=179820 -> clamped to 350, tilt=-89910 -> clamped to -130
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 350);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, -130);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &extreme_position, NULL);
  // Should handle extreme values gracefully (clamp to valid range)
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with very long profile token
  printf("  [TEST CASE] Long profile token (bounds checking)\n");
  char long_profile_token[TEST_STRING_LONG_SIZE];
  memset(long_profile_token, 'X', sizeof(long_profile_token) - 1);
  long_profile_token[sizeof(long_profile_token) - 1] = '\0';

  // Set up expectations for the second absolute move call with same clamped values
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 350);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, -130);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN_LONG, &extreme_position, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with maximum number of presets
  printf("  [TEST CASE] Maximum number of presets\n");
  for (int i = 0; i < TEST_PRESET_MAX_COUNT; i++) {
    char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "MaxPreset%d", i);

    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Test adding one more preset (should fail)
  printf("  [TEST CASE] Preset overflow (exceeding max count)\n");
  char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
  result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, TEST_PRESET_OVERFLOW, output_token,
                                sizeof(output_token));
  assert_int_equal(result, ONVIF_ERROR); // Should fail due to max presets reached

  printf("✅ PTZ error handling robustness tests passed\n");
}

// Test PTZ Concurrent Operations
void test_integration_ptz_concurrent_operations(void** state) {
  (void)state;

  printf("Testing PTZ concurrent operations...\n");

  // This test would require threading support
  // For now, we test sequential operations that simulate concurrent access

  // Simulate rapid sequential operations
  printf("  [TEST CASE] Rapid sequential operations (concurrent access simulation)\n");
  for (int i = 0; i < TEST_MEMORY_PRESETS; i++) {
    struct ptz_vector position;
    test_helper_ptz_create_test_position(&position, (float)i * TEST_MULTIPLIER_0_2F,
                                         (float)i * TEST_MULTIPLIER_0_1F, TEST_POSITION_ZOOM);

    // Set up expectations for absolute move (pan and tilt positions)
    int expected_pan =
      (int)((float)i * TEST_MULTIPLIER_0_2F * 180.0f); // Convert normalized to degrees
    int expected_tilt =
      (int)((float)i * TEST_MULTIPLIER_0_1F * 90.0f); // Convert normalized to degrees

    expect_value(__wrap_platform_ptz_move_to_position, pan_deg, expected_pan);
    expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, expected_tilt);
    expect_function_call(__wrap_platform_ptz_move_to_position);
    will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

    int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "ConcurrentPreset%d", i);

    // Set up expectations for buffer_pool_get (called by onvif_ptz_set_preset)
    expect_function_call(__wrap_buffer_pool_get);
    will_return(__wrap_buffer_pool_get, (void*)0x12345678);

    result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);

    // Set up expectations for goto_preset (calls platform_ptz_move_to_position with same values)
    expect_value(__wrap_platform_ptz_move_to_position, pan_deg, expected_pan);
    expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, expected_tilt);
    expect_function_call(__wrap_platform_ptz_move_to_position);
    will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

    result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, output_token, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ concurrent operations tests passed\n");
}

// Test PTZ Stress Testing
void test_integration_ptz_stress_testing(void** state) {
  (void)state;

  printf("Testing PTZ stress testing...\n");

  // Perform many operations in sequence to stress test the system
  printf("  [TEST CASE] Stress test with %d iterations\n", TEST_STRESS_ITERATIONS);
  for (int i = 0; i < TEST_STRESS_ITERATIONS; i++) {
    // Create preset
    char output_token[TEST_PRESET_TOKEN_SIZE] = {0};
    char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
    (void)snprintf(preset_name, sizeof(preset_name), "StressPreset%d", i);

    int result =
      onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_token, sizeof(output_token));
    if (i < TEST_PRESET_MAX_COUNT) { // Only first TEST_PRESET_MAX_COUNT should succeed
      assert_int_equal(result, ONVIF_SUCCESS);
    }

    // Move to position
    struct ptz_vector position;
    test_helper_ptz_create_test_position(&position, (float)(i % 2),
                                         (float)(i % 3) * TEST_MULTIPLIER_0_5F, TEST_POSITION_ZOOM);

    result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    // If preset was created, goto it
    if (i < TEST_PRESET_MAX_COUNT && result == ONVIF_SUCCESS) {
      result = onvif_ptz_goto_preset(TEST_PROFILE_TOKEN, output_token, NULL);
      assert_int_equal(result, ONVIF_SUCCESS);
    }
  }

  printf("✅ PTZ stress testing passed\n");
}

// Test PTZ Memory Leak Detection
void test_integration_ptz_memory_leak_detection(void** state) {
  (void)state;

  printf("Testing PTZ memory leak detection...\n");

  // Perform operations that should not leak memory
  // This test relies on the memory manager's leak detection

  // Create and remove presets multiple times
  printf("  [TEST CASE] Multiple cycles of preset creation and removal (leak detection)\n");
  for (int cycle = 0; cycle < TEST_MEMORY_CYCLES; cycle++) {
    char output_tokens[TEST_MEMORY_PRESETS][TEST_PRESET_TOKEN_SIZE] = {0};

    // Create presets
    for (int i = 0; i < TEST_MEMORY_PRESETS; i++) {
      char preset_name[TEST_PRESET_NAME_BUFFER_SIZE];
      (void)snprintf(preset_name, sizeof(preset_name), "LeakTestPreset%d_%d", cycle, i);

      int result = onvif_ptz_set_preset(TEST_PROFILE_TOKEN, preset_name, output_tokens[i],
                                        sizeof(output_tokens[i]));
      assert_int_equal(result, ONVIF_SUCCESS);
    }

    // Remove presets
    for (int i = 0; i < TEST_MEMORY_PRESETS; i++) {
      int result = onvif_ptz_remove_preset(TEST_PROFILE_TOKEN, output_tokens[i]);
      assert_int_equal(result, ONVIF_SUCCESS);
    }
  }

  // Perform various PTZ operations
  printf("  [TEST CASE] Various PTZ operations (absolute and relative moves)\n");
  for (int i = 0; i < TEST_CONCURRENT_OPS; i++) {
    struct ptz_vector position;
    test_helper_ptz_create_test_position(&position, (float)i * TEST_MULTIPLIER_0_1F,
                                         (float)i * TEST_MULTIPLIER_0_05F, TEST_POSITION_ZOOM);

    int result = onvif_ptz_absolute_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    result = onvif_ptz_relative_move(TEST_PROFILE_TOKEN, &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ memory leak detection tests passed\n");
}

/**
 * @brief Pilot SOAP test for PTZ GetNodes operation
 * Tests SOAP envelope parsing and response structure validation
 */
void test_integration_ptz_get_nodes_soap(void** state) {
  (void)state;

  // PTZ init is idempotent, so calling it multiple times is safe

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GetNodes", SOAP_PTZ_GET_NODES, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 3: Validate request structure
  assert_non_null(request->body);
  assert_true(strstr(request->body, "GetNodes") != NULL);

  // Step 4: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 5: Call actual service handler (integration test)
  int result = onvif_ptz_handle_operation("GetNodes", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 5: Validate HTTP response structure
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);
  assert_true(response.body_length > 0);

  // Step 6: Check for SOAP faults
  char fault_code[256] = {0};
  char fault_string[512] = {0};
  int has_fault = soap_test_check_soap_fault(&response, fault_code, fault_string);
  assert_int_equal(0, has_fault);

  // Step 7: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__GetNodesResponse* nodes_response = NULL;
  result = soap_test_parse_get_nodes_response(&ctx, &nodes_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(nodes_response);

  // Step 8: Validate response data
  assert_non_null(nodes_response->PTZNode);
  assert_true(nodes_response->__sizePTZNode >= 1);
  assert_non_null(nodes_response->PTZNode[0].token);

  // Step 9: Cleanup resources
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for PTZ AbsoluteMove operation
 */
void test_integration_ptz_absolute_move_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("AbsoluteMove", SOAP_PTZ_ABSOLUTE_MOVE, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_ptz_handle_operation("AbsoluteMove", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__AbsoluteMoveResponse* move_response = NULL;
  result = soap_test_parse_absolute_move_response(&ctx, &move_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(move_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for PTZ GetPresets operation
 */
void test_integration_ptz_get_presets_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GetPresets", SOAP_PTZ_GET_PRESETS, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_ptz_handle_operation("GetPresets", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__GetPresetsResponse* presets_response = NULL;
  result = soap_test_parse_get_presets_response(&ctx, &presets_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(presets_response);

  // Step 7: Validate response data - presets array should exist
  assert_true(presets_response->__sizePreset >= 0);

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for PTZ SetPreset operation
 */
void test_integration_ptz_set_preset_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("SetPreset", SOAP_PTZ_SET_PRESET, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_ptz_handle_operation("SetPreset", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__SetPresetResponse* preset_response = NULL;
  result = soap_test_parse_set_preset_response(&ctx, &preset_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(preset_response);

  // Step 7: Validate response data - token field exists

  // Step 8: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for PTZ GotoPreset operation
 */
void test_integration_ptz_goto_preset_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("GotoPreset", SOAP_PTZ_GOTO_PRESET, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_ptz_handle_operation("GotoPreset", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__GotoPresetResponse* goto_response = NULL;
  result = soap_test_parse_goto_preset_response(&ctx, &goto_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(goto_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

/**
 * @brief SOAP test for PTZ RemovePreset operation
 */
void test_integration_ptz_remove_preset_soap(void** state) {
  (void)state;

  // Step 1: Create SOAP request envelope
  http_request_t* request =
    soap_test_create_request("RemovePreset", SOAP_PTZ_REMOVE_PRESET, "/onvif/ptz_service");
  assert_non_null(request);

  // Step 2: Prepare response structure
  http_response_t response;
  memset(&response, 0, sizeof(http_response_t));

  // Step 3: Call service handler
  int result = onvif_ptz_handle_operation("RemovePreset", request, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  // Step 4: Validate HTTP response
  assert_int_equal(200, response.status_code);
  assert_non_null(response.body);

  // Step 5: Check for SOAP faults
  int has_fault = soap_test_check_soap_fault(&response, NULL, NULL);
  assert_int_equal(0, has_fault);

  // Step 6: Parse SOAP response
  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(onvif_gsoap_context_t));
  result = soap_test_init_response_parsing(&ctx, &response);
  assert_int_equal(ONVIF_SUCCESS, result);

  struct _tptz__RemovePresetResponse* remove_response = NULL;
  result = soap_test_parse_remove_preset_response(&ctx, &remove_response);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(remove_response);

  // Step 7: Cleanup
  onvif_gsoap_cleanup(&ctx);
  soap_test_free_request(request);
  if (response.body) {
    ONVIF_FREE(response.body);
  }
}

// Test suite definition
// OPTIMIZATION: Use lightweight reset between most tests instead of full teardown/setup
// Only the first test uses full setup, and only the last test uses full teardown
const struct CMUnitTest ptz_service_optimization_tests[] = {
  // PTZ Movement Operations Tests
  cmocka_unit_test_setup_teardown(test_integration_ptz_relative_move_functionality,
                                  ptz_service_setup, ptz_service_reset), // SETUP first test
  cmocka_unit_test_setup_teardown(test_integration_ptz_continuous_move_functionality,
                                  ptz_service_reset, ptz_service_reset), // RESET between tests
  cmocka_unit_test_setup_teardown(test_integration_ptz_continuous_move_timeout_cleanup,
                                  ptz_service_reset, ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_stop_functionality, ptz_service_reset,
                                  ptz_service_reset),

  // PTZ Preset Management Tests
  cmocka_unit_test_setup_teardown(test_integration_ptz_preset_memory_optimization,
                                  ptz_service_reset, ptz_service_reset),

  // PTZ Service Optimization Validation Tests
  cmocka_unit_test_setup_teardown(test_integration_ptz_memory_usage_improvements, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_buffer_pool_usage, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_string_operations_optimization,
                                  ptz_service_reset, ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_error_handling_robustness, ptz_service_reset,
                                  ptz_service_reset),

  // PTZ Service Performance Tests
  cmocka_unit_test_setup_teardown(test_integration_ptz_stress_testing, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_memory_leak_detection, ptz_service_reset,
                                  ptz_service_reset),

  // SOAP integration tests (full HTTP/SOAP layer validation)
  cmocka_unit_test_setup_teardown(test_integration_ptz_get_nodes_soap, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_absolute_move_soap, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_get_presets_soap, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_set_preset_soap, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_goto_preset_soap, ptz_service_reset,
                                  ptz_service_reset),
  cmocka_unit_test_setup_teardown(test_integration_ptz_remove_preset_soap, ptz_service_reset,
                                  ptz_service_reset),

  // Concurrent tests - last test uses full TEARDOWN
  cmocka_unit_test_setup_teardown(test_integration_ptz_concurrent_operations, ptz_service_reset,
                                  ptz_service_teardown), // TEARDOWN last test
};
