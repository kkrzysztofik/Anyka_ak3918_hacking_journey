/**
 * @file ptz_adapter_mock.c
 * @brief Mock implementation for PTZ adapter layer
 * @author kkrzysztofik
 * @date 2025
 *
 * This mock provides a test double for the PTZ adapter layer, allowing
 * unit tests to isolate and test the ONVIF service layer independently.
 */

#include "ptz_adapter_mock.h"

#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platform/platform_common.h"
#include "services/ptz/onvif_ptz.h"

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

typedef struct {
  // Initialization state
  int initialized;
  int init_call_count;
  int cleanup_call_count;

  // Current position state
  int current_pan_deg;
  int current_tilt_deg;

  // Movement tracking
  int last_absolute_pan;
  int last_absolute_tilt;
  int last_absolute_speed;
  int absolute_move_called;

  int last_relative_pan_delta;
  int last_relative_tilt_delta;
  int last_relative_speed;
  int relative_move_called;

  int last_continuous_pan_vel;
  int last_continuous_tilt_vel;
  int last_continuous_timeout;
  int continuous_move_called;

  int stop_called;

  // Preset tracking
  char last_set_preset_name[64];
  int last_set_preset_id;
  int set_preset_called;

  int last_goto_preset_id;
  int goto_preset_called;

  // Status tracking
  int get_status_called;

  // Error simulation
  int error_simulation_enabled;
  platform_result_t error_code;

  pthread_mutex_t mutex;
} ptz_adapter_mock_state_t;

static ptz_adapter_mock_state_t g_ptz_adapter_mock = {0}; // NOLINT

/* ============================================================================
 * Mock Initialization and Cleanup
 * ============================================================================ */

void ptz_adapter_mock_init(void) {
  pthread_mutex_init(&g_ptz_adapter_mock.mutex, NULL);
  memset(&g_ptz_adapter_mock, 0, sizeof(g_ptz_adapter_mock));
  g_ptz_adapter_mock.initialized = 1;
  g_ptz_adapter_mock.error_code = PLATFORM_SUCCESS;
}

void ptz_adapter_mock_cleanup(void) {
  pthread_mutex_destroy(&g_ptz_adapter_mock.mutex);
  memset(&g_ptz_adapter_mock, 0, sizeof(g_ptz_adapter_mock));
}

void ptz_adapter_mock_reset(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  // Reset call tracking
  g_ptz_adapter_mock.absolute_move_called = 0;
  g_ptz_adapter_mock.relative_move_called = 0;
  g_ptz_adapter_mock.continuous_move_called = 0;
  g_ptz_adapter_mock.stop_called = 0;
  g_ptz_adapter_mock.set_preset_called = 0;
  g_ptz_adapter_mock.goto_preset_called = 0;
  g_ptz_adapter_mock.get_status_called = 0;

  // Reset position to home
  g_ptz_adapter_mock.current_pan_deg = 0;
  g_ptz_adapter_mock.current_tilt_deg = 0;

  // Disable error simulation
  g_ptz_adapter_mock.error_simulation_enabled = 0;
  g_ptz_adapter_mock.error_code = PLATFORM_SUCCESS;

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

void ptz_adapter_mock_set_position(int pan_deg, int tilt_deg) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.current_pan_deg = pan_deg;
  g_ptz_adapter_mock.current_tilt_deg = tilt_deg;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

void ptz_adapter_mock_get_position(int* pan_deg, int* tilt_deg) {
  if (!pan_deg || !tilt_deg) {
    return;
  }

  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  *pan_deg = g_ptz_adapter_mock.current_pan_deg;
  *tilt_deg = g_ptz_adapter_mock.current_tilt_deg;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

void ptz_adapter_mock_enable_error(platform_result_t error_code) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.error_simulation_enabled = 1;
  g_ptz_adapter_mock.error_code = error_code;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

void ptz_adapter_mock_disable_error(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.error_simulation_enabled = 0;
  g_ptz_adapter_mock.error_code = PLATFORM_SUCCESS;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

/* ============================================================================
 * Mock Query Functions
 * ============================================================================ */

int ptz_adapter_mock_get_init_call_count(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  int count = g_ptz_adapter_mock.init_call_count;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return count;
}

int ptz_adapter_mock_was_absolute_move_called(int* pan, int* tilt, int* speed) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  int called = g_ptz_adapter_mock.absolute_move_called;
  if (called && pan && tilt && speed) {
    *pan = g_ptz_adapter_mock.last_absolute_pan;
    *tilt = g_ptz_adapter_mock.last_absolute_tilt;
    *speed = g_ptz_adapter_mock.last_absolute_speed;
  }
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return called;
}

int ptz_adapter_mock_was_relative_move_called(int* pan_delta, int* tilt_delta, int* speed) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  int called = g_ptz_adapter_mock.relative_move_called;
  if (called && pan_delta && tilt_delta && speed) {
    *pan_delta = g_ptz_adapter_mock.last_relative_pan_delta;
    *tilt_delta = g_ptz_adapter_mock.last_relative_tilt_delta;
    *speed = g_ptz_adapter_mock.last_relative_speed;
  }
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return called;
}

int ptz_adapter_mock_was_continuous_move_called(int* pan_vel, int* tilt_vel, int* timeout) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  int called = g_ptz_adapter_mock.continuous_move_called;
  if (called && pan_vel && tilt_vel && timeout) {
    *pan_vel = g_ptz_adapter_mock.last_continuous_pan_vel;
    *tilt_vel = g_ptz_adapter_mock.last_continuous_tilt_vel;
    *timeout = g_ptz_adapter_mock.last_continuous_timeout;
  }
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return called;
}

int ptz_adapter_mock_was_stop_called(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  int called = g_ptz_adapter_mock.stop_called;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return called;
}

/* ============================================================================
 * PTZ Adapter Mock Implementations
 * ============================================================================ */

platform_result_t ptz_adapter_init(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.init_call_count++;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  g_ptz_adapter_mock.initialized = 1;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

void ptz_adapter_cleanup(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.cleanup_call_count++;
  g_ptz_adapter_mock.initialized = 0;
  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
}

platform_result_t ptz_adapter_get_status(struct ptz_device_status* status) {
  if (!status) {
    return PLATFORM_ERROR_NULL;
  }

  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);
  g_ptz_adapter_mock.get_status_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  status->h_pos_deg = g_ptz_adapter_mock.current_pan_deg;
  status->v_pos_deg = g_ptz_adapter_mock.current_tilt_deg;
  status->h_speed = 0;
  status->v_speed = 0;

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees, int move_speed) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  g_ptz_adapter_mock.last_absolute_pan = pan_degrees;
  g_ptz_adapter_mock.last_absolute_tilt = tilt_degrees;
  g_ptz_adapter_mock.last_absolute_speed = move_speed;
  g_ptz_adapter_mock.absolute_move_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  // Update position on success
  g_ptz_adapter_mock.current_pan_deg = pan_degrees;
  g_ptz_adapter_mock.current_tilt_deg = tilt_degrees;

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_relative_move(int pan_delta_degrees, int tilt_delta_degrees,
                                            int move_speed) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  g_ptz_adapter_mock.last_relative_pan_delta = pan_delta_degrees;
  g_ptz_adapter_mock.last_relative_tilt_delta = tilt_delta_degrees;
  g_ptz_adapter_mock.last_relative_speed = move_speed;
  g_ptz_adapter_mock.relative_move_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  // Update position on success
  g_ptz_adapter_mock.current_pan_deg += pan_delta_degrees;
  g_ptz_adapter_mock.current_tilt_deg += tilt_delta_degrees;

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity,
                                              int timeout_seconds) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  g_ptz_adapter_mock.last_continuous_pan_vel = pan_velocity;
  g_ptz_adapter_mock.last_continuous_tilt_vel = tilt_velocity;
  g_ptz_adapter_mock.last_continuous_timeout = timeout_seconds;
  g_ptz_adapter_mock.continuous_move_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_stop(void) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  g_ptz_adapter_mock.stop_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_set_preset(const char* name, int preset_id) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  if (name) {
    strncpy(g_ptz_adapter_mock.last_set_preset_name, name,
            sizeof(g_ptz_adapter_mock.last_set_preset_name) - 1);
    g_ptz_adapter_mock.last_set_preset_name[sizeof(g_ptz_adapter_mock.last_set_preset_name) - 1] = '\0';
  }
  g_ptz_adapter_mock.last_set_preset_id = preset_id;
  g_ptz_adapter_mock.set_preset_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_goto_preset(int preset_id) {
  pthread_mutex_lock(&g_ptz_adapter_mock.mutex);

  g_ptz_adapter_mock.last_goto_preset_id = preset_id;
  g_ptz_adapter_mock.goto_preset_called = 1;

  if (g_ptz_adapter_mock.error_simulation_enabled) {
    platform_result_t error = g_ptz_adapter_mock.error_code;
    pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
    return error;
  }

  // For preset 1 (home), return to origin
  if (preset_id == 1) {
    g_ptz_adapter_mock.current_pan_deg = 0;
    g_ptz_adapter_mock.current_tilt_deg = 0;
  }

  pthread_mutex_unlock(&g_ptz_adapter_mock.mutex);
  return PLATFORM_SUCCESS;
}
