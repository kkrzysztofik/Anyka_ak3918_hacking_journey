/**
 * @file platform_ptz_mock.c
 * @brief Mock implementation for platform PTZ functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "platform_ptz_mock.h"

#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../../src/platform/platform_common.h"
#include "../../src/services/ptz/onvif_ptz.h"

/* ============================================================================
 * Mock Constants
 * ============================================================================ */

#define PTZ_MOCK_PRESET_NAME_SIZE 64
#define PTZ_MOCK_DEFAULT_SPEED    50

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

// Global PTZ mock state
static struct {
  int initialized;
  int init_call_count;
  int cleanup_call_count;
  int error_simulation_enabled;
  platform_result_t error_code;

  // PTZ status
  struct ptz_device_status current_status;
  int status_requested;

  // Movement tracking
  int last_absolute_move_pan;
  int last_absolute_move_tilt;
  int last_absolute_move_speed;
  int absolute_move_called;

  int last_relative_move_pan_delta;
  int last_relative_move_tilt_delta;
  int last_relative_move_speed;
  int relative_move_called;

  int last_continuous_move_pan_vel;
  int last_continuous_move_tilt_vel;
  int last_continuous_move_timeout;
  int continuous_move_called;

  // Preset tracking
  char last_set_preset_name[PTZ_MOCK_PRESET_NAME_SIZE];
  int last_set_preset_id;
  int set_preset_called;

  int last_goto_preset_id;
  int goto_preset_called;

  // Position tracking
  int current_pan_deg;
  int current_tilt_deg;

  // Turn tracking
  platform_ptz_direction_t last_turn_direction;
  int last_turn_steps;
  int turn_called;

  platform_ptz_direction_t last_turn_stop_direction;
  int turn_stop_called;

  pthread_mutex_t mutex;
} g_ptz_mock_state = {0}; // NOLINT

/* ============================================================================
 * Mock Initialization and Cleanup
 * ============================================================================ */

void platform_ptz_mock_init(void) {
  pthread_mutex_init(&g_ptz_mock_state.mutex, NULL);
  g_ptz_mock_state.initialized = 1;
  g_ptz_mock_state.init_call_count = 0;
  g_ptz_mock_state.cleanup_call_count = 0;
  g_ptz_mock_state.error_simulation_enabled = 0;
  g_ptz_mock_state.error_code = PLATFORM_SUCCESS;

  // Initialize PTZ status
  memset(&g_ptz_mock_state.current_status, 0, sizeof(struct ptz_device_status));
  g_ptz_mock_state.current_status.h_pos_deg = 0;
  g_ptz_mock_state.current_status.v_pos_deg = 0;
  g_ptz_mock_state.current_status.h_speed = 0;
  g_ptz_mock_state.current_status.v_speed = 0;
  g_ptz_mock_state.status_requested = 0;

  // Initialize movement tracking
  g_ptz_mock_state.absolute_move_called = 0;
  g_ptz_mock_state.relative_move_called = 0;
  g_ptz_mock_state.continuous_move_called = 0;

  // Initialize preset tracking
  g_ptz_mock_state.set_preset_called = 0;
  g_ptz_mock_state.goto_preset_called = 0;

  // Initialize position
  g_ptz_mock_state.current_pan_deg = 0;
  g_ptz_mock_state.current_tilt_deg = 0;

  // Initialize turn tracking
  g_ptz_mock_state.turn_called = 0;
  g_ptz_mock_state.turn_stop_called = 0;
}

void platform_ptz_mock_cleanup(void) {
  pthread_mutex_destroy(&g_ptz_mock_state.mutex);
  memset(&g_ptz_mock_state, 0, sizeof(g_ptz_mock_state));
}

void platform_ptz_mock_reset(void) {
  platform_ptz_mock_cleanup();
  platform_ptz_mock_init();
}

/* ============================================================================
 * PTZ Status Mock Functions
 * ============================================================================ */

void platform_mock_set_ptz_status(const struct ptz_device_status* status) {
  if (!status) {
    return;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  memcpy(&g_ptz_mock_state.current_status, status, sizeof(struct ptz_device_status));
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

const struct ptz_device_status* platform_mock_get_last_ptz_status_request(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  const struct ptz_device_status* status =
    g_ptz_mock_state.status_requested ? &g_ptz_mock_state.current_status : NULL;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return status;
}

/* ============================================================================
 * PTZ Movement Mock Functions
 * ============================================================================ */

void platform_mock_set_ptz_move_result(platform_result_t result) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_code = result;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

void platform_mock_set_ptz_stop_result(platform_result_t result) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_code = result;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

int platform_mock_get_last_ptz_absolute_move(int* pan_deg, int* tilt_deg, int* speed) {
  if (!pan_deg || !tilt_deg || !speed) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.absolute_move_called;
  if (called) {
    *pan_deg = g_ptz_mock_state.last_absolute_move_pan;
    *tilt_deg = g_ptz_mock_state.last_absolute_move_tilt;
    *speed = g_ptz_mock_state.last_absolute_move_speed;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

int platform_mock_get_last_ptz_relative_move(int* pan_delta, int* tilt_delta, int* speed) {
  if (!pan_delta || !tilt_delta || !speed) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.relative_move_called;
  if (called) {
    *pan_delta = g_ptz_mock_state.last_relative_move_pan_delta;
    *tilt_delta = g_ptz_mock_state.last_relative_move_tilt_delta;
    *speed = g_ptz_mock_state.last_relative_move_speed;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

int platform_mock_get_last_ptz_continuous_move(int* pan_vel, int* tilt_vel, int* timeout_s) {
  if (!pan_vel || !tilt_vel || !timeout_s) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.continuous_move_called;
  if (called) {
    *pan_vel = g_ptz_mock_state.last_continuous_move_pan_vel;
    *tilt_vel = g_ptz_mock_state.last_continuous_move_tilt_vel;
    *timeout_s = g_ptz_mock_state.last_continuous_move_timeout;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

/* ============================================================================
 * PTZ Initialization Mock Functions
 * ============================================================================ */

void platform_mock_set_ptz_init_result(platform_result_t result) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_code = result;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

void platform_mock_set_ptz_cleanup_result(platform_result_t result) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_code = result;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

int platform_mock_get_ptz_init_call_count(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int count = g_ptz_mock_state.init_call_count;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return count;
}

int platform_mock_get_ptz_cleanup_call_count(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int count = g_ptz_mock_state.cleanup_call_count;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return count;
}

/* ============================================================================
 * PTZ Preset Mock Functions
 * ============================================================================ */

void platform_mock_set_ptz_preset_result(platform_result_t result) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_code = result;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

int platform_mock_get_last_ptz_set_preset(char* name, size_t name_size, int* preset_id) {
  if (!name || !preset_id || name_size == 0) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.set_preset_called;
  if (called) {
    strncpy(name, g_ptz_mock_state.last_set_preset_name, name_size - 1);
    name[name_size - 1] = '\0';
    *preset_id = g_ptz_mock_state.last_set_preset_id;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

int platform_mock_get_last_ptz_goto_preset(int* preset_id) {
  if (!preset_id) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.goto_preset_called;
  if (called) {
    *preset_id = g_ptz_mock_state.last_goto_preset_id;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

/* ============================================================================
 * PTZ Position Mock Functions
 * ============================================================================ */

void platform_mock_set_ptz_position(int pan_deg, int tilt_deg) { // NOLINT
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.current_pan_deg = pan_deg;
  g_ptz_mock_state.current_tilt_deg = tilt_deg;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

void platform_mock_get_ptz_position(int* pan_deg, int* tilt_deg) {
  if (!pan_deg || !tilt_deg) {
    return;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  *pan_deg = g_ptz_mock_state.current_pan_deg;
  *tilt_deg = g_ptz_mock_state.current_tilt_deg;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

/* ============================================================================
 * PTZ Movement Direction Mock Functions
 * ============================================================================ */

int platform_mock_get_last_ptz_turn(platform_ptz_direction_t* direction, int* steps) {
  if (!direction || !steps) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.turn_called;
  if (called) {
    *direction = g_ptz_mock_state.last_turn_direction;
    *steps = g_ptz_mock_state.last_turn_steps;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

int platform_mock_get_last_ptz_turn_stop(platform_ptz_direction_t* direction) {
  if (!direction) {
    return 0;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int called = g_ptz_mock_state.turn_stop_called;
  if (called) {
    *direction = g_ptz_mock_state.last_turn_stop_direction;
  }
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return called;
}

/* ============================================================================
 * PTZ Error Simulation
 * ============================================================================ */

void platform_mock_enable_ptz_error(platform_result_t error_code) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_simulation_enabled = 1;
  g_ptz_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

void platform_mock_disable_ptz_error(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.error_simulation_enabled = 0;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

int platform_mock_is_ptz_error_enabled(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int enabled = g_ptz_mock_state.error_simulation_enabled;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return enabled;
}

/* ============================================================================
 * PTZ Function Implementations
 * ============================================================================ */

platform_result_t platform_ptz_init(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.init_call_count++;

  if (g_ptz_mock_state.error_simulation_enabled) {
    platform_result_t error = g_ptz_mock_state.error_code;
    pthread_mutex_unlock(&g_ptz_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

void platform_ptz_cleanup(void) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.cleanup_call_count++;
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
}

platform_result_t platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg) { // NOLINT
  (void)pan_range_deg;
  (void)tilt_range_deg;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_check_self(void) {
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg) { // NOLINT
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.last_absolute_move_pan = pan_deg;
  g_ptz_mock_state.last_absolute_move_tilt = tilt_deg;
  g_ptz_mock_state.last_absolute_move_speed = PTZ_MOCK_DEFAULT_SPEED; // Default speed
  g_ptz_mock_state.absolute_move_called = 1;

  if (g_ptz_mock_state.error_simulation_enabled) {
    platform_result_t error = g_ptz_mock_state.error_code;
    pthread_mutex_unlock(&g_ptz_mock_state.mutex);
    return error;
  }

  // Update position
  g_ptz_mock_state.current_pan_deg = pan_deg;
  g_ptz_mock_state.current_tilt_deg = tilt_deg;

  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

int platform_ptz_get_step_position(platform_ptz_axis_t axis) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  int position = 0;

  switch (axis) {
  case PLATFORM_PTZ_AXIS_PAN:
    position = g_ptz_mock_state.current_pan_deg;
    break;
  case PLATFORM_PTZ_AXIS_TILT:
    position = g_ptz_mock_state.current_tilt_deg;
    break;
  case PLATFORM_PTZ_AXIS_ZOOM:
    position = 0; // Mock zoom position
    break;
  }

  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return position;
}

platform_result_t platform_ptz_get_status(platform_ptz_axis_t axis, platform_ptz_status_t* status) {
  if (!status) {
    return PLATFORM_ERROR_NULL;
  }

  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.status_requested = 1;
  *status = PLATFORM_PTZ_STATUS_STOPPED; // Mock status
  pthread_mutex_unlock(&g_ptz_mock_state.mutex);

  (void)axis; // Mock doesn't differentiate by axis
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_set_speed(platform_ptz_axis_t axis, int speed) { // NOLINT
  (void)axis;
  (void)speed;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn(platform_ptz_direction_t direction, int steps) { // NOLINT
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.last_turn_direction = direction;
  g_ptz_mock_state.last_turn_steps = steps;
  g_ptz_mock_state.turn_called = 1;

  if (g_ptz_mock_state.error_simulation_enabled) {
    platform_result_t error = g_ptz_mock_state.error_code;
    pthread_mutex_unlock(&g_ptz_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn_stop(platform_ptz_direction_t direction) {
  pthread_mutex_lock(&g_ptz_mock_state.mutex);
  g_ptz_mock_state.last_turn_stop_direction = direction;
  g_ptz_mock_state.turn_stop_called = 1;

  if (g_ptz_mock_state.error_simulation_enabled) {
    platform_result_t error = g_ptz_mock_state.error_code;
    pthread_mutex_unlock(&g_ptz_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_mock_state.mutex);
  return PLATFORM_SUCCESS;
}
