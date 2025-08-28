/* onvif_config.c - Configuration loader for ONVIF daemon */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <ctype.h>
#include "onvif_config.h"

/* trim whitespace in-place */
static void trim(char *s) {
    char *p = s;
    char *end;
    while (isspace((unsigned char)*p)) p++;
    if (p != s) memmove(s, p, strlen(p) + 1);
    end = s + strlen(s) - 1;
    while (end >= s && isspace((unsigned char)*end)) { *end = '\0'; end--; }
}

/* case-insensitive key compare */
static int key_eq(const char *a, const char *b) {
    while (*a && *b) {
        if (tolower((unsigned char)*a) != tolower((unsigned char)*b)) return 0;
        a++; b++;
    }
    return *a == '\0' && *b == '\0';
}

int onvif_config_load(struct onvif_config *cfg, const char *config_file) {
    FILE *fp;
    char line[512];
    char *eq;
    char key[128], value[256];
    char section[128] = "";

    if (!cfg || !config_file) return -1;

    /* sensible defaults */
    cfg->enabled = 1;
    cfg->http_port = 8080;
    cfg->username[0] = '\0';
    cfg->password[0] = '\0';

    fp = fopen(config_file, "r");
    if (!fp) {
        printf("Warning: Could not open config file %s: %s\n", config_file, strerror(errno));
        return -1;
    }

    while (fgets(line, sizeof(line), fp)) {
        trim(line);
        if (line[0] == '\0') continue; /* empty */
        if (line[0] == ';' || line[0] == '#') continue; /* comment */

        /* section header [name] */
        if (line[0] == '[') {
            char *end = strchr(line, ']');
            if (end) {
                size_t len = end - line - 1;
                if (len >= sizeof(section)) len = sizeof(section)-1;
                strncpy(section, line+1, len);
                section[len] = '\0';
                trim(section);
            } else {
                section[0] = '\0';
            }
            continue;
        }

        eq = strchr(line, '=');
        if (!eq) continue;
        *eq = '\0';
        strncpy(key, line, sizeof(key)-1); key[sizeof(key)-1] = '\0';
        strncpy(value, eq+1, sizeof(value)-1); value[sizeof(value)-1] = '\0';
        trim(key); trim(value);

        /* handle common device config keys from anyka_cfg.ini */
        if (key_eq(key, "user") || key_eq(key, "username")) {
            strncpy(cfg->username, value, sizeof(cfg->username)-1);
            cfg->username[sizeof(cfg->username)-1] = '\0';
        } else if (key_eq(key, "secret") || key_eq(key, "password")) {
            strncpy(cfg->password, value, sizeof(cfg->password)-1);
            cfg->password[sizeof(cfg->password)-1] = '\0';
        } else if (key_eq(key, "enabled")) {
            cfg->enabled = atoi(value);
        } else if (key_eq(key, "http_port") || key_eq(key, "port")) {
            cfg->http_port = atoi(value);
        }

        /* also accept some keys under the [onvif] section specifically */
        if (section[0]) {
            if (key_eq(section, "onvif")) {
                /* if an explicit enabled flag exists under [onvif] */
                if (key_eq(key, "enabled")) cfg->enabled = atoi(value);
                if (key_eq(key, "http_port")) cfg->http_port = atoi(value);
                /* username/password may sometimes be placed in [global] only */
            }
        }
    }

    fclose(fp);
    printf("Loaded ONVIF configuration from %s\n", config_file);
    return 0;
}
