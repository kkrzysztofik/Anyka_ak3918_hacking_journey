/**
 * @file stream_config_utils.h
 * @brief Utility functions for video stream configuration management
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef STREAM_CONFIG_UTILS_H
#define STREAM_CONFIG_UTILS_H

#include <stdbool.h>
#include <stddef.h>

#include "platform/platform_common.h"
#include "services/common/video_config_types.h"

/* Forward declarations */
struct application_config;

/**
 * @brief Initialize video stream configuration with default values
 * @param stream_config Stream configuration structure to initialize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Sets appropriate default values based on stream type
 */
int stream_config_init_defaults(video_config_t* stream_config, bool is_main_stream);

/**
 * @brief Initialize video stream configuration from anyka_cfg.ini parameters
 * @param stream_config Stream configuration structure to initialize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param bitrate_kbps Bitrate in kbps from config file
 * @param fps Frames per second from config file
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Converts anyka_cfg.ini parameters to internal stream configuration
 */
int stream_config_init_from_anyka(video_config_t* stream_config, bool is_main_stream, unsigned int bitrate_kbps, int fps);

/**
 * @brief Validate video stream configuration parameters
 * @param stream_config Stream configuration to validate
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS if valid, ONVIF_ERROR_INVALID if invalid
 * @note Validates ranges and constraints based on stream type
 */
int stream_config_validate(const video_config_t* stream_config, bool is_main_stream);

/**
 * @brief Convert stream configuration to platform video config
 * @param stream_config Stream configuration to convert
 * @param platform_config Platform video configuration structure to fill
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Converts internal stream config to platform-specific video config
 */
int stream_config_to_platform(const video_config_t* stream_config, platform_video_config_t* platform_config, bool is_main_stream);

/**
 * @brief Get stream configuration summary for logging
 * @param stream_config Stream configuration to summarize
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param summary Buffer to store summary string
 * @param summary_size Size of summary buffer
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Generates human-readable summary of stream configuration
 */
int stream_config_get_summary(const video_config_t* stream_config, bool is_main_stream, char* summary, size_t summary_size);

/**
 * @brief Apply stream configuration to RTSP stream config
 * @param stream_config Stream configuration to apply
 * @param rtsp_config RTSP stream configuration to modify
 * @param is_main_stream True for main stream (vs0), false for sub stream (vs1)
 * @param vi_handle Video input handle
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Applies stream configuration to RTSP stream configuration structure
 */
int stream_config_apply_to_rtsp(const video_config_t* stream_config, void* rtsp_config, bool is_main_stream, platform_vi_handle_t vi_handle);

/**
 * @brief Clean up stream configuration resources
 * @param stream_config Stream configuration to clean up
 * @note Frees any allocated resources in stream configuration
 */
void stream_config_cleanup(video_config_t* stream_config);

/**
 * @brief Copy stream configuration
 * @param dest Destination stream configuration
 * @param src Source stream configuration
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_INVALID on failure
 * @note Performs deep copy of stream configuration
 */
int stream_config_copy(video_config_t* dest, const video_config_t* src);

/**
 * @brief Compare two stream configurations
 * @param config1 First stream configuration
 * @param config2 Second stream configuration
 * @return true if configurations are equal, false otherwise
 * @note Performs field-by-field comparison of stream configurations
 */
bool stream_config_equals(const video_config_t* config1, const video_config_t* config2);

#endif /* STREAM_CONFIG_UTILS_H */
