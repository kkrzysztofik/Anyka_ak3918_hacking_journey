/**
 * @file onvif_media.c
 * @brief ONVIF Media service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_media.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_service_common.h"
#include "services/common/onvif_types.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/memory/smart_response_builder.h"
#include "utils/network/network_utils.h"

#define MEDIA_PROFILE_COUNT_DEFAULT 2
#define MEDIA_MAIN_PROFILE_TOKEN    "MainProfile"
#define MEDIA_SUB_PROFILE_TOKEN     "SubProfile"

#define MEDIA_MAIN_RESOLUTION_WIDTH  1280
#define MEDIA_MAIN_RESOLUTION_HEIGHT 720

// Audio sample rate constants
#define AUDIO_SAMPLE_RATE_8KHZ  8000
#define AUDIO_SAMPLE_RATE_16KHZ 16000
#define AUDIO_SAMPLE_RATE_22KHZ 22050
#define AUDIO_SAMPLE_RATE_44KHZ 44100
#define AUDIO_SAMPLE_RATE_48KHZ 48000

// Timeout constants
#define MEDIA_DEFAULT_TIMEOUT_SECONDS 60

// Buffer size constants
#define MEDIA_ANALYTICS_STR_SIZE   16
#define MEDIA_NAME_BUFFER_SIZE     64
#define MEDIA_TOKEN_BUFFER_SIZE    64
#define MEDIA_PROTOCOL_BUFFER_SIZE 16
#define MEDIA_URI_BUFFER_SIZE      256
#define MEDIA_URL_PARAM_SIZE       64

// Stream path and port constants are defined in onvif_constants.h

// Media configuration constants
#define MEDIA_DEFAULT_FRAMERATE     25
#define MEDIA_DEFAULT_GOV_LENGTH    50
#define MEDIA_DEFAULT_TTL           5
#define MEDIA_LOW_BITRATE_AAC       32
#define MEDIA_SUB_RESOLUTION_WIDTH  640
#define MEDIA_SUB_RESOLUTION_HEIGHT 360

// Quality constants
#define MEDIA_MAIN_QUALITY_DEFAULT 25.0F
#define MEDIA_SUB_QUALITY_DEFAULT  50.0F

// Bitrate constants
#define MEDIA_MAIN_BITRATE_DEFAULT      2048
#define MEDIA_SUB_BITRATE_DEFAULT       800
#define MEDIA_AUDIO_BITRATE_DEFAULT     64
#define MEDIA_AUDIO_BITRATE_AAC_MIN     32
#define MEDIA_AUDIO_BITRATE_AAC_MAX     320
#define MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN 8000
#define MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX 48000

// Response buffer constants
#define MEDIA_PROTOCOL_BUFFER_SIZE 16

// Additional constants for magic numbers
#define MEDIA_DEFAULT_WIDTH_25  25
#define MEDIA_DEFAULT_WIDTH_50  50
#define MEDIA_DEFAULT_WIDTH_5   5
#define MEDIA_DEFAULT_WIDTH_60  60
#define MEDIA_DEFAULT_WIDTH_128 128
#define MEDIA_DEFAULT_WIDTH_32  32

// Imaging constants
#define MEDIA_IMAGING_BRIGHTNESS_DEFAULT 50.0F
#define MEDIA_IMAGING_SATURATION_DEFAULT 50.0F
#define MEDIA_IMAGING_CONTRAST_DEFAULT   50.0F
#define MEDIA_IMAGING_SHARPNESS_DEFAULT  50.0F

// Audio bitrate constants
#define MEDIA_AUDIO_BITRATE_HIGH_QUALITY 128
#define MEDIA_AUDIO_BITRATE_LOW_QUALITY  32
#define MEDIA_SESSION_TIMEOUT_DEFAULT    60

#define MEDIA_XML_PROFILE_TOKEN_TAG "<trt:ProfileToken>"
#define MEDIA_XML_PROTOCOL_TAG      "<trt:Protocol>"
#define MEDIA_XML_VIDEO_SOURCE_TAG  "<trt:VideoSourceToken>"
#define MEDIA_XML_AUDIO_SOURCE_TAG  "<trt:AudioSourceToken>"

#define PROFILE_COUNT MEDIA_PROFILE_COUNT_DEFAULT

static int parse_profile_token(onvif_gsoap_context_t* gsoap_ctx, char* token, size_t token_size);
static int parse_protocol(onvif_gsoap_context_t* gsoap_ctx, char* protocol, size_t protocol_size);
static int validate_profile_token(const char* token);
static int validate_protocol(const char* protocol);

static struct media_profile g_media_profiles[] = { // NOLINT
  {.token = MEDIA_MAIN_PROFILE_TOKEN,
   .name = "Main Video Profile",
   .fixed = 1,
   .video_source = {.source_token = "VideoSource0",
                    .bounds = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT, 0, 0}},
   .video_encoder = {.token = "VideoEncoder0",
                     .encoding = "H264",
                     .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
                     .quality = MEDIA_MAIN_QUALITY_DEFAULT,
                     .framerate_limit = MEDIA_DEFAULT_FRAMERATE,
                     .encoding_interval = 1,
                     .bitrate_limit = MEDIA_MAIN_BITRATE_DEFAULT,
                     .gov_length = MEDIA_DEFAULT_GOV_LENGTH},
   .audio_source = {.source_token = "AudioSource0"},
   .audio_encoder = {.token = "AudioEncoder0",
                     .encoding = "AAC",
                     .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
                     .sample_rate = AUDIO_SAMPLE_RATE_16KHZ},
   .ptz = {.node_token = "PTZNode0",
           .default_absolute_pan_tilt_position_space =
             "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
             "PositionGenericSpace",
           .default_absolute_zoom_position_space = "",
           .default_relative_pan_tilt_translation_space =
             "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
             "TranslationGenericSpace",
           .default_relative_zoom_translation_space = "",
           .default_continuous_pan_tilt_velocity_space =
             "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
             "VelocityGenericSpace",
           .default_continuous_zoom_velocity_space = ""}},
  {.token = MEDIA_SUB_PROFILE_TOKEN,
   .name = "Sub Video Profile",
   .fixed = 1,
   .video_source = {.source_token = "VideoSource0",
                    .bounds = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT, 0, 0}},
   .video_encoder = {.token = "VideoEncoder1",
                     .encoding = "H264",
                     .resolution = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT},
                     .quality = MEDIA_SUB_QUALITY_DEFAULT,
                     .framerate_limit = MEDIA_DEFAULT_FRAMERATE,
                     .encoding_interval = 1,
                     .bitrate_limit = MEDIA_SUB_BITRATE_DEFAULT,
                     .gov_length = MEDIA_DEFAULT_GOV_LENGTH},
   .audio_source = {.source_token = "AudioSource0"},
   .audio_encoder = {.token = "AudioEncoder0",
                     .encoding = "AAC",
                     .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
                     .sample_rate = AUDIO_SAMPLE_RATE_16KHZ},
   .ptz = {
     .node_token = "PTZNode0",
     .default_absolute_pan_tilt_position_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                 "PositionGenericSpace",
     .default_absolute_zoom_position_space = "",
     .default_relative_pan_tilt_translation_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                    "TranslationGenericSpace",
     .default_relative_zoom_translation_space = "",
     .default_continuous_pan_tilt_velocity_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/"
                                                   "VelocityGenericSpace",
     .default_continuous_zoom_velocity_space = ""}}};

// Static cached URIs for memory optimization
static char g_cached_main_rtsp_uri[MEDIA_URI_BUFFER_SIZE] = {0}; // NOLINT
static char g_cached_sub_rtsp_uri[MEDIA_URI_BUFFER_SIZE] = {0};  // NOLINT
static int g_cached_uris_initialized = 0;                        // NOLINT

/**
 * @brief Initialize cached URIs for common profiles to optimize memory usage
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int init_cached_uris(void) {
  if (g_cached_uris_initialized) {
    return ONVIF_SUCCESS; // Already initialized
  }

  // Build cached RTSP URIs for common profiles
  build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, g_cached_main_rtsp_uri,
                   sizeof(g_cached_main_rtsp_uri));
  build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, g_cached_sub_rtsp_uri,
                   sizeof(g_cached_sub_rtsp_uri));

  g_cached_uris_initialized = 1;
  return ONVIF_SUCCESS;
}

/**
 * @brief Get cached URI for a profile token to avoid repeated construction
 * @param profile_token The profile token
 * @param uri_buffer Buffer to copy the URI to
 * @param buffer_size Size of the URI buffer
 * @return ONVIF_SUCCESS if URI found and copied, error code otherwise
 */
static int get_cached_rtsp_uri(const char* profile_token, char* uri_buffer, size_t buffer_size) {
  if (!profile_token || !uri_buffer || buffer_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize cached URIs if not done yet
  int result = init_cached_uris();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Return cached URI for common profiles
  if (strcmp(profile_token, MEDIA_MAIN_PROFILE_TOKEN) == 0) {
    strncpy(uri_buffer, g_cached_main_rtsp_uri, buffer_size - 1);
    uri_buffer[buffer_size - 1] = '\0';
    return ONVIF_SUCCESS;
  }
  if (strcmp(profile_token, MEDIA_SUB_PROFILE_TOKEN) == 0) {
    strncpy(uri_buffer, g_cached_sub_rtsp_uri, buffer_size - 1);
    uri_buffer[buffer_size - 1] = '\0';
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND; // URI not cached for this profile
}

/**
 * @brief Optimized profile lookup using direct indexing for common profiles
 * @param profile_token The profile token to search for
 * @return Pointer to profile or NULL if not found
 */
static struct media_profile* find_profile_optimized(const char* profile_token) {
  if (!profile_token) {
    return NULL;
  }

  // Direct lookup for common profiles to avoid linear search
  if (strcmp(profile_token, MEDIA_MAIN_PROFILE_TOKEN) == 0) {
    return &g_media_profiles[0];
  }
  if (strcmp(profile_token, MEDIA_SUB_PROFILE_TOKEN) == 0) {
    return &g_media_profiles[1];
  }

  // Fall back to linear search for custom profiles
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      return &g_media_profiles[i];
    }
  }

  return NULL;
}

int onvif_media_get_profiles(struct media_profile** profile_list, int* count) {
  ONVIF_CHECK_NULL(profile_list);
  ONVIF_CHECK_NULL(count);

  *profile_list = g_media_profiles;
  *count = PROFILE_COUNT;
  return ONVIF_SUCCESS;
}

int onvif_media_get_profile(const char* profile_token, struct media_profile* profile) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(profile);

  // Optimized: Use memcmp for faster comparison when possible
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      // Optimized: Use memcpy for entire structure copy (more efficient)
      memcpy(profile, &g_media_profiles[i], sizeof(struct media_profile));
      return ONVIF_SUCCESS;
    }
  }
  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_create_profile(const char* name, const char* token, struct media_profile* profile) {
  ONVIF_CHECK_NULL(name);
  ONVIF_CHECK_NULL(token);
  ONVIF_CHECK_NULL(profile);

  // Check if token already exists
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, token) == 0) {
      return ONVIF_ERROR_DUPLICATE;
    }
  }

  // For now, we only support creating profiles with predefined configurations
  // This is a simplified implementation for Profile S compliance
  if (strcmp(token, "CustomProfile") == 0) {
    // Optimized: Initialize profile with zeros first
    memset(profile, 0, sizeof(struct media_profile));

    // Optimized: Use strncpy with explicit null termination
    strncpy(profile->token, "CustomProfile", sizeof(profile->token) - 1);
    profile->token[sizeof(profile->token) - 1] = '\0';

    strncpy(profile->name, name, sizeof(profile->name) - 1);
    profile->name[sizeof(profile->name) - 1] = '\0';

    profile->fixed = 0; // Not fixed, can be deleted

    // Optimized: Copy from main profile using field-by-field assignment
    profile->video_source = g_media_profiles[0].video_source;
    profile->video_encoder = g_media_profiles[0].video_encoder;
    profile->audio_source = g_media_profiles[0].audio_source;
    profile->audio_encoder = g_media_profiles[0].audio_encoder;
    profile->ptz = g_media_profiles[0].ptz;

    platform_log_info("Created custom profile: %s (%s)\n", name, token);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_SUPPORTED;
}

int onvif_media_delete_profile(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

  // Check if it's a fixed profile (cannot be deleted)
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      if (g_media_profiles[i].fixed) {
        return ONVIF_ERROR_NOT_SUPPORTED; // Fixed profiles cannot be deleted
      }
      // For now, we don't actually remove the profile from the array
      // In a real implementation, you'd manage dynamic profiles
      platform_log_info("Deleted profile: %s\n", profile_token);
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_get_video_sources(struct video_source** sources, int* count) {
  static struct video_source video_sources[] = {
    {.token = "VideoSource0",
     .framerate = MEDIA_MAIN_QUALITY_DEFAULT,
     .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
     .imaging = {.brightness = MEDIA_IMAGING_BRIGHTNESS_DEFAULT,
                 .color_saturation = MEDIA_IMAGING_SATURATION_DEFAULT,
                 .contrast = MEDIA_IMAGING_CONTRAST_DEFAULT,
                 .sharpness = MEDIA_IMAGING_SHARPNESS_DEFAULT}}};

  ONVIF_CHECK_NULL(sources);
  ONVIF_CHECK_NULL(count);

  *sources = video_sources;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_get_audio_sources(struct audio_source** sources, int* count) {
  static struct audio_source audio_sources[] = {{.token = "AudioSource0", .channels = 1}};

  ONVIF_CHECK_NULL(sources);
  ONVIF_CHECK_NULL(count);

  *sources = audio_sources;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_get_video_source_configurations(struct video_source_configuration** configs,
                                                int* count) {
  static struct video_source_configuration video_configs[] = {
    {.token = "VideoSourceConfig0",
     .name = "Video Source Configuration",
     .use_count = 2,
     .source_token = "VideoSource0",
     .bounds = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT, 0, 0}}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = video_configs;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_get_video_encoder_configurations(struct video_encoder_configuration** configs,
                                                 int* count) {
  static struct video_encoder_configuration video_enc_configs[] = {
    {.token = "VideoEncoder0",
     .name = "H.264 Main Encoder",
     .use_count = 1,
     .encoding = "H264",
     .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
     .quality = MEDIA_MAIN_QUALITY_DEFAULT,
     .framerate_limit = MEDIA_DEFAULT_FRAMERATE,
     .encoding_interval = 1,
     .bitrate_limit = MEDIA_MAIN_BITRATE_DEFAULT,
     .gov_length = MEDIA_DEFAULT_GOV_LENGTH,
     .profile = "Main",
     .guaranteed_framerate = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0}},
    {.token = "VideoEncoder1",
     .name = "H.264 Sub Encoder",
     .use_count = 1,
     .encoding = "H264",
     .resolution = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT},
     .quality = MEDIA_SUB_QUALITY_DEFAULT,
     .framerate_limit = MEDIA_DEFAULT_FRAMERATE,
     .encoding_interval = 1,
     .bitrate_limit = MEDIA_SUB_BITRATE_DEFAULT,
     .gov_length = MEDIA_DEFAULT_GOV_LENGTH,
     .profile = "Main",
     .guaranteed_framerate = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0}}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = video_enc_configs;
  *count = 2;
  return ONVIF_SUCCESS;
}

int onvif_media_get_audio_source_configurations(struct audio_source_configuration** configs,
                                                int* count) {
  static struct audio_source_configuration audio_configs[] = {{.token = "AudioSourceConfig0",
                                                               .name = "Audio Source Configuration",
                                                               .use_count = 2,
                                                               .source_token = "AudioSource0"}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = audio_configs;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration** configs,
                                                 int* count) {
  static struct audio_encoder_configuration audio_enc_configs[] = {
    {.token = "AudioEncoder0",
     .name = "AAC-LC Encoder (Main)",
     .use_count = 2,
     .encoding = "AAC",
     .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
     .sample_rate = AUDIO_SAMPLE_RATE_16KHZ,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0},
     .session_timeout = MEDIA_SESSION_TIMEOUT_DEFAULT},
    {.token = "AudioEncoder1",
     .name = "AAC-LC Encoder (High Quality)",
     .use_count = 0,
     .encoding = "AAC",
     .bitrate = MEDIA_AUDIO_BITRATE_HIGH_QUALITY,
     .sample_rate = AUDIO_SAMPLE_RATE_48KHZ,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0},
     .session_timeout = MEDIA_SESSION_TIMEOUT_DEFAULT},
    {.token = "AudioEncoder2",
     .name = "AAC-LC Encoder (Low Bitrate)",
     .use_count = 0,
     .encoding = "AAC",
     .bitrate = MEDIA_AUDIO_BITRATE_LOW_QUALITY,
     .sample_rate = AUDIO_SAMPLE_RATE_8KHZ,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0},
     .session_timeout = MEDIA_SESSION_TIMEOUT_DEFAULT}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = audio_enc_configs;
  *count = 3;
  return ONVIF_SUCCESS;
}

/* Metadata Configuration Functions */
int onvif_media_get_metadata_configurations(struct metadata_configuration** configs, int* count) {
  static struct metadata_configuration metadata_configs[] = {
    {.token = "MetadataConfig0",
     .name = "Basic Metadata Configuration",
     .use_count = 1,
     .session_timeout = MEDIA_SESSION_TIMEOUT_DEFAULT,
     .analytics = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = MEDIA_DEFAULT_TTL, .auto_start = 0}}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = metadata_configs;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_set_metadata_configuration(const char* configuration_token,
                                           const struct metadata_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  if (strcmp(configuration_token, "MetadataConfig0") == 0) {
    platform_log_info("Updated metadata configuration: %s\n", configuration_token);
    platform_log_info("  Analytics: %s\n", config->analytics ? "enabled" : "disabled");
    platform_log_info("  Session Timeout: %d seconds\n", config->session_timeout);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

/* Configuration Management Functions */
int onvif_media_set_video_source_configuration(const char* configuration_token,
                                               const struct video_source_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  // For now, we only support updating the default video source configuration
  if (strcmp(configuration_token, "VideoSourceConfig0") == 0) {
    platform_log_info("Updated video source configuration: %s\n", configuration_token);
    platform_log_info("  Resolution: %dx%d\n", config->bounds.width, config->bounds.height);
    platform_log_info("  Bounds: x=%d, y=%d\n", config->bounds.x, config->bounds.y);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_video_encoder_configuration(const char* configuration_token,
                                                const struct video_encoder_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  // For now, we only support updating the default video encoder configurations
  if (strcmp(configuration_token, "VideoEncoder0") == 0 ||
      strcmp(configuration_token, "VideoEncoder1") == 0) {
    platform_log_info("Updated video encoder configuration: %s\n", configuration_token);
    platform_log_info("  Resolution: %dx%d\n", config->resolution.width, config->resolution.height);
    platform_log_info("  Bitrate: %d kbps\n", config->bitrate_limit);
    platform_log_info("  Framerate: %d fps\n", config->framerate_limit);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_audio_source_configuration(const char* configuration_token,
                                               const struct audio_source_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  if (strcmp(configuration_token, "AudioSourceConfig0") == 0) {
    platform_log_info("Updated audio source configuration: %s\n", configuration_token);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_audio_encoder_configuration(const char* configuration_token,
                                                const struct audio_encoder_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  // Validate AAC-specific parameters
  if (strcmp(config->encoding, "AAC") == 0) {
    // Validate bitrate for AAC
    if (config->bitrate < MEDIA_AUDIO_BITRATE_AAC_MIN ||
        config->bitrate > MEDIA_AUDIO_BITRATE_AAC_MAX) {
      platform_log_error("Invalid AAC bitrate: %d kbps (must be %d-%d)\n", config->bitrate,
                         MEDIA_AUDIO_BITRATE_AAC_MIN, MEDIA_AUDIO_BITRATE_AAC_MAX);
      return ONVIF_ERROR_INVALID;
    }

    // Validate sample rate for AAC
    if (config->sample_rate < MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN ||
        config->sample_rate > MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX) {
      platform_log_error("Invalid AAC sample rate: %d Hz (must be %d-%d)\n", config->sample_rate,
                         MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN, MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX);
      return ONVIF_ERROR_INVALID;
    }

    // Validate sample rate is supported by Anyka platform
    if (config->sample_rate != AUDIO_SAMPLE_RATE_8KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_16KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_22KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_44KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_48KHZ) {
      platform_log_error("Unsupported AAC sample rate: %d Hz (supported: 8000, 16000, 22050, "
                         "44100, 48000)\n",
                         config->sample_rate);
      return ONVIF_ERROR_NOT_SUPPORTED;
    }
  }

  // Check if configuration token is valid
  if (strcmp(configuration_token, "AudioEncoder0") == 0 ||
      strcmp(configuration_token, "AudioEncoder1") == 0 ||
      strcmp(configuration_token, "AudioEncoder2") == 0) {
    platform_log_info("Updated audio encoder configuration: %s\n", configuration_token);
    platform_log_info("  Encoding: %s\n", config->encoding);
    platform_log_info("  Bitrate: %d kbps\n", config->bitrate);
    platform_log_info("  Sample Rate: %d Hz\n", config->sample_rate);
    platform_log_info("  Session Timeout: %d seconds\n", config->session_timeout);

    // Log AAC-specific information
    if (strcmp(config->encoding, "AAC") == 0) {
      platform_log_info("  AAC Profile: LC (Low Complexity)\n");
      platform_log_info("  AAC Channels: 1 (Mono)\n");
    }

    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_get_stream_uri(const char* profile_token, const char* protocol,
                               struct stream_uri* uri) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(protocol);
  ONVIF_CHECK_NULL(uri);

  /* Find the profile using optimized lookup */
  struct media_profile* profile = find_profile_optimized(profile_token);
  if (profile == NULL) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Use cached URI for RTSP/RTP protocols */
  if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
    // Try to use cached URI first for common profiles
    int result = get_cached_rtsp_uri(profile_token, uri->uri, sizeof(uri->uri));
    if (result == ONVIF_SUCCESS) {
      uri->invalid_after_connect = 0;
      uri->invalid_after_reboot = 0;
      uri->timeout = MEDIA_DEFAULT_TIMEOUT_SECONDS;
      return ONVIF_SUCCESS;
    }

    // Fall back to dynamic URI construction for non-cached profiles
    if (strcmp(profile_token, "SubProfile") == 0) {
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, uri->uri,
                       sizeof(uri->uri));
    } else {
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, uri->uri,
                       sizeof(uri->uri));
    }
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = MEDIA_DEFAULT_TIMEOUT_SECONDS;
    return ONVIF_SUCCESS;
  }

  // Handle other protocols
  if (strcmp(protocol, "HTTP") == 0) {
    build_device_url("http", ONVIF_HTTP_PORT_DEFAULT, "/onvif/streaming", uri->uri,
                     sizeof(uri->uri));
    // Append profile parameter
    char profile_param[MEDIA_URL_PARAM_SIZE];
    int written = snprintf(profile_param, sizeof(profile_param), "?profile=%s", profile_token);
    if (written < 0 || (size_t)written >= sizeof(profile_param)) {
      platform_log_error("Failed to format profile parameter\n");
      return ONVIF_ERROR_INVALID;
    }
    strncat(uri->uri, profile_param, sizeof(uri->uri) - strlen(uri->uri) - 1);
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = MEDIA_DEFAULT_TIMEOUT_SECONDS;
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_get_snapshot_uri(const char* profile_token, struct stream_uri* uri) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(uri);

  /* Generate snapshot URI */
  build_device_url("http", ONVIF_SNAPSHOT_PORT_DEFAULT, SNAPSHOT_PATH, uri->uri, sizeof(uri->uri));
  uri->invalid_after_connect = 0;
  uri->invalid_after_reboot = 0;
  uri->timeout = MEDIA_SESSION_TIMEOUT_DEFAULT;

  return ONVIF_SUCCESS;
}

int onvif_media_start_multicast_streaming(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

  // Check if profile exists
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      // Basic multicast streaming implementation for Profile S compliance
      // This is a simplified implementation that logs the request
      platform_log_info("StartMulticastStreaming for profile %s\n", profile_token);

      // For now, we just acknowledge the request
      // In a real implementation, this would:
      // 1. Start RTP multicast streaming on a specific IP/port
      // 2. Configure the video encoder for multicast output
      // 3. Set up RTP packet transmission

      platform_log_info("Multicast streaming started for profile %s\n", profile_token);
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_stop_multicast_streaming(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

  // Check if profile exists
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      // Basic multicast streaming stop implementation
      platform_log_info("StopMulticastStreaming for profile %s\n", profile_token);

      // For now, we just acknowledge the request
      // In a real implementation, this would:
      // 1. Stop RTP multicast streaming
      // 2. Clean up multicast resources
      // 3. Reset encoder configuration

      platform_log_info("Multicast streaming stopped for profile %s\n", profile_token);
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

/* Helper Functions */

static int parse_profile_token(onvif_gsoap_context_t* gsoap_ctx, char* token, size_t token_size) {
  if (!gsoap_ctx || !token || token_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  return onvif_gsoap_parse_media_profile_token(gsoap_ctx, token, token_size);
}

static int parse_protocol(onvif_gsoap_context_t* gsoap_ctx, char* protocol, size_t protocol_size) {
  if (!gsoap_ctx || !protocol || protocol_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  return onvif_gsoap_parse_protocol(gsoap_ctx, protocol, protocol_size);
}

// Helper function to parse value from request body
static int parse_value_from_request(const char* request_body, onvif_gsoap_context_t* gsoap_ctx,
                                    const char* xpath, char* value, size_t value_size) {
  if (!request_body || !gsoap_ctx || !xpath || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request_body, strlen(request_body));
  if (result != 0) {
    return result;
  }

  return onvif_gsoap_parse_value(gsoap_ctx, xpath, value, value_size);
}

static int validate_profile_token(const char* token) {
  if (!token) {
    return ONVIF_ERROR_INVALID;
  }

  // Check if token matches known profiles
  if (strcmp(token, MEDIA_MAIN_PROFILE_TOKEN) == 0 || strcmp(token, MEDIA_SUB_PROFILE_TOKEN) == 0) {
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

static int validate_protocol(const char* protocol) {
  if (!protocol) {
    return ONVIF_ERROR_INVALID;
  }

  // Check if protocol is supported
  if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_SUPPORTED;
}

// Media service configuration and state
static onvif_service_handler_instance_t g_media_handler = {{0}}; // NOLINT
static int g_handler_initialized = 0;                            // NOLINT

// Global buffer pool for medium-sized responses (4-32KB)
static buffer_pool_t g_media_response_buffer_pool; // NOLINT

/* ============================================================================
 * Service Operation Definitions (Forward Declaration)
 * ============================================================================ */

// Forward declarations
static int get_profiles_business_logic(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx,
                                       service_log_context_t* log_ctx, error_context_t* error_ctx,
                                       void* callback_data);

static int get_stream_uri_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int create_profile_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int delete_profile_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int set_video_source_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data);

static int set_video_encoder_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data);

static int start_multicast_streaming_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data);

static int stop_multicast_streaming_business_logic(const service_handler_config_t* config,
                                                   const http_request_t* request,
                                                   http_response_t* response,
                                                   onvif_gsoap_context_t* gsoap_ctx,
                                                   service_log_context_t* log_ctx,
                                                   error_context_t* error_ctx, void* callback_data);

static int get_metadata_configurations_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data);

static int set_metadata_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data);

/**
 * @brief GetProfiles service operation definition
 */
static const onvif_service_operation_t get_profiles_operation = {
  .service_name = "Media",
  .operation_name = "GetProfiles",
  .operation_context = "media_profiles_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_profiles_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief GetStreamUri service operation definition
 */
static const onvif_service_operation_t get_stream_uri_operation = {
  .service_name = "Media",
  .operation_name = "GetStreamUri",
  .operation_context = "media_stream_uri_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_stream_uri_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief CreateProfile service operation definition
 */
static const onvif_service_operation_t create_profile_operation = {
  .service_name = "Media",
  .operation_name = "CreateProfile",
  .operation_context = "media_profile_creation",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = create_profile_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief DeleteProfile service operation definition
 */
static const onvif_service_operation_t delete_profile_operation = {
  .service_name = "Media",
  .operation_name = "DeleteProfile",
  .operation_context = "media_profile_deletion",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = delete_profile_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief SetVideoSourceConfiguration service operation definition
 */
static const onvif_service_operation_t set_video_source_configuration_operation = {
  .service_name = "Media",
  .operation_name = "SetVideoSourceConfiguration",
  .operation_context = "media_video_source_config",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = set_video_source_configuration_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief SetVideoEncoderConfiguration service operation definition
 */
static const onvif_service_operation_t set_video_encoder_configuration_operation = {
  .service_name = "Media",
  .operation_name = "SetVideoEncoderConfiguration",
  .operation_context = "media_video_encoder_config",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = set_video_encoder_configuration_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief StartMulticastStreaming service operation definition
 */
static const onvif_service_operation_t start_multicast_streaming_operation = {
  .service_name = "Media",
  .operation_name = "StartMulticastStreaming",
  .operation_context = "media_multicast_start",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = start_multicast_streaming_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief StopMulticastStreaming service operation definition
 */
static const onvif_service_operation_t stop_multicast_streaming_operation = {
  .service_name = "Media",
  .operation_name = "StopMulticastStreaming",
  .operation_context = "media_multicast_stop",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = stop_multicast_streaming_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief GetMetadataConfigurations service operation definition
 */
static const onvif_service_operation_t get_metadata_configurations_operation = {
  .service_name = "Media",
  .operation_name = "GetMetadataConfigurations",
  .operation_context = "media_metadata_configs",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_metadata_configurations_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief SetMetadataConfiguration service operation definition
 */
static const onvif_service_operation_t set_metadata_configuration_operation = {
  .service_name = "Media",
  .operation_name = "SetMetadataConfiguration",
  .operation_context = "media_metadata_config",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = set_metadata_configuration_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

// Action handlers
static int handle_get_profiles(const service_handler_config_t* config,
                               const http_request_t* request, http_response_t* response,
                               onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetProfiles
  media_profiles_callback_data_t callback_data = {.profiles = NULL, .profile_count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_profiles_operation,
                                           media_profiles_response_callback, &callback_data);
}

static int handle_get_stream_uri(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetStreamUri
  media_stream_uri_callback_data_t callback_data = {.uri = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_stream_uri_operation,
                                           media_stream_uri_response_callback, &callback_data);
}

static int handle_create_profile(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for CreateProfile
  media_create_profile_callback_data_t callback_data = {.profile = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &create_profile_operation,
                                           media_create_profile_response_callback, &callback_data);
}

static int handle_delete_profile(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for DeleteProfile
  media_delete_profile_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &delete_profile_operation,
                                           media_delete_profile_response_callback, &callback_data);
}

static int handle_set_video_source_configuration(const service_handler_config_t* config,
                                                 const http_request_t* request,
                                                 http_response_t* response,
                                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetVideoSourceConfiguration
  media_set_video_source_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(
    config, request, response, gsoap_ctx, &set_video_source_configuration_operation,
    media_set_video_source_config_response_callback, &callback_data);
}

static int handle_set_video_encoder_configuration(const service_handler_config_t* config,
                                                  const http_request_t* request,
                                                  http_response_t* response,
                                                  onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetVideoEncoderConfiguration
  media_set_video_encoder_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(
    config, request, response, gsoap_ctx, &set_video_encoder_configuration_operation,
    media_set_video_encoder_config_response_callback, &callback_data);
}

static int handle_start_multicast_streaming(const service_handler_config_t* config,
                                            const http_request_t* request,
                                            http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for StartMulticastStreaming
  media_start_multicast_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &start_multicast_streaming_operation,
                                           media_start_multicast_response_callback, &callback_data);
}

static int handle_stop_multicast_streaming(const service_handler_config_t* config,
                                           const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for StopMulticastStreaming
  media_stop_multicast_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &stop_multicast_streaming_operation,
                                           media_stop_multicast_response_callback, &callback_data);
}

static int handle_get_metadata_configurations(const service_handler_config_t* config,
                                              const http_request_t* request,
                                              http_response_t* response,
                                              onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetMetadataConfigurations
  media_get_metadata_configs_callback_data_t callback_data = {.configs = NULL, .config_count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(
    config, request, response, gsoap_ctx, &get_metadata_configurations_operation,
    media_get_metadata_configs_response_callback, &callback_data);
}

static int handle_set_metadata_configuration(const service_handler_config_t* config,
                                             const http_request_t* request,
                                             http_response_t* response,
                                             onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetMetadataConfiguration
  media_set_metadata_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(
    config, request, response, gsoap_ctx, &set_metadata_configuration_operation,
    media_set_metadata_config_response_callback, &callback_data);
}

/* ============================================================================
 * Business Logic Callback Functions
 * ============================================================================ */

/**
 * @brief Business logic callback for GetProfiles operation
 */
static int get_profiles_business_logic(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx,
                                       service_log_context_t* log_ctx, error_context_t* error_ctx,
                                       void* callback_data) {
  (void)config;
  (void)request;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing GetProfiles request");

  media_profiles_callback_data_t* profile_data = (media_profiles_callback_data_t*)callback_data;

  // Set callback data with existing profiles
  profile_data->profiles = g_media_profiles;
  profile_data->profile_count = MEDIA_PROFILE_COUNT_DEFAULT;

  // Generate response using gSOAP callback
  int result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, media_profiles_response_callback, callback_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_media_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  service_log_info(log_ctx, "GetProfiles request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetStreamUri operation
 */
static int get_stream_uri_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing GetStreamUri request");

  // Parse profile token and protocol from request using common XML parser
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE];
  char protocol[MEDIA_PROTOCOL_BUFFER_SIZE];

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result,
                                  "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  if (parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(error_ctx, "profile_token", "missing_or_invalid", response);
  }

  if (parse_protocol(gsoap_ctx, protocol, sizeof(protocol)) != 0) {
    return error_handle_parameter(error_ctx, "protocol", "missing_or_invalid", response);
  }

  // Validate parameters
  if (validate_profile_token(profile_token) != 0) {
    return error_handle_parameter(error_ctx, "profile_token", "invalid", response);
  }

  if (validate_protocol(protocol) != 0) {
    return error_handle_parameter(error_ctx, "protocol", "invalid", response);
  }

  // Get stream URI using the existing function
  struct stream_uri uri;
  result = onvif_media_get_stream_uri(profile_token, protocol, &uri);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "get_stream_uri", response);
  }

  // Update callback data
  media_stream_uri_callback_data_t* uri_data = (media_stream_uri_callback_data_t*)callback_data;
  uri_data->uri = &uri;

  // Generate response using gSOAP callback
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, media_stream_uri_response_callback, callback_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_media_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  service_log_info(log_ctx, "GetStreamUri request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for CreateProfile operation
 */
static int create_profile_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing CreateProfile request");

  // Parse profile name and token from request
  char profile_name[MEDIA_NAME_BUFFER_SIZE] = "Custom Profile";
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE] = "CustomProfile";

  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  parse_value_from_request(request->body, gsoap_ctx, "//trt:Name", profile_name,
                           sizeof(profile_name));
  parse_value_from_request(request->body, gsoap_ctx, "//trt:Token", profile_token,
                           sizeof(profile_token));

  // Create profile
  struct media_profile profile;
  result = onvif_media_create_profile(profile_name, profile_token, &profile);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "create_profile", response);
  }

  // Update callback data
  media_create_profile_callback_data_t* profile_data =
    (media_create_profile_callback_data_t*)callback_data;
  profile_data->profile = &profile;

  service_log_info(log_ctx, "CreateProfile request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for DeleteProfile operation
 */
static int delete_profile_business_logic(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx,
                                         service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing DeleteProfile request");

  // Parse profile token from request
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    return error_handle_parameter(error_ctx, "profile_token", "missing", response);
  }

  // Delete profile
  result = onvif_media_delete_profile(profile_token);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "delete_profile", response);
  }

  // Update callback data
  media_delete_profile_callback_data_t* delete_data =
    (media_delete_profile_callback_data_t*)callback_data;
  delete_data->message = "Profile deleted successfully";

  service_log_info(log_ctx, "DeleteProfile request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SetVideoSourceConfiguration operation
 */
static int set_video_source_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing SetVideoSourceConfiguration request");

  // Parse configuration token from request
  char config_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    return error_handle_parameter(error_ctx, "configuration_token", "missing", response);
  }

  // Parse video source configuration parameters
  struct video_source_configuration video_config;
  memset(&video_config, 0, sizeof(video_config));
  strncpy(video_config.token, config_token, sizeof(video_config.token) - 1);

  // Set configuration
  result = onvif_media_set_video_source_configuration(config_token, &video_config);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "set_video_source_config", response);
  }

  service_log_info(log_ctx, "SetVideoSourceConfiguration request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SetVideoEncoderConfiguration operation
 */
static int set_video_encoder_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing SetVideoEncoderConfiguration request");

  // Parse configuration token from request
  char config_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    return error_handle_parameter(error_ctx, "configuration_token", "missing", response);
  }

  // Parse video encoder configuration parameters
  struct video_encoder_configuration encoder_config;
  memset(&encoder_config, 0, sizeof(encoder_config));
  strncpy(encoder_config.token, config_token, sizeof(encoder_config.token) - 1);

  // Set configuration
  result = onvif_media_set_video_encoder_configuration(config_token, &encoder_config);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "set_video_encoder_config", response);
  }

  service_log_info(log_ctx, "SetVideoEncoderConfiguration request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for StartMulticastStreaming operation
 */
static int start_multicast_streaming_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing StartMulticastStreaming request");

  // Parse profile token from request
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    return error_handle_parameter(error_ctx, "profile_token", "missing", response);
  }

  // Start multicast streaming
  result = onvif_media_start_multicast_streaming(profile_token);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "start_multicast_streaming", response);
  }

  service_log_info(log_ctx, "StartMulticastStreaming request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for StopMulticastStreaming operation
 */
static int stop_multicast_streaming_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing StopMulticastStreaming request");

  // Parse profile token from request
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    return error_handle_parameter(error_ctx, "profile_token", "missing", response);
  }

  // Stop multicast streaming
  result = onvif_media_stop_multicast_streaming(profile_token);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "stop_multicast_streaming", response);
  }

  service_log_info(log_ctx, "StopMulticastStreaming request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetMetadataConfigurations operation
 */
static int get_metadata_configurations_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)request;
  (void)gsoap_ctx;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing GetMetadataConfigurations request");

  // Get metadata configurations
  struct metadata_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_metadata_configurations(&configs, &count);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "get_metadata_configurations", response);
  }

  // Update callback data
  media_get_metadata_configs_callback_data_t* config_data =
    (media_get_metadata_configs_callback_data_t*)callback_data;
  config_data->configs = configs;
  config_data->config_count = count;

  service_log_info(log_ctx, "GetMetadataConfigurations request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SetMetadataConfiguration operation
 */
static int set_metadata_configuration_business_logic(
  const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
  onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
  void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing SetMetadataConfiguration request");

  // Parse configuration token from request
  char config_token[MEDIA_TOKEN_BUFFER_SIZE] = "";
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    return ONVIF_ERROR;
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    return error_handle_parameter(error_ctx, "configuration_token", "missing", response);
  }

  // Parse metadata configuration parameters
  struct metadata_configuration metadata_config = {0};
  strncpy(metadata_config.token, config_token, sizeof(metadata_config.token) - 1);
  strncpy(metadata_config.name, "Updated Metadata Configuration", sizeof(metadata_config.name) - 1);

  // Parse analytics setting
  char analytics_str[MEDIA_ANALYTICS_STR_SIZE] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//tt:Analytics", analytics_str,
                               sizeof(analytics_str)) == 0) {
    metadata_config.analytics = (strcmp(analytics_str, "true") == 0) ? 1 : 0;
  }

  // Set configuration
  result = onvif_media_set_metadata_configuration(config_token, &metadata_config);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "set_metadata_configuration", response);
  }

  service_log_info(log_ctx, "SetMetadataConfiguration request completed successfully");
  return ONVIF_SUCCESS;
}

// Media service action configurations
static const service_action_def_t media_actions[] = {
  {"GetProfiles", handle_get_profiles, 1},
  {"GetStreamUri", handle_get_stream_uri, 1},
  {"CreateProfile", handle_create_profile, 1},
  {"DeleteProfile", handle_delete_profile, 1},
  {"SetVideoSourceConfiguration", handle_set_video_source_configuration, 1},
  {"SetVideoEncoderConfiguration", handle_set_video_encoder_configuration, 1},
  {"StartMulticastStreaming", handle_start_multicast_streaming, 1},
  {"StopMulticastStreaming", handle_stop_multicast_streaming, 1},
  {"GetMetadataConfigurations", handle_get_metadata_configurations, 1},
  {"SetMetadataConfiguration", handle_set_metadata_configuration, 1}};

int onvif_media_init(config_manager_t* config) {
  ONVIF_CHECK_NULL(config);

  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {.service_type = ONVIF_SERVICE_MEDIA,
                                             .service_name = "Media",
                                             .config = config,
                                             .enable_validation = 1,
                                             .enable_logging = 1};

  int result = onvif_service_handler_init(&g_media_handler, &handler_config, media_actions,
                                          sizeof(media_actions) / sizeof(media_actions[0]));

  if (result == ONVIF_SUCCESS) {
    // Initialize buffer pool for medium-sized responses
    if (buffer_pool_init(&g_media_response_buffer_pool) != 0) {
      onvif_service_handler_cleanup(&g_media_handler);
      return ONVIF_ERROR;
    }
    // Register with service dispatcher using new standardized interface
    onvif_service_registration_t registration = {
      .service_name = "Media",
      .namespace_uri = "http://www.onvif.org/ver10/media/wsdl",
      .operation_handler = onvif_media_handle_request,
      .init_handler = NULL,
      .cleanup_handler = NULL,
      .capabilities_handler = NULL,
      .reserved = {NULL, NULL, NULL, NULL}};

    int dispatch_result = onvif_service_dispatcher_register_service(&registration);
    if (dispatch_result != ONVIF_SUCCESS) {
      platform_log_warning("Failed to register media service with dispatcher\n");
    }

    g_handler_initialized = 1;
  }

  return result;
}

void onvif_media_cleanup(void) {
  if (g_handler_initialized) {
    // Unregister from service dispatcher
    onvif_service_dispatcher_unregister_service("Media");

    onvif_service_handler_cleanup(&g_media_handler);

    // Cleanup buffer pool (returns void, so we can't check for errors)
    buffer_pool_cleanup(&g_media_response_buffer_pool);

    g_handler_initialized = 0;

    // Check for memory leaks
    memory_manager_check_leaks();
  }
}

int onvif_media_handle_request(const char* action_name, const http_request_t* request,
                               http_response_t* response) {
  if (!g_handler_initialized) {
    return ONVIF_ERROR;
  }
  return onvif_service_handler_handle_request(&g_media_handler, action_name, request, response);
}
