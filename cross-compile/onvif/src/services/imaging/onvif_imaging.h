/**
 * @file onvif_imaging.h
 * @brief ONVIF Imaging service API: brightness/contrast etc + day/night logic.
 */

#ifndef ONVIF_IMAGING_H
#define ONVIF_IMAGING_H

#include <stdint.h>
#include "../common/onvif_types.h"
#include "../common/onvif_request.h"

enum day_night_mode {
    DAY_NIGHT_AUTO = 0,
    DAY_NIGHT_DAY = 1,
    DAY_NIGHT_NIGHT = 2
};

enum ir_led_mode {
    IR_LED_OFF = 0,
    IR_LED_ON = 1,
    IR_LED_AUTO = 2
};

/** Auto day/night switching configuration */
struct auto_daynight_config {
    enum day_night_mode mode;
    int day_to_night_threshold;     /* Luminance threshold to switch to night mode */
    int night_to_day_threshold;     /* Luminance threshold to switch to day mode */
    int lock_time_seconds;          /* Time to wait before switching back */
    enum ir_led_mode ir_led_mode;
    int ir_led_level;               /* IR LED brightness level (0-100) */
    int enable_auto_switching;      /* Enable automatic day/night switching */
};

/** Imaging tuning parameters exposed via ONVIF */
struct imaging_settings {
    int brightness;                 /* Brightness level (-100 to 100) */
    int contrast;                   /* Contrast level (-100 to 100) */
    int saturation;                 /* Saturation level (-100 to 100) */
    int sharpness;                  /* Sharpness level (-100 to 100) */
    int hue;                        /* Hue level (-180 to 180) */
    struct auto_daynight_config daynight;
};

int onvif_imaging_init(void *vi_handle); /**< Initialize imaging module with VI handle (nullable). */
void onvif_imaging_cleanup(void);        /**< Release imaging resources. */
int onvif_imaging_get_settings(struct imaging_settings *settings); /**< Fetch current settings. */
int onvif_imaging_set_settings(const struct imaging_settings *settings); /**< Apply settings. */
int onvif_imaging_set_day_night_mode(enum day_night_mode mode); /**< Force day/night mode. */
int onvif_imaging_get_day_night_mode(void); /**< Query current forced/auto mode. */
int onvif_imaging_set_irled_mode(enum ir_led_mode mode); /**< Set IR LED mode. */
int onvif_imaging_get_irled_status(void); /**< Get IR LED status/level. */
int onvif_imaging_set_flip_mirror(int flip, int mirror); /**< Configure image flip/mirror. */
int onvif_imaging_set_auto_config(const struct auto_daynight_config *config); /**< Persist auto config. */
int onvif_imaging_get_auto_config(struct auto_daynight_config *config); /**< Retrieve auto config. */
int onvif_imaging_get_imaging_settings(char *response, int response_size); /**< SOAP: GetImagingSettings. */
int onvif_imaging_set_imaging_settings(const char *request, char *response, int response_size); /**< SOAP: SetImagingSettings. */
int onvif_imaging_get_options(char *response, int response_size); /**< SOAP: GetOptions. */
int onvif_imaging_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response); /**< Handle ONVIF imaging service requests. */

#endif
