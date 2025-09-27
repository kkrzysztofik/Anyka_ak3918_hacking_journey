/**
 * @file video_lifecycle.h
 * @brief Video input and RTSP server lifecycle management
 *
 * This module provides centralized management for video input initialization,
 * RTSP server creation, and video stream configuration. It handles the complete
 * video pipeline from sensor detection to stream delivery.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_VIDEO_LIFECYCLE_H
#define ONVIF_VIDEO_LIFECYCLE_H

#include "core/config/config.h"
#include "networking/rtsp/rtsp_multistream.h"

#include <stdbool.h>

/**
 * @brief Initialize video input and RTSP streaming system
 * @param cfg Application configuration containing stream parameters
 * @return 0 on success, -1 on failure
 * @note This function is non-fatal - video streaming will be disabled if it
 * fails
 */
int video_lifecycle_init(const struct application_config* cfg);

/**
 * @brief Cleanup video input and RTSP streaming system
 * @note This function is idempotent and safe to call multiple times
 */
void video_lifecycle_cleanup(void);

/**
 * @brief Check if video system is initialized
 * @return true if video system is ready, false otherwise
 */
bool video_lifecycle_initialized(void);

/**
 * @brief Get the video input handle
 * @return Video input handle or NULL if not initialized
 */
platform_vi_handle_t video_lifecycle_get_vi_handle(void);

/**
 * @brief Get the RTSP multistream server
 * @return RTSP server instance or NULL if not initialized
 */
rtsp_multistream_server_t* video_lifecycle_get_rtsp_server(void);

/**
 * @brief Stop RTSP servers (for graceful shutdown)
 */
void video_lifecycle_stop_servers(void);

#endif /* ONVIF_VIDEO_LIFECYCLE_H */
