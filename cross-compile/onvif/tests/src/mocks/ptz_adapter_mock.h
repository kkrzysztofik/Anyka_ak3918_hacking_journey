/**
 * @file ptz_adapter_mock.h
 * @brief Mock interface for PTZ adapter layer
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PTZ_ADAPTER_MOCK_H
#define PTZ_ADAPTER_MOCK_H

#include "platform/platform_common.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Mock Lifecycle Management
 * ============================================================================ */

/**
 * @brief Initialize PTZ adapter mock
 */
void ptz_adapter_mock_init(void);

/**
 * @brief Cleanup PTZ adapter mock
 */
void ptz_adapter_mock_cleanup(void);

/**
 * @brief Reset PTZ adapter mock state
 * @note Resets call tracking and position, disables error simulation
 */
void ptz_adapter_mock_reset(void);

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

/**
 * @brief Set mock PTZ position
 * @param pan_deg Pan position in degrees
 * @param tilt_deg Tilt position in degrees
 */
void ptz_adapter_mock_set_position(int pan_deg, int tilt_deg);

/**
 * @brief Get mock PTZ position
 * @param pan_deg Output parameter for pan position
 * @param tilt_deg Output parameter for tilt position
 */
void ptz_adapter_mock_get_position(int* pan_deg, int* tilt_deg);

/**
 * @brief Enable error simulation
 * @param error_code Error code to return from adapter functions
 */
void ptz_adapter_mock_enable_error(platform_result_t error_code);

/**
 * @brief Disable error simulation
 */
void ptz_adapter_mock_disable_error(void);

/* ============================================================================
 * Mock Query Functions
 * ============================================================================ */

/**
 * @brief Get number of times ptz_adapter_init was called
 * @return Init call count
 */
int ptz_adapter_mock_get_init_call_count(void);

/**
 * @brief Check if absolute move was called and get parameters
 * @param pan Output parameter for pan degrees
 * @param tilt Output parameter for tilt degrees
 * @param speed Output parameter for speed
 * @return 1 if called, 0 if not
 */
int ptz_adapter_mock_was_absolute_move_called(int* pan, int* tilt, int* speed);

/**
 * @brief Check if relative move was called and get parameters
 * @param pan_delta Output parameter for pan delta
 * @param tilt_delta Output parameter for tilt delta
 * @param speed Output parameter for speed
 * @return 1 if called, 0 if not
 */
int ptz_adapter_mock_was_relative_move_called(int* pan_delta, int* tilt_delta, int* speed);

/**
 * @brief Check if continuous move was called and get parameters
 * @param pan_vel Output parameter for pan velocity
 * @param tilt_vel Output parameter for tilt velocity
 * @param timeout Output parameter for timeout
 * @return 1 if called, 0 if not
 */
int ptz_adapter_mock_was_continuous_move_called(int* pan_vel, int* tilt_vel, int* timeout);

/**
 * @brief Check if stop was called
 * @return 1 if called, 0 if not
 */
int ptz_adapter_mock_was_stop_called(void);

#ifdef __cplusplus
}
#endif

#endif // PTZ_ADAPTER_MOCK_H
