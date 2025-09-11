/* hal_ptz_anyka.c - Anyka PTZ backend */

#include <time.h>
#include "hal_ptz.h"
#include "ak_common.h"
#include "ak_drv_ptz.h"

int hal_ptz_open(void){
    return ak_drv_ptz_open();
}

void hal_ptz_close(void){
    ak_drv_ptz_close();
}

int hal_ptz_set_degree(int pan_range_deg, int tilt_range_deg){
    return ak_drv_ptz_set_degree(pan_range_deg, tilt_range_deg);
}

int hal_ptz_check_self(void){
    return ak_drv_ptz_check_self(PTZ_FEEDBACK_PIN_NONE);
}

int hal_ptz_turn_to_pos(int pan_deg, int tilt_deg){
    return ak_drv_ptz_turn_to_pos(pan_deg, tilt_deg);
}

int hal_ptz_get_step_pos(enum hal_ptz_axis axis){
    return ak_drv_ptz_get_step_pos(axis == HAL_PTZ_AXIS_H ? PTZ_DEV_H : PTZ_DEV_V);
}

int hal_ptz_get_status(enum hal_ptz_axis axis, enum hal_ptz_status *out_status){
    enum ptz_status st;
    int dev = (axis == HAL_PTZ_AXIS_H) ? PTZ_DEV_H : PTZ_DEV_V;
    if (ak_drv_ptz_get_status(dev, &st) != 0) return -1;
    if (!out_status) return 0;
    *out_status = (st == PTZ_INIT_OK) ? HAL_PTZ_STATUS_OK : HAL_PTZ_STATUS_BUSY;
    return 0;
}

int hal_ptz_set_speed(enum hal_ptz_axis axis, int speed){
    int dev = (axis == HAL_PTZ_AXIS_H) ? PTZ_DEV_H : PTZ_DEV_V;
    return ak_drv_ptz_set_speed(dev, speed);
}

int hal_ptz_turn(enum hal_ptz_turn_direction dir, int steps){
    enum ptz_turn_direction d;
    switch (dir){
        case HAL_PTZ_TURN_LEFT: d = PTZ_TURN_LEFT; break;
        case HAL_PTZ_TURN_RIGHT: d = PTZ_TURN_RIGHT; break;
        case HAL_PTZ_TURN_UP: d = PTZ_TURN_UP; break;
        case HAL_PTZ_TURN_DOWN: d = PTZ_TURN_DOWN; break;
        default: return -1;
    }
    return ak_drv_ptz_turn(d, steps);
}

int hal_ptz_turn_stop(enum hal_ptz_turn_direction dir){
    enum ptz_turn_direction d;
    switch (dir){
        case HAL_PTZ_TURN_LEFT: d = PTZ_TURN_LEFT; break;
        case HAL_PTZ_TURN_RIGHT: d = PTZ_TURN_RIGHT; break;
        case HAL_PTZ_TURN_UP: d = PTZ_TURN_UP; break;
        case HAL_PTZ_TURN_DOWN: d = PTZ_TURN_DOWN; break;
        default: return -1;
    }
    return ak_drv_ptz_turn_stop(d);
}


