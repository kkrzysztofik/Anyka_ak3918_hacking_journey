/* onvif_imaging.c - ONVIF Imaging service implementation */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>
#include <pthread.h>
#include "onvif_imaging.h"
#include "config.h"
#include "constants.h"
#include "ak_common.h"
#include "platform.h"

static struct auto_daynight_config g_auto_config;
static struct imaging_settings g_imaging_settings;
static platform_vi_handle_t g_vi_handle = NULL;
static pthread_mutex_t g_imaging_mutex = PTHREAD_MUTEX_INITIALIZER;
static int g_imaging_initialized = 0;

#define DEFAULT_DAY_TO_NIGHT_LUM 30
#define DEFAULT_NIGHT_TO_DAY_LUM 70
#define DEFAULT_LOCK_TIME 10
#define DEFAULT_IRLED_LEVEL 1

int onvif_imaging_init(void *vi_handle) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex); 
        return 0; 
    } 
    
    g_vi_handle = vi_handle; 
    
    // Initialize default imaging settings
    memset(&g_imaging_settings, 0, sizeof(g_imaging_settings)); 
    g_imaging_settings.brightness = 0;
    g_imaging_settings.contrast = 0;
    g_imaging_settings.saturation = 0;
    g_imaging_settings.sharpness = 0;
    g_imaging_settings.hue = 0;
    g_imaging_settings.daynight.mode = DAY_NIGHT_AUTO;
    g_imaging_settings.daynight.day_to_night_threshold = DEFAULT_DAY_TO_NIGHT_LUM;
    g_imaging_settings.daynight.night_to_day_threshold = DEFAULT_NIGHT_TO_DAY_LUM;
    g_imaging_settings.daynight.lock_time_seconds = DEFAULT_LOCK_TIME;
    g_imaging_settings.daynight.ir_led_mode = IR_LED_AUTO;
    g_imaging_settings.daynight.ir_led_level = DEFAULT_IRLED_LEVEL;
    g_imaging_settings.daynight.enable_auto_switching = 1;
    
    // Pull imaging settings from application config (already loaded in main)
    const struct application_config *ucfg = config_get();
    if (ucfg) {
        g_imaging_settings = ucfg->imaging;
    platform_log_notice("Loaded imaging settings from application config\n");
    } else {
    platform_log_notice("Application config not loaded; using defaults\n");
    }
    
    // Initialize auto day/night configuration
    memset(&g_auto_config, 0, sizeof(g_auto_config)); 
    g_auto_config.mode = DAY_NIGHT_AUTO;
    g_auto_config.day_to_night_threshold = DEFAULT_DAY_TO_NIGHT_LUM; 
    g_auto_config.night_to_day_threshold = DEFAULT_NIGHT_TO_DAY_LUM; 
    g_auto_config.lock_time_seconds = DEFAULT_LOCK_TIME; 
    g_auto_config.ir_led_mode = IR_LED_AUTO;
    g_auto_config.ir_led_level = DEFAULT_IRLED_LEVEL;
    g_auto_config.enable_auto_switching = 1;
    
    // Load auto configuration
    if (ucfg) {
        g_auto_config = ucfg->auto_daynight;
    }
    
    // Initialize IR LED driver via HAL
    if (platform_irled_init(g_auto_config.ir_led_level) != 0) {
        platform_log_error("Failed to initialize IR LED driver\n");
    } else {
        platform_log_notice("IR LED driver initialized with level %d\n", g_auto_config.ir_led_level);
        
        // Set initial IR LED mode
        if (g_auto_config.ir_led_mode == IR_LED_ON) {
            platform_irled_set_mode(1);
        } else if (g_auto_config.ir_led_mode == IR_LED_OFF) {
            platform_irled_set_mode(0);
        }
    }
    
    // Apply current imaging settings to VPSS
    if (g_vi_handle) {
        // Set brightness
        int brightness_vpss = g_imaging_settings.brightness / 2;
        platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_BRIGHTNESS, brightness_vpss);
        
        // Set contrast
        int contrast_vpss = g_imaging_settings.contrast / 2;
        platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_CONTRAST, contrast_vpss);
        
        // Set saturation
        int saturation_vpss = g_imaging_settings.saturation / 2;
        platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_SATURATION, saturation_vpss);
        
        // Set sharpness
        int sharpness_vpss = g_imaging_settings.sharpness / 2;
        platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_SHARPNESS, sharpness_vpss);
        
        // Set hue
        int hue_vpss = g_imaging_settings.hue * 50 / 180;
        platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_HUE, hue_vpss);
        
        platform_log_notice("Applied imaging settings to VPSS\n");
    }
    
    // Initialize auto day/night mode if enabled
    if (g_auto_config.enable_auto_switching) {
        platform_log_notice("Auto day/night mode enabled (thresholds: %d/%d)\n", 
                       g_auto_config.day_to_night_threshold, 
                       g_auto_config.night_to_day_threshold);
    }
    
    g_imaging_initialized = 1; 
    pthread_mutex_unlock(&g_imaging_mutex); 
    platform_log_notice("ONVIF Imaging service initialized successfully\n"); 
    return 0; 
}

void onvif_imaging_cleanup(void) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (g_imaging_initialized) {
        g_imaging_initialized = 0;
        platform_log_notice("ONVIF Imaging service cleaned up\n");
    }
    pthread_mutex_unlock(&g_imaging_mutex);
}

int onvif_imaging_get_settings(struct imaging_settings *settings) {
    if (!settings) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    *settings = g_imaging_settings;
    pthread_mutex_unlock(&g_imaging_mutex);
    return 0;
}

int onvif_imaging_set_settings(const struct imaging_settings *settings) {
    if (!settings) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    int ret = 0;
    
    // Set brightness (VPSS effect range: -50 to 50, ONVIF range: -100 to 100)
    int brightness_vpss = settings->brightness / 2;
    if (platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_BRIGHTNESS, brightness_vpss) != 0) {
        platform_log_error("Failed to set brightness\n");
        ret = -1;
    } else {
        g_imaging_settings.brightness = settings->brightness;
    }
    
    // Set contrast (VPSS effect range: -50 to 50, ONVIF range: -100 to 100)
    int contrast_vpss = settings->contrast / 2;
    if (platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_CONTRAST, contrast_vpss) != 0) {
        platform_log_error("Failed to set contrast\n");
        ret = -1;
    } else {
        g_imaging_settings.contrast = settings->contrast;
    }
    
    // Set saturation (VPSS effect range: -50 to 50, ONVIF range: -100 to 100)
    int saturation_vpss = settings->saturation / 2;
    if (platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_SATURATION, saturation_vpss) != 0) {
        platform_log_error("Failed to set saturation\n");
        ret = -1;
    } else {
        g_imaging_settings.saturation = settings->saturation;
    }
    
    // Set sharpness (VPSS effect range: -50 to 50, ONVIF range: -100 to 100)
    int sharpness_vpss = settings->sharpness / 2;
    if (platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_SHARPNESS, sharpness_vpss) != 0) {
        platform_log_error("Failed to set sharpness\n");
        ret = -1;
    } else {
        g_imaging_settings.sharpness = settings->sharpness;
    }
    
    // Set hue (VPSS effect range: -50 to 50, ONVIF range: -180 to 180)
    int hue_vpss = settings->hue * 50 / 180;
    if (platform_vpss_effect_set(g_vi_handle, PLATFORM_VPSS_EFFECT_HUE, hue_vpss) != 0) {
        platform_log_error("Failed to set hue\n");
        ret = -1;
    } else {
        g_imaging_settings.hue = settings->hue;
    }
    
    // Update day/night configuration
    g_imaging_settings.daynight = settings->daynight;
    
    // Save settings to configuration (imaging section)
    if (ret == 0) {
        config_save_imaging(&g_imaging_settings);
        platform_log_notice("Imaging settings updated successfully\n");
    }
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return ret;
}

int onvif_imaging_set_day_night_mode(enum day_night_mode mode) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    int ret = 0;
    platform_daynight_mode_t vi_mode;
    
    switch (mode) {
        case DAY_NIGHT_DAY:
            vi_mode = PLATFORM_DAYNIGHT_DAY;
            break;
        case DAY_NIGHT_NIGHT:
            vi_mode = PLATFORM_DAYNIGHT_NIGHT;
            break;
        case DAY_NIGHT_AUTO:
        default:
            // For auto mode, we'll use day mode by default and let the auto switching handle it
            vi_mode = PLATFORM_DAYNIGHT_DAY;
            break;
    }
    
    if (platform_vi_switch_day_night(g_vi_handle, vi_mode) != 0) {
        platform_log_error("Failed to switch day/night mode\n");
        ret = -1;
    } else {
        g_imaging_settings.daynight.mode = mode;
        platform_log_notice("Day/night mode set to %d\n", mode);
    }
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return ret;
}

int onvif_imaging_get_day_night_mode(void) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    enum day_night_mode mode = g_imaging_settings.daynight.mode;
    pthread_mutex_unlock(&g_imaging_mutex);
    return mode;
}

int onvif_imaging_set_irled_mode(enum ir_led_mode mode) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    int ret = 0;
    switch (mode) {
        case IR_LED_OFF:
            platform_irled_set_mode(0);
            break;
        case IR_LED_ON:
            platform_irled_set_mode(1);
            break;
        case IR_LED_AUTO:
        default:
            // For auto mode, we'll enable it by default and let the auto day/night control it
            platform_irled_set_mode(2);
            break;
    }
    g_imaging_settings.daynight.ir_led_mode = mode;
    platform_log_notice("IR LED mode set to %d\n", mode);
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return ret;
}

int onvif_imaging_get_irled_status(void) {
    int status = platform_irled_get_status();
    if (status < 0) {
        platform_log_error("Failed to get IR LED status\n");
        return 0; // Return off if we can't determine status
    }
    return status;
}

int onvif_imaging_set_flip_mirror(int flip, int mirror) {
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    int ret = platform_vi_set_flip_mirror(g_vi_handle, flip, mirror);
    if (ret != 0) {
        platform_log_error("Failed to set flip/mirror: flip=%d, mirror=%d\n", flip, mirror);
    } else {
        platform_log_notice("Flip/mirror set: flip=%d, mirror=%d\n", flip, mirror);
    }
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return ret;
}

int onvif_imaging_set_auto_config(const struct auto_daynight_config *config) {
    if (!config) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    g_auto_config = *config;
    g_imaging_settings.daynight = *config;
    
    // Save auto configuration section
    config_save_auto_daynight(&g_auto_config);
    platform_log_notice("Auto day/night configuration updated\n");
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return 0;
}

int onvif_imaging_get_auto_config(struct auto_daynight_config *config) {
    if (!config) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    *config = g_auto_config;
    pthread_mutex_unlock(&g_imaging_mutex);
    return 0;
}

int onvif_imaging_get_imaging_settings(char *response, int response_size) {
    if (!response || response_size <= 0) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    pthread_mutex_lock(&g_imaging_mutex);
    if (!g_imaging_initialized) {
        pthread_mutex_unlock(&g_imaging_mutex);
        platform_log_error("Imaging service not initialized\n");
        return -1;
    }
    
    // Get current VPSS effect values
    int brightness_vpss = 0, contrast_vpss = 0, saturation_vpss = 0, sharpness_vpss = 0, hue_vpss = 0;
    platform_vpss_effect_get(g_vi_handle, PLATFORM_VPSS_EFFECT_BRIGHTNESS, &brightness_vpss);
    platform_vpss_effect_get(g_vi_handle, PLATFORM_VPSS_EFFECT_CONTRAST, &contrast_vpss);
    platform_vpss_effect_get(g_vi_handle, PLATFORM_VPSS_EFFECT_SATURATION, &saturation_vpss);
    platform_vpss_effect_get(g_vi_handle, PLATFORM_VPSS_EFFECT_SHARPNESS, &sharpness_vpss);
    platform_vpss_effect_get(g_vi_handle, PLATFORM_VPSS_EFFECT_HUE, &hue_vpss);
    
    // Convert VPSS values back to ONVIF range
    int brightness_onvif = brightness_vpss * 2;
    int contrast_onvif = contrast_vpss * 2;
    int saturation_onvif = saturation_vpss * 2;
    int sharpness_onvif = sharpness_vpss * 2;
    int hue_onvif = hue_vpss * 180 / 50;
    
    // Create XML response using template
    snprintf(response, response_size, ONVIF_SOAP_IMAGING_GET_SETTINGS_RESPONSE,
             brightness_onvif, contrast_onvif, saturation_onvif, sharpness_onvif, hue_onvif);
    
    pthread_mutex_unlock(&g_imaging_mutex);
    return 0;
}

int onvif_imaging_set_imaging_settings(const char *request, char *response, int response_size) {
    if (!request || !response || response_size <= 0) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    // Simple XML parsing - in a real implementation, you'd use a proper XML parser
    struct imaging_settings settings = g_imaging_settings;
    
    // Parse brightness
    char *brightness_start = strstr(request, "<tt:Brightness>");
    if (brightness_start) {
        brightness_start += strlen("<tt:Brightness>");
        char *brightness_end = strstr(brightness_start, "</tt:Brightness>");
        if (brightness_end) {
            char brightness_str[16];
            int len = brightness_end - brightness_start;
            if (len < sizeof(brightness_str)) {
                strncpy(brightness_str, brightness_start, len);
                brightness_str[len] = '\0';
                settings.brightness = atoi(brightness_str);
            }
        }
    }
    
    // Parse contrast
    char *contrast_start = strstr(request, "<tt:Contrast>");
    if (contrast_start) {
        contrast_start += strlen("<tt:Contrast>");
        char *contrast_end = strstr(contrast_start, "</tt:Contrast>");
        if (contrast_end) {
            char contrast_str[16];
            int len = contrast_end - contrast_start;
            if (len < sizeof(contrast_str)) {
                strncpy(contrast_str, contrast_start, len);
                contrast_str[len] = '\0';
                settings.contrast = atoi(contrast_str);
            }
        }
    }
    
    // Parse saturation
    char *saturation_start = strstr(request, "<tt:Saturation>");
    if (saturation_start) {
        saturation_start += strlen("<tt:Saturation>");
        char *saturation_end = strstr(saturation_start, "</tt:Saturation>");
        if (saturation_end) {
            char saturation_str[16];
            int len = saturation_end - saturation_start;
            if (len < sizeof(saturation_str)) {
                strncpy(saturation_str, saturation_start, len);
                saturation_str[len] = '\0';
                settings.saturation = atoi(saturation_str);
            }
        }
    }
    
    // Parse sharpness
    char *sharpness_start = strstr(request, "<tt:Sharpness>");
    if (sharpness_start) {
        sharpness_start += strlen("<tt:Sharpness>");
        char *sharpness_end = strstr(sharpness_start, "</tt:Sharpness>");
        if (sharpness_end) {
            char sharpness_str[16];
            int len = sharpness_end - sharpness_start;
            if (len < sizeof(sharpness_str)) {
                strncpy(sharpness_str, sharpness_start, len);
                sharpness_str[len] = '\0';
                settings.sharpness = atoi(sharpness_str);
            }
        }
    }
    
    // Apply the settings
    int ret = onvif_imaging_set_settings(&settings);
    
    // Create response
    if (ret == 0) {
    snprintf(response, response_size, ONVIF_SOAP_IMAGING_SET_SETTINGS_OK);
    } else {
    snprintf(response, response_size, ONVIF_SOAP_IMAGING_SET_SETTINGS_FAIL);
    }
    
    return ret;
}

int onvif_imaging_get_options(char *response, int response_size) {
    if (!response || response_size <= 0) {
        platform_log_error("Invalid parameters\n");
        return -1;
    }
    
    // Create XML response with imaging capability ranges using template
    snprintf(response, response_size, ONVIF_SOAP_IMAGING_GET_OPTIONS_RESPONSE);
    
    return 0;
}