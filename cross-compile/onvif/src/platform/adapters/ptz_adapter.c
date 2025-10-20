/**
 * @file ptz_adapter.c
 * @brief PTZ control abstraction adapter for Anyka platform
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides the adapter layer between ONVIF PTZ service and platform-specific
 * PTZ hardware operations. It handles coordinate transformations, state management,
 * and provides a mockable interface for testing.
 */

#include "platform/adapters/ptz_adapter.h"

#include <errno.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "platform/platform.h"
#include "platform/platform_common.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * PTZ Adapter Constants
 * ============================================================================ */

#define PTZ_MAX_PAN_DEGREES    350
#define PTZ_MIN_PAN_DEGREES    -350
#define PTZ_MAX_TILT_DEGREES   130
#define PTZ_MIN_TILT_DEGREES   -130
#define PTZ_MAX_STEP_SIZE_PAN  16
#define PTZ_MAX_STEP_SIZE_TILT 8

/* ============================================================================
 * PTZ Adapter Global State
 * ============================================================================ */

static pthread_mutex_t g_ptz_adapter_lock = PTHREAD_MUTEX_INITIALIZER;     // NOLINT
static pthread_cond_t g_ptz_adapter_timer_cond = PTHREAD_COND_INITIALIZER; // NOLINT
static int g_ptz_adapter_initialized = 0;                                  // NOLINT
static int g_ptz_adapter_current_pan_pos = 0;                              // NOLINT
static int g_ptz_adapter_current_tilt_pos = 0;                             // NOLINT
static pthread_t g_ptz_adapter_continuous_move_timer_thread = 0;           // NOLINT
static int g_ptz_adapter_continuous_move_timeout_s = 0;                    // NOLINT
static volatile int g_ptz_adapter_continuous_move_active = 0;              // NOLINT
static volatile int g_ptz_adapter_timer_shutdown_requested = 0;            // NOLINT

/* ============================================================================
 * PTZ Adapter Helper Functions
 * ============================================================================ */

/**
 * @brief Absolute value helper (simple implementation)
 * @param value Input value
 * @return Absolute value
 */
static int simple_abs(int value) {
  return value < 0 ? -value : value;
}

/**
 * @brief Timeout thread for continuous move operations
 * @param unused_arg Unused argument (required by pthread interface)
 * @return NULL
 */
static void* continuous_move_timeout_thread(void* unused_arg) {
  (void)unused_arg; /* Suppress unused parameter warning */

  /* Use condition variable with timed wait instead of sleep */
  struct timespec timeout;
  clock_gettime(CLOCK_REALTIME, &timeout);
  timeout.tv_sec += g_ptz_adapter_continuous_move_timeout_s;

  platform_log_debug("[PTZ][timeout-thread] armed (timeout=%d s)\n", g_ptz_adapter_continuous_move_timeout_s);

  pthread_mutex_lock(&g_ptz_adapter_lock);

  /* Wait for timeout or shutdown signal */
  int wait_result = pthread_cond_timedwait(&g_ptz_adapter_timer_cond, &g_ptz_adapter_lock, &timeout);

  /* Check if shutdown was requested */
  if (g_ptz_adapter_timer_shutdown_requested) {
    platform_log_debug("[PTZ][timeout-thread] shutdown requested before timeout\n");
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return NULL;
  }

  /* If timeout occurred and continuous move is still active, stop it */
  if (wait_result == ETIMEDOUT && g_ptz_adapter_continuous_move_active) {
    platform_log_info("PTZ continuous move timeout after %ds, stopping movement\n", g_ptz_adapter_continuous_move_timeout_s);

    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
    platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
    g_ptz_adapter_continuous_move_active = 0;
  } else {
    platform_log_debug("[PTZ][timeout-thread] woke; wait_result=%d active=%d\n", wait_result, g_ptz_adapter_continuous_move_active);
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ][timeout-thread] exiting\n");
  return NULL;
}

/* ============================================================================
 * PTZ Adapter Public Interface
 * ============================================================================ */

platform_result_t ptz_adapter_init(void) {
  platform_result_t ret = PLATFORM_SUCCESS;
  platform_log_debug("[PTZ] init requested\n");
  pthread_mutex_lock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] init state before call: initialized=%d\n", g_ptz_adapter_initialized);
  if (!g_ptz_adapter_initialized) {
    ret = platform_ptz_init();
    if (ret == PLATFORM_SUCCESS) {
      /* PTZ is already initialized with proper parameters in
       * platform_ptz_init() */
      /* Reset to center position */
      g_ptz_adapter_current_pan_pos = 0;
      g_ptz_adapter_current_tilt_pos = 0;
      platform_ptz_move_to_position(g_ptz_adapter_current_pan_pos, g_ptz_adapter_current_tilt_pos);

      g_ptz_adapter_initialized = 1;
      platform_log_notice("PTZ adapter initialized successfully\n");
    } else {
      platform_log_error("PTZ initialization failed: %d\n", ret);
    }
  }
  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] init returning %d\n", ret);
  return ret;
}

void ptz_adapter_cleanup(void) {
  platform_log_debug("[PTZ] cleanup requested\n");
  pthread_mutex_lock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] cleanup state: initialized=%d active=%d thread=%lu\n", g_ptz_adapter_initialized, g_ptz_adapter_continuous_move_active,
                     (unsigned long)g_ptz_adapter_continuous_move_timer_thread);
  if (g_ptz_adapter_initialized) {
    /* Stop any ongoing continuous movement */
    if (g_ptz_adapter_continuous_move_active) {
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
      platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);
      g_ptz_adapter_continuous_move_active = 0;
    }

    /* Signal and wait for timeout thread to finish if it exists */
    if (g_ptz_adapter_continuous_move_timer_thread != 0) {
      pthread_t timer_thread = g_ptz_adapter_continuous_move_timer_thread;
      g_ptz_adapter_continuous_move_timer_thread = 0;

      /* Signal shutdown to timer thread */
      g_ptz_adapter_timer_shutdown_requested = 1;
      pthread_cond_signal(&g_ptz_adapter_timer_cond);

      /* Unlock mutex before joining to avoid deadlock */
      pthread_mutex_unlock(&g_ptz_adapter_lock);
      platform_log_debug("[PTZ] cleanup joining timeout thread\n");
      pthread_join(timer_thread, NULL);
      pthread_mutex_lock(&g_ptz_adapter_lock);

      /* Reset shutdown flag for next use */
      g_ptz_adapter_timer_shutdown_requested = 0;
    }

    platform_ptz_cleanup();
    g_ptz_adapter_initialized = 0;
  }
  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] cleanup complete\n");
}

platform_result_t ptz_adapter_get_status(struct ptz_device_status* status) {
  ONVIF_CHECK_NULL(status);

  pthread_mutex_lock(&g_ptz_adapter_lock);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  // Use current position tracking instead of platform function
  status->h_pos_deg = g_ptz_adapter_current_pan_pos;
  status->v_pos_deg = g_ptz_adapter_current_tilt_pos;
  status->h_speed = 0;
  status->v_speed = 0;

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees, // NOLINT
                                            int move_speed) {                  // NOLINT
  (void)move_speed;                                                            // Speed parameter not used by current platform implementation

  platform_log_debug("[PTZ] absolute move request pan=%d tilt=%d speed=%d\n", pan_degrees, tilt_degrees, move_speed);
  pthread_mutex_lock(&g_ptz_adapter_lock);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  /* Clamp values to safe ranges - based on akipc implementation */
  if (pan_degrees > PTZ_MAX_PAN_DEGREES) {
    pan_degrees = PTZ_MAX_PAN_DEGREES;
  }
  if (pan_degrees < PTZ_MIN_PAN_DEGREES) {
    pan_degrees = PTZ_MIN_PAN_DEGREES;
  }
  if (tilt_degrees > PTZ_MAX_TILT_DEGREES) {
    tilt_degrees = PTZ_MAX_TILT_DEGREES;
  }
  if (tilt_degrees < PTZ_MIN_TILT_DEGREES) {
    tilt_degrees = PTZ_MIN_TILT_DEGREES;
  }

  platform_log_info("PTZ absolute move to pan=%d, tilt=%d\n", pan_degrees, tilt_degrees);

  platform_result_t ret = platform_ptz_move_to_position(pan_degrees, tilt_degrees);
  if (ret == PLATFORM_SUCCESS) {
    g_ptz_adapter_current_pan_pos = pan_degrees;
    g_ptz_adapter_current_tilt_pos = tilt_degrees;
    platform_log_debug("[PTZ] absolute move updated position pan=%d tilt=%d\n", g_ptz_adapter_current_pan_pos, g_ptz_adapter_current_tilt_pos);
  } else {
    platform_log_debug("[PTZ] absolute move platform call failed ret=%d\n", ret);
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  return ret;
}

platform_result_t ptz_adapter_relative_move(int pan_delta_degrees,  // NOLINT
                                            int tilt_delta_degrees, // NOLINT
                                            int move_speed) {       // NOLINT
  (void)move_speed;                                                 // Speed parameter not used by current platform implementation

  platform_log_debug("[PTZ] relative move request pan_delta=%d tilt_delta=%d speed=%d\n", pan_delta_degrees, tilt_delta_degrees, move_speed);
  pthread_mutex_lock(&g_ptz_adapter_lock);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  platform_log_info("PTZ relative move pan_delta=%d, tilt_delta=%d\n", pan_delta_degrees, tilt_delta_degrees);

  platform_result_t ret = PLATFORM_SUCCESS;

  /* Horizontal movement - based on akipc implementation with step size 16 */
  if (pan_delta_degrees != 0) {
    platform_ptz_direction_t dir = (pan_delta_degrees > 0) ? PLATFORM_PTZ_DIRECTION_LEFT : PLATFORM_PTZ_DIRECTION_RIGHT;
    int steps = simple_abs(pan_delta_degrees);
    if (steps > PTZ_MAX_STEP_SIZE_PAN) {
      steps = PTZ_MAX_STEP_SIZE_PAN; /* Limit step size like in akipc */
    }

    platform_log_debug("[PTZ] relative move pan dir=%d steps=%d\n", dir, steps);
    ret = platform_ptz_turn(dir, steps);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_adapter_current_pan_pos += (dir == PLATFORM_PTZ_DIRECTION_LEFT) ? steps : -steps;
      platform_log_debug("[PTZ] relative pan new position=%d\n", g_ptz_adapter_current_pan_pos);
    }
  }

  /* Vertical movement - based on akipc implementation with step size 8 */
  if (tilt_delta_degrees != 0) {
    platform_ptz_direction_t dir = (tilt_delta_degrees > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
    int steps = simple_abs(tilt_delta_degrees);
    if (steps > PTZ_MAX_STEP_SIZE_TILT) {
      steps = PTZ_MAX_STEP_SIZE_TILT; /* Limit step size like in akipc */
    }

    platform_log_debug("[PTZ] relative move tilt dir=%d steps=%d\n", dir, steps);
    ret = platform_ptz_turn(dir, steps);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_adapter_current_tilt_pos += (dir == PLATFORM_PTZ_DIRECTION_DOWN) ? steps : -steps;
      platform_log_debug("[PTZ] relative tilt new position=%d\n", g_ptz_adapter_current_tilt_pos);
    }
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] relative move returning %d\n", ret);
  return ret;
}

platform_result_t ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity, // NOLINT
                                              int timeout_seconds) {
  platform_log_debug("[PTZ] continuous move request pan_vel=%d tilt_vel=%d timeout=%d\n", pan_velocity, tilt_velocity, timeout_seconds);
  pthread_mutex_lock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] continuous move state before start: initialized=%d active=%d thread=%lu\n", g_ptz_adapter_initialized,
                     g_ptz_adapter_continuous_move_active, (unsigned long)g_ptz_adapter_continuous_move_timer_thread);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  /* Stop any existing continuous movement */
  if (g_ptz_adapter_continuous_move_active) {
    platform_log_debug("[PTZ] stopping existing continuous move before starting new one\n");
    g_ptz_adapter_continuous_move_active = 0;

    /* Wait for existing timer thread to finish */
    if (g_ptz_adapter_continuous_move_timer_thread != 0) {
      pthread_t timer_thread = g_ptz_adapter_continuous_move_timer_thread;
      g_ptz_adapter_continuous_move_timer_thread = 0;

      /* Signal shutdown to timer thread */
      g_ptz_adapter_timer_shutdown_requested = 1;
      pthread_cond_signal(&g_ptz_adapter_timer_cond);

      /* Unlock mutex before joining to avoid deadlock */
      pthread_mutex_unlock(&g_ptz_adapter_lock);
      platform_log_debug("[PTZ] joining previous timeout thread\n");
      pthread_join(timer_thread, NULL);
      pthread_mutex_lock(&g_ptz_adapter_lock);

      /* Reset shutdown flag for next use */
      g_ptz_adapter_timer_shutdown_requested = 0;
    }
  }

  /* Start movement in appropriate directions */
  if (pan_velocity != 0) {
    platform_ptz_direction_t dir = (pan_velocity > 0) ? PLATFORM_PTZ_DIRECTION_RIGHT : PLATFORM_PTZ_DIRECTION_LEFT;
    platform_log_debug("[PTZ] continuous move pan dir=%d\n", dir);
    platform_ptz_turn(dir, PTZ_MAX_PAN_DEGREES); /* Large number for continuous movement */
  }

  if (tilt_velocity != 0) {
    platform_ptz_direction_t dir = (tilt_velocity > 0) ? PLATFORM_PTZ_DIRECTION_DOWN : PLATFORM_PTZ_DIRECTION_UP;
    platform_log_debug("[PTZ] continuous move tilt dir=%d\n", dir);
    platform_ptz_turn(dir, PTZ_MAX_TILT_DEGREES); /* Large number for continuous movement */
  }

  /* Start timeout timer if timeout is specified and > 0 */
  if (timeout_seconds > 0) {
    g_ptz_adapter_continuous_move_timeout_s = timeout_seconds;
    g_ptz_adapter_continuous_move_active = 1;
    g_ptz_adapter_timer_shutdown_requested = 0; /* Reset shutdown flag for new timer */

    if (pthread_create(&g_ptz_adapter_continuous_move_timer_thread, NULL, continuous_move_timeout_thread, NULL) != 0) {
      platform_log_error("Failed to create continuous move timeout thread\n");
      g_ptz_adapter_continuous_move_active = 0;
      pthread_mutex_unlock(&g_ptz_adapter_lock);
      return PLATFORM_ERROR;
    }

    platform_log_info("PTZ continuous move started with %ds timeout\n", timeout_seconds);
  } else {
    platform_log_info("PTZ continuous move started (no timeout)\n");
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] continuous move setup done active=%d thread=%lu\n", g_ptz_adapter_continuous_move_active,
                     (unsigned long)g_ptz_adapter_continuous_move_timer_thread);
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_stop(void) {
  platform_log_debug("[PTZ] stop request received\n");
  pthread_mutex_lock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] stop state: initialized=%d active=%d thread=%lu\n", g_ptz_adapter_initialized, g_ptz_adapter_continuous_move_active,
                     (unsigned long)g_ptz_adapter_continuous_move_timer_thread);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  platform_log_info("PTZ stop all movement\n");

  /* Stop all PTZ movement */
  platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_LEFT);
  platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_RIGHT);
  platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_UP);
  platform_ptz_turn_stop(PLATFORM_PTZ_DIRECTION_DOWN);

  /* Clear continuous move state */
  g_ptz_adapter_continuous_move_active = 0;

  /* Signal and wait for timeout thread to finish if it exists */
  if (g_ptz_adapter_continuous_move_timer_thread != 0) {
    pthread_t timer_thread = g_ptz_adapter_continuous_move_timer_thread;
    g_ptz_adapter_continuous_move_timer_thread = 0;

    /* Signal shutdown to timer thread */
    g_ptz_adapter_timer_shutdown_requested = 1;
    pthread_cond_signal(&g_ptz_adapter_timer_cond);

    /* Unlock mutex before joining to avoid deadlock */
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    platform_log_debug("[PTZ] stop joining timeout thread\n");
    pthread_join(timer_thread, NULL);
    pthread_mutex_lock(&g_ptz_adapter_lock);

    /* Reset shutdown flag for next use */
    g_ptz_adapter_timer_shutdown_requested = 0;
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  platform_log_debug("[PTZ] stop completed\n");
  return PLATFORM_SUCCESS;
}

platform_result_t ptz_adapter_set_preset(const char* name, int preset_id) {
  pthread_mutex_lock(&g_ptz_adapter_lock);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  platform_log_info("PTZ set preset %s (id=%d) at pan=%d, tilt=%d\n", name ? name : "unnamed", preset_id, g_ptz_adapter_current_pan_pos,
                    g_ptz_adapter_current_tilt_pos);

  /* For now, just store current position - could be enhanced to save to file */
  pthread_mutex_unlock(&g_ptz_adapter_lock);
  return PLATFORM_SUCCESS; /* Basic implementation */
}

platform_result_t ptz_adapter_goto_preset(int preset_id) {
  pthread_mutex_lock(&g_ptz_adapter_lock);
  if (!g_ptz_adapter_initialized) {
    pthread_mutex_unlock(&g_ptz_adapter_lock);
    return PLATFORM_ERROR_INVALID;
  }

  platform_log_info("PTZ goto preset id=%d\n", preset_id);

  /* Basic implementation - could be enhanced to load from saved presets */
  platform_result_t ret = PLATFORM_ERROR;
  switch (preset_id) {
  case 1: /* Home position */
    ret = platform_ptz_move_to_position(0, 0);
    if (ret == PLATFORM_SUCCESS) {
      g_ptz_adapter_current_pan_pos = 0;
      g_ptz_adapter_current_tilt_pos = 0;
    }
    break;
  default:
    platform_log_info("Preset %d not implemented\n", preset_id);
    break;
  }

  pthread_mutex_unlock(&g_ptz_adapter_lock);
  return ret;
}
