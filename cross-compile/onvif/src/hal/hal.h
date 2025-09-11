/**
 * @file hal.h
 * @brief Hardware Abstraction Layer for video input, VPSS effects, and IR LED.
 *
 * This interface decouples the ONVIF application from the Anyka SDK so that
 * alternative chipsets can be supported by providing another HAL backend.
 */

#ifndef HAL_H
#define HAL_H

#include <stddef.h>

/* Opaque video input handle (backend-defined) */
typedef void* hal_vi_handle_t;

struct hal_video_resolution {
    int width;
    int height;
};

enum hal_daynight_mode {
    HAL_DAYNIGHT_DAY = 0,
    HAL_DAYNIGHT_NIGHT = 1
};

enum hal_vpss_effect {
    HAL_VPSS_EFFECT_BRIGHTNESS = 0,
    HAL_VPSS_EFFECT_CONTRAST   = 1,
    HAL_VPSS_EFFECT_SATURATION = 2,
    HAL_VPSS_EFFECT_SHARPNESS  = 3,
    HAL_VPSS_EFFECT_HUE        = 4
};

/* Video input */
int hal_vi_open(hal_vi_handle_t *out_handle);
void hal_vi_close(hal_vi_handle_t handle);
int hal_vi_get_sensor_resolution(hal_vi_handle_t handle, struct hal_video_resolution *out_res);
int hal_vi_switch_day_night(hal_vi_handle_t handle, enum hal_daynight_mode mode);
int hal_vi_set_flip_mirror(hal_vi_handle_t handle, int flip, int mirror);

/* VPSS effects */
int hal_vpss_effect_set(hal_vi_handle_t handle, enum hal_vpss_effect effect, int value);
int hal_vpss_effect_get(hal_vi_handle_t handle, enum hal_vpss_effect effect, int *out_value);

/* IR LED control */
int hal_irled_init(int level);
int hal_irled_set_mode(int mode /* 0=off,1=on,2=auto (backend-defined) */);
int hal_irled_get_status(void);

#endif /* HAL_H */


