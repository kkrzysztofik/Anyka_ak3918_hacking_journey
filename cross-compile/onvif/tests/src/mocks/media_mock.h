/**
 * @file media_mock.h
 * @brief CMocka mock functions for ONVIF media service
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MEDIA_MOCK_H
#define MEDIA_MOCK_H

#include "services/media/onvif_media.h"

/**
 * @brief Mock implementation of onvif_media_get_profiles
 */
int __wrap_onvif_media_get_profiles(struct media_profile** profile_list, int* count);

/**
 * @brief Mock implementation of onvif_media_get_profile
 */
int __wrap_onvif_media_get_profile(const char* profile_token, struct media_profile* profile);

/**
 * @brief Mock implementation of onvif_media_create_profile
 */
int __wrap_onvif_media_create_profile(const char* name, const char* token, struct media_profile* profile);

/**
 * @brief Mock implementation of onvif_media_delete_profile
 */
int __wrap_onvif_media_delete_profile(const char* profile_token);

/**
 * @brief Mock implementation of onvif_media_get_video_sources
 */
int __wrap_onvif_media_get_video_sources(struct video_source** sources, int* count);

/**
 * @brief Mock implementation of onvif_media_get_audio_sources
 */
int __wrap_onvif_media_get_audio_sources(struct audio_source** sources, int* count);

/**
 * @brief Mock implementation of onvif_media_get_video_encoder_configurations
 */
int __wrap_onvif_media_get_video_encoder_configurations(struct video_encoder_configuration** configs, int* count);

/**
 * @brief Mock implementation of onvif_media_get_audio_encoder_configurations
 */
int __wrap_onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration** configs, int* count);

/**
 * @brief Mock implementation of onvif_media_get_stream_uri
 */
int __wrap_onvif_media_get_stream_uri(const char* profile_token, const char* protocol, struct stream_uri* uri);

/**
 * @brief Mock implementation of onvif_media_get_snapshot_uri
 */
int __wrap_onvif_media_get_snapshot_uri(const char* profile_token, struct stream_uri* uri);

/**
 * @brief Mock implementation of onvif_media_start_multicast_streaming
 */
int __wrap_onvif_media_start_multicast_streaming(const char* profile_token);

/**
 * @brief Mock implementation of onvif_media_stop_multicast_streaming
 */
int __wrap_onvif_media_stop_multicast_streaming(const char* profile_token);

/**
 * @brief Mock implementation of onvif_media_get_metadata_configurations
 */
int __wrap_onvif_media_get_metadata_configurations(struct metadata_configuration** configs, int* count);

/**
 * @brief Mock implementation of onvif_media_set_metadata_configuration
 */
int __wrap_onvif_media_set_metadata_configuration(const char* configuration_token, const struct metadata_configuration* config);

/**
 * @brief Mock implementation of onvif_media_init
 */
int __wrap_onvif_media_init(void);

#endif // MEDIA_MOCK_H
