/**
 * @file stream_config_utils.c
 * @brief Utility functions for video stream configuration management
 * @author kkrzysztofik
 * @date 2025
 */

#include "stream_config_utils.h"

#include "platform/platform.h"
#include "utils/error/error_handling.h"

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

// Default configuration values
#define DEFAULT_MAIN_FPS      25
#define DEFAULT_MAIN_BITRATE  2048
#define DEFAULT_MAIN_QUALITY  50
#define DEFAULT_MAIN_WIDTH    1280
#define DEFAULT_MAIN_HEIGHT   720
#define DEFAULT_MAIN_GOP_SIZE 50

#define DEFAULT_SUB_FPS      15
#define DEFAULT_SUB_BITRATE  512
#define DEFAULT_SUB_QUALITY  50
#define DEFAULT_SUB_WIDTH    640
#define DEFAULT_SUB_HEIGHT   360
#define DEFAULT_SUB_GOP_SIZE 30

// Validation ranges
#define MIN_FPS          1
#define MAX_FPS          60
#define MIN_BITRATE      100
#define MAX_MAIN_BITRATE 10000
#define MAX_SUB_BITRATE  5000
#define MIN_QUALITY      1
#define MAX_QUALITY      100
#define MIN_WIDTH        320
#define MAX_WIDTH        1920
#define MIN_HEIGHT       180
#define MAX_HEIGHT       1080
#define MIN_GOP_SIZE     1
#define MAX_GOP_SIZE     300

/**
 * @brief Initialize video stream configuration with default values
 * @param stream_config Stream configuration structure to initialize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Sets appropriate default values based on stream type
 */
int stream_config_init_defaults(video_config_t* stream_config, bool is_main_stream) {
  if (!stream_config) {
    platform_log_error("error: stream_config_init_defaults() called with NULL "
                       "stream_config\n");
    return ONVIF_ERROR_INVALID;
  }

  memset(stream_config, 0, sizeof(video_config_t));

  if (is_main_stream) {
    stream_config->fps = DEFAULT_MAIN_FPS;
    stream_config->bitrate = DEFAULT_MAIN_BITRATE;
    stream_config->width = DEFAULT_MAIN_WIDTH;
    stream_config->height = DEFAULT_MAIN_HEIGHT;
    stream_config->gop_size = DEFAULT_MAIN_GOP_SIZE;
  } else {
    stream_config->fps = DEFAULT_SUB_FPS;
    stream_config->bitrate = DEFAULT_SUB_BITRATE;
    stream_config->width = DEFAULT_SUB_WIDTH;
    stream_config->height = DEFAULT_SUB_HEIGHT;
    stream_config->gop_size = DEFAULT_SUB_GOP_SIZE;
  }

  // Common settings for both streams
  stream_config->profile = PLATFORM_PROFILE_MAIN;
  stream_config->codec_type = PLATFORM_H264_ENC_TYPE;
  stream_config->br_mode = PLATFORM_BR_MODE_CBR;

  return ONVIF_SUCCESS;
}

/**
 * @brief Initialize video stream configuration from anyka_cfg.ini parameters
 * @param stream_config Stream configuration structure to initialize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param bitrate_kbps Bitrate in kbps from config file
 * @param fps Frames per second from config file
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Converts anyka_cfg.ini parameters to internal stream configuration
 */
// NOLINTNEXTLINE(bugprone-easily-swappable-parameters)
int stream_config_init_from_anyka(video_config_t* stream_config, bool is_main_stream,
                                  unsigned int bitrate_kbps, int fps) {
  if (!stream_config) {
    platform_log_error("error: stream_config_init_from_anyka() called with NULL "
                       "stream_config\n");
    return ONVIF_ERROR_INVALID;
  }

  // Initialize with defaults first
  int result = stream_config_init_defaults(stream_config, is_main_stream);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Apply anyka_cfg.ini parameters
  if (bitrate_kbps > 0) {
    stream_config->bitrate = (int)bitrate_kbps;
  }
  if (fps > 0) {
    stream_config->fps = fps;
    // Calculate GOP size based on FPS: 2 seconds worth of frames
    stream_config->gop_size = (fps > 0) ? fps * 2 : DEFAULT_MAIN_GOP_SIZE;
  }
  // Note: quality parameter is not used in RTSP video_config_t structure

  // Set appropriate dimensions based on stream type
  if (is_main_stream) {
    stream_config->width = DEFAULT_MAIN_WIDTH;
    stream_config->height = DEFAULT_MAIN_HEIGHT;
  } else {
    stream_config->width = DEFAULT_SUB_WIDTH;
    stream_config->height = DEFAULT_SUB_HEIGHT;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate video stream configuration parameters
 * @param stream_config Stream configuration to validate
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS if valid, ONVIF_ERROR_INVALID if invalid
 * @note Validates ranges and constraints based on stream type
 */
int stream_config_validate(const video_config_t* stream_config, bool is_main_stream) {
  if (!stream_config) {
    platform_log_error("error: stream_config_validate() called with NULL stream_config\n");
    return ONVIF_ERROR_INVALID;
  }

  // Validate FPS
  if (stream_config->fps < MIN_FPS || stream_config->fps > MAX_FPS) {
    platform_log_error("error: invalid FPS %d, must be between %d and %d\n", stream_config->fps,
                       MIN_FPS, MAX_FPS);
    return ONVIF_ERROR_INVALID;
  }

  // Validate bitrate based on stream type
  int max_bitrate = is_main_stream ? MAX_MAIN_BITRATE : MAX_SUB_BITRATE;
  if (stream_config->bitrate < MIN_BITRATE || stream_config->bitrate > max_bitrate) {
    platform_log_error("error: invalid bitrate %d, must be between %d and %d for %s stream\n",
                       stream_config->bitrate, MIN_BITRATE, max_bitrate,
                       is_main_stream ? "main" : "sub");
    return ONVIF_ERROR_INVALID;
  }

  // Note: quality validation removed as it's not part of video_config_t
  // structure

  // Validate dimensions
  if (stream_config->width < MIN_WIDTH || stream_config->width > MAX_WIDTH) {
    platform_log_error("error: invalid width %d, must be between %d and %d\n", stream_config->width,
                       MIN_WIDTH, MAX_WIDTH);
    return ONVIF_ERROR_INVALID;
  }

  if (stream_config->height < MIN_HEIGHT || stream_config->height > MAX_HEIGHT) {
    platform_log_error("error: invalid height %d, must be between %d and %d\n",
                       stream_config->height, MIN_HEIGHT, MAX_HEIGHT);
    return ONVIF_ERROR_INVALID;
  }

  // Validate GOP size
  if (stream_config->gop_size < MIN_GOP_SIZE || stream_config->gop_size > MAX_GOP_SIZE) {
    platform_log_error("error: invalid GOP size %d, must be between %d and %d\n",
                       stream_config->gop_size, MIN_GOP_SIZE, MAX_GOP_SIZE);
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Convert stream configuration to platform video config
 * @param stream_config Stream configuration to convert
 * @param platform_config Platform video configuration structure to fill
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Converts internal stream config to platform-specific video config
 */
int stream_config_to_platform(const video_config_t* stream_config,
                              platform_video_config_t* platform_config, bool is_main_stream) {
  if (!stream_config || !platform_config) {
    platform_log_error("error: stream_config_to_platform() called with NULL parameter\n");
    return ONVIF_ERROR_INVALID;
  }

  // Validate input configuration first
  int result = stream_config_validate(stream_config, is_main_stream);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Convert to platform video configuration
  platform_config->width = stream_config->width;
  platform_config->height = stream_config->height;
  platform_config->fps = stream_config->fps;
  platform_config->bitrate = stream_config->bitrate;
  platform_config->codec = (platform_video_codec_t)stream_config->codec_type;

  return ONVIF_SUCCESS;
}

/**
 * @brief Get stream configuration summary for logging
 * @param stream_config Stream configuration to summarize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param summary Buffer to store summary string
 * @param summary_size Size of summary buffer
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Generates human-readable summary of stream configuration
 */
int stream_config_get_summary(const video_config_t* stream_config, bool is_main_stream,
                              char* summary, size_t summary_size) {
  if (!stream_config || !summary || summary_size == 0) {
    platform_log_error("error: stream_config_get_summary() called with NULL parameter\n");
    return ONVIF_ERROR_INVALID;
  }

  const char* stream_type = is_main_stream ? "main" : "sub";
  const char* stream_path = is_main_stream ? "/vs0" : "/vs1";

  int written = snprintf(summary, summary_size, "%s stream (%s): %dx%d@%dfps, %dkbps, GOP=%d",
                         stream_type, stream_path, stream_config->width, stream_config->height,
                         stream_config->fps, stream_config->bitrate, stream_config->gop_size);

  if (written < 0 || (size_t)written >= summary_size) {
    platform_log_error("error: insufficient buffer size for stream config summary\n");
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Apply stream configuration to RTSP stream config
 * @param stream_config Stream configuration to apply
 * @param rtsp_config RTSP stream configuration to modify
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param vi_handle Video input handle
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Applies stream configuration to RTSP stream configuration structure
 */
int stream_config_apply_to_rtsp(const video_config_t* stream_config, void* rtsp_config,
                                bool is_main_stream, platform_vi_handle_t vi_handle) {
  if (!stream_config || !rtsp_config) {
    platform_log_error("error: stream_config_apply_to_rtsp() called with NULL parameter\n");
    return ONVIF_ERROR_INVALID;
  }

  // Validate input configuration
  int result = stream_config_validate(stream_config, is_main_stream);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Cast to RTSP stream config (assuming it matches the structure from
  // rtsp_server.h) This is a simplified implementation - in practice, you'd
  // need to match the exact structure For now, we'll assume the RTSP config has
  // the same fields
  video_config_t* rtsp_video_config = (video_config_t*)rtsp_config;

  // Copy configuration
  result = stream_config_copy(rtsp_video_config, stream_config);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Clean up stream configuration resources
 * @param stream_config Stream configuration to clean up
 * @note Frees any allocated resources in stream configuration
 */
void stream_config_cleanup(video_config_t* stream_config) {
  if (!stream_config) {
    return;
  }

  // Currently no dynamic resources to free
  // This function is provided for future extensibility
  memset(stream_config, 0, sizeof(video_config_t));
}

/**
 * @brief Copy stream configuration
 * @param dest Destination stream configuration
 * @param src Source stream configuration
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Performs deep copy of stream configuration
 */
int stream_config_copy(video_config_t* dest, const video_config_t* src) {
  if (!dest || !src) {
    platform_log_error("error: stream_config_copy() called with NULL parameter\n");
    return ONVIF_ERROR_INVALID;
  }

  memcpy(dest, src, sizeof(video_config_t));
  return ONVIF_SUCCESS;
}

/**
 * @brief Compare two stream configurations
 * @param config1 First stream configuration
 * @param config2 Second stream configuration
 * @return true if configurations are equal, false otherwise
 * @note Performs field-by-field comparison of stream configurations
 */
bool stream_config_equals(const video_config_t* config1, const video_config_t* config2) {
  if (!config1 || !config2) {
    return false;
  }

  return (config1->fps == config2->fps && config1->bitrate == config2->bitrate &&
          config1->width == config2->width && config1->height == config2->height &&
          config1->gop_size == config2->gop_size && config1->profile == config2->profile &&
          config1->codec_type == config2->codec_type && config1->br_mode == config2->br_mode);
}
