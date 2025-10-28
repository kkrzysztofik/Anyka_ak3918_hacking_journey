/**
 * @file video_adapter.h
 * @brief Video input/output abstraction adapter for Anyka platform
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides the interface for video input/output operations
 * on the Anyka AK3918 platform.
 */

#ifndef VIDEO_ADAPTER_H
#define VIDEO_ADAPTER_H

#include <bits/types.h>
#include <stdbool.h>
#include <stdint.h>

#include "platform/platform_common.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Video input operations */
platform_result_t video_adapter_vi_open(platform_vi_handle_t* handle);
void video_adapter_vi_close(platform_vi_handle_t handle);
platform_result_t video_adapter_vi_get_sensor_resolution(platform_vi_handle_t handle, int* width, int* height);
platform_result_t video_adapter_vi_switch_day_night(platform_vi_handle_t handle, bool day_mode);
platform_result_t video_adapter_vi_set_flip_mirror(platform_vi_handle_t handle, bool flip, bool mirror);
platform_result_t video_adapter_vpss_effect_set(platform_vi_handle_t handle, platform_vpss_effect_t effect);
platform_result_t video_adapter_vpss_effect_get(platform_vi_handle_t handle, platform_vpss_effect_t* effect);

/* Video encoding operations */
platform_result_t video_adapter_venc_init(platform_venc_handle_t* handle, const platform_video_config_t* config);
void video_adapter_venc_cleanup(platform_venc_handle_t handle);
platform_result_t video_adapter_venc_get_frame(platform_venc_handle_t handle, uint8_t** data, size_t* size, uint64_t* timestamp);
void video_adapter_venc_release_frame(platform_venc_handle_t handle, uint8_t* data);
platform_result_t video_adapter_venc_get_stream(platform_venc_handle_t handle, uint8_t** data, size_t* size, uint64_t* timestamp);
void video_adapter_venc_release_stream(platform_venc_handle_t handle, uint8_t* data);

#ifdef __cplusplus
}
#endif

#endif /* VIDEO_ADAPTER_H */
