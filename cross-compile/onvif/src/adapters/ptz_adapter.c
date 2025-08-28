/* ptz_adapter.c - Proper PTZ adapter implementation using Anyka SDK */

#include "ptz_adapter.h"
#include "ak_drv_ptz.h"
#include <time.h>
#include "ak_common.h"
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>

static pthread_mutex_t ptz_lock = PTHREAD_MUTEX_INITIALIZER;
static int ptz_initialized = 0;
static int current_pan_pos = 0;
static int current_tilt_pos = 0;

static int simple_abs(int value) { return (value < 0) ? -value : value; }

int ptz_adapter_init(void) {
    int ret = 0;
    pthread_mutex_lock(&ptz_lock);
    if (!ptz_initialized) {
        ret = ak_drv_ptz_open();
        if (ret == 0) {
            /* Initialize PTZ with proper motor parameters */
            ak_drv_ptz_set_degree(350, 130);
            ak_drv_ptz_check_self(PTZ_FEEDBACK_PIN_NONE);
            
            /* Reset to center position */
            current_pan_pos = 0;
            current_tilt_pos = 0;
            ak_drv_ptz_turn_to_pos(current_pan_pos, current_tilt_pos);
            
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
        ak_drv_ptz_close();
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
    
    int h = ak_drv_ptz_get_step_pos(PTZ_DEV_H);
    int v = ak_drv_ptz_get_step_pos(PTZ_DEV_V);
    
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
    
    int ret = ak_drv_ptz_turn_to_pos(pan_deg, tilt_deg);
    if (ret == 0) {
        current_pan_pos = pan_deg;
        current_tilt_pos = tilt_deg;
        
        /* Wait for movement to complete */
        enum ptz_status h_status, v_status;
        do {
            usleep(5000); /* 5ms delay */
            ak_drv_ptz_get_status(PTZ_DEV_H, &h_status);
            ak_drv_ptz_get_status(PTZ_DEV_V, &v_status);
        } while ((h_status != PTZ_INIT_OK) || (v_status != PTZ_INIT_OK));
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
    enum ptz_status h_status, v_status;
    
    /* Horizontal movement - based on akipc implementation with step size 16 */
    if (pan_delta_deg != 0) {
        enum ptz_turn_direction dir = (pan_delta_deg > 0) ? PTZ_TURN_LEFT : PTZ_TURN_RIGHT;
        int steps = simple_abs(pan_delta_deg);
        if (steps > 16) steps = 16; /* Limit step size like in akipc */
        
        ret = ak_drv_ptz_turn(dir, steps);
        if (ret == 0) {
            current_pan_pos += (dir == PTZ_TURN_LEFT) ? steps : -steps;
        }
    }
    
    /* Vertical movement - based on akipc implementation with step size 8 */
    if (tilt_delta_deg != 0) {
        enum ptz_turn_direction dir = (tilt_delta_deg > 0) ? PTZ_TURN_DOWN : PTZ_TURN_UP;
        int steps = simple_abs(tilt_delta_deg);
        if (steps > 8) steps = 8; /* Limit step size like in akipc */
        
        ret |= ak_drv_ptz_turn(dir, steps);
        if ((ret & 2) == 0) { /* Check if vertical movement succeeded */
            current_tilt_pos += (dir == PTZ_TURN_DOWN) ? steps : -steps;
        }
    }
    
    /* Wait for movement to complete */
    if (ret == 0) {
        do {
            usleep(5000); /* 5ms delay */
            ak_drv_ptz_get_status(PTZ_DEV_H, &h_status);
            ak_drv_ptz_get_status(PTZ_DEV_V, &v_status);
        } while ((h_status != PTZ_INIT_OK) || (v_status != PTZ_INIT_OK));
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
    
    if (pan_vel > 0) ak_drv_ptz_set_speed(PTZ_DEV_H, pan_vel);
    if (tilt_vel > 0) ak_drv_ptz_set_speed(PTZ_DEV_V, tilt_vel);
    
    if (pan_vel != 0) {
        enum ptz_turn_direction dir = (pan_vel > 0) ? PTZ_TURN_RIGHT : PTZ_TURN_LEFT;
        ak_drv_ptz_turn(dir, 360);
    }
    
    if (tilt_vel != 0) {
        enum ptz_turn_direction dir = (tilt_vel > 0) ? PTZ_TURN_DOWN : PTZ_TURN_UP;
        ak_drv_ptz_turn(dir, 180);
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
    ak_drv_ptz_turn_stop(PTZ_TURN_LEFT);
    ak_drv_ptz_turn_stop(PTZ_TURN_RIGHT);
    ak_drv_ptz_turn_stop(PTZ_TURN_UP);
    ak_drv_ptz_turn_stop(PTZ_TURN_DOWN);
    
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
            ret = ak_drv_ptz_turn_to_pos(0, 0);
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
