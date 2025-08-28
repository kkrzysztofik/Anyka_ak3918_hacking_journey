/* config.c - application ONVIF configuration implementation */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <errno.h>
#include "config.h"
#include "constants.h"

static struct application_config g_config; /* singleton */
static int g_config_loaded = 0;

static void set_core_defaults(struct onvif_settings *s){
    s->enabled = 1;
    s->http_port = 8080;
    strcpy(s->username, "admin");
    strcpy(s->password, "admin");
}

static void set_imaging_defaults(struct imaging_settings *cfg){
    cfg->brightness = 50; cfg->contrast = 50; cfg->saturation = 50; cfg->sharpness = 50; cfg->hue = 0;
    cfg->daynight.mode = DAY_NIGHT_AUTO;
    cfg->daynight.day_to_night_threshold = 30;
    cfg->daynight.night_to_day_threshold = 70;
    cfg->daynight.lock_time_seconds = 5;
    cfg->daynight.ir_led_mode = IR_LED_AUTO;
    cfg->daynight.ir_led_level = 80;
    cfg->daynight.enable_auto_switching = 1;
}

/* trim whitespace in-place */
static void trim(char *s) {
    char *p = s, *end; while (isspace((unsigned char)*p)) p++; if (p!=s) memmove(s,p,strlen(p)+1);
    end = s + strlen(s) - 1; while (end>=s && isspace((unsigned char)*end)) {*end='\0'; end--;}
}
static int key_eq(const char *a, const char *b){ while(*a && *b){ if (tolower((unsigned char)*a)!=tolower((unsigned char)*b)) return 0; a++; b++; } return *a=='\0' && *b=='\0'; }

int config_load(struct application_config *cfg, const char *config_file){
    FILE *fp; char line[512]; char section[128]=""; char key[128], value[256]; char *eq;
    if (!cfg || !config_file) return -1;
    set_core_defaults(&cfg->onvif);
    set_imaging_defaults(&cfg->imaging);
    cfg->auto_daynight = cfg->imaging.daynight; /* mirror defaults */
    fp = fopen(config_file, "r");
    if (!fp){
        fprintf(stderr, "warning: could not open %s: %s (using defaults)\n", config_file, strerror(errno));
        return -1;
    }
    while (fgets(line,sizeof(line),fp)){
        trim(line); if(!line[0] || line[0]=='#' || line[0]==';') continue;
        if (line[0]=='['){ char *end=strchr(line,']'); if(end){ size_t len=end-line-1; if(len>=sizeof(section)) len=sizeof(section)-1; strncpy(section,line+1,len); section[len]='\0'; trim(section);} else section[0]='\0'; continue; }
        eq = strchr(line,'='); if(!eq) continue; *eq='\0'; strncpy(key,line,sizeof(key)-1); key[sizeof(key)-1]='\0'; strncpy(value,eq+1,sizeof(value)-1); value[sizeof(value)-1]='\0'; trim(key); trim(value);
        if (!section[0] || key_eq(section,"onvif")){
            if (key_eq(key,"enabled")) cfg->onvif.enabled = atoi(value);
            else if (key_eq(key,"user") || key_eq(key,"username")) { strncpy(cfg->onvif.username,value,sizeof(cfg->onvif.username)-1); cfg->onvif.username[sizeof(cfg->onvif.username)-1]='\0'; }
            else if (key_eq(key,"secret") || key_eq(key,"password")) { strncpy(cfg->onvif.password,value,sizeof(cfg->onvif.password)-1); cfg->onvif.password[sizeof(cfg->onvif.password)-1]='\0'; }
            else if (key_eq(key,"http_port") || key_eq(key,"port")) cfg->onvif.http_port = atoi(value);
        }
        if (key_eq(section,"imaging")){
            if (key_eq(key,"brightness")) cfg->imaging.brightness = atoi(value);
            else if (key_eq(key,"contrast")) cfg->imaging.contrast = atoi(value);
            else if (key_eq(key,"saturation")) cfg->imaging.saturation = atoi(value);
            else if (key_eq(key,"sharpness")) cfg->imaging.sharpness = atoi(value);
            else if (key_eq(key,"hue")) cfg->imaging.hue = atoi(value);
        }
        if (key_eq(section,"autoir")){
            if (key_eq(key,"auto_day_night_enable")) cfg->auto_daynight.enable_auto_switching = atoi(value);
            else if (key_eq(key,"day_night_mode")) { int m=atoi(value); if(m==0) cfg->auto_daynight.mode=DAY_NIGHT_AUTO; else if(m==1) cfg->auto_daynight.mode=DAY_NIGHT_DAY; else if(m==2) cfg->auto_daynight.mode=DAY_NIGHT_NIGHT; }
            else if (key_eq(key,"day_to_night_lum") || key_eq(key,"day_to_night_threshold")) cfg->auto_daynight.day_to_night_threshold = atoi(value);
            else if (key_eq(key,"night_to_day_lum") || key_eq(key,"night_to_day_threshold")) cfg->auto_daynight.night_to_day_threshold = atoi(value);
            else if (key_eq(key,"lock_time") || key_eq(key,"lock_time_seconds")) cfg->auto_daynight.lock_time_seconds = atoi(value);
        }
    }
    fclose(fp);
    g_config = *cfg; g_config_loaded = 1; /* cache */
    return 0;
}

const struct application_config *config_get(void){ return g_config_loaded ? &g_config : NULL; }

int config_save_imaging(const struct imaging_settings *s){ if(!s) return -1; /* rewrite imaging section only */
    FILE *fp = fopen(ONVIF_CONFIG_FILE, "a"); if(!fp) return -1; fprintf(fp,"\n[imaging]\nbrightness=%d\ncontrast=%d\nsaturation=%d\nsharpness=%d\nhue=%d\n", s->brightness,s->contrast,s->saturation,s->sharpness,s->hue); fclose(fp); return 0; }
int config_save_auto_daynight(const struct auto_daynight_config *c){ if(!c) return -1; FILE *fp=fopen(ONVIF_CONFIG_FILE,"a"); if(!fp) return -1; fprintf(fp,"\n[autoir]\nauto_day_night_enable=%d\nday_night_mode=%d\nday_to_night_lum=%d\nnight_to_day_lum=%d\nlock_time=%d\n", c->enable_auto_switching,c->mode,c->day_to_night_threshold,c->night_to_day_threshold,c->lock_time_seconds); fclose(fp); return 0; }
