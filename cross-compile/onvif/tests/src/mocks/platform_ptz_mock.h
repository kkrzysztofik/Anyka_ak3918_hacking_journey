/**
 * @file platform_ptz_mock.h
 * @brief Mock functions for platform PTZ operations
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PLATFORM_PTZ_MOCK_H
#define PLATFORM_PTZ_MOCK_H

#include "../../src/platform/platform_common.h"
#include "../../src/services/ptz/onvif_ptz.h"

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

/**
 * @brief Initialize platform PTZ mock
 */
void platform_ptz_mock_init(void);

/**
 * @brief Cleanup platform PTZ mock
 */
void platform_ptz_mock_cleanup(void);

/**
 * @brief Reset platform PTZ mock state
 */
void platform_ptz_mock_reset(void);

/* ============================================================================
 * PTZ Status Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ status
 * @param status PTZ device status to return
 */
void platform_mock_set_ptz_status(const struct ptz_device_status* status);

/**
 * @brief Get last PTZ status request
 * @return Pointer to last requested status (or NULL)
 */
const struct ptz_device_status* platform_mock_get_last_ptz_status_request(void);

/* ============================================================================
 * PTZ Movement Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ move result
 * @param result Platform result to return
 */
void platform_mock_set_ptz_move_result(platform_result_t result);

/**
 * @brief Set mock PTZ stop result
 * @param result Platform result to return
 */
void platform_mock_set_ptz_stop_result(platform_result_t result);

/**
 * @brief Get last PTZ absolute move request
 * @param pan_deg Output parameter for pan degrees
 * @param tilt_deg Output parameter for tilt degrees
 * @param speed Output parameter for speed
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_absolute_move(int* pan_deg, int* tilt_deg, int* speed);

/**
 * @brief Get last PTZ relative move request
 * @param pan_delta Output parameter for pan delta
 * @param tilt_delta Output parameter for tilt delta
 * @param speed Output parameter for speed
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_relative_move(int* pan_delta, int* tilt_delta, int* speed);

/**
 * @brief Get last PTZ continuous move request
 * @param pan_vel Output parameter for pan velocity
 * @param tilt_vel Output parameter for tilt velocity
 * @param timeout_s Output parameter for timeout seconds
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_continuous_move(int* pan_vel, int* tilt_vel, int* timeout_s);

/* ============================================================================
 * PTZ Initialization Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ initialization result
 * @param result Platform result to return
 */
void platform_mock_set_ptz_init_result(platform_result_t result);

/**
 * @brief Set mock PTZ cleanup result
 * @param result Platform result to return
 */
void platform_mock_set_ptz_cleanup_result(platform_result_t result);

/**
 * @brief Get PTZ initialization call count
 * @return Number of times PTZ was initialized
 */
int platform_mock_get_ptz_init_call_count(void);

/**
 * @brief Get PTZ cleanup call count
 * @return Number of times PTZ was cleaned up
 */
int platform_mock_get_ptz_cleanup_call_count(void);

/* ============================================================================
 * PTZ Preset Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ preset result
 * @param result Platform result to return
 */
void platform_mock_set_ptz_preset_result(platform_result_t result);

/**
 * @brief Get last PTZ set preset request
 * @param name Output parameter for preset name
 * @param preset_id Output parameter for preset ID
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_set_preset(char* name, size_t name_size, int* preset_id);

/**
 * @brief Get last PTZ goto preset request
 * @param preset_id Output parameter for preset ID
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_goto_preset(int* preset_id);

/* ============================================================================
 * PTZ Position Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ position
 * @param pan_deg Pan position in degrees
 * @param tilt_deg Tilt position in degrees
 */
void platform_mock_set_ptz_position(int pan_deg, int tilt_deg);

/**
 * @brief Get current mock PTZ position
 * @param pan_deg Output parameter for pan position
 * @param tilt_deg Output parameter for tilt position
 */
void platform_mock_get_ptz_position(int* pan_deg, int* tilt_deg);

/* ============================================================================
 * PTZ Movement Direction Mock Functions
 * ============================================================================ */

/**
 * @brief Get last PTZ turn request
 * @param direction Output parameter for direction
 * @param steps Output parameter for steps
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_turn(platform_ptz_direction_t* direction, int* steps);

/**
 * @brief Get last PTZ turn stop request
 * @param direction Output parameter for direction
 * @return 1 if request was made, 0 if not
 */
int platform_mock_get_last_ptz_turn_stop(platform_ptz_direction_t* direction);

/* ============================================================================
 * PTZ Error Simulation
 * ============================================================================ */

/**
 * @brief Enable PTZ error simulation
 * @param error_code Error code to return
 */
void platform_mock_enable_ptz_error(platform_result_t error_code);

/**
 * @brief Disable PTZ error simulation
 */
void platform_mock_disable_ptz_error(void);

/**
 * @brief Check if PTZ error simulation is enabled
 * @return 1 if enabled, 0 if disabled
 */
int platform_mock_is_ptz_error_enabled(void);

#endif // PLATFORM_PTZ_MOCK_H
