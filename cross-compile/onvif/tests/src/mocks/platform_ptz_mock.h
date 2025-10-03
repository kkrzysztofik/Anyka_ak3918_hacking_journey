/**
 * @file platform_ptz_mock.h
 * @brief CMocka PTZ platform mock helper functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides helper functions that wrap CMocka's native mocking capabilities
 * for easier PTZ test setup. All functions configure CMocka's will_return() internally.
 */

#ifndef PLATFORM_PTZ_MOCK_H
#define PLATFORM_PTZ_MOCK_H

#include "platform/platform_common.h"
#include "platform_mock_cmocka.h"

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

/**
 * @brief Initialize PTZ mock state
 * @note This resets call counters and tracked data
 */
void platform_ptz_mock_init(void);

/**
 * @brief Cleanup PTZ mock state
 * @note This is a no-op with CMocka - state is managed automatically
 */
void platform_ptz_mock_cleanup(void);

/**
 * @brief Reset PTZ mock state (call counters and tracked data)
 */
void platform_ptz_mock_reset(void);

/* ============================================================================
 * Mock Result Configuration (wraps will_return)
 * ============================================================================ */

/**
 * @brief Set result for next PTZ init call
 * @param result Result code to return from __wrap_platform_ptz_init
 */
void platform_mock_set_ptz_init_result(platform_result_t result);

/**
 * @brief Set result for next PTZ move call
 * @param result Result code to return from __wrap_platform_ptz_set_degree
 */
void platform_mock_set_ptz_move_result(platform_result_t result);

/**
 * @brief Set result for next PTZ stop call
 * @param result Result code to return from __wrap_platform_ptz_turn_stop
 */
void platform_mock_set_ptz_stop_result(platform_result_t result);

/**
 * @brief Set result for next PTZ preset call
 * @param result Result code to return from __wrap_platform_ptz_move_to_position
 */
void platform_mock_set_ptz_preset_result(platform_result_t result);

/* ============================================================================
 * Error Simulation
 * ============================================================================ */

/**
 * @brief Enable PTZ error simulation (next calls will return error)
 * @param error Error code to return
 */
void platform_mock_enable_ptz_error(platform_result_t error);

/**
 * @brief Disable PTZ error simulation (calls return to normal)
 */
void platform_mock_disable_ptz_error(void);

/* ============================================================================
 * Mock Call Tracking
 * ============================================================================ */

/**
 * @brief Get number of times PTZ init was called
 * @return Call count
 */
int platform_mock_get_ptz_init_call_count(void);

/**
 * @brief Get last PTZ absolute move parameters
 * @param pan Output pan value
 * @param tilt Output tilt value
 * @param speed Output speed value
 * @return 1 if data available, 0 if not
 */
int platform_mock_get_last_ptz_absolute_move(int* pan, int* tilt, int* speed);

/**
 * @brief Get last PTZ turn parameters
 * @param dir Output direction
 * @param steps Output steps
 * @return 1 if data available, 0 if not
 */
int platform_mock_get_last_ptz_turn(platform_ptz_direction_t* dir, int* steps);

/**
 * @brief Get last PTZ turn stop parameters
 * @param dir Output direction
 * @return 1 if data available, 0 if not
 */
int platform_mock_get_last_ptz_turn_stop(platform_ptz_direction_t* dir);

#endif /* PLATFORM_PTZ_MOCK_H */
