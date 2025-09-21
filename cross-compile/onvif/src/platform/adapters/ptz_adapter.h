/**
 * @file ptz_adapter.h
 * @brief PTZ control abstraction adapter for Anyka platform
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides the interface for pan-tilt-zoom control operations
 * on the Anyka AK3918 platform.
 */

#ifndef PTZ_ADAPTER_H
#define PTZ_ADAPTER_H

#include "platform/platform_common.h"

/* Forward declaration */
struct ptz_device_status;

#ifdef __cplusplus
extern "C" {
#endif

/* PTZ initialization and cleanup */
platform_result_t ptz_adapter_init(void);
void ptz_adapter_cleanup(void);

/* PTZ configuration */
platform_result_t ptz_adapter_set_degree(int pan_range_deg, int tilt_range_deg);
platform_result_t ptz_adapter_check_self(void);

/* PTZ movement operations */
platform_result_t ptz_adapter_move_to_position(int pan_deg, int tilt_deg);
int ptz_adapter_get_step_position(platform_ptz_axis_t axis);
platform_result_t ptz_adapter_get_status(struct ptz_device_status *status);
platform_result_t ptz_adapter_set_speed(platform_ptz_axis_t axis, int speed);
platform_result_t ptz_adapter_turn(platform_ptz_direction_t direction,
                                   int steps);
platform_result_t ptz_adapter_turn_stop(platform_ptz_direction_t direction);

#ifdef __cplusplus
}
#endif

#endif /* PTZ_ADAPTER_H */
