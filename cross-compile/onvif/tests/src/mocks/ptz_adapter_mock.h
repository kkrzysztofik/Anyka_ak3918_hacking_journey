/**
 * @file ptz_adapter_mock.h
 * @brief CMocka-based PTZ adapter mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PTZ_ADAPTER_MOCK_H
#define PTZ_ADAPTER_MOCK_H

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#include "platform/platform_common.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Forward declaration */
struct ptz_device_status;

/* ============================================================================
 * CMocka Wrapped PTZ Adapter Functions
 * ============================================================================
 * All PTZ adapter functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped PTZ adapter initialization
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_init(void);

/**
 * @brief CMocka wrapped PTZ adapter cleanup
 */
void __wrap_ptz_adapter_cleanup(void);

/**
 * @brief CMocka wrapped PTZ get status
 * @param status Output parameter for device status
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_get_status(struct ptz_device_status* status);

/**
 * @brief CMocka wrapped PTZ absolute move
 * @param pan_degrees Pan position in degrees
 * @param tilt_degrees Tilt position in degrees
 * @param move_speed Movement speed
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees,
                                                    int move_speed);

/**
 * @brief CMocka wrapped PTZ relative move
 * @param pan_delta_degrees Pan movement delta
 * @param tilt_delta_degrees Tilt movement delta
 * @param move_speed Movement speed
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_relative_move(int pan_delta_degrees, int tilt_delta_degrees,
                                                    int move_speed);

/**
 * @brief CMocka wrapped PTZ continuous move
 * @param pan_velocity Pan velocity
 * @param tilt_velocity Tilt velocity
 * @param timeout_seconds Timeout in seconds
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity,
                                                      int timeout_seconds);

/**
 * @brief CMocka wrapped PTZ stop
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_stop(void);

/**
 * @brief CMocka wrapped PTZ set preset
 * @param name Preset name
 * @param preset_id Preset identifier
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_set_preset(const char* name, int preset_id);

/**
 * @brief CMocka wrapped PTZ goto preset
 * @param preset_id Preset identifier
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_ptz_adapter_goto_preset(int preset_id);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful PTZ adapter initialization
 */
#define EXPECT_PTZ_ADAPTER_INIT_SUCCESS() will_return(__wrap_ptz_adapter_init, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for PTZ adapter initialization failure
 * @param error_code Error code to return
 */
#define EXPECT_PTZ_ADAPTER_INIT_ERROR(error_code)                                                  \
  will_return(__wrap_ptz_adapter_init, error_code)

/**
 * @brief Set up expectations for successful PTZ absolute move
 * @param pan Pan position in degrees
 * @param tilt Tilt position in degrees
 * @param speed Movement speed
 */
#define EXPECT_PTZ_ADAPTER_ABSOLUTE_MOVE(pan, tilt, speed)                                         \
  expect_value(__wrap_ptz_adapter_absolute_move, pan_degrees, pan);                               \
  expect_value(__wrap_ptz_adapter_absolute_move, tilt_degrees, tilt);                             \
  expect_value(__wrap_ptz_adapter_absolute_move, move_speed, speed);                              \
  will_return(__wrap_ptz_adapter_absolute_move, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for successful PTZ relative move
 * @param pan_delta Pan delta in degrees
 * @param tilt_delta Tilt delta in degrees
 * @param speed Movement speed
 */
#define EXPECT_PTZ_ADAPTER_RELATIVE_MOVE(pan_delta, tilt_delta, speed)                             \
  expect_value(__wrap_ptz_adapter_relative_move, pan_delta_degrees, pan_delta);                   \
  expect_value(__wrap_ptz_adapter_relative_move, tilt_delta_degrees, tilt_delta);                 \
  expect_value(__wrap_ptz_adapter_relative_move, move_speed, speed);                              \
  will_return(__wrap_ptz_adapter_relative_move, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for successful PTZ continuous move
 * @param pan_vel Pan velocity
 * @param tilt_vel Tilt velocity
 * @param timeout Timeout in seconds
 */
#define EXPECT_PTZ_ADAPTER_CONTINUOUS_MOVE(pan_vel, tilt_vel, timeout)                             \
  expect_value(__wrap_ptz_adapter_continuous_move, pan_velocity, pan_vel);                        \
  expect_value(__wrap_ptz_adapter_continuous_move, tilt_velocity, tilt_vel);                      \
  expect_value(__wrap_ptz_adapter_continuous_move, timeout_seconds, timeout);                     \
  will_return(__wrap_ptz_adapter_continuous_move, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for successful PTZ stop
 */
#define EXPECT_PTZ_ADAPTER_STOP() will_return(__wrap_ptz_adapter_stop, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for successful PTZ set preset
 * @param preset_name Preset name
 * @param id Preset ID
 */
#define EXPECT_PTZ_ADAPTER_SET_PRESET(preset_name, id)                                             \
  expect_string(__wrap_ptz_adapter_set_preset, name, preset_name);                                \
  expect_value(__wrap_ptz_adapter_set_preset, preset_id, id);                                     \
  will_return(__wrap_ptz_adapter_set_preset, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for successful PTZ goto preset
 * @param id Preset ID
 */
#define EXPECT_PTZ_ADAPTER_GOTO_PRESET(id)                                                         \
  expect_value(__wrap_ptz_adapter_goto_preset, preset_id, id);                                    \
  will_return(__wrap_ptz_adapter_goto_preset, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for PTZ get status with return values
 * @param h_pos Horizontal position
 * @param v_pos Vertical position
 * @param h_spd Horizontal speed
 * @param v_spd Vertical speed
 */
#define EXPECT_PTZ_ADAPTER_GET_STATUS(h_pos, v_pos, h_spd, v_spd)                                  \
  will_return(__wrap_ptz_adapter_get_status, h_pos);                                              \
  will_return(__wrap_ptz_adapter_get_status, v_pos);                                              \
  will_return(__wrap_ptz_adapter_get_status, h_spd);                                              \
  will_return(__wrap_ptz_adapter_get_status, v_spd);                                              \
  will_return(__wrap_ptz_adapter_get_status, PLATFORM_SUCCESS)

#ifdef __cplusplus
}
#endif

#endif // PTZ_ADAPTER_MOCK_H
