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

#include <stdbool.h>

#include "platform/platform_common.h"
#include "platform_mock.h"

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

/**
 * @brief Get bitmask of PTZ turn stop directions seen since last reset
 * @return Bitmask using (1u << PLATFORM_PTZ_DIRECTION_*) encoding
 */
unsigned int platform_mock_get_ptz_turn_stop_mask(void);

/**
 * @brief Enable or disable async-safe mode for PTZ platform mocks
 * @param enable true to bypass CMocka expectations on non-owner threads
 */
void platform_ptz_mock_set_async_mode(bool enable);

/**
 * @brief Determine whether the current thread should bypass CMocka expectations
 * @return true if expectations must be skipped for the calling thread
 */
bool platform_ptz_mock_should_bypass_expectations(void);

/**
 * @brief Record that platform cleanup was called (resets init flag)
 */
void platform_ptz_mock_record_cleanup(void);

/**
 * @brief Check whether PTZ mock currently considers the adapter initialized
 * @return Non-zero if initialization has been recorded without cleanup
 */
int platform_mock_is_ptz_initialized(void);

#endif /* PLATFORM_PTZ_MOCK_H */
