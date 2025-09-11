/**
 * @file hal_ptz.h
 * @brief HAL for PTZ motor control to abstract Anyka driver.
 */

#ifndef HAL_PTZ_H
#define HAL_PTZ_H

enum hal_ptz_axis {
    HAL_PTZ_AXIS_H = 0,
    HAL_PTZ_AXIS_V = 1
};

enum hal_ptz_status {
    HAL_PTZ_STATUS_OK = 0,
    HAL_PTZ_STATUS_BUSY = 1,
    HAL_PTZ_STATUS_ERR = 2
};

enum hal_ptz_turn_direction {
    HAL_PTZ_TURN_LEFT = 0,
    HAL_PTZ_TURN_RIGHT = 1,
    HAL_PTZ_TURN_UP = 2,
    HAL_PTZ_TURN_DOWN = 3
};

int hal_ptz_open(void);
void hal_ptz_close(void);
int hal_ptz_set_degree(int pan_range_deg, int tilt_range_deg);
int hal_ptz_check_self(void);
int hal_ptz_turn_to_pos(int pan_deg, int tilt_deg);
int hal_ptz_get_step_pos(enum hal_ptz_axis axis);
int hal_ptz_get_status(enum hal_ptz_axis axis, enum hal_ptz_status *out_status);
int hal_ptz_set_speed(enum hal_ptz_axis axis, int speed);
int hal_ptz_turn(enum hal_ptz_turn_direction dir, int steps);
int hal_ptz_turn_stop(enum hal_ptz_turn_direction dir);

#endif /* HAL_PTZ_H */


