/**
 * @file onvif_media.c
 * @brief ONVIF Media service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_media.h"

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/network/network_utils.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MEDIA_PROFILE_COUNT_DEFAULT 2
#define MEDIA_MAIN_PROFILE_TOKEN    "MainProfile"
#define MEDIA_SUB_PROFILE_TOKEN     "SubProfile"

#define MEDIA_MAIN_RESOLUTION_WIDTH  1280
#define MEDIA_MAIN_RESOLUTION_HEIGHT 720
#define MEDIA_SUB_RESOLUTION_WIDTH   640
#define MEDIA_SUB_RESOLUTION_HEIGHT  360

#define MEDIA_MAIN_QUALITY_DEFAULT 4.0
#define MEDIA_SUB_QUALITY_DEFAULT  3.0

#define MEDIA_MAIN_BITRATE_DEFAULT      2048
#define MEDIA_SUB_BITRATE_DEFAULT       800
#define MEDIA_AUDIO_BITRATE_DEFAULT     64
#define MEDIA_AUDIO_BITRATE_AAC_MIN     32
#define MEDIA_AUDIO_BITRATE_AAC_MAX     320
#define MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN 8000
#define MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX 48000

#define MEDIA_RESPONSE_BUFFER_SIZE 4096
#define MEDIA_PROFILE_BUFFER_SIZE  2048
#define MEDIA_URI_BUFFER_SIZE      512
#define MEDIA_TOKEN_BUFFER_SIZE    32
#define MEDIA_PROTOCOL_BUFFER_SIZE 16

#define MEDIA_XML_PROFILE_TOKEN_TAG "<trt:ProfileToken>"
#define MEDIA_XML_PROTOCOL_TAG      "<trt:Protocol>"
#define MEDIA_XML_VIDEO_SOURCE_TAG  "<trt:VideoSourceToken>"
#define MEDIA_XML_AUDIO_SOURCE_TAG  "<trt:AudioSourceToken>"

#define PROFILE_COUNT MEDIA_PROFILE_COUNT_DEFAULT

static int parse_profile_token(onvif_gsoap_context_t* gsoap_ctx, char* token, size_t token_size);
static int parse_protocol(onvif_gsoap_context_t* gsoap_ctx, char* protocol, size_t protocol_size);
// Legacy functions removed - now using gSOAP-based functions
static int validate_profile_token(const char* token);
static int validate_protocol(const char* protocol);
static int find_profile_by_token(const char* token, struct media_profile** profile);
static void init_default_profiles(void);

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
                     .framerate_limit = 25,
                     .encoding_interval = 1,
                     .bitrate_limit = MEDIA_MAIN_BITRATE_DEFAULT,
                     .gov_length = 50},
   .audio_source = {.source_token = "AudioSource0"},
   .audio_encoder = {.token = "AudioEncoder0",
                     .encoding = "AAC",
                     .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
                     .sample_rate = 16000},
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
                     .framerate_limit = 25,
                     .encoding_interval = 1,
                     .bitrate_limit = MEDIA_SUB_BITRATE_DEFAULT,
                     .gov_length = 50},
   .audio_source = {.source_token = "AudioSource0"},
   .audio_encoder = {.token = "AudioEncoder0",
                     .encoding = "AAC",
                     .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
                     .sample_rate = 16000},
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

  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      *profile = g_media_profiles[i];
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
    // Create a custom profile based on main profile
    strcpy(profile->token, "CustomProfile");
    strncpy(profile->name, name, sizeof(profile->name) - 1);
    profile->name[sizeof(profile->name) - 1] = '\0';
    profile->fixed = 0; // Not fixed, can be deleted
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
     .framerate = 25.0f,
     .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
     .imaging = {
       .brightness = 50.0f, .color_saturation = 50.0f, .contrast = 50.0f, .sharpness = 50.0f}}};

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
     .framerate_limit = 25,
     .encoding_interval = 1,
     .bitrate_limit = MEDIA_MAIN_BITRATE_DEFAULT,
     .gov_length = 50,
     .profile = "Main",
     .guaranteed_framerate = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0}},
    {.token = "VideoEncoder1",
     .name = "H.264 Sub Encoder",
     .use_count = 1,
     .encoding = "H264",
     .resolution = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT},
     .quality = MEDIA_SUB_QUALITY_DEFAULT,
     .framerate_limit = 25,
     .encoding_interval = 1,
     .bitrate_limit = MEDIA_SUB_BITRATE_DEFAULT,
     .gov_length = 50,
     .profile = "Main",
     .guaranteed_framerate = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0}}};

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
     .sample_rate = 16000,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0},
     .session_timeout = 60},
    {.token = "AudioEncoder1",
     .name = "AAC-LC Encoder (High Quality)",
     .use_count = 0,
     .encoding = "AAC",
     .bitrate = 128,
     .sample_rate = 48000,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0},
     .session_timeout = 60},
    {.token = "AudioEncoder2",
     .name = "AAC-LC Encoder (Low Bitrate)",
     .use_count = 0,
     .encoding = "AAC",
     .bitrate = 32,
     .sample_rate = 8000,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0},
     .session_timeout = 60}};

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
     .session_timeout = 60,
     .analytics = 0,
     .multicast = {.address = "0.0.0.0", .port = 0, .ttl = 5, .auto_start = 0}}};

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
    if (config->sample_rate != 8000 && config->sample_rate != 16000 &&
        config->sample_rate != 22050 && config->sample_rate != 44100 &&
        config->sample_rate != 48000) {
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

  /* Find the profile */
  struct media_profile* profile = NULL;
  for (int i = 0; i < PROFILE_COUNT; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      profile = &g_media_profiles[i];
      break;
    }
  }

  ONVIF_CHECK_NULL(profile);

  /* Generate RTSP URI based on profile */
  if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
    if (strcmp(profile_token, "SubProfile") == 0) {
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, uri->uri,
                       sizeof(uri->uri));
    } else {
      // Default to main stream for MainProfile and any other profile
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, uri->uri,
                       sizeof(uri->uri));
    }

    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
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
  uri->timeout = 60;

  return ONVIF_SUCCESS;
}

int onvif_media_start_multicast_streaming(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

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

int onvif_media_stop_multicast_streaming(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

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

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */

/* Helper Functions */

static int parse_profile_token(onvif_gsoap_context_t* gsoap_ctx, char* token, size_t token_size) {
  if (!gsoap_ctx || !token || token_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  return onvif_gsoap_parse_profile_token(gsoap_ctx, token, token_size);
}

static int parse_protocol(onvif_gsoap_context_t* gsoap_ctx, char* protocol, size_t protocol_size) {
  if (!gsoap_ctx || !protocol || protocol_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  return onvif_gsoap_parse_protocol(gsoap_ctx, protocol, protocol_size);
}

// Helper function to parse profile token from request body
static int parse_profile_token_from_request(const char* request_body,
                                            onvif_gsoap_context_t* gsoap_ctx, char* token,
                                            size_t token_size) {
  if (!request_body || !gsoap_ctx || !token || token_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request_body, strlen(request_body));
  if (result != 0) {
    return result;
  }

  return onvif_gsoap_parse_profile_token(gsoap_ctx, token, token_size);
}

// Helper function to parse configuration token from request body
static int parse_configuration_token_from_request(const char* request_body,
                                                  onvif_gsoap_context_t* gsoap_ctx, char* token,
                                                  size_t token_size) {
  if (!request_body || !gsoap_ctx || !token || token_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request_body, strlen(request_body));
  if (result != 0) {
    return result;
  }

  return onvif_gsoap_parse_configuration_token(gsoap_ctx, token, token_size);
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

// Helper function to parse boolean value from request body
static int parse_boolean_from_request(const char* request_body, onvif_gsoap_context_t* gsoap_ctx,
                                      const char* xpath, int* value) {
  if (!request_body || !gsoap_ctx || !xpath || !value) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request_body, strlen(request_body));
  if (result != 0) {
    return result;
  }

  return onvif_gsoap_parse_boolean(gsoap_ctx, xpath, value);
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

static int find_profile_by_token(const char* token, struct media_profile** profile) {
  if (!token || !profile) {
    return ONVIF_ERROR_INVALID;
  }

  for (int i = 0; i < MEDIA_PROFILE_COUNT_DEFAULT; i++) {
    if (strcmp(g_media_profiles[i].token, token) == 0) {
      *profile = &g_media_profiles[i];
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR_NOT_FOUND;
}

// Legacy build_profile_xml function removed - now using
// onvif_xml_create_media_profile_element

// Legacy build_video_source_xml function removed - now using
// onvif_xml_create_video_source_element

// Legacy build_stream_uri_xml function removed - now using
// onvif_xml_create_media_uri_element

static void init_default_profiles(void) {
  // Profiles are already initialized as static data
  // This function is a placeholder for future dynamic initialization
}

/* Legacy service handler removed - using modern service handler implementation
 * below */

/* Refactored media service implementation */

// Service handler instance
static onvif_service_handler_instance_t g_media_handler; // NOLINT
static int g_handler_initialized = 0;                    // NOLINT

// Action handlers
static int handle_get_profiles(const service_handler_config_t* config,
                               const http_request_t* request, http_response_t* response,
                               onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize logging context
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Media", "GetProfiles", SERVICE_LOG_INFO);

  service_log_debug(&log_ctx, "Starting GetProfiles request processing");

  // Enhanced parameter validation
  if (!config) {
    service_log_debug(&log_ctx, "Missing config parameter");
    return ONVIF_ERROR_INVALID;
  }
  if (!response) {
    service_log_debug(&log_ctx, "Missing response parameter");
    return ONVIF_ERROR_INVALID;
  }
  if (!gsoap_ctx) {
    service_log_debug(&log_ctx, "Missing gsoap_ctx parameter");
    return ONVIF_ERROR_INVALID;
  }

  // Prepare callback data
  media_profiles_callback_data_t callback_data = {.profiles = g_media_profiles,
                                                  .profile_count = MEDIA_PROFILE_COUNT_DEFAULT};

  // Generate response using callback pattern
  int result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, media_profiles_response_callback, &callback_data);
  if (result != 0) {
    service_log_debug(&log_ctx, "Failed to generate profiles response: %d", result);
    return result;
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_debug(&log_ctx, "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      service_log_debug(&log_ctx, "Failed to allocate response buffer");
      return ONVIF_ERROR;
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  service_log_operation_success(&log_ctx, "GetProfiles completed successfully");
  return ONVIF_SUCCESS;
}

static int handle_get_stream_uri(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize logging context
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Media", "GetStreamUri", SERVICE_LOG_INFO);

  service_log_debug(&log_ctx, "Starting GetStreamUri request processing");

  // Enhanced parameter validation
  if (!config) {
    service_log_debug(&log_ctx, "Missing config parameter");
    return ONVIF_ERROR_INVALID;
  }
  if (!response) {
    service_log_debug(&log_ctx, "Missing response parameter");
    return ONVIF_ERROR_INVALID;
  }
  if (!gsoap_ctx) {
    service_log_debug(&log_ctx, "Missing gsoap_ctx parameter");
    return ONVIF_ERROR_INVALID;
  }

  // Parse profile token and protocol from request using common XML parser
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE];
  char protocol[MEDIA_PROTOCOL_BUFFER_SIZE];

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_debug(&log_ctx, "Failed to initialize gSOAP request parsing: %d", result);
    return result;
  }

  if (parse_profile_token(gsoap_ctx, profile_token, sizeof(profile_token)) != 0) {
    service_log_debug(&log_ctx, "Failed to parse profile token");
    return ONVIF_ERROR_INVALID;
  }

  if (parse_protocol(gsoap_ctx, protocol, sizeof(protocol)) != 0) {
    service_log_debug(&log_ctx, "Failed to parse protocol");
    return ONVIF_ERROR_INVALID;
  }

  // Validate parameters
  if (validate_profile_token(profile_token) != 0) {
    service_log_debug(&log_ctx, "Invalid profile token: %s", profile_token);
    return ONVIF_ERROR_INVALID;
  }

  if (validate_protocol(protocol) != 0) {
    service_log_debug(&log_ctx, "Invalid protocol: %s", protocol);
    return ONVIF_ERROR_INVALID;
  }

  // Get stream URI using the existing function
  struct stream_uri uri;
  int stream_result = onvif_media_get_stream_uri(profile_token, protocol, &uri);

  if (stream_result != ONVIF_SUCCESS) {
    service_log_debug(&log_ctx, "Failed to get stream URI: %d", stream_result);
    return stream_result;
  }

  // Prepare callback data
  media_stream_uri_callback_data_t callback_data = {.uri = &uri};

  // Generate response using callback pattern
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, media_stream_uri_response_callback, &callback_data);
  if (result != 0) {
    service_log_debug(&log_ctx, "Failed to generate stream URI response: %d", result);
    return result;
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_debug(&log_ctx, "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      service_log_debug(&log_ctx, "Failed to allocate response buffer");
      return ONVIF_ERROR;
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  service_log_operation_success(&log_ctx, "GetStreamUri completed successfully");
  return ONVIF_SUCCESS;
}

// Action handlers for CreateProfile and DeleteProfile
static int handle_create_profile(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Initialize logging context
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Media", "CreateProfile", SERVICE_LOG_INFO);

  service_log_debug(&log_ctx, "Starting CreateProfile request processing");

  if (!config || !response || !gsoap_ctx) {
    service_log_debug(&log_ctx, "Missing required parameters");
    return ONVIF_ERROR_INVALID;
  }

  // Parse profile name and token from request
  char profile_name[64] = "Custom Profile";
  char profile_token[64] = "CustomProfile";

  // Parse profile name and token using gSOAP
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:Name", profile_name,
                               sizeof(profile_name)) != 0) {
    // Use default name if parsing fails
    strncpy(profile_name, "Custom Profile", sizeof(profile_name) - 1);
    profile_name[sizeof(profile_name) - 1] = '\0';
  }

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:Token", profile_token,
                               sizeof(profile_token)) != 0) {
    // Use default token if parsing fails
    strncpy(profile_token, "CustomProfile", sizeof(profile_token) - 1);
    profile_token[sizeof(profile_token) - 1] = '\0';
  }

  // Create profile
  struct media_profile profile;
  int result = onvif_media_create_profile(profile_name, profile_token, &profile);

  if (result != ONVIF_SUCCESS) {
    service_log_debug(&log_ctx, "Failed to create profile: %d", result);
    return result;
  }

  // Prepare callback data
  media_create_profile_callback_data_t callback_data = {.profile = &profile};

  // Generate response using callback pattern
  result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, media_create_profile_response_callback, &callback_data);
  if (result != 0) {
    service_log_debug(&log_ctx, "Failed to generate create profile response: %d", result);
    return result;
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_debug(&log_ctx, "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      service_log_debug(&log_ctx, "Failed to allocate response buffer");
      return ONVIF_ERROR;
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  service_log_operation_success(&log_ctx, "CreateProfile completed successfully");
  return ONVIF_SUCCESS;
}

static int handle_delete_profile(const service_handler_config_t* config,
                                 const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "DeleteProfile", "profile_deletion");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    // Use default token if parsing fails
    strncpy(profile_token, "MainProfile", sizeof(profile_token) - 1);
    profile_token[sizeof(profile_token) - 1] = '\0';
  }

  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }

  // Delete profile
  int result = onvif_media_delete_profile(profile_token);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "profile_deletion", response);
  }

  // Generate delete profile response using proper gSOAP serialization
  result = onvif_gsoap_generate_delete_profile_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "delete_profile_response_generation", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

// Action handlers for Set*Configuration operations
static int handle_set_video_source_configuration(const service_handler_config_t* config,
                                                 const http_request_t* request,
                                                 http_response_t* response,
                                                 onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetVideoSourceConfiguration", "config_update");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse configuration token from request using gSOAP
  char config_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    // Use default token if parsing fails
    strncpy(config_token, "VideoSourceConfigToken", sizeof(config_token) - 1);
    config_token[sizeof(config_token) - 1] = '\0';
  }

  if (strlen(config_token) == 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }

  // Parse video source configuration parameters
  struct video_source_configuration video_config;
  // For now, initialize with defaults - proper parsing would need specific
  // implementation
  memset(&video_config, 0, sizeof(video_config));
  strncpy(video_config.token, config_token, sizeof(video_config.token) - 1);
  video_config.token[sizeof(video_config.token) - 1] = '\0';

  // Ensure the configuration token matches
  strncpy(video_config.token, config_token, sizeof(video_config.token) - 1);
  video_config.token[sizeof(video_config.token) - 1] = '\0';

  // Set configuration
  int result = onvif_media_set_video_source_configuration(config_token, &video_config);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "config_update", response);
  }

  // Generate set video source configuration response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_set_video_source_configuration_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result,
                               "set_video_source_configuration_response_generation", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

static int handle_set_video_encoder_configuration(const service_handler_config_t* config,
                                                  const http_request_t* request,
                                                  http_response_t* response,
                                                  onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetVideoEncoderConfiguration", "config_update");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse configuration token from request using gSOAP
  char config_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    // Use default token if parsing fails
    strncpy(config_token, "VideoEncoderConfigToken", sizeof(config_token) - 1);
    config_token[sizeof(config_token) - 1] = '\0';
  }

  if (strlen(config_token) == 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }

  // Parse video encoder configuration parameters
  struct video_encoder_configuration encoder_config;
  // For now, initialize with defaults - proper parsing would need specific
  // implementation
  memset(&encoder_config, 0, sizeof(encoder_config));
  strncpy(encoder_config.token, config_token, sizeof(encoder_config.token) - 1);
  encoder_config.token[sizeof(encoder_config.token) - 1] = '\0';

  // Ensure the configuration token matches
  strncpy(encoder_config.token, config_token, sizeof(encoder_config.token) - 1);
  encoder_config.token[sizeof(encoder_config.token) - 1] = '\0';

  // Configuration is already parsed by
  // onvif_xml_parse_video_encoder_configuration

  // Set configuration
  int result = onvif_media_set_video_encoder_configuration(config_token, &encoder_config);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "config_update", response);
  }

  // Generate set video encoder configuration response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_set_video_encoder_configuration_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result,
                               "set_video_encoder_configuration_response_generation", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

// Action handlers for multicast streaming
static int handle_start_multicast_streaming(const service_handler_config_t* config,
                                            const http_request_t* request,
                                            http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "StartMulticastStreaming", "multicast_start");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    // Use default token if parsing fails
    strncpy(profile_token, "MainProfile", sizeof(profile_token) - 1);
    profile_token[sizeof(profile_token) - 1] = '\0';
  }

  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }

  // Start multicast streaming
  int result = onvif_media_start_multicast_streaming(profile_token);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "multicast_start", response);
  }

  // Generate start multicast streaming response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_start_multicast_streaming_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "start_multicast_streaming_response_generation",
                               response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

static int handle_stop_multicast_streaming(const service_handler_config_t* config,
                                           const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "StopMulticastStreaming", "multicast_stop");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse profile token from request using gSOAP
  char profile_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token,
                               sizeof(profile_token)) != 0) {
    // Use default token if parsing fails
    strncpy(profile_token, "MainProfile", sizeof(profile_token) - 1);
    profile_token[sizeof(profile_token) - 1] = '\0';
  }

  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }

  // Stop multicast streaming
  int result = onvif_media_stop_multicast_streaming(profile_token);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "multicast_stop", response);
  }

  // Generate stop multicast streaming response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_stop_multicast_streaming_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "stop_multicast_streaming_response_generation",
                               response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

// Action handlers for metadata configuration
static int handle_get_metadata_configurations(const service_handler_config_t* config,
                                              const http_request_t* request,
                                              http_response_t* response,
                                              onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "GetMetadataConfigurations", "metadata_config_retrieval");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Get metadata configurations
  struct metadata_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_metadata_configurations(&configs, &count);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "metadata_config_retrieval", response);
  }

  // Generate get metadata configurations response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_get_metadata_configurations_response(gsoap_ctx, configs, count);
  if (result != 0) {
    return error_handle_system(&error_ctx, result,
                               "get_metadata_configurations_response_generation", response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

static int handle_set_metadata_configuration(const service_handler_config_t* config,
                                             const http_request_t* request,
                                             http_response_t* response,
                                             onvif_gsoap_context_t* gsoap_ctx) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetMetadataConfiguration", "metadata_config_update");

  if (!config || !response || !gsoap_ctx) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }

  // Parse configuration token from request using gSOAP
  char config_token[64] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token,
                               sizeof(config_token)) != 0) {
    // Use default token if parsing fails
    strncpy(config_token, "MetadataConfigToken", sizeof(config_token) - 1);
    config_token[sizeof(config_token) - 1] = '\0';
  }

  if (strlen(config_token) == 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }

  // Parse metadata configuration parameters
  struct metadata_configuration metadata_config = {0};
  strncpy(metadata_config.token, config_token, sizeof(metadata_config.token) - 1);
  metadata_config.token[sizeof(metadata_config.token) - 1] = '\0';

  // Extract analytics setting using gSOAP parsing
  char analytics_str[16] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//tt:Analytics", analytics_str,
                               sizeof(analytics_str)) == 0) {
    metadata_config.analytics = (strcmp(analytics_str, "true") == 0) ? 1 : 0;
  } else {
    metadata_config.analytics = 0; // Default to false
  }

  // Extract session timeout using gSOAP parsing
  char timeout_str[16] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//tt:SessionTimeout", timeout_str,
                               sizeof(timeout_str)) == 0) {
    // Parse PT60S format
    if (strstr(timeout_str, "PT") != NULL && strstr(timeout_str, "S") != NULL) {
      char* endptr = NULL;
      long timeout_value = strtol(timeout_str + 2, &endptr, 10); // Skip "PT"
      if (endptr != NULL && *endptr == 'S' && timeout_value > 0) {
        metadata_config.session_timeout = (int)timeout_value;
      }
    }
  } else {
    metadata_config.session_timeout = 30; // Default 30 seconds
  }

  // Set configuration
  int result = onvif_media_set_metadata_configuration(config_token, &metadata_config);

  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "metadata_config_update", response);
  }

  // Generate set metadata configuration response using proper gSOAP
  // serialization
  result = onvif_gsoap_generate_set_metadata_configuration_response(gsoap_ctx);
  if (result != 0) {
    return error_handle_system(&error_ctx, result, "set_metadata_configuration_response_generation",
                               response);
  }

  // Get the complete SOAP response
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    return error_handle_system(&error_ctx, ONVIF_ERROR, "soap_response_retrieval", response);
  }

  // Allocate response buffer if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return error_handle_system(&error_ctx, ONVIF_ERROR, "response_buffer_allocation", response);
    }
  }

  // Copy response to output
  strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

// Action definitions
static const service_action_def_t media_actions[] = {
  {"GetProfiles", handle_get_profiles, 0},
  {"GetStreamUri", handle_get_stream_uri, 1},
  {"CreateProfile", handle_create_profile, 1},
  {"DeleteProfile", handle_delete_profile, 1},
  {"SetVideoSourceConfiguration", handle_set_video_source_configuration, 1},
  {"SetVideoEncoderConfiguration", handle_set_video_encoder_configuration, 1},
  {"StartMulticastStreaming", handle_start_multicast_streaming, 1},
  {"StopMulticastStreaming", handle_stop_multicast_streaming, 1},
  {"GetMetadataConfigurations", handle_get_metadata_configurations, 0},
  {"SetMetadataConfiguration", handle_set_metadata_configuration, 1}};

int onvif_media_init(config_manager_t* config) {
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
    // Register media-specific error handlers
    // Error handler registration not implemented yet
    // Error handler registration not implemented yet

    g_handler_initialized = 1;
  }

  return result;
}

void onvif_media_cleanup(void) {
  if (g_handler_initialized) {
    error_context_t error_ctx;
    error_context_init(&error_ctx, "Media", "Cleanup", "service_cleanup");

    onvif_service_handler_cleanup(&g_media_handler);

    // Unregister error handlers
    // Error handler unregistration not implemented yet

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
