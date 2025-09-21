/**
 * @file onvifd.c
 * @brief Main ONVIF daemon entry point
 * @author kkrzysztofik
 * @date 2025
 */

#include <string.h>

#include "core/config/config.h"
#include "core/lifecycle/config_lifecycle.h"
#include "core/lifecycle/network_lifecycle.h"
#include "core/lifecycle/platform_lifecycle.h"
#include "core/lifecycle/signal_lifecycle.h"
#include "core/lifecycle/video_lifecycle.h"
#include "networking/rtsp/rtsp_multistream.h"
#include "platform/platform.h"

/* ---------------------------- Utility Functions ------------------------- */

/**
 * @brief Print service endpoint URLs for user convenience.
 * @param cfg Application configuration
 */
static void print_endpoints(const struct application_config *cfg) {
  platform_log_notice("ONVIF daemon started successfully on port %d\n",
                      cfg->onvif.http_port);
  platform_log_notice("Device services available at:\n");
  platform_log_notice("  Device:  http://[IP]:%d/onvif/device_service\n",
                      cfg->onvif.http_port);
  platform_log_notice("  Media:   http://[IP]:%d/onvif/media_service\n",
                      cfg->onvif.http_port);
  platform_log_notice("  PTZ:     http://[IP]:%d/onvif/ptz_service\n",
                      cfg->onvif.http_port);
  platform_log_notice("  Imaging: http://[IP]:%d/onvif/imaging_service\n",
                      cfg->onvif.http_port);

  rtsp_multistream_server_t *rtsp_server = video_lifecycle_get_rtsp_server();
  if (rtsp_server) {
    int stream_count = rtsp_multistream_get_stream_count(rtsp_server);
    platform_log_notice("  RTSP Streams: %d streams available on port 554\n",
                        stream_count);
    platform_log_notice("    Main: rtsp://[IP]:554/vs0\n");
    platform_log_notice("    Sub:  rtsp://[IP]:554/vs1\n");
  }

  platform_log_notice("  Snapshot: http://[IP]:%d/snapshot.jpeg\n",
                      cfg->onvif.http_port);
  platform_log_notice("Press Ctrl-C to stop.\n");
}

/* ---------------------------- Main Function ------------------------- */

int main(int argc, char **argv) {
  struct application_config cfg;

  /* Initialize configuration structure to prevent garbage data */
  memset(&cfg, 0, sizeof(struct application_config));

  /* Register signal handlers for graceful shutdown */
  if (signal_lifecycle_register_handlers() != 0) {
    return 1;
  }

  /* Initialize platform and memory management */
  if (platform_lifecycle_init() != 0) {
    platform_log_error("Failed to initialize platform\n");
    return 1;
  }

  /* Allocate memory for configuration structures */
  if (config_lifecycle_allocate_memory(&cfg) != 0) {
    platform_lifecycle_cleanup();
    return 1;
  }

  /* Load configuration using config system */
  if (config_lifecycle_load_configuration(&cfg) != 0) {
    config_lifecycle_free_memory(&cfg);
    platform_lifecycle_cleanup();
    return 1;
  }

  if (!cfg.onvif.enabled) {
    platform_log_notice("ONVIF service is disabled in configuration\n");
    config_lifecycle_free_memory(&cfg);
    platform_lifecycle_cleanup();
    return 0;
  }

  platform_log_notice("Starting ONVIF daemon...\n");

  /* Initialize video system (non-fatal on failure) */
  video_lifecycle_init(&cfg);

  /* Initialize network services */
  if (network_lifecycle_init(&cfg) != 0) {
    platform_log_error("Failed to initialize network services\n");
    config_lifecycle_free_memory(&cfg);
    platform_lifecycle_cleanup();
    return 1;
  }

  /* Print service endpoints */
  print_endpoints(&cfg);

  /* Main daemon loop with signal handling */
  signal_lifecycle_run_daemon_loop(&cfg);

  platform_log_notice("Shutting down ONVIF daemon...\n");
  platform_lifecycle_cleanup();

  /* Free allocated memory */
  config_lifecycle_free_memory(&cfg);

  platform_log_notice("ONVIF daemon exited\n");
  return 0;
}
