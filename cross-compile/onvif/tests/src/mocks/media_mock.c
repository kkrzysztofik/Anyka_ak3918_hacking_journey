/**
 * @file media_mock.c
 * @brief CMocka mock implementations for ONVIF media service
 * @author kkrzysztofik
 * @date 2025
 */

#include "media_mock.h"

#include <string.h>

#include "cmocka_wrapper.h"
#include "utils/error/error_handling.h"

/* Static test data for media mocks */
static struct media_profile g_mock_profiles[2] = {{.token = "MainProfile", .name = "Main Profile", .fixed = 1},
                                                  {.token = "SubProfile", .name = "Sub Profile", .fixed = 0}};

static struct video_source g_mock_video_sources[1] = {{.token = "VideoSource0", .framerate = 25.0F, .resolution = {.width = 1280, .height = 720}}};

static struct audio_source g_mock_audio_sources[1] = {{.token = "AudioSource0", .channels = 1}};

static struct video_encoder_configuration g_mock_video_encoder_configs[2] = {{.token = "VideoEncoderConfig0",
                                                                              .name = "Main Video Encoder",
                                                                              .use_count = 1,
                                                                              .encoding = "H264",
                                                                              .resolution = {.width = 1280, .height = 720},
                                                                              .quality = 25.0F,
                                                                              .framerate_limit = 25,
                                                                              .bitrate_limit = 2048,
                                                                              .gov_length = 50},
                                                                             {.token = "VideoEncoderConfig1",
                                                                              .name = "Sub Video Encoder",
                                                                              .use_count = 1,
                                                                              .encoding = "H264",
                                                                              .resolution = {.width = 640, .height = 360},
                                                                              .quality = 50.0F,
                                                                              .framerate_limit = 25,
                                                                              .bitrate_limit = 800,
                                                                              .gov_length = 50}};

static struct audio_encoder_configuration g_mock_audio_encoder_configs[3] = {
  {.token = "AudioEncoderConfig0", .name = "Audio Encoder G711", .use_count = 1, .encoding = "G711", .bitrate = 64, .sample_rate = 8000},
  {.token = "AudioEncoderConfig1", .name = "Audio Encoder AAC", .use_count = 1, .encoding = "AAC", .bitrate = 128, .sample_rate = 16000},
  {.token = "AudioEncoderConfig2", .name = "Audio Encoder PCM", .use_count = 0, .encoding = "PCM", .bitrate = 128, .sample_rate = 8000}};

static struct metadata_configuration g_mock_metadata_configs[1] = {
  {.token = "MetadataConfig0", .name = "Default Metadata", .use_count = 1, .analytics = 1}};

/**
 * @brief Mock implementation of onvif_media_get_profiles
 */
int __wrap_onvif_media_get_profiles(struct media_profile** profile_list, int* count) {
  if (profile_list == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *profile_list = g_mock_profiles;
    *count = 2;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_profile
 */
int __wrap_onvif_media_get_profile(const char* profile_token, struct media_profile* profile) {
  if (profile_token == NULL || profile == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    if (strcmp(profile_token, "MainProfile") == 0) {
      memcpy(profile, &g_mock_profiles[0], sizeof(struct media_profile));
    } else if (strcmp(profile_token, "SubProfile") == 0) {
      memcpy(profile, &g_mock_profiles[1], sizeof(struct media_profile));
    }
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_create_profile
 */
int __wrap_onvif_media_create_profile(const char* name, const char* token, struct media_profile* profile) {
  if (name == NULL || token == NULL || profile == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(name);
  check_expected_ptr(token);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    strncpy(profile->name, name, MEDIA_NAME_SIZE - 1);
    profile->name[MEDIA_NAME_SIZE - 1] = '\0';
    strncpy(profile->token, token, MEDIA_TOKEN_SIZE - 1);
    profile->token[MEDIA_TOKEN_SIZE - 1] = '\0';
    profile->fixed = 0;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_delete_profile
 */
int __wrap_onvif_media_delete_profile(const char* profile_token) {
  if (profile_token == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);

  return (int)mock();
}

/**
 * @brief Mock implementation of onvif_media_get_video_sources
 */
int __wrap_onvif_media_get_video_sources(struct video_source** sources, int* count) {
  if (sources == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *sources = g_mock_video_sources;
    *count = 1;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_audio_sources
 */
int __wrap_onvif_media_get_audio_sources(struct audio_source** sources, int* count) {
  if (sources == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *sources = g_mock_audio_sources;
    *count = 1;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_video_encoder_configurations
 */
int __wrap_onvif_media_get_video_encoder_configurations(struct video_encoder_configuration** configs, int* count) {
  if (configs == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *configs = g_mock_video_encoder_configs;
    *count = 2;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_audio_encoder_configurations
 */
int __wrap_onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration** configs, int* count) {
  if (configs == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *configs = g_mock_audio_encoder_configs;
    *count = 3;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_stream_uri
 */
int __wrap_onvif_media_get_stream_uri(const char* profile_token, const char* protocol, struct stream_uri* uri) {
  if (profile_token == NULL || protocol == NULL || uri == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);
  check_expected_ptr(protocol);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    strncpy(uri->uri, "rtsp://192.168.1.10:554/main", MEDIA_URI_BUFFER_SIZE - 1);
    uri->uri[MEDIA_URI_BUFFER_SIZE - 1] = '\0';
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_get_snapshot_uri
 */
int __wrap_onvif_media_get_snapshot_uri(const char* profile_token, struct stream_uri* uri) {
  if (profile_token == NULL || uri == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    strncpy(uri->uri, "http://192.168.1.10:80/snapshot", MEDIA_URI_BUFFER_SIZE - 1);
    uri->uri[MEDIA_URI_BUFFER_SIZE - 1] = '\0';
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_start_multicast_streaming
 */
int __wrap_onvif_media_start_multicast_streaming(const char* profile_token) {
  if (profile_token == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);

  return (int)mock();
}

/**
 * @brief Mock implementation of onvif_media_stop_multicast_streaming
 */
int __wrap_onvif_media_stop_multicast_streaming(const char* profile_token) {
  if (profile_token == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(profile_token);

  return (int)mock();
}

/**
 * @brief Mock implementation of onvif_media_get_metadata_configurations
 */
int __wrap_onvif_media_get_metadata_configurations(struct metadata_configuration** configs, int* count) {
  if (configs == NULL || count == NULL) {
    return ONVIF_ERROR_NULL;
  }

  int result = (int)mock();
  if (result == ONVIF_SUCCESS) {
    *configs = g_mock_metadata_configs;
    *count = 1;
  }

  return result;
}

/**
 * @brief Mock implementation of onvif_media_set_metadata_configuration
 */
int __wrap_onvif_media_set_metadata_configuration(const char* configuration_token, const struct metadata_configuration* config) {
  if (configuration_token == NULL || config == NULL) {
    return ONVIF_ERROR_NULL;
  }

  check_expected_ptr(configuration_token);

  return (int)mock();
}

/**
 * @brief Mock implementation of onvif_media_init
 */
int __wrap_onvif_media_init(void) {
  function_called();
  return (int)mock();
}
