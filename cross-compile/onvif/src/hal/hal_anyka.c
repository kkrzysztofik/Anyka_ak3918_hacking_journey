/* hal_anyka.c - Anyka backend for HAL interface */

#include "hal.h"
#include "ak_vi.h"
#include "ak_vpss.h"
#include "ak_drv_irled.h"

int hal_vi_open(hal_vi_handle_t *out_handle){
    if (!out_handle) return -1;
    void *h = ak_vi_open(VIDEO_DEV0);
    if (!h) return -1;
    *out_handle = h;
    return 0;
}

void hal_vi_close(hal_vi_handle_t handle){
    if (handle) ak_vi_close(handle);
}

int hal_vi_get_sensor_resolution(hal_vi_handle_t handle, struct hal_video_resolution *out_res){
    if (!handle || !out_res) return -1;
    struct video_resolution r;
    if (ak_vi_get_sensor_resolution(handle, &r) != 0) return -1;
    out_res->width = r.width;
    out_res->height = r.height;
    return 0;
}

int hal_vi_switch_day_night(hal_vi_handle_t handle, enum hal_daynight_mode mode){
    if (!handle) return -1;
    enum video_daynight_mode vi_mode = (mode == HAL_DAYNIGHT_NIGHT) ? VI_MODE_NIGHT : VI_MODE_DAY;
    return ak_vi_switch_mode(handle, vi_mode);
}

int hal_vi_set_flip_mirror(hal_vi_handle_t handle, int flip, int mirror){
    if (!handle) return -1;
    return ak_vi_set_flip_mirror(handle, flip, mirror);
}

static int map_effect(enum hal_vpss_effect e){
    switch (e){
        case HAL_VPSS_EFFECT_BRIGHTNESS: return VPSS_EFFECT_BRIGHTNESS;
        case HAL_VPSS_EFFECT_CONTRAST:   return VPSS_EFFECT_CONTRAST;
        case HAL_VPSS_EFFECT_SATURATION: return VPSS_EFFECT_SATURATION;
        case HAL_VPSS_EFFECT_SHARPNESS:  return VPSS_EFFECT_SHARP;
        case HAL_VPSS_EFFECT_HUE:        return VPSS_EFFECT_HUE;
        default: return -1;
    }
}

int hal_vpss_effect_set(hal_vi_handle_t handle, enum hal_vpss_effect effect, int value){
    int eff = map_effect(effect);
    if (eff < 0) return -1;
    return ak_vpss_effect_set(handle, eff, value);
}

int hal_vpss_effect_get(hal_vi_handle_t handle, enum hal_vpss_effect effect, int *out_value){
    int eff = map_effect(effect);
    if (eff < 0 || !out_value) return -1;
    return ak_vpss_effect_get(handle, eff, out_value);
}

int hal_irled_init(int level){
    struct ak_drv_irled_hw_param p; p.irled_working_level = level;
    return ak_drv_irled_init(&p);
}

int hal_irled_set_mode(int mode){
    /* mode: 0=off, 1=on, 2=auto -> for auto we enable working stat */
    int stat = (mode == 0) ? 0 : 1;
    return ak_drv_irled_set_working_stat(stat);
}

int hal_irled_get_status(void){
    return ak_drv_irled_get_working_stat();
}


