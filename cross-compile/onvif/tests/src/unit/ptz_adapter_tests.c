/**
 * @file ptz_adapter_tests.c
 * @brief Unit tests for PTZ adapter layer
 * @author kkrzysztofik
 * @date 2025
 *
 * These tests verify the PTZ adapter layer in isolation using platform mocks.
 * Tests cover coordinate transformations, state management, and boundary conditions.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <stdint.h>
#include <cmocka.h>

#include "platform/adapters/ptz_adapter.h"
#include "platform/platform_common.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/memory/memory_manager.h"

// Platform mocks
#include "mocks/platform_mock.h"
#include "mocks/platform_ptz_mock.h"

/* ============================================================================
 * Test Fixture Setup/Teardown
 * ============================================================================ */

static int ptz_adapter_test_setup(void** state) {
  (void)state;

  // Initialize memory manager
  memory_manager_init();

  // Initialize platform mocks
  platform_ptz_mock_init();

  // Set default success results
  platform_mock_set_ptz_init_result(PLATFORM_SUCCESS);
  platform_mock_set_ptz_move_result(PLATFORM_SUCCESS);
  platform_mock_set_ptz_stop_result(PLATFORM_SUCCESS);

  return 0;
}

static int ptz_adapter_test_teardown(void** state) {
  (void)state;

  // Cleanup adapter
  ptz_adapter_cleanup();

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

  platform_result_t result = ptz_adapter_init();
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Verify platform was initialized
  assert_int_equal(1, platform_mock_get_ptz_init_call_count());
}

void test_unit_ptz_adapter_init_idempotent(void** state) {
  (void)state;

  // Initialize twice
  platform_result_t result1 = ptz_adapter_init();
  platform_result_t result2 = ptz_adapter_init();

  assert_int_equal(PLATFORM_SUCCESS, result1);
  assert_int_equal(PLATFORM_SUCCESS, result2);

  // Should only call platform init once
  assert_int_equal(1, platform_mock_get_ptz_init_call_count());
}

void test_unit_ptz_adapter_init_failure(void** state) {
  (void)state;

  // Ensure adapter is cleaned up from previous tests
  ptz_adapter_cleanup();

  // Reset platform mock and simulate platform init failure
  platform_ptz_mock_reset();
  platform_mock_enable_ptz_error(PLATFORM_ERROR);

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

  ptz_adapter_init();

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

  ptz_adapter_init();

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

  ptz_adapter_init();

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

  ptz_adapter_init();

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

  ptz_adapter_init();

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

  ptz_adapter_init();

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

  ptz_adapter_init();

  // Move to position 1
  ptz_adapter_absolute_move(100, 50, 50);

  struct ptz_device_status status;
  ptz_adapter_get_status(&status);
  assert_int_equal(100, status.h_pos_deg);
  assert_int_equal(50, status.v_pos_deg);

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

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_relative_move(10, 5, 50);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Relative move uses platform_ptz_turn, verify it was called
  platform_ptz_direction_t dir;
  int steps;
  assert_int_equal(1, platform_mock_get_last_ptz_turn(&dir, &steps));
}

void test_unit_ptz_adapter_relative_move_delta_clamping(void** state) {
  (void)state;

  ptz_adapter_init();

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

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_continuous_move(1, -1, 5);
  assert_int_equal(PLATFORM_SUCCESS, result);

  // Continuous move uses platform_ptz_turn with large values
  platform_ptz_direction_t dir;
  int steps;
  assert_int_equal(1, platform_mock_get_last_ptz_turn(&dir, &steps));
}

void test_unit_ptz_adapter_continuous_move_no_timeout(void** state) {
  (void)state;

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_continuous_move(1, 0, 0);
  assert_int_equal(PLATFORM_SUCCESS, result);
}

/* ============================================================================
 * Stop Tests
 * ============================================================================ */

void test_unit_ptz_adapter_stop_success(void** state) {
  (void)state;

  ptz_adapter_init();

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

  ptz_adapter_init();

  platform_result_t result = ptz_adapter_set_preset("TestPreset", 1);
  assert_int_equal(PLATFORM_SUCCESS, result);
}

void test_unit_ptz_adapter_goto_preset_home(void** state) {
  (void)state;

  ptz_adapter_init();

  // Move away from home
  ptz_adapter_absolute_move(100, 50, 50);

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
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_success, ptz_adapter_test_setup,
                                  ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_idempotent, ptz_adapter_test_setup,
                                  ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_init_failure, ptz_adapter_test_setup,
                                  ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_cleanup_safe_when_not_initialized,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Absolute move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_success,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_pan_max,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_pan_min,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_tilt_max,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_clamping_tilt_min,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_absolute_move_not_initialized,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Status tracking tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_success,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_null_parameter,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_get_status_not_initialized,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_position_tracking_after_move,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Relative move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_relative_move_positive_delta,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_relative_move_delta_clamping,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Continuous move tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_continuous_move_with_timeout,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_continuous_move_no_timeout,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),

  // Stop tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_stop_success, ptz_adapter_test_setup,
                                  ptz_adapter_test_teardown),

  // Preset tests
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_set_preset, ptz_adapter_test_setup,
                                  ptz_adapter_test_teardown),
  cmocka_unit_test_setup_teardown(test_unit_ptz_adapter_goto_preset_home,
                                  ptz_adapter_test_setup, ptz_adapter_test_teardown),
};
