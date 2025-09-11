/* ptz_adapter.c - Proper PTZ adapter implementation using Anyka SDK */

#include "ptz_adapter.h"
#include "hal/hal_ptz.h"
#include <time.h>
#include "utils.h"
#include "ak_common.h"
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>

static pthread_mutex_t ptz_lock = PTHREAD_MUTEX_INITIALIZER;
static int ptz_initialized = 0;
static int current_pan_pos = 0;
static int current_tilt_pos = 0;

static int simple_abs(int value) { return (value < 0) ? -value : value; }

int ptz_adapter_init(void) {
    int ret = 0;
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        ret = hal_ptz_open();
        if (ret == 0) {
            /* Initialize PTZ with proper motor parameters */
            hal_ptz_set_degree(350, 130);
            hal_ptz_check_self();
            
            /* Reset to center position */
            current_pan_pos = 0;
            current_tilt_pos = 0;
            hal_ptz_turn_to_pos(current_pan_pos, current_tilt_pos);
            
            ptz_initialized = 1;
            printf("PTZ adapter initialized successfully\n");
        } else {
            fprintf(stderr, "ak_drv_ptz_open failed: %d\n", ret);
        }
    }
    pthread_mutex_unlock(&ptz_lock);
    return ptz_initialized ? 0 : -1;
}

int ptz_adapter_shutdown(void) {
    pthread_mutex_lock(&ptz_lock);
    if (ptz_initialized) {
        hal_ptz_close();
        ptz_initialized = 0;
    }
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_get_status(struct ptz_device_status *status) {
    if (!status) return -1;
    
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    int h = hal_ptz_get_step_pos(HAL_PTZ_AXIS_H);
    int v = hal_ptz_get_step_pos(HAL_PTZ_AXIS_V);
    
    status->h_pos_deg = h;
    status->v_pos_deg = v;
    status->h_speed = 0;
    status->v_speed = 0;
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    /* Clamp values to safe ranges - based on akipc implementation */
    if (pan_deg > 350) pan_deg = 350;
    if (pan_deg < -350) pan_deg = -350;
    if (tilt_deg > 130) tilt_deg = 130;
    if (tilt_deg < -130) tilt_deg = -130;
    
    printf("PTZ absolute move to pan=%d, tilt=%d\n", pan_deg, tilt_deg);
    
    int ret = hal_ptz_turn_to_pos(pan_deg, tilt_deg);
    if (ret == 0) {
        current_pan_pos = pan_deg;
        current_tilt_pos = tilt_deg;
        
        /* Wait for movement to complete */
        enum hal_ptz_status h_status, v_status;
        do {
            sleep_us(5000); /* 5ms delay */
            hal_ptz_get_status(HAL_PTZ_AXIS_H, &h_status);
            hal_ptz_get_status(HAL_PTZ_AXIS_V, &v_status);
        } while ((h_status != HAL_PTZ_STATUS_OK) || (v_status != HAL_PTZ_STATUS_OK));
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ relative move pan_delta=%d, tilt_delta=%d\n", pan_delta_deg, tilt_delta_deg);
    
    int ret = 0;
    enum hal_ptz_status h_status, v_status;
    
    /* Horizontal movement - based on akipc implementation with step size 16 */
    if (pan_delta_deg != 0) {
        enum hal_ptz_turn_direction dir = (pan_delta_deg > 0) ? HAL_PTZ_TURN_LEFT : HAL_PTZ_TURN_RIGHT;
        int steps = simple_abs(pan_delta_deg);
        if (steps > 16) steps = 16; /* Limit step size like in akipc */
        
        ret = hal_ptz_turn(dir, steps);
        if (ret == 0) {
            current_pan_pos += (dir == HAL_PTZ_TURN_LEFT) ? steps : -steps;
        }
    }
    
    /* Vertical movement - based on akipc implementation with step size 8 */
    if (tilt_delta_deg != 0) {
        enum hal_ptz_turn_direction dir = (tilt_delta_deg > 0) ? HAL_PTZ_TURN_DOWN : HAL_PTZ_TURN_UP;
        int steps = simple_abs(tilt_delta_deg);
        if (steps > 8) steps = 8; /* Limit step size like in akipc */
        
        ret |= hal_ptz_turn(dir, steps);
        if (ret == 0) current_tilt_pos += (dir == HAL_PTZ_TURN_DOWN) ? steps : -steps;
    }
    
    /* Wait for movement to complete */
    if (ret == 0) {
        do {
            sleep_us(5000); /* 5ms delay */
            hal_ptz_get_status(HAL_PTZ_AXIS_H, &h_status);
            hal_ptz_get_status(HAL_PTZ_AXIS_V, &v_status);
        } while ((h_status != HAL_PTZ_STATUS_OK) || (v_status != HAL_PTZ_STATUS_OK));
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}

int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    if (pan_vel > 0) hal_ptz_set_speed(HAL_PTZ_AXIS_H, pan_vel);
    if (tilt_vel > 0) hal_ptz_set_speed(HAL_PTZ_AXIS_V, tilt_vel);
    
    if (pan_vel != 0) {
        enum hal_ptz_turn_direction dir = (pan_vel > 0) ? HAL_PTZ_TURN_RIGHT : HAL_PTZ_TURN_LEFT;
        hal_ptz_turn(dir, 360);
    }
    
    if (tilt_vel != 0) {
        enum hal_ptz_turn_direction dir = (tilt_vel > 0) ? HAL_PTZ_TURN_DOWN : HAL_PTZ_TURN_UP;
        hal_ptz_turn(dir, 180);
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_stop(void) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ stop all movement\n");
    
    /* Stop all directions of movement */
    hal_ptz_turn_stop(HAL_PTZ_TURN_LEFT);
    hal_ptz_turn_stop(HAL_PTZ_TURN_RIGHT);
    hal_ptz_turn_stop(HAL_PTZ_TURN_UP);
    hal_ptz_turn_stop(HAL_PTZ_TURN_DOWN);
    
    pthread_mutex_unlock(&ptz_lock);
    return 0;
}

int ptz_adapter_set_preset(const char *name, int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ set preset %s (id=%d) at pan=%d, tilt=%d\n", 
           name ? name : "unnamed", id, current_pan_pos, current_tilt_pos);
    
    /* For now, just store current position - could be enhanced to save to file */
    pthread_mutex_unlock(&ptz_lock);
    return 0;   /* Basic implementation */
}

int ptz_adapter_goto_preset(int id) {
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        pthread_mutex_unlock(&ptz_lock);
        return -1;
    }
    
    printf("PTZ goto preset id=%d\n", id);
    
    /* Basic implementation - could be enhanced to load from saved presets */
    int ret = -1;
    switch (id) {
        case 1: /* Home position */
            ret = hal_ptz_turn_to_pos(0, 0);
            if (ret == 0) {
                current_pan_pos = 0;
                current_tilt_pos = 0;
            }
            break;
        default:
            printf("Preset %d not implemented\n", id);
            break;
    }
    
    pthread_mutex_unlock(&ptz_lock);
    return ret;
}
