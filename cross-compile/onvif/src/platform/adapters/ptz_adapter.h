/**
 * @file ptz_adapter.h
 * @brief PTZ control abstraction adapter for Anyka platform
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides the adapter layer interface between ONVIF PTZ service and
 * platform-specific PTZ hardware operations. The adapter handles:
 * - Coordinate transformations (ONVIF normalized → hardware degrees)
 * - Position state tracking
 * - Hardware abstraction for testability
 * - Thread-safe operations
 */

#ifndef PTZ_ADAPTER_H
#define PTZ_ADAPTER_H

#include "platform/platform_common.h"

/* Forward declaration */
struct ptz_device_status;

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * PTZ Adapter Initialization and Cleanup
 * ============================================================================ */

/**
 * @brief Initialize PTZ adapter layer
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note This function initializes the platform PTZ hardware and resets to home position
 * @note Thread-safe, can be called multiple times (subsequent calls are no-ops)
 */
platform_result_t ptz_adapter_init(void);

/**
 * @brief Cleanup PTZ adapter layer
 * @note This function stops all movements, joins timeout threads, and cleans up resources
 * @note Thread-safe, safe to call even if not initialized
 */
void ptz_adapter_cleanup(void);

/* ============================================================================
 * PTZ Adapter Status Operations
 * ============================================================================ */

/**
 * @brief Get current PTZ device status
 * @param status Output parameter for device status
 * @return PLATFORM_SUCCESS on success, PLATFORM_ERROR_INVALID if not initialized
 * @note Returns cached position state (not queried from hardware)
 * @note Thread-safe
 */
platform_result_t ptz_adapter_get_status(struct ptz_device_status* status);

/* ============================================================================
 * PTZ Adapter Movement Operations
 * ============================================================================ */

/**
 * @brief Move PTZ to absolute position
 * @param pan_degrees Pan position in degrees [-350, 350]
 * @param tilt_degrees Tilt position in degrees [-130, 130]
 * @param move_speed Movement speed [15, 100] (currently unused by platform)
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Values are automatically clamped to safe ranges
 * @note Updates internal position state on success
 * @note Thread-safe
 */
platform_result_t ptz_adapter_absolute_move(int pan_degrees, int tilt_degrees, int move_speed);

/**
 * @brief Move PTZ by relative delta
 * @param pan_delta_degrees Pan movement delta in degrees
 * @param tilt_delta_degrees Tilt movement delta in degrees
 * @param move_speed Movement speed [15, 100] (currently unused by platform)
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Delta values are clamped to max step sizes (pan: 16, tilt: 8)
 * @note Updates internal position state on success
 * @note Thread-safe
 */
platform_result_t ptz_adapter_relative_move(int pan_delta_degrees, int tilt_delta_degrees,
                                            int move_speed);

/**
 * @brief Start continuous PTZ movement
 * @param pan_velocity Pan velocity (>0 right, <0 left, 0 no movement)
 * @param tilt_velocity Tilt velocity (>0 down, <0 up, 0 no movement)
 * @param timeout_seconds Timeout in seconds (0 = no timeout)
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Stops any existing continuous movement before starting new one
 * @note Creates timeout thread if timeout > 0
 * @note Thread-safe
 */
platform_result_t ptz_adapter_continuous_move(int pan_velocity, int tilt_velocity,
                                              int timeout_seconds);

/**
 * @brief Stop all PTZ movement
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Stops movement in all directions (pan and tilt)
 * @note Joins timeout thread if active
 * @note Thread-safe
 */
platform_result_t ptz_adapter_stop(void);

/* ============================================================================
 * PTZ Adapter Preset Operations
 * ============================================================================ */

/**
 * @brief Set PTZ preset at current position
 * @param name Preset name (can be NULL)
 * @param preset_id Preset identifier
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Current implementation logs preset but doesn't persist to storage
 * @note Thread-safe
 */
platform_result_t ptz_adapter_set_preset(const char* name, int preset_id);

/**
 * @brief Move PTZ to preset position
 * @param preset_id Preset identifier
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Currently only implements preset 1 (home position)
 * @note Thread-safe
 */
platform_result_t ptz_adapter_goto_preset(int preset_id);

#ifdef __cplusplus
}
#endif

#endif /* PTZ_ADAPTER_H */
