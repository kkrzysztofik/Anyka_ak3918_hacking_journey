/**
 * @file onvifd.c
 * @brief Main ONVIF daemon entry point: initializes subsystems & runs event loop.
 *
 * Responsibilities:
 *  - Load configuration
 *  - Initialize PTZ adapter (best-effort)
 *  - Initialize video input & launch RTSP servers (main/sub)
 *  - Start Imaging, HTTP SOAP server, WS-Discovery responder
 *  - Provide Ctrl-C signal handling and orderly shutdown
 */

#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <string.h>
#include "utils/config.h"
#include "services/service_manager.h"
#include "server/http/http_server.h"
#include "utils/memory_debug.h"
#include "server/rtsp/rtsp_server.h"
#include "server/discovery/ws_discovery.h"
#include "utils/error_handling.h"
#include "common/onvif_imaging_types.h"
#include "utils/safe_string.h"
#include "utils/error_context.h"
#include "utils/memory_manager.h"
#include "utils/constants_clean.h"

static volatile int running = 1;
/* Primary (main) and secondary (sub) RTSP servers */
static rtsp_server_t *rtsp_server_main = NULL;
static rtsp_server_t *rtsp_server_sub = NULL;
static platform_vi_handle_t vi_handle = NULL;

/* ---------------------------- Utility / Lifecycle ------------------------- */
/**
 * @brief Stop (and destroy) active RTSP servers.
 */
static void stop_rtsp_servers(void){
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
}

/**
 * @brief Perform full shutdown of all subsystems (idempotent).
 */
static void full_cleanup(void){
    stop_rtsp_servers();
    if (vi_handle) { platform_vi_close(vi_handle); vi_handle = NULL; }
    ws_discovery_stop();
    http_server_stop();
    onvif_services_cleanup();
    memory_debug_cleanup();
    memory_manager_cleanup();
}

/**
 * @brief Signal handler (SIGINT) to request graceful termination.
 */
static void sigint_handler(int sig){
    (void)sig;
    running = 0;
    /* Ask RTSP servers to stop promptly */
    if (rtsp_server_main) rtsp_server_stop(rtsp_server_main);
    if (rtsp_server_sub) rtsp_server_stop(rtsp_server_sub);
}

/* Print summary of service endpoints */
/**
 * @brief Print service endpoint URLs for user convenience.
 */
static void print_endpoints(const struct application_config *cfg){
    platform_log_notice("ONVIF daemon started successfully on port %d\n", cfg->onvif.http_port);
    platform_log_notice("Device services available at:\n");
    platform_log_notice("  Device:  http://[IP]:%d/onvif/device_service\n", cfg->onvif.http_port);
    platform_log_notice("  Media:   http://[IP]:%d/onvif/media_service\n", cfg->onvif.http_port);
    platform_log_notice("  PTZ:     http://[IP]:%d/onvif/ptz_service\n", cfg->onvif.http_port);
    platform_log_notice("  Imaging: http://[IP]:%d/onvif/imaging_service\n", cfg->onvif.http_port);
    if (rtsp_server_main) platform_log_notice("  RTSP Stream (main): rtsp://[IP]:554/vs0\n");
    if (rtsp_server_sub)  platform_log_notice("  RTSP Stream (sub):  rtsp://[IP]:554/vs1\n");
    platform_log_notice("Press Ctrl-C to stop.\n");
}

/* Configure and start an RTSP server given a prepared config struct */
/**
 * @brief Helper to create + start an RTSP server; destroys on failure.
 * @param cfg Stream configuration (modified internally by library as needed).
 * @param label Human-readable label for logging.
 * @return Server instance or NULL on failure.
 */
static rtsp_server_t *create_and_start_rtsp(rtsp_stream_config_t *cfg, const char *label){
    rtsp_server_t *srv = rtsp_server_create(cfg);
    if (!srv){
        platform_log_warning("warning: failed to create %s RTSP server\n", label);
        return NULL;
    }
    if (rtsp_server_start(srv) != 0){
        platform_log_warning("warning: failed to start %s RTSP stream\n", label);
        rtsp_server_destroy(srv);
        return NULL;
    }
    return srv;
}

/* Initialize video input and both RTSP streams (main + sub). Non-fatal if fails. */
/**
 * @brief Initialize video input and launch main + sub RTSP streams if possible.
 */
static void init_video_and_streams(void){
    platform_log_info("Initializing video input...\n");
    if (platform_vi_open(&vi_handle) != 0){
        platform_log_warning("warning: failed to open video input, RTSP streaming disabled\n");
        return;
    }

    platform_video_resolution_t resolution;
    platform_vi_get_sensor_resolution(vi_handle, &resolution);
    platform_log_info("Video input initialized: %dx%d\n", resolution.width, resolution.height);

    /* Main (/vs0) */
    rtsp_stream_config_t main_cfg; memset(&main_cfg, 0, sizeof(main_cfg));
    safe_strcpy(main_cfg.stream_path, sizeof(main_cfg.stream_path), "/vs0");
    safe_strcpy(main_cfg.stream_name, sizeof(main_cfg.stream_name), "main");
    main_cfg.port = 554; main_cfg.enabled = true; main_cfg.vi_handle = vi_handle;
    main_cfg.video_config.width  = (resolution.width  > 1920) ? 1920 : resolution.width;
    main_cfg.video_config.height = (resolution.height > 1080) ? 1080 : resolution.height;
    main_cfg.video_config.fps = 25; main_cfg.video_config.bitrate = 2000; /* kbps */
    main_cfg.video_config.gop_size = 50; main_cfg.video_config.profile = PLATFORM_PROFILE_MAIN;
    main_cfg.video_config.codec_type = PLATFORM_H264_ENC_TYPE; main_cfg.video_config.br_mode = PLATFORM_BR_MODE_CBR;
    rtsp_server_main = create_and_start_rtsp(&main_cfg, "main");
    if (rtsp_server_main) platform_log_notice("RTSP main stream started: rtsp://[IP]:554/vs0\n");

    /* Sub (/vs1) */
    rtsp_stream_config_t sub_cfg; memset(&sub_cfg, 0, sizeof(sub_cfg));
    safe_strcpy(sub_cfg.stream_path, sizeof(sub_cfg.stream_path), "/vs1");
    safe_strcpy(sub_cfg.stream_name, sizeof(sub_cfg.stream_name), "sub");
    sub_cfg.port = 554; sub_cfg.enabled = true; sub_cfg.vi_handle = vi_handle;
    int target_w = main_cfg.video_config.width / 2;
    int target_h = main_cfg.video_config.height / 2;
    if (target_w > 640) target_w = 640; else if (target_w < 320) target_w = 320;
    if (target_h > 360) target_h = 360; else if (target_h < 180) target_h = 180;
    if (target_w & 1) target_w--; if (target_h & 1) target_h--; /* even */
    sub_cfg.video_config.width = target_w; sub_cfg.video_config.height = target_h;
    sub_cfg.video_config.fps = 15; sub_cfg.video_config.bitrate = 512;
    sub_cfg.video_config.gop_size = 30; sub_cfg.video_config.profile = PLATFORM_PROFILE_MAIN;
    sub_cfg.video_config.codec_type = PLATFORM_H264_ENC_TYPE; sub_cfg.video_config.br_mode = PLATFORM_BR_MODE_CBR;
    rtsp_server_sub = create_and_start_rtsp(&sub_cfg, "sub");
    if (rtsp_server_sub){
        platform_log_notice("RTSP sub stream started: rtsp://[IP]:554/vs1 (%dx%d @ %dfps %dkbps)\n",
               sub_cfg.video_config.width, sub_cfg.video_config.height,
               sub_cfg.video_config.fps, sub_cfg.video_config.bitrate);
    }
}

/**
 * @brief Start imaging, HTTP server, and WS-Discovery (non-fatal if some fail).
 */
static void start_optional_services(const struct application_config *cfg){
    if (onvif_services_init(vi_handle) != 0)
        platform_log_warning("warning: failed to initialize ONVIF services\n");
    if (http_server_start(cfg->onvif.http_port) != 0) {
        platform_log_error("failed to start HTTP server on port %d\n", cfg->onvif.http_port);
        full_cleanup();
        exit(1);
    }
    if (ws_discovery_start(cfg->onvif.http_port) != 0)
        platform_log_warning("warning: WS-Discovery failed to start\n");
    else
        platform_log_notice("WS-Discovery responder active (multicast %s:%d)\n", "239.255.255.250", 3702);
}

int main(int argc, char **argv){
    struct application_config cfg;

    signal(SIGINT, sigint_handler);

    /* Initialize memory manager */
    if (memory_manager_init() != 0) {
        platform_log_error("Failed to initialize memory manager\n");
        return 1;
    }
    
    /* Initialize memory debugging */
    if (memory_debug_init() != 0) {
        platform_log_warning("warning: failed to initialize memory debugging\n");
    }

    /* Load configuration */
    if (config_load(&cfg, ONVIF_CONFIG_FILE) != 0) {
        platform_log_warning("warning: failed to read config at %s\n", ONVIF_CONFIG_FILE);
        platform_log_warning("warning: using default configuration (embedded)\n");
    }
    if (!cfg.onvif.enabled) {
        platform_log_notice("ONVIF service is disabled in configuration\n");
        return 0;
    }

    platform_log_notice("Starting ONVIF daemon...\n");
    platform_log_notice("Configuration: port=%d, user=%s (imaging.brightness=%d)\n",
           cfg.onvif.http_port, cfg.onvif.username, cfg.imaging->brightness);


    init_video_and_streams(); /* Non-fatal on failure */
    start_optional_services(&cfg);
    print_endpoints(&cfg);

    while (running) sleep(1);

    platform_log_notice("Shutting down ONVIF daemon...\n");
    full_cleanup();
    platform_log_notice("ONVIF daemon exited\n");
    return 0;
}
