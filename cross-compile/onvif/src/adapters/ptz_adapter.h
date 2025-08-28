/**
 * @file ptz_adapter.h
 * @brief Hardware/PTZ abstraction translating ONVIF PTZ calls to device controls.
 */
#ifndef PTZ_ADAPTER_H
#define PTZ_ADAPTER_H
#include <stdint.h>

/**
 * @struct ptz_device_status
 * @brief Current device PTZ position & speed in degrees / speed units.
 */
struct ptz_device_status {
    int h_pos_deg; /**< Horizontal position (degrees). */
    int v_pos_deg; /**< Vertical position (degrees). */
    int h_speed;   /**< Current horizontal speed. */
    int v_speed;   /**< Current vertical speed. */
};

/** Initialize underlying PTZ hardware or control channel. */
int ptz_adapter_init(void);
/** Shutdown / release PTZ hardware resources. */
int ptz_adapter_shutdown(void);
/** Retrieve current PTZ absolute position & speed. */
int ptz_adapter_get_status(struct ptz_device_status *status);
/** Move to absolute pan/tilt with speed. */
int ptz_adapter_absolute_move(int pan_deg, int tilt_deg, int speed);
/** Move relative delta in pan/tilt. */
int ptz_adapter_relative_move(int pan_delta_deg, int tilt_delta_deg, int speed);
/** Start continuous velocity move (timeout_s seconds, 0 = indefinite). */
int ptz_adapter_continuous_move(int pan_vel, int tilt_vel, int timeout_s);
/** Stop any motion (pan & tilt). */
int ptz_adapter_stop(void);
/** Store current position as preset id with optional name. */
int ptz_adapter_set_preset(const char *name, int id);
/** Move to a previously stored preset id. */
int ptz_adapter_goto_preset(int id);

#endif
