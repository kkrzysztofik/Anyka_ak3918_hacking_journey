/**
 * @file ptz_adapter_mock.c
 * @brief CMocka-based PTZ adapter mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include "ptz_adapter_mock.h"

#include <stdbool.h>
#include <stddef.h>

#include "mocks/platform_ptz_mock.h"
#include "services/ptz/onvif_ptz.h"

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void ptz_adapter_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
  platform_ptz_mock_set_async_mode(use_real);
}

/* ============================================================================
 * PTZ Adapter Initialization and Cleanup
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_init(void) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_init();
  }

  function_called();
  return (platform_result_t)mock();
}

void __wrap_ptz_adapter_cleanup(void) {
  if (g_use_real_functions) {
    __real_ptz_adapter_cleanup();
    return;
  }

  function_called();
}

/* ============================================================================
 * PTZ Adapter Status Operations
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_get_status(struct ptz_device_status* status) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_get_status(status);
  }

  check_expected_ptr(status);
  function_called();

  if (status) {
    status->h_pos_deg = (int)mock();
    status->v_pos_deg = (int)mock();
    status->h_speed = (int)mock();
    status->v_speed = (int)mock();
  }

  return (platform_result_t)mock();
}

/* ============================================================================
 * PTZ Adapter Movement Operations
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees,
                                                   int move_speed) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_absolute_move(pan_degrees, tilt_degrees, move_speed);
  }

  check_expected(pan_degrees);
  check_expected(tilt_degrees);
  check_expected(move_speed);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_relative_move(int pan_delta_degrees, int tilt_delta_degrees,
                                                   int move_speed) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_relative_move(pan_delta_degrees, tilt_delta_degrees, move_speed);
  }

  check_expected(pan_delta_degrees);
  check_expected(tilt_delta_degrees);
  check_expected(move_speed);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity,
                                                     int timeout_seconds) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_continuous_move(pan_velocity, tilt_velocity, timeout_seconds);
  }

  check_expected(pan_velocity);
  check_expected(tilt_velocity);
  check_expected(timeout_seconds);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_stop(void) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_stop();
  }

  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * PTZ Adapter Preset Operations
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_set_preset(const char* name, int preset_id) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_set_preset(name, preset_id);
  }

  check_expected_ptr(name);
  check_expected(preset_id);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_goto_preset(int preset_id) {
  if (g_use_real_functions) {
    return __real_ptz_adapter_goto_preset(preset_id);
  }

  check_expected(preset_id);
  function_called();
  return (platform_result_t)mock();
}
