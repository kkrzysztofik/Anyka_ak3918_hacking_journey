/**
 * @file video_lifecycle.c
 * @brief Video input and RTSP server lifecycle management implementation
 *
 * This module implements the complete video pipeline lifecycle management,
 * including sensor detection, video input initialization, channel
 * configuration, and RTSP server management.
 *
 * The module uses platform abstraction layer constants for video channel
 * definitions (PLATFORM_VIDEO_CHN_MAIN, PLATFORM_VIDEO_CHN_SUB) instead of
 * direct hardware-specific includes, ensuring better portability and
 * maintainability.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#include "core/lifecycle/video_lifecycle.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

#include "networking/rtsp/rtsp_multistream.h"
#include "networking/rtsp/rtsp_types.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "services/common/video_config_types.h"
#include "utils/error/error_handling.h"
#include "utils/stream/stream_config_utils.h"

/* Global video system state - static variables with internal linkage only */
static platform_vi_handle_t g_video_vi_handle = NULL;                // NOLINT
static rtsp_multistream_server_t *g_video_rtsp_multistream_server =  // NOLINT
    NULL;
static volatile bool g_video_rtsp_server_initialized = false;  // NOLINT
static volatile bool g_video_system_initialized = false;       // NOLINT

/* ---------------------------- Video Initialization Helpers
 * ------------------------- */

/**
 * @brief Match sensor configuration for proper hardware initialization
 * @return 0 on success, -1 on failure
 */
static int match_sensor_configuration(void) {
  if (platform_vi_match_sensor("/etc/jffs2") != 0) {
    platform_log_warning(
        "warning: failed to match sensor at /etc/jffs2, trying backup path\n");
    if (platform_vi_match_sensor("/data/sensor") != 0) {
      platform_log_warning(
          "warning: failed to match sensor at /data/sensor, trying /data\n");
      if (platform_vi_match_sensor("/data") != 0) {
        platform_log_warning(
            "warning: failed to match sensor, video input disabled\n");
        return -1;
      }
    }
  }
  return 0;
}

/**
 * @brief Initialize video input handle and get sensor information
 * @param resolution Output parameter for sensor resolution
 * @param sensor_fps Output parameter for sensor frame rate
 * @return 0 on success, -1 on failure
 */
static int initialize_video_input(platform_video_resolution_t *resolution,
                                  int *sensor_fps) {
  if (platform_vi_open(&g_video_vi_handle) != 0) {
    platform_log_warning(
        "warning: failed to open video input, RTSP streaming disabled\n");
    return -1;
  }

  platform_vi_get_sensor_resolution(g_video_vi_handle, resolution);
  platform_log_info("Video input initialized: %dx%d\n", resolution->width,
                    resolution->height);

  // Detect actual sensor frame rate for proper encoder configuration
  *sensor_fps = 15;  // Default fallback
  if (platform_vi_get_fps(g_video_vi_handle, sensor_fps) == PLATFORM_SUCCESS) {
    platform_log_info("Detected sensor frame rate: %d fps\n", *sensor_fps);
  } else {
    platform_log_warning(
        "warning: failed to detect sensor frame rate, using default 15fps\n");
    *sensor_fps = 15;
  }

  return 0;
}

/**
 * @brief Configure video channel attributes for main and sub streams
 * @param vi_handle Video input handle
 * @param resolution Sensor resolution
 * @return 0 on success, -1 on failure
 */
static int configure_video_channels(
    platform_vi_handle_t vi_handle,
    const platform_video_resolution_t *resolution) {
  platform_video_channel_attr_t channel_attr = {
      .crop = {.left = 0,
               .top = 0,
               .width = resolution->width,
               .height = resolution->height},
      .res = {
          [PLATFORM_VIDEO_CHN_MAIN] =
              {// Main channel - full resolution
               .width = resolution->width,
               .height = resolution->height},
          [PLATFORM_VIDEO_CHN_SUB] = {
              // Sub channel - 1/2 main resolution, but
              // constrained to hardware limits
              .width =
                  (resolution->width / 2 > 640) ? 640 : resolution->width / 2,
              .height = (resolution->height / 2 > 480)
                            ? 480
                            : resolution->height / 2}}};

  if (platform_vi_set_channel_attr(g_video_vi_handle, &channel_attr) !=
      PLATFORM_SUCCESS) {
    platform_log_error(
        "Failed to set video channel attributes, RTSP streaming disabled\n");
    return -1;
  }

  platform_log_debug("Video channel attributes set successfully\n");
  return 0;
}

/**
 * @brief Start global video capture
 * @param vi_handle Video input handle
 * @return 0 on success, -1 on failure
 */
static int start_global_capture(platform_vi_handle_t vi_handle) {
  if (platform_vi_start_global_capture(vi_handle) != PLATFORM_SUCCESS) {
    platform_log_error(
        "Failed to start global video capture, RTSP streaming disabled\n");
    return -1;
  }
  platform_log_info("Global video capture started successfully\n");
  return 0;
}

/**
 * @brief Initialize audio configuration with default values
 * @param audio_config Output audio configuration
 */
static void init_audio_config(audio_config_t *audio_config) {
  audio_config->sample_rate = 16000;
  audio_config->channels = 1;
  audio_config->bits_per_sample = 16;
  audio_config->codec_type = PLATFORM_AUDIO_CODEC_AAC;
  audio_config->bitrate = 64000;
}

/**
 * @brief Set video resolution with hardware limits
 * @param video_config Output video configuration
 * @param resolution Sensor resolution
 */
static void set_video_resolution(
    video_config_t *video_config,
    const platform_video_resolution_t *resolution) {
  video_config->width = (resolution->width > 1920) ? 1920 : resolution->width;
  video_config->height =
      (resolution->height > 1080) ? 1080 : resolution->height;
}

/**
 * @brief Configure frame rate with sensor validation
 * @param video_config Output video configuration
 * @param cfg Application configuration
 * @param sensor_fps Detected sensor frame rate
 */
static void configure_frame_rate(video_config_t *video_config,
                                 const struct application_config *cfg,
                                 int sensor_fps) {
  if (cfg && cfg->main_stream && cfg->main_stream->fps > 0) {
    // Config specifies FPS, validate against sensor capabilities
    if (cfg->main_stream->fps <= sensor_fps * 2 && cfg->main_stream->fps >= 5) {
      video_config->fps = cfg->main_stream->fps;
    } else {
      platform_log_warning(
          "Config FPS %d outside valid range (5-%d), using sensor FPS %d\n",
          cfg->main_stream->fps, sensor_fps * 2, sensor_fps);
      video_config->fps = sensor_fps;
    }
  } else {
    // Config doesn't specify FPS, use sensor frame rate
    video_config->fps = sensor_fps;
  }
}

/**
 * @brief Copy configuration values from config structure
 * @param video_config Output video configuration
 * @param cfg Application configuration
 */
static void copy_config_values(video_config_t *video_config,
                               const struct application_config *cfg) {
  if (cfg && cfg->main_stream) {
    video_config->bitrate = cfg->main_stream->bitrate;
    video_config->gop_size = cfg->main_stream->gop_size;
    video_config->profile = cfg->main_stream->profile;
    video_config->codec_type = cfg->main_stream->codec_type;
    video_config->br_mode = cfg->main_stream->br_mode;
  }
}

/**
 * @brief Set default video configuration values
 * @param video_config Output video configuration
 * @param sensor_fps Detected sensor frame rate
 */
static void set_default_video_config(video_config_t *video_config,
                                     int sensor_fps) {
  video_config->fps = sensor_fps;
  video_config->bitrate = 2000;
  video_config->gop_size = (sensor_fps > 0) ? sensor_fps * 2 : 50;
  video_config->profile = PLATFORM_PROFILE_MAIN;
  video_config->codec_type = PLATFORM_H264_ENC_TYPE;
  video_config->br_mode = PLATFORM_BR_MODE_CBR;
  platform_log_warning(
      "warning: using default main stream configuration with sensor fps %d\n",
      sensor_fps);
}

/**
 * @brief Validate and adjust GOP size
 * @param video_config Video configuration to validate
 */
static void validate_gop_size(video_config_t *video_config) {
  if (video_config->gop_size <= 0) {
    video_config->gop_size =
        (video_config->fps > 0) ? video_config->fps * 2 : 50;
    platform_log_debug(
        "validate_gop_size: Adjusted GOP size to %d for main stream (fps: "
        "%d)\n",
        video_config->gop_size, video_config->fps);
  }

  // Ensure GOP size is reasonable for the frame rate
  int max_reasonable_gop =
      video_config->fps * 10;  // Max 10 seconds worth of frames
  if (video_config->gop_size > max_reasonable_gop) {
    video_config->gop_size = max_reasonable_gop;
    platform_log_debug(
        "validate_gop_size: Reduced GOP size to %d for main stream (max "
        "reasonable: %d)\n",
        video_config->gop_size, max_reasonable_gop);
  }
}

/**
 * @brief Validate codec type
 * @param video_config Video configuration to validate
 */
static void validate_codec_type(video_config_t *video_config) {
  if (video_config->codec_type < 0) {
    video_config->codec_type = PLATFORM_H264_ENC_TYPE;
    platform_log_debug(
        "validate_codec_type: Set default codec to H264 for main stream\n");
  }
}

/**
 * @brief Configure main stream video parameters
 * @param cfg Application configuration
 * @param resolution Sensor resolution
 * @param sensor_fps Detected sensor frame rate
 * @param video_config Output video configuration
 * @param audio_config Output audio configuration
 */
static void configure_main_stream(const struct application_config *cfg,
                                  const platform_video_resolution_t *resolution,
                                  int sensor_fps, video_config_t *video_config,
                                  audio_config_t *audio_config) {
  // Initialize audio configuration
  init_audio_config(audio_config);

  // Set video resolution with hardware limits
  set_video_resolution(video_config, resolution);

  // Configure frame rate with sensor validation
  configure_frame_rate(video_config, cfg, sensor_fps);

  if (cfg && cfg->main_stream) {
    // Copy configuration values
    copy_config_values(video_config, cfg);

    // Validate configuration before using
    if (stream_config_validate(cfg->main_stream, true) != ONVIF_SUCCESS) {
      platform_log_warning(
          "warning: main stream configuration validation failed, using "
          "defaults\n");
      stream_config_init_defaults(cfg->main_stream, true);
      *video_config = *cfg->main_stream;
      // Override FPS with sensor rate if validation failed
      video_config->fps = sensor_fps;
    }

    // Log configuration summary
    char config_summary[256];
    if (stream_config_get_summary(cfg->main_stream, true, config_summary,
                                  sizeof(config_summary)) == ONVIF_SUCCESS) {
      platform_log_info("Main stream configuration: %s\n", config_summary);
    }
  } else {
    // Fallback to hardcoded defaults
    set_default_video_config(video_config, sensor_fps);
  }

  // Validate and adjust configuration
  validate_gop_size(video_config);
  validate_codec_type(video_config);
}

/**
 * @brief Create and configure multi-stream RTSP server
 * @param video_config Main stream video configuration
 * @param audio_config Main stream audio configuration
 * @return 0 on success, -1 on failure
 */
static int create_rtsp_server(const video_config_t *video_config,
                              const audio_config_t *audio_config) {
  // Create multi-stream RTSP server (only if not already created)
  if (!g_video_rtsp_server_initialized) {
    g_video_rtsp_multistream_server =
        rtsp_multistream_server_create(554, g_video_vi_handle);
    if (!g_video_rtsp_multistream_server) {
      platform_log_error("Failed to create multi-stream RTSP server\n");
      return -1;
    }
    g_video_rtsp_server_initialized = true;
    platform_log_info("Multi-stream RTSP server created successfully\n");
  }

  platform_log_info(
      "Multi-stream RTSP server already initialized, skipping creation\n");

  if (rtsp_multistream_server_add_stream(g_video_rtsp_multistream_server,
                                         "/vs0", "main", video_config,
                                         audio_config, false) != 0) {
    platform_log_error("Failed to add main stream to multi-stream server\n");
    rtsp_multistream_server_destroy(g_video_rtsp_multistream_server);
    g_video_rtsp_multistream_server = NULL;
    return -1;
  }

  platform_log_info(
      "Sub stream (/vs1) disabled - only main stream (/vs0) available\n");
  return 0;
}

/**
 * @brief Start the multi-stream RTSP server
 * @return 0 on success, -1 on failure
 */
static int start_rtsp_server(void) {
  if (!g_video_rtsp_multistream_server) {
    platform_log_warning("Cannot start RTSP server - server not created\n");
    return -1;
  }

  int start_result =
      rtsp_multistream_server_start(g_video_rtsp_multistream_server);
  if (start_result != 0) {
    platform_log_error("Failed to start multi-stream RTSP server\n");
    rtsp_multistream_server_destroy(g_video_rtsp_multistream_server);
    g_video_rtsp_multistream_server = NULL;
    g_video_rtsp_server_initialized = false;
    return -1;
  }

  platform_log_notice("Multi-stream RTSP server started successfully\n");
  return 0;
}

/* ---------------------------- Public Interface ------------------------- */

int video_lifecycle_init(const struct application_config *cfg) {
  platform_log_info("Initializing video input...\n");

  // Step 1: Match sensor configuration
  if (match_sensor_configuration() != 0) {
    return -1;
  }

  // Step 2: Initialize video input and get sensor info
  platform_video_resolution_t resolution;
  int sensor_fps = 15;  // Initialize with default value
  if (initialize_video_input(&resolution, &sensor_fps) != 0) {
    return -1;
  }

  // Step 3: Configure video channels
  if (configure_video_channels(g_video_vi_handle, &resolution) != 0) {
    platform_vi_close(g_video_vi_handle);
    g_video_vi_handle = NULL;
    return -1;
  }

  // Step 4: Start global video capture
  if (start_global_capture(g_video_vi_handle) != 0) {
    platform_vi_close(g_video_vi_handle);
    g_video_vi_handle = NULL;
    return -1;
  }

  // Step 5: Configure main stream
  video_config_t main_video_config;
  audio_config_t main_audio_config;
  configure_main_stream(cfg, &resolution, sensor_fps, &main_video_config,
                        &main_audio_config);

  // Step 6: Create RTSP server and add main stream
  if (create_rtsp_server(&main_video_config, &main_audio_config) != 0) {
    platform_vi_close(g_video_vi_handle);
    g_video_vi_handle = NULL;
    return -1;
  }

  // Step 7: Start RTSP server
  if (start_rtsp_server() != 0) {
    platform_vi_close(g_video_vi_handle);
    g_video_vi_handle = NULL;
    return -1;
  }

  g_video_system_initialized = true;
  return 0;
}

void video_lifecycle_cleanup(void) {
  static bool cleanup_done = false;

  if (cleanup_done) {
    platform_log_debug("Video cleanup already performed, skipping\n");
    return;
  }

  platform_log_info("Cleaning up video system...\n");

  // Stop RTSP servers first to prevent new connections
  if (g_video_rtsp_multistream_server) {
    platform_log_info("Stopping multi-stream RTSP server...\n");
    rtsp_multistream_server_stop(g_video_rtsp_multistream_server);
    rtsp_multistream_server_destroy(g_video_rtsp_multistream_server);
    g_video_rtsp_multistream_server = NULL;
    g_video_rtsp_server_initialized = false;
    platform_log_info("Multi-stream RTSP server stopped and cleaned up\n");
  }

  // Close video input
  if (g_video_vi_handle) {
    platform_log_info("Closing video input...\n");
    platform_vi_close(g_video_vi_handle);
    g_video_vi_handle = NULL;
  }

  g_video_system_initialized = false;
  cleanup_done = true;
  platform_log_info("Video system cleanup completed\n");
}

bool video_lifecycle_initialized(void) { return g_video_system_initialized; }

platform_vi_handle_t video_lifecycle_get_vi_handle(void) {
  return g_video_vi_handle;
}

rtsp_multistream_server_t *video_lifecycle_get_rtsp_server(void) {
  return g_video_rtsp_multistream_server;
}

void video_lifecycle_stop_servers(void) {
  if (g_video_rtsp_multistream_server) {
    platform_log_info("Stopping RTSP server...\n");
    rtsp_multistream_server_stop(g_video_rtsp_multistream_server);
  }
}
