/**
 * @file ptz_adapter_tests.c
 * @brief Unit tests for PTZ adapter layer
 * @author kkrzysztofik
 * @date 2025
 *
 * These tests verify the PTZ adapter layer in isolation using platform mocks.
 * Tests cover coordinate transformations, state management, and boundary conditions.
 */
#include <time.h>

#include "cmocka_wrapper.h"
#include "platform/adapters/ptz_adapter.h"
#include "platform/platform_common.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/memory/memory_manager.h"

// Platform mocks
#include "mocks/platform_mock.h"
#include "mocks/platform_ptz_mock.h"
#include "mocks/ptz_adapter_mock.h"

#define PTZ_TURN_STOP_BIT(direction) (1U << (direction))
#define PTZ_TIMEOUT_EXPECTED_MASK                                                                                                                    \
  (PTZ_TURN_STOP_BIT(PLATFORM_PTZ_DIRECTION_LEFT) | PTZ_TURN_STOP_BIT(PLATFORM_PTZ_DIRECTION_RIGHT) | PTZ_TURN_STOP_BIT(PLATFORM_PTZ_DIRECTION_UP) | \
   PTZ_TURN_STOP_BIT(PLATFORM_PTZ_DIRECTION_DOWN))

static void wait_for_turn_stop_mask(unsigned int expected_mask, int timeout_ms) {
  const int sleep_step_ms = 20;
  int waited_ms = 0;

  while (waited_ms <= timeout_ms) {
    if ((platform_mock_get_ptz_turn_stop_mask() & expected_mask) == expected_mask) {
      return;
    }

    struct timespec req = {.tv_sec = 0, .tv_nsec = sleep_step_ms * 1000000L};
    nanosleep(&req, NULL);
    waited_ms += sleep_step_ms;
  }

  fail_msg("Timed out waiting for PTZ turn stop mask: expected=0x%02x actual=0x%02x", expected_mask, platform_mock_get_ptz_turn_stop_mask());
}

/* ============================================================================
 * Test Fixture Setup/Teardown
 * ============================================================================ */

static int ptz_adapter_test_setup(void** state) {
  (void)state;

  // Initialize memory manager
  memory_manager_init();

  // Initialize platform mocks
  platform_ptz_mock_init();

  // Use REAL PTZ adapter functions (we're testing the adapter, not mocking it)
  ptz_adapter_mock_use_real_function(true);

  // Platform PTZ functions remain mocked (they call unavailable hardware APIs)
  // Each test must set up platform mock expectations (will_return, expect_function_call, etc.)

  return 0;
}

static int ptz_adapter_test_teardown(void** state) {
  (void)state;

  if (platform_mock_is_ptz_initialized()) {
    expect_function_call(__wrap_platform_ptz_cleanup);
    will_return(__wrap_platform_ptz_cleanup, PLATFORM_SUCCESS);
  }

  // Cleanup adapter (ensure real cleanup completes when initialization occurred)
  ptz_adapter_cleanup();

  // Restore mock mode for subsequent tests
  ptz_adapter_mock_use_real_function(false);

  // Cleanup mocks
  platform_ptz_mock_cleanup();

  // Cleanup memory manager
  memory_manager_cleanup();

  return 0;
}

/* ============================================================================
 * Initialization and Cleanup Tests
 * ============================================================================ */

void test_unit_ptz_adapter_init_success(void** state) {
  (void)state;

  // Mock platform_ptz_init (called by adapter init)
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);

  // Mock platform_ptz_move_to_position (called by adapter init to reset to center)
  // Adapter calls with (0, 0) for pan/tilt, mock expects pan/tilt/zoom
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_init();
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify platform was initialized
  assert_int_equal(1, platform_mock_get_ptz_init_call_count());
}

void test_unit_ptz_adapter_init_idempotent(void** state) {
  (void)state;

  // First call expectations - platform functions will be called
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Initialize twice
  platform_result_t result1 = ptz_adapter_init();
  platform_result_t result2 = ptz_adapter_init(); // Second call shouldn't call platform (idempotent)

  assert_int_equal(PLATFORM_SUCCESS, result1);
  assert_int_equal(PLATFORM_SUCCESS, result2);

  // Should only call platform init once
  assert_int_equal(1, platform_mock_get_ptz_init_call_count());
}

void test_unit_ptz_adapter_init_failure(void** state) {
  (void)state;
  // Reset platform mock and simulate platform init failure
  platform_ptz_mock_reset();
  platform_mock_enable_ptz_error(PLATFORM_ERROR);

  // Add expect_function_call for platform_ptz_init (will_return already set by enable_ptz_error)
  expect_function_call(__wrap_platform_ptz_init);

  platform_result_t result = ptz_adapter_init();
  assert_int_equal(PLATFORM_ERROR, result);

  // Disable error simulation for subsequent tests
  platform_mock_disable_ptz_error();
}

void test_unit_ptz_adapter_cleanup_safe_when_not_initialized(void** state) {
  (void)state;

  // Should not crash when cleaning up uninitialized adapter
  ptz_adapter_cleanup();

  // No assertions needed - just verify no crash
}

/* ============================================================================
 * Absolute Move Tests
 * ============================================================================ */

void test_unit_ptz_adapter_absolute_move_success(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 90);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 45);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_absolute_move(90, 45, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify platform was called with correct values
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(90, pan);
  assert_int_equal(45, tilt);
}

void test_unit_ptz_adapter_absolute_move_clamping_pan_max(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call - value will be clamped to 350
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 350); // Clamped max
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Try to move beyond max pan range
  platform_result_t result = ptz_adapter_absolute_move(400, 0, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify value was clamped to 350 (PTZ_MAX_PAN_DEGREES)
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(350, pan);
}

void test_unit_ptz_adapter_absolute_move_clamping_pan_min(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call - value will be clamped to -350
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, -350); // Clamped min
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Try to move below min pan range
  platform_result_t result = ptz_adapter_absolute_move(-400, 0, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify value was clamped to -350 (PTZ_MIN_PAN_DEGREES)
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(-350, pan);
}

void test_unit_ptz_adapter_absolute_move_clamping_tilt_max(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call - value will be clamped to 130
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 130); // Clamped max
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Try to move beyond max tilt range
  platform_result_t result = ptz_adapter_absolute_move(0, 200, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify value was clamped to 130 (PTZ_MAX_TILT_DEGREES)
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(130, tilt);
}

void test_unit_ptz_adapter_absolute_move_clamping_tilt_min(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call - value will be clamped to -130
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, -130); // Clamped min
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Try to move below min tilt range
  platform_result_t result = ptz_adapter_absolute_move(0, -200, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify value was clamped to -130 (PTZ_MIN_TILT_DEGREES)
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(-130, tilt);
}

void test_unit_ptz_adapter_absolute_move_not_initialized(void** state) {
  (void)state;

  // Try to move without initializing
  platform_result_t result = ptz_adapter_absolute_move(90, 45, 50);
  assert_int_equal(PLATFORM_ERROR_INVALID, result);
}

/* ============================================================================
 * Status Tracking Tests
 * ============================================================================ */

void test_unit_ptz_adapter_get_status_success(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock the absolute move call
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 90);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 45);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Move to a known position
  ptz_adapter_absolute_move(90, 45, 50);

  // Get status
  struct ptz_device_status status;
  platform_result_t result = ptz_adapter_get_status(&status);

  assert_int_equal(PLATFORM_SUCCESS, result);
  assert_int_equal(90, status.h_pos_deg);
  assert_int_equal(45, status.v_pos_deg);
}

void test_unit_ptz_adapter_get_status_null_parameter(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_get_status(NULL);
  assert_int_equal(PLATFORM_ERROR_NULL, result);
}

void test_unit_ptz_adapter_get_status_not_initialized(void** state) {
  (void)state;

  struct ptz_device_status status;
  platform_result_t result = ptz_adapter_get_status(&status);
  assert_int_equal(PLATFORM_ERROR_INVALID, result);
}

void test_unit_ptz_adapter_position_tracking_after_move(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock first move to position (100, 50)
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 100);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 50);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Move to position 1
  ptz_adapter_absolute_move(100, 50, 50);

  struct ptz_device_status status;
  ptz_adapter_get_status(&status);
  assert_int_equal(100, status.h_pos_deg);
  assert_int_equal(50, status.v_pos_deg);

  // Mock second move to position (-50, -30)
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, -50);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, -30);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Move to position 2
  ptz_adapter_absolute_move(-50, -30, 50);

  ptz_adapter_get_status(&status);
  assert_int_equal(-50, status.h_pos_deg);
  assert_int_equal(-30, status.v_pos_deg);
}

/* ============================================================================
 * Relative Move Tests
 * ============================================================================ */

void test_unit_ptz_adapter_relative_move_positive_delta(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock platform_ptz_turn for pan (positive delta = LEFT direction)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_value(__wrap_platform_ptz_turn, steps, 10);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // Mock platform_ptz_turn for tilt (positive delta = DOWN direction)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  expect_value(__wrap_platform_ptz_turn, steps, 5);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_relative_move(10, 5, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Relative move uses platform_ptz_turn, verify it was called
  platform_ptz_direction_t dir;
  int steps;
  assert_int_equal(1, platform_mock_get_last_ptz_turn(&dir, &steps));
}

void test_unit_ptz_adapter_relative_move_delta_clamping(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock platform_ptz_turn - steps will be clamped to 16 (PTZ_MAX_STEP_SIZE_PAN)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  expect_value(__wrap_platform_ptz_turn, steps, 16); // Clamped from 20 to 16
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // Try to move with delta > max step size (16 for pan)
  platform_result_t result = ptz_adapter_relative_move(20, 0, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify step was clamped to PTZ_MAX_STEP_SIZE_PAN (16)
  platform_ptz_direction_t dir;
  int steps;
  assert_int_equal(1, platform_mock_get_last_ptz_turn(&dir, &steps));
  assert_int_equal(16, steps);
}

/* ============================================================================
 * Continuous Move Tests
 * ============================================================================ */

void test_unit_ptz_adapter_continuous_move_with_timeout(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock platform_ptz_turn for pan (velocity 1 = RIGHT direction with PTZ_MAX_PAN_DEGREES steps)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_any(__wrap_platform_ptz_turn, steps); // Large value for continuous
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  // Mock platform_ptz_turn for tilt (velocity -1 = UP direction with PTZ_MAX_TILT_DEGREES steps)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_UP);
  expect_any(__wrap_platform_ptz_turn, steps); // Large value for continuous
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_continuous_move(1, -1, 5);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Continuous move uses platform_ptz_turn with large values
  platform_ptz_direction_t dir;
  int steps;
  assert_int_equal(1, platform_mock_get_last_ptz_turn(&dir, &steps));

  // Wait for timeout thread to stop all directions (increased timeout to 10 seconds)
  wait_for_turn_stop_mask(PTZ_TIMEOUT_EXPECTED_MASK, 10000);
}

void test_unit_ptz_adapter_continuous_move_no_timeout(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock platform_ptz_turn for pan only (tilt velocity is 0)
  expect_function_call(__wrap_platform_ptz_turn);
  expect_value(__wrap_platform_ptz_turn, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  expect_any(__wrap_platform_ptz_turn, steps);
  will_return(__wrap_platform_ptz_turn, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_continuous_move(1, 0, 0);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // No timeout configured, so stop events should not be recorded automatically
  assert_int_equal(0U, platform_mock_get_ptz_turn_stop_mask());
}

/* ============================================================================
 * Stop Tests
 * ============================================================================ */

void test_unit_ptz_adapter_stop_success(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock platform_ptz_turn_stop for all 4 directions
  expect_function_call(__wrap_platform_ptz_turn_stop);
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_LEFT);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_function_call(__wrap_platform_ptz_turn_stop);
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_RIGHT);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_function_call(__wrap_platform_ptz_turn_stop);
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_UP);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  expect_function_call(__wrap_platform_ptz_turn_stop);
  expect_value(__wrap_platform_ptz_turn_stop, direction, PLATFORM_PTZ_DIRECTION_DOWN);
  will_return(__wrap_platform_ptz_turn_stop, PLATFORM_SUCCESS);

  platform_result_t result = ptz_adapter_stop();
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify platform stop was called for all directions
  platform_ptz_direction_t dir;
  assert_int_equal(1, platform_mock_get_last_ptz_turn_stop(&dir));
}

/* ============================================================================
 * Preset Tests
 * ============================================================================ */

void test_unit_ptz_adapter_set_preset(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_set_preset("TestPreset", 1);
  assert_int_equal(PLATFORM_SUCCESS, result);
}

void test_unit_ptz_adapter_goto_preset_home(void** state) {
  (void)state;

  // Mock adapter init sequence
  expect_function_call(__wrap_platform_ptz_init);
  will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS);
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  ptz_adapter_init();

  // Mock first move away from home (100, 50)
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 100);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 50);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Move away from home
  ptz_adapter_absolute_move(100, 50, 50);

  // Mock goto home preset (preset 1 moves to 0, 0)
  expect_function_call(__wrap_platform_ptz_move_to_position);
  expect_value(__wrap_platform_ptz_move_to_position, pan_deg, 0);
  expect_value(__wrap_platform_ptz_move_to_position, tilt_deg, 0);
  will_return(__wrap_platform_ptz_move_to_position, PLATFORM_SUCCESS);

  // Go to home preset (preset 1)
  platform_result_t result = ptz_adapter_goto_preset(1);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify move to 0, 0
  int pan, tilt, speed;
  assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
  assert_int_equal(0, pan);
  assert_int_equal(0, tilt);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest ptz_adapter_unit_tests[] = {
  // Initialization tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_success, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_idempotent, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_failure, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_cleanup_safe_when_not_initialized, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Absolute move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_success, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_pan_max, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_pan_min, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_tilt_max, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_tilt_min, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_not_initialized, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Status tracking tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_success, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_null_parameter, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_not_initialized, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_position_tracking_after_move, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Relative move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_relative_move_positive_delta, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_relative_move_delta_clamping, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Continuous move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_continuous_move_with_timeout, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_continuous_move_no_timeout, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Stop tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_stop_success, ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Preset tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_set_preset, ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_goto_preset_home, ptz_adapter_test_setup, ptz_adapter_test_teardown),
};
