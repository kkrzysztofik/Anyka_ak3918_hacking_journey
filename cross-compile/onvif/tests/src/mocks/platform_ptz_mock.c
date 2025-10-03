/**
 * @file platform_ptz_mock.c
 * @brief CMocka PTZ platform mock helper implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "platform_ptz_mock.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <cmocka.h>
#include <string.h>

/* ============================================================================
 * Static State Variables
 * ============================================================================ */

static int g_ptz_init_call_count = 0;
static int g_ptz_error_enabled = 0;
static platform_result_t g_ptz_error_code = PLATFORM_ERROR;

// Last call parameters
static struct {
  int pan;
  int tilt;
  int speed;
  int valid;
} g_last_absolute_move = {0, 0, 0, 0};

static struct {
  platform_ptz_direction_t dir;
  int steps;
  int valid;
} g_last_turn = {0, 0, 0};

static struct {
  platform_ptz_direction_t dir;
  int valid;
} g_last_turn_stop = {0, 0};

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

void platform_ptz_mock_init(void) {
  platform_ptz_mock_reset();
}

void platform_ptz_mock_cleanup(void) {
  // No-op with CMocka - cleanup is automatic
}

void platform_ptz_mock_reset(void) {
  g_ptz_init_call_count = 0;
  g_ptz_error_enabled = 0;
  g_ptz_error_code = PLATFORM_ERROR;

  g_last_absolute_move.valid = 0;
  g_last_turn.valid = 0;
  g_last_turn_stop.valid = 0;
}

/* ============================================================================
 * Mock Result Configuration
 * ============================================================================ */

void platform_mock_set_ptz_init_result(platform_result_t result) {
  will_return(__wrap_platform_ptz_init, result);
}

void platform_mock_set_ptz_move_result(platform_result_t result) {
  will_return(__wrap_platform_ptz_set_degree, result);
}

void platform_mock_set_ptz_stop_result(platform_result_t result) {
  will_return(__wrap_platform_ptz_turn_stop, result);
}

void platform_mock_set_ptz_preset_result(platform_result_t result) {
  will_return(__wrap_platform_ptz_move_to_position, result);
}

/* ============================================================================
 * Error Simulation
 * ============================================================================ */

void platform_mock_enable_ptz_error(platform_result_t error) {
  g_ptz_error_enabled = 1;
  g_ptz_error_code = error;
  will_return(__wrap_platform_ptz_init, error);
}

void platform_mock_disable_ptz_error(void) {
  g_ptz_error_enabled = 0;
}

/* ============================================================================
 * Mock Call Tracking
 * ============================================================================ */

int platform_mock_get_ptz_init_call_count(void) {
  return g_ptz_init_call_count;
}

int platform_mock_get_last_ptz_absolute_move(int* pan, int* tilt, int* speed) {
  if (!g_last_absolute_move.valid) {
    return 0;
  }

  if (pan) *pan = g_last_absolute_move.pan;
  if (tilt) *tilt = g_last_absolute_move.tilt;
  if (speed) *speed = g_last_absolute_move.speed;

  return 1;
}

int platform_mock_get_last_ptz_turn(platform_ptz_direction_t* dir, int* steps) {
  if (!g_last_turn.valid) {
    return 0;
  }

  if (dir) *dir = g_last_turn.dir;
  if (steps) *steps = g_last_turn.steps;

  return 1;
}

int platform_mock_get_last_ptz_turn_stop(platform_ptz_direction_t* dir) {
  if (!g_last_turn_stop.valid) {
    return 0;
  }

  if (dir) *dir = g_last_turn_stop.dir;

  return 1;
}

/* ============================================================================
 * Internal Helpers (called by CMocka wrapped functions or tracking)
 * ============================================================================ */

/**
 * @brief Record PTZ init call
 */
void platform_ptz_mock_record_init(void) {
  g_ptz_init_call_count++;
}

/**
 * @brief Record PTZ absolute move call
 * @param pan Pan value
 * @param tilt Tilt value
 * @param speed Speed value
 */
void platform_ptz_mock_record_absolute_move(int pan, int tilt, int speed) {
  g_last_absolute_move.pan = pan;
  g_last_absolute_move.tilt = tilt;
  g_last_absolute_move.speed = speed;
  g_last_absolute_move.valid = 1;
}

/**
 * @brief Record PTZ turn call
 * @param dir Direction
 * @param steps Steps
 */
void platform_ptz_mock_record_turn(platform_ptz_direction_t dir, int steps) {
  g_last_turn.dir = dir;
  g_last_turn.steps = steps;
  g_last_turn.valid = 1;
}

/**
 * @brief Record PTZ turn stop call
 * @param dir Direction
 */
void platform_ptz_mock_record_turn_stop(platform_ptz_direction_t dir) {
  g_last_turn_stop.dir = dir;
  g_last_turn_stop.valid = 1;
}
