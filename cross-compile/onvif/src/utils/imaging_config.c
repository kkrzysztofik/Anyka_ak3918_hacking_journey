/* imaging_config.c - Configuration loader for ONVIF imaging settings */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include "imaging_config.h"
#include "../services/imaging/onvif_imaging.h"

static int parse_ini_line(const char *line, char *section, char *key, char *value) {
    char *p; 
    while (*line == ' ' || *line == '\t') line++; 
    if (*line == '\0' || *line == '\n' || *line == '#') return 0; 
    if (*line == '[') { 
        line++; 
        p = strchr(line, ']'); 
        if (p) { 
            *p = '\0'; 
            strcpy(section, line); 
            return 1; 
        } 
    } 
    p = strchr(line, '='); 
    if (p) { 
        *p = '\0'; 
        strcpy(key, line); 
        strcpy(value, p + 1); 
        p = value + strlen(value) - 1; 
        while (p >= value && (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r')) { 
            *p-- = '\0'; 
        } 
        return 2; 
    } 
    return 0; 
}

int imaging_config_load(const char *path, struct imaging_config *cfg) {
    FILE *fp;
    char line[256];
    char section[64] = "";
    char key[64], value[64];
    int result;
    
    if (!cfg || !path) return -1;
    
    fp = fopen(path, "r");
    if (!fp) {
        printf("Warning: Could not open imaging config file %s: %s\n", path, strerror(errno));
        return -1;
    }
    
    while (fgets(line, sizeof(line), fp)) {
        result = parse_ini_line(line, section, key, value);
        if (result == 2 && strcmp(section, "imaging") == 0) {
            if (strcmp(key, "brightness") == 0) {
                cfg->brightness = atoi(value);
            } else if (strcmp(key, "contrast") == 0) {
                cfg->contrast = atoi(value);
            } else if (strcmp(key, "saturation") == 0) {
                cfg->saturation = atoi(value);
            } else if (strcmp(key, "sharpness") == 0) {
                cfg->sharpness = atoi(value);
            }
        }
    }
    
    fclose(fp);
    printf("Loaded imaging configuration from %s\n", path);
    return 0;
}

int imaging_config_save(const char *path, const struct imaging_config *cfg) {
    FILE *fp;
    
    if (!cfg || !path) return -1;
    
    fp = fopen(path, "w");
    if (!fp) {
        printf("Error: Could not write imaging config file %s: %s\n", path, strerror(errno));
        return -1;
    }
    
    fprintf(fp, "# ONVIF Imaging Configuration\n");
    fprintf(fp, "# Auto-generated configuration file\n\n");
    fprintf(fp, "[imaging]\n");
    fprintf(fp, "brightness=%d\n", cfg->brightness);
    fprintf(fp, "contrast=%d\n", cfg->contrast);
    fprintf(fp, "saturation=%d\n", cfg->saturation);
    fprintf(fp, "sharpness=%d\n", cfg->sharpness);
    
    fclose(fp);
    printf("Saved imaging configuration to %s\n", path);
    return 0;
}

int imaging_config_load_auto(const char *path, struct auto_daynight_config *cfg) {
    FILE *fp;
    char line[256];
    char section[64] = "";
    char key[64], value[256];
    int result;

    if (!cfg || !path) return -1;

    fp = fopen(path, "r");
    if (!fp) return -1;

    while (fgets(line, sizeof(line), fp)) {
        result = parse_ini_line(line, section, key, value);
        if (result == 2) {
            /* support device config section name [autoir] only */
            if (strcmp(section, "autoir") == 0) {
                if (strcmp(key, "day_to_night_lum") == 0 || strcmp(key, "day_to_night_threshold") == 0) {
                    cfg->day_to_night_threshold = atoi(value);
                } else if (strcmp(key, "night_to_day_lum") == 0 || strcmp(key, "night_to_day_threshold") == 0) {
                    cfg->night_to_day_threshold = atoi(value);
                } else if (strcmp(key, "lock_time") == 0 || strcmp(key, "lock_time_seconds") == 0) {
                    cfg->lock_time_seconds = atoi(value);
                } else if (strcmp(key, "auto_day_night_enable") == 0) {
                    cfg->enable_auto_switching = atoi(value);
                } else if (strcmp(key, "day_night_mode") == 0) {
                    int m = atoi(value);
                    if (m == 0) cfg->mode = DAY_NIGHT_AUTO;
                    else if (m == 1) cfg->mode = DAY_NIGHT_DAY;
                    else if (m == 2) cfg->mode = DAY_NIGHT_NIGHT;
                }
            }
        }
    }

    fclose(fp);
    return 0;
}

int imaging_config_save_auto(const char *path, const struct auto_daynight_config *cfg) {
    FILE *fp;
    
    if (!cfg || !path) return -1;
    
    fp = fopen(path, "a");
    if (!fp) return -1;
    
    fprintf(fp, "\n[autoir]\n");
    fprintf(fp, "auto_day_night_enable=%d\n", cfg->enable_auto_switching);
    fprintf(fp, "day_night_mode=%d\n", cfg->mode);
    fprintf(fp, "day_to_night_lum=%d\n", cfg->day_to_night_threshold);
    fprintf(fp, "night_to_day_lum=%d\n", cfg->night_to_day_threshold);
    fprintf(fp, "lock_time=%d\n", cfg->lock_time_seconds);
    
    fclose(fp);
    return 0;
}

int imaging_config_load_defaults(struct imaging_settings *cfg) {
    if (!cfg) return -1;
    
    /* Set default imaging values */
    cfg->brightness = 50;
    cfg->contrast = 50;
    cfg->saturation = 50;
    cfg->sharpness = 50;
    cfg->hue = 0;
    
    /* Set default day/night configuration */
    cfg->daynight.mode = DAY_NIGHT_AUTO;
    cfg->daynight.day_to_night_threshold = 30;
    cfg->daynight.night_to_day_threshold = 70;
    cfg->daynight.lock_time_seconds = 5;
    cfg->daynight.ir_led_mode = IR_LED_AUTO;
    cfg->daynight.ir_led_level = 80;
    cfg->daynight.enable_auto_switching = 1;
    
    printf("Loaded default imaging configuration\n");
    return 0;
}
