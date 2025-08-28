/*
 * onvifd.c
 * ONVIF daemon with HTTP/SOAP server. This file initializes the PTZ adapter,
 * ONVIF services, and starts the HTTP server to handle ONVIF requests.
 */

#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <string.h>
#include "ptz_adapter.h"
#include "onvif_config.h"
#include "services/imaging/onvif_imaging.h"
#include "http_server.h"
#include "rtsp_server.h"
#include "ak_vi.h"
#include "ak_venc.h"

static volatile int running = 1;
/* Primary (main) and secondary (sub) RTSP servers */
static rtsp_server_t *rtsp_server_main = NULL;
static rtsp_server_t *rtsp_server_sub = NULL;
static void *vi_handle = NULL;

static void sigint_handler(int sig){
    (void)sig;
    running = 0;
    
    /* Cleanup RTSP server on signal */
    if (rtsp_server_main) {
        rtsp_server_stop(rtsp_server_main);
    }
    if (rtsp_server_sub) {
        rtsp_server_stop(rtsp_server_sub);
    }
}

int main(int argc, char **argv){
    struct onvif_config cfg;

    signal(SIGINT, sigint_handler);

    /* Load configuration */
    if (onvif_config_load(&cfg, "/etc/jffs2/ankya_cfg.ini") != 0) {
        fprintf(stderr, "warning: failed to read config at /etc/jffs2/ankya_cfg.ini\n");
        if (onvif_config_load(&cfg, "/etc/jffs2/ankya_cfg.ini") != 0) {
            fprintf(stderr, "warning: using default configuration\n");
            cfg.enabled = 1;
            cfg.http_port = 8080;
            strcpy(cfg.username, "admin");
            strcpy(cfg.password, "admin");
        }
    }

    if (!cfg.enabled) {
        printf("ONVIF service is disabled in configuration\n");
        return 0;
    }

    printf("Starting ONVIF daemon...\n");
    printf("Configuration: port=%d, user=%s\n", cfg.http_port, cfg.username);

    /* Initialize PTZ adapter */
    if (ptz_adapter_init() != 0) {
        fprintf(stderr, "warning: failed to initialize ptz adapter\n");
        /* Continue anyway - device service should still work */
    }

    /* Initialize video input for RTSP streaming */
    printf("Initializing video input...\n");
    vi_handle = ak_vi_open(VIDEO_DEV0);
    if (!vi_handle) {
        fprintf(stderr, "warning: failed to open video input, RTSP streaming disabled\n");
    } else {
        struct video_resolution resolution;
        ak_vi_get_sensor_resolution(vi_handle, &resolution);
        printf("Video input initialized: %dx%d\n", resolution.width, resolution.height);
        
        /* Main stream configuration (/vs0) */
        rtsp_stream_config_t main_cfg;
        memset(&main_cfg, 0, sizeof(main_cfg));
        strcpy(main_cfg.stream_path, "/vs0");
        strcpy(main_cfg.stream_name, "main");
        main_cfg.port = 554;
        main_cfg.enabled = true;
        main_cfg.vi_handle = vi_handle;
        main_cfg.video_config.width = (resolution.width > 1920) ? 1920 : resolution.width;
        main_cfg.video_config.height = (resolution.height > 1080) ? 1080 : resolution.height;
        main_cfg.video_config.fps = 25;
        main_cfg.video_config.bitrate = 2000; /* kbps */
        main_cfg.video_config.gop_size = 50;
        main_cfg.video_config.profile = PROFILE_MAIN;
        main_cfg.video_config.codec_type = H264_ENC_TYPE;
        main_cfg.video_config.br_mode = BR_MODE_CBR;

        rtsp_server_main = rtsp_server_create(&main_cfg);
        if (rtsp_server_main) {
            if (rtsp_server_start(rtsp_server_main) == 0) {
                printf("RTSP main stream started: rtsp://[IP]:554/vs0\n");
            } else {
                fprintf(stderr, "warning: failed to start main RTSP stream\n");
                rtsp_server_destroy(rtsp_server_main);
                rtsp_server_main = NULL;
            }
        } else {
            fprintf(stderr, "warning: failed to create main RTSP server\n");
        }

        /* Sub stream configuration (/vs1) - lower resolution / bitrate */
        rtsp_stream_config_t sub_cfg;
        memset(&sub_cfg, 0, sizeof(sub_cfg));
        strcpy(sub_cfg.stream_path, "/vs1");
        strcpy(sub_cfg.stream_name, "sub");
        sub_cfg.port = 554; /* same RTSP port, path differentiates */
        sub_cfg.enabled = true;
        sub_cfg.vi_handle = vi_handle;

        /* Derive sub-stream resolution (approx half), clamp to typical 640x360 minimum */
        int target_w = main_cfg.video_config.width / 2;
        int target_h = main_cfg.video_config.height / 2;
        if (target_w > 640) target_w = 640; if (target_w < 320) target_w = 320;
        if (target_h > 360) target_h = 360; if (target_h < 180) target_h = 180;
        /* Ensure even values */
        if (target_w & 1) target_w--; if (target_h & 1) target_h--;
        sub_cfg.video_config.width = target_w;
        sub_cfg.video_config.height = target_h;
        sub_cfg.video_config.fps = 15;             /* lower fps for sub-stream */
        sub_cfg.video_config.bitrate = 512;        /* kbps */
        sub_cfg.video_config.gop_size = 30;
        sub_cfg.video_config.profile = PROFILE_MAIN;
        sub_cfg.video_config.codec_type = H264_ENC_TYPE;
        sub_cfg.video_config.br_mode = BR_MODE_CBR;

        rtsp_server_sub = rtsp_server_create(&sub_cfg);
        if (rtsp_server_sub) {
            if (rtsp_server_start(rtsp_server_sub) == 0) {
                printf("RTSP sub stream started: rtsp://[IP]:554/vs1 (%dx%d @ %dfps %dkbps)\n",
                       sub_cfg.video_config.width, sub_cfg.video_config.height,
                       sub_cfg.video_config.fps, sub_cfg.video_config.bitrate);
            } else {
                fprintf(stderr, "warning: failed to start sub RTSP stream\n");
                rtsp_server_destroy(rtsp_server_sub);
                rtsp_server_sub = NULL;
            }
        } else {
            fprintf(stderr, "warning: failed to create sub RTSP server\n");
        }
    }

    if (onvif_imaging_init(NULL) != 0) {
        fprintf(stderr, "warning: failed to initialize imaging service\n");
    }

    if (http_server_start(cfg.http_port) != 0) {
        fprintf(stderr, "failed to start HTTP server on port %d\n", cfg.http_port);
        ptz_adapter_shutdown();
        return 1;
    }

    printf("ONVIF daemon started successfully on port %d\n", cfg.http_port);
    printf("Device services available at:\n");
    printf("  Device: http://[IP]:%d/onvif/device_service\n", cfg.http_port);
    printf("  Media:  http://[IP]:%d/onvif/media_service\n", cfg.http_port);
    printf("  PTZ:    http://[IP]:%d/onvif/ptz_service\n", cfg.http_port);
    printf("  Imaging: http://[IP]:%d/onvif/imaging_service\n", cfg.http_port);
    if (rtsp_server_main) {
        printf("  RTSP Stream (main): rtsp://[IP]:554/vs0\n");
    }
    if (rtsp_server_sub) {
        printf("  RTSP Stream (sub):  rtsp://[IP]:554/vs1\n");
    }
    printf("Press Ctrl-C to stop.\n");

    while (running) { 
        sleep(1); 
    }

    printf("Shutting down ONVIF daemon...\n");
    if (rtsp_server_sub) {
        rtsp_server_stop(rtsp_server_sub);
        rtsp_server_destroy(rtsp_server_sub);
        rtsp_server_sub = NULL;
    }
    if (rtsp_server_main) { 
        rtsp_server_stop(rtsp_server_main); 
        rtsp_server_destroy(rtsp_server_main); 
        rtsp_server_main = NULL; 
    }
    if (vi_handle) { 
        ak_vi_close(vi_handle); 
        vi_handle = NULL; 
    }
    http_server_stop();
    onvif_imaging_cleanup();
    ptz_adapter_shutdown();
    printf("ONVIF daemon exited\n");
    return 0;
}
