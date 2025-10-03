/**
 * @file ptz_adapter_mock.c
 * @brief CMocka-based PTZ adapter mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include "ptz_adapter_mock.h"

#include <stddef.h>

#include "services/ptz/onvif_ptz.h"

/* ============================================================================
 * PTZ Adapter Initialization and Cleanup
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_init(void) {
  function_called();
  return (platform_result_t)mock();
}

void __wrap_ptz_adapter_cleanup(void) {
  function_called();
}

/* ============================================================================
 * PTZ Adapter Status Operations
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_get_status(struct ptz_device_status* status) {
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
  check_expected(pan_degrees);
  check_expected(tilt_degrees);
  check_expected(move_speed);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_relative_move(int pan_delta_degrees, int tilt_delta_degrees,
                                                    int move_speed) {
  check_expected(pan_delta_degrees);
  check_expected(tilt_delta_degrees);
  check_expected(move_speed);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity,
                                                      int timeout_seconds) {
  check_expected(pan_velocity);
  check_expected(tilt_velocity);
  check_expected(timeout_seconds);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_stop(void) {
  function_called();
  return (platform_result_t)mock();
}

/* ============================================================================
 * PTZ Adapter Preset Operations
 * ============================================================================ */

platform_result_t __wrap_ptz_adapter_set_preset(const char* name, int preset_id) {
  check_expected_ptr(name);
  check_expected(preset_id);
  function_called();
  return (platform_result_t)mock();
}

platform_result_t __wrap_ptz_adapter_goto_preset(int preset_id) {
  check_expected(preset_id);
  function_called();
  return (platform_result_t)mock();
}
