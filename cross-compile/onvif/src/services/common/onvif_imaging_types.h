/**
 * @file onvif_imaging_types.h
 * @brief ONVIF imaging type definitions to break circular dependencies
 * @author kkrzysztofik
 * @date 2025
 *
 */

#ifndef ONVIF_IMAGING_TYPES_H
#define ONVIF_IMAGING_TYPES_H

enum day_night_mode {
  DAY_NIGHT_AUTO = 0,
  DAY_NIGHT_DAY = 1,
  DAY_NIGHT_NIGHT = 2
};

enum ir_led_mode { IR_LED_OFF = 0, IR_LED_ON = 1, IR_LED_AUTO = 2 };

/** Auto day/night switching configuration */
struct auto_daynight_config {
  enum day_night_mode mode;
  int day_to_night_threshold; /* Luminance threshold to switch to night mode */
  int night_to_day_threshold; /* Luminance threshold to switch to day mode */
  int lock_time_seconds;      /* Time to wait before switching back */
  enum ir_led_mode ir_led_mode;
  int ir_led_level;          /* IR LED brightness level (0-100) */
  int enable_auto_switching; /* Enable automatic day/night switching */
};

/** Imaging tuning parameters exposed via ONVIF */
struct imaging_settings {
  int brightness; /* Brightness level (-100 to 100) */
  int contrast;   /* Contrast level (-100 to 100) */
  int saturation; /* Saturation level (-100 to 100) */
  int sharpness;  /* Sharpness level (-100 to 100) */
  int hue;        /* Hue level (-180 to 180) */
  struct auto_daynight_config daynight;
};

#endif /* ONVIF_IMAGING_TYPES_H */
