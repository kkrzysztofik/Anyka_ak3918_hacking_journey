/* onvif_imaging.h - ONVIF Imaging service header */

#ifndef ONVIF_IMAGING_H
#define ONVIF_IMAGING_H

#include <stdint.h>

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

struct auto_daynight_config {
    enum day_night_mode mode;
    int day_to_night_threshold;     /* Luminance threshold to switch to night mode */
    int night_to_day_threshold;     /* Luminance threshold to switch to day mode */
    int lock_time_seconds;          /* Time to wait before switching back */
    enum ir_led_mode ir_led_mode;
    int ir_led_level;               /* IR LED brightness level (0-100) */
    int enable_auto_switching;      /* Enable automatic day/night switching */
};

struct imaging_settings {
    int brightness;                 /* Brightness level (-100 to 100) */
    int contrast;                   /* Contrast level (-100 to 100) */
    int saturation;                 /* Saturation level (-100 to 100) */
    int sharpness;                  /* Sharpness level (-100 to 100) */
    int hue;                        /* Hue level (-180 to 180) */
    struct auto_daynight_config daynight;
};

int onvif_imaging_init(void *vi_handle);
void onvif_imaging_cleanup(void);
int onvif_imaging_get_settings(struct imaging_settings *settings);
int onvif_imaging_set_settings(const struct imaging_settings *settings);
int onvif_imaging_set_day_night_mode(enum day_night_mode mode);
int onvif_imaging_get_day_night_mode(void);
int onvif_imaging_set_irled_mode(enum ir_led_mode mode);
int onvif_imaging_get_irled_status(void);
int onvif_imaging_set_flip_mirror(int flip, int mirror);
int onvif_imaging_set_auto_config(const struct auto_daynight_config *config);
int onvif_imaging_get_auto_config(struct auto_daynight_config *config);
int onvif_imaging_get_imaging_settings(char *response, int response_size);
int onvif_imaging_set_imaging_settings(const char *request, char *response, int response_size);
int onvif_imaging_get_options(char *response, int response_size);

#endif
