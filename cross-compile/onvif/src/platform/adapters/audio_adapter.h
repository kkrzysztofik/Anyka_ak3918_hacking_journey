/**
 * @file audio_adapter.h
 * @brief Audio input/output abstraction adapter for Anyka platform
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides the interface for audio input/output operations
 * on the Anyka AK3918 platform.
 */

#ifndef AUDIO_ADAPTER_H
#define AUDIO_ADAPTER_H

#include <bits/types.h>
#include <stdint.h>

#include "platform/platform_common.h"


#ifdef __cplusplus
extern "C" {
#endif

/* Audio input operations */
platform_result_t audio_adapter_ai_open(platform_ai_handle_t *handle);
void audio_adapter_ai_close(platform_ai_handle_t handle);

/* Audio encoding operations */
platform_result_t audio_adapter_aenc_init(
    platform_aenc_stream_handle_t *handle,
    const platform_audio_config_t *config);
void audio_adapter_aenc_cleanup(platform_aenc_stream_handle_t handle);
platform_result_t audio_adapter_aenc_get_frame(
    platform_aenc_stream_handle_t handle, uint8_t **data, size_t *size,
    uint64_t *timestamp);
void audio_adapter_aenc_release_frame(platform_aenc_stream_handle_t handle,
                                      uint8_t *data);
platform_result_t audio_adapter_aenc_get_stream(
    platform_aenc_stream_handle_t handle, uint8_t **data, size_t *size,
    uint64_t *timestamp);
void audio_adapter_aenc_release_stream(platform_aenc_stream_handle_t handle,
                                       uint8_t *data);

#ifdef __cplusplus
}
#endif

#endif /* AUDIO_ADAPTER_H */
