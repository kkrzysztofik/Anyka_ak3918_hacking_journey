#ifndef PTZ_ADAPTER_H
#define PTZ_ADAPTER_H
#include <stdint.h>

struct ptz_device_status { 
    int h_pos_deg; 
    int v_pos_deg; 
    int h_speed; 
    int v_speed; 
};

int ptz_adapter_init(void);
int ptz_adapter_shutdown(void);
int ptz_adapter_get_status(struct ptz_device_status *status);
int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed);
int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed);
int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s);
int ptz_adapter_stop(void);
int ptz_adapter_set_preset(const char *name, int id);
int ptz_adapter_goto_preset(int id);

#endif
