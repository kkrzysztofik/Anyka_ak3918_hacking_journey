/**
 * @file onvif_media.c
 * @brief ONVIF Media service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_media.h"

#include <bits/types.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "generated/soapH.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_service_common.h"
#include "services/common/onvif_types.h"
#include "services/common/service_dispatcher.h"
#include "services/common/video_config_types.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/memory/smart_response_builder.h"
#include "utils/network/network_utils.h"

#ifdef UNIT_TESTING
#include "services/common/onvif_service_test_helpers.h"
#endif

/* Profile count now determined by runtime configuration (max 4 profiles) */
#define MEDIA_PROFILE_TOKEN_PREFIX "Profile"
#define MEDIA_MAIN_PROFILE_TOKEN   "Profile1"
#define MEDIA_SUB_PROFILE_TOKEN    "Profile2"

#define MEDIA_MAIN_RESOLUTION_WIDTH  1280
#define MEDIA_MAIN_RESOLUTION_HEIGHT 720

// Audio sample rate constants
#define AUDIO_SAMPLE_RATE_8KHZ  8000
#define AUDIO_SAMPLE_RATE_16KHZ 16000
#define AUDIO_SAMPLE_RATE_22KHZ 22050
#define AUDIO_SAMPLE_RATE_44KHZ 44100
#define AUDIO_SAMPLE_RATE_48KHZ 48000

// Network configuration buffer sizes and defaults

// Token parsing constants
#define MEDIA_VIDEO_ENCODER_PREFIX_LEN 12 /* Length of "VideoEncoder" prefix */

// Error message buffer sizes
#define MEDIA_ERROR_MESSAGE_SIZE 512 /* Error message buffer size */

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

/* ============================================================================
 * Forward Declarations and Global State
 * ============================================================================ */

/* Helper functions */
static int validate_profile_token(const char* token);
static int validate_protocol(const char* protocol);
static int build_media_profile_from_config(int profile_index, struct media_profile* profile);
static int get_active_profile_count(void);

/* Cached media profiles dynamically built from runtime configuration */
static struct media_profile g_media_profiles[4] = {{0}}; // NOLINT - max 4 profiles
static int g_profiles_loaded = 0;                        // NOLINT

/* ============================================================================
 * INTERNAL HELPERS - Profile Management
 * ============================================================================ */

/**
 * @brief Get active profile count from runtime configuration
 */
static int get_active_profile_count(void) {
  return config_runtime_get_stream_profile_count();
}

/**
 * @brief Build media_profile structure from runtime configuration
 * @param profile_index Profile index (0-3)
 * @param profile Output media_profile structure
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int build_media_profile_from_config(int profile_index, struct media_profile* profile) {
  if (profile_index < 0 || profile_index >= get_active_profile_count()) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Get video configuration from runtime */
  video_config_t video_config;
  int result = config_runtime_get_stream_profile(profile_index, &video_config);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[MEDIA] Failed to get stream profile %d from configuration\n", profile_index + 1);
    return result;
  }

  /* Initialize profile structure */
  memset(profile, 0, sizeof(struct media_profile));

  /* Set profile token (Profile1, Profile2, etc.) */
  (void)snprintf(profile->token, sizeof(profile->token), "%s%d", MEDIA_PROFILE_TOKEN_PREFIX, profile_index + 1);

  /* Set profile name from configuration, fallback to auto-generated if not set */
  if (video_config.name[0] != '\0') {
    (void)snprintf(profile->name, sizeof(profile->name), "%s", video_config.name);
  } else {
    (void)snprintf(profile->name, sizeof(profile->name), "Video Profile %d", profile_index + 1);
  }

  /* All profiles are fixed (cannot be deleted) */
  profile->fixed = 1;

  /* Configure video source */
  strncpy(profile->video_source.source_token, "VideoSource0", sizeof(profile->video_source.source_token) - 1);
  profile->video_source.bounds.width = video_config.width;
  profile->video_source.bounds.height = video_config.height;
  profile->video_source.bounds.x = 0;
  profile->video_source.bounds.y = 0;

  /* Configure video encoder */
  (void)snprintf(profile->video_encoder.token, sizeof(profile->video_encoder.token), "VideoEncoder%d", profile_index);

  /* Map codec_type to encoding string */
  const char* encoding = "H264"; /* Default */
  if (video_config.codec_type == 0) {
    encoding = "H264";
  } else if (video_config.codec_type == 1) {
    encoding = "H265";
  } else if (video_config.codec_type == 2) {
    encoding = "MJPEG";
  }
  strncpy(profile->video_encoder.encoding, encoding, sizeof(profile->video_encoder.encoding) - 1);

  profile->video_encoder.resolution.width = video_config.width;
  profile->video_encoder.resolution.height = video_config.height;
  profile->video_encoder.quality = (profile_index == 0) ? MEDIA_MAIN_QUALITY_DEFAULT : MEDIA_SUB_QUALITY_DEFAULT;
  profile->video_encoder.framerate_limit = video_config.fps;
  profile->video_encoder.encoding_interval = 1;
  profile->video_encoder.bitrate_limit = video_config.bitrate;
  profile->video_encoder.gov_length = video_config.gop_size;

  /* Configure audio source */
  strncpy(profile->audio_source.source_token, "AudioSource0", sizeof(profile->audio_source.source_token) - 1);

  /* Configure audio encoder */
  (void)snprintf(profile->audio_encoder.token, sizeof(profile->audio_encoder.token), "AudioEncoder%d", profile_index);
  strncpy(profile->audio_encoder.encoding, "AAC", sizeof(profile->audio_encoder.encoding) - 1);
  profile->audio_encoder.bitrate = MEDIA_AUDIO_BITRATE_DEFAULT;
  profile->audio_encoder.sample_rate = AUDIO_SAMPLE_RATE_16KHZ;

  /* Configure PTZ */
  strncpy(profile->ptz.node_token, "PTZNode0", sizeof(profile->ptz.node_token) - 1);
  strncpy(profile->ptz.default_absolute_pan_tilt_position_space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
          sizeof(profile->ptz.default_absolute_pan_tilt_position_space) - 1);
  strncpy(profile->ptz.default_relative_pan_tilt_translation_space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace",
          sizeof(profile->ptz.default_relative_pan_tilt_translation_space) - 1);
  strncpy(profile->ptz.default_continuous_pan_tilt_velocity_space, "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace",
          sizeof(profile->ptz.default_continuous_pan_tilt_velocity_space) - 1);

  platform_log_debug("[MEDIA] Built profile %d: %s - %dx%d@%dfps %dkbps %s\n", profile_index + 1, profile->token, video_config.width,
                     video_config.height, video_config.fps, video_config.bitrate, encoding);

  return ONVIF_SUCCESS;
}

/**
 * @brief Load all profiles from runtime configuration
 */
static int load_profiles_from_config(void) {
  if (g_profiles_loaded) {
    return ONVIF_SUCCESS;
  }

  int profile_count = get_active_profile_count();
  for (int i = 0; i < profile_count; i++) {
    int result = build_media_profile_from_config(i, &g_media_profiles[i]);
    if (result != ONVIF_SUCCESS) {
      platform_log_error("[MEDIA] Failed to load profile %d from configuration\n", i + 1);
      return result;
    }
  }

  g_profiles_loaded = 1;
  platform_log_info("[MEDIA] Loaded %d profiles from runtime configuration\n", profile_count);
  return ONVIF_SUCCESS;
}

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
  build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, g_cached_main_rtsp_uri, sizeof(g_cached_main_rtsp_uri));
  build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, g_cached_sub_rtsp_uri, sizeof(g_cached_sub_rtsp_uri));

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

  /* Ensure profiles are loaded from configuration */
  if (load_profiles_from_config() != ONVIF_SUCCESS) {
    return NULL;
  }

  /* Extract profile number from token (Profile1 -> 0, Profile2 -> 1, etc.) */
  if (strncmp(profile_token, MEDIA_PROFILE_TOKEN_PREFIX, strlen(MEDIA_PROFILE_TOKEN_PREFIX)) == 0) {
    char* endptr;
    long profile_num_long = strtol(profile_token + strlen(MEDIA_PROFILE_TOKEN_PREFIX), &endptr, 10);
    if (endptr != profile_token + strlen(MEDIA_PROFILE_TOKEN_PREFIX) && *endptr == '\0' && profile_num_long >= 1 &&
        profile_num_long <= get_active_profile_count()) {
      int profile_num = (int)profile_num_long;
      return &g_media_profiles[profile_num - 1];
    }
  }

  /* Legacy support for MainProfile/SubProfile tokens */
  if (strcmp(profile_token, "MainProfile") == 0 && get_active_profile_count() >= 1) {
    return &g_media_profiles[0];
  }
  if (strcmp(profile_token, "SubProfile") == 0 && get_active_profile_count() >= 2) {
    return &g_media_profiles[1];
  }

  /* Fall back to linear search */
  int profile_count = get_active_profile_count();
  for (int i = 0; i < profile_count; i++) {
    if (strcmp(g_media_profiles[i].token, profile_token) == 0) {
      return &g_media_profiles[i];
    }
  }

  return NULL;
}

/* ============================================================================
 * PUBLIC API - Profile Operations
 * ============================================================================ */

int onvif_media_get_profiles(struct media_profile** profile_list, int* count) {
  ONVIF_CHECK_NULL(profile_list);
  ONVIF_CHECK_NULL(count);

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    platform_log_error("[MEDIA] Failed to load profiles from configuration\n");
    return result;
  }

  *profile_list = g_media_profiles;
  *count = get_active_profile_count();
  return ONVIF_SUCCESS;
}

int onvif_media_get_profile(const char* profile_token, struct media_profile* profile) {
  ONVIF_CHECK_NULL(profile_token);
  ONVIF_CHECK_NULL(profile);

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Find profile using optimized lookup */
  struct media_profile* found = find_profile_optimized(profile_token);
  if (found) {
    memcpy(profile, found, sizeof(struct media_profile));
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_create_profile(
  const char* name, const char* token,
  struct media_profile* profile) { // NOLINT(bugprone-easily-swappable-parameters) - name and token are semantically different profile attributes
  ONVIF_CHECK_NULL(name);
  ONVIF_CHECK_NULL(token);
  ONVIF_CHECK_NULL(profile);

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Enforce profile limit (T062) - maximum 4 profiles */
  int profile_count = get_active_profile_count();
  if (profile_count >= 4) {
    platform_log_error("[MEDIA] Cannot create profile: maximum limit of 4 profiles reached\n");
    return ONVIF_ERROR_NOT_SUPPORTED;
  }

  /* Check if token already exists */
  for (int i = 0; i < profile_count; i++) {
    if (strcmp(g_media_profiles[i].token, token) == 0) {
      platform_log_error("[MEDIA] Cannot create profile: token '%s' already exists\n", token);
      return ONVIF_ERROR_DUPLICATE;
    }
  }

  /* For now, profile creation is not fully supported as profiles are managed via configuration */
  platform_log_warning("[MEDIA] Profile creation requested but not implemented: use configuration system\n");
  return ONVIF_ERROR_NOT_SUPPORTED;
}

int onvif_media_delete_profile(const char* profile_token) {
  ONVIF_CHECK_NULL(profile_token);

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Find the profile */
  struct media_profile* found = find_profile_optimized(profile_token);
  if (!found) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* All configuration-managed profiles are fixed and cannot be deleted */
  if (found->fixed) {
    platform_log_error("[MEDIA] Cannot delete fixed profile: %s\n", profile_token);
    return ONVIF_ERROR_NOT_SUPPORTED;
  }

  /* Profile deletion not supported for config-managed profiles */
  platform_log_warning("[MEDIA] Profile deletion requested but not implemented: use configuration system\n");
  return ONVIF_ERROR_NOT_SUPPORTED;
}

/* ============================================================================
 * PUBLIC API - Video/Audio Source Operations
 * ============================================================================ */

int onvif_media_get_video_sources(struct video_source** sources, int* count) {
  static struct video_source video_sources[] = {{.token = "VideoSource0",
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

int onvif_media_get_video_source_configurations(struct video_source_configuration** configs, int* count) {
  static struct video_source_configuration video_configs[] = {{.token = "VideoSourceConfig0",
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

int onvif_media_get_video_encoder_configurations(struct video_encoder_configuration** configs, int* count) {
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

int onvif_media_get_audio_source_configurations(struct audio_source_configuration** configs, int* count) {
  static struct audio_source_configuration audio_configs[] = {
    {.token = "AudioSourceConfig0", .name = "Audio Source Configuration", .use_count = 2, .source_token = "AudioSource0"}};

  ONVIF_CHECK_NULL(configs);
  ONVIF_CHECK_NULL(count);

  *configs = audio_configs;
  *count = 1;
  return ONVIF_SUCCESS;
}

int onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration** configs, int* count) {
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

int onvif_media_set_metadata_configuration(const char* configuration_token, const struct metadata_configuration* config) {
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

/* ============================================================================
 * PUBLIC API - Configuration Management
 * ============================================================================ */

int onvif_media_set_video_source_configuration(const char* configuration_token, const struct video_source_configuration* config) {
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

int onvif_media_set_video_encoder_configuration(const char* configuration_token, const struct video_encoder_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  // For now, we only support updating the default video encoder configurations
  if (strcmp(configuration_token, "VideoEncoder0") == 0 || strcmp(configuration_token, "VideoEncoder1") == 0) {
    platform_log_info("Updated video encoder configuration: %s\n", configuration_token);
    platform_log_info("  Resolution: %dx%d\n", config->resolution.width, config->resolution.height);
    platform_log_info("  Bitrate: %d kbps\n", config->bitrate_limit);
    platform_log_info("  Framerate: %d fps\n", config->framerate_limit);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_audio_source_configuration(const char* configuration_token, const struct audio_source_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  if (strcmp(configuration_token, "AudioSourceConfig0") == 0) {
    platform_log_info("Updated audio source configuration: %s\n", configuration_token);
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_audio_encoder_configuration(const char* configuration_token, const struct audio_encoder_configuration* config) {
  ONVIF_CHECK_NULL(configuration_token);
  ONVIF_CHECK_NULL(config);

  // Validate AAC-specific parameters
  if (strcmp(config->encoding, "AAC") == 0) {
    // Validate bitrate for AAC
    if (config->bitrate < MEDIA_AUDIO_BITRATE_AAC_MIN || config->bitrate > MEDIA_AUDIO_BITRATE_AAC_MAX) {
      platform_log_error("Invalid AAC bitrate: %d kbps (must be %d-%d)\n", config->bitrate, MEDIA_AUDIO_BITRATE_AAC_MIN, MEDIA_AUDIO_BITRATE_AAC_MAX);
      return ONVIF_ERROR_INVALID;
    }

    // Validate sample rate for AAC
    if (config->sample_rate < MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN || config->sample_rate > MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX) {
      platform_log_error("Invalid AAC sample rate: %d Hz (must be %d-%d)\n", config->sample_rate, MEDIA_AUDIO_SAMPLE_RATE_AAC_MIN,
                         MEDIA_AUDIO_SAMPLE_RATE_AAC_MAX);
      return ONVIF_ERROR_INVALID;
    }

    // Validate sample rate is supported by Anyka platform
    if (config->sample_rate != AUDIO_SAMPLE_RATE_8KHZ && config->sample_rate != AUDIO_SAMPLE_RATE_16KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_22KHZ && config->sample_rate != AUDIO_SAMPLE_RATE_44KHZ &&
        config->sample_rate != AUDIO_SAMPLE_RATE_48KHZ) {
      platform_log_error("Unsupported AAC sample rate: %d Hz (supported: 8000, 16000, 22050, "
                         "44100, 48000)\n",
                         config->sample_rate);
      return ONVIF_ERROR_NOT_SUPPORTED;
    }
  }

  // Check if configuration token is valid
  if (strcmp(configuration_token, "AudioEncoder0") == 0 || strcmp(configuration_token, "AudioEncoder1") == 0 ||
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

/* ============================================================================
 * PUBLIC API - Stream URI Operations
 * ============================================================================ */

int onvif_media_get_stream_uri(const char* profile_token, const char* protocol, struct stream_uri* uri) {
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
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, uri->uri, sizeof(uri->uri));
    } else {
      build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, uri->uri, sizeof(uri->uri));
    }
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = MEDIA_DEFAULT_TIMEOUT_SECONDS;
    return ONVIF_SUCCESS;
  }

  // Handle other protocols
  if (strcmp(protocol, "HTTP") == 0) {
    build_device_url("http", HTTP_PORT_DEFAULT, "/onvif/streaming", uri->uri, sizeof(uri->uri));
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

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Check if profile exists */
  struct media_profile* found = find_profile_optimized(profile_token);
  if (!found) {
    return ONVIF_ERROR_NOT_FOUND;
  }

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

  /* Load profiles from runtime configuration */
  int result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Check if profile exists */
  struct media_profile* found = find_profile_optimized(profile_token);
  if (!found) {
    return ONVIF_ERROR_NOT_FOUND;
  }

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

// Helper function to parse value from request body
static int parse_value_from_request(const char* request_body, onvif_gsoap_context_t* gsoap_ctx, const char* xpath, char* value, size_t value_size) {
  if (!request_body || !gsoap_ctx || !xpath || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Stub: Return default/empty value since old XML parsing was removed
  // Handlers using this need to be migrated to new gSOAP parsing
  value[0] = '\0';
  return ONVIF_SUCCESS;
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
static int get_profiles_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                       void* callback_data);

static int get_stream_uri_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int create_profile_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int delete_profile_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data);

static int set_video_source_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                         http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                         error_context_t* error_ctx, void* callback_data);

static int set_video_encoder_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                          http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                          error_context_t* error_ctx, void* callback_data);

static int start_multicast_streaming_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                                    onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                                    void* callback_data);

static int stop_multicast_streaming_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                                   onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                                   void* callback_data);

static int get_metadata_configurations_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                      http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                      error_context_t* error_ctx, void* callback_data);

static int set_metadata_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                                     onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                                     void* callback_data);

/**
 * @brief GetProfiles service operation definition
 */
static const onvif_service_operation_t get_profiles_operation = {.service_name = "Media",
                                                                 .operation_name = "GetProfiles",
                                                                 .operation_context = "media_profiles_retrieval",
                                                                 .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                                                                               .execute_business_logic = get_profiles_business_logic,
                                                                               .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief GetStreamUri service operation definition
 */
static const onvif_service_operation_t get_stream_uri_operation = {.service_name = "Media",
                                                                   .operation_name = "GetStreamUri",
                                                                   .operation_context = "media_stream_uri_retrieval",
                                                                   .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                                                                                 .execute_business_logic = get_stream_uri_business_logic,
                                                                                 .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief CreateProfile service operation definition
 */
static const onvif_service_operation_t create_profile_operation = {.service_name = "Media",
                                                                   .operation_name = "CreateProfile",
                                                                   .operation_context = "media_profile_creation",
                                                                   .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                                                                                 .execute_business_logic = create_profile_business_logic,
                                                                                 .post_process_response = onvif_util_standard_post_process}};

/**
 * @brief DeleteProfile service operation definition
 */
static const onvif_service_operation_t delete_profile_operation = {.service_name = "Media",
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
static int handle_get_profiles(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                               onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetProfiles
  media_profiles_callback_data_t callback_data = {.profiles = NULL, .profile_count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &get_profiles_operation, media_profiles_response_callback,
                                           &callback_data);
}

static int handle_get_stream_uri(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetStreamUri
  media_stream_uri_callback_data_t callback_data = {.uri = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &get_stream_uri_operation, media_stream_uri_response_callback,
                                           &callback_data);
}

static int handle_create_profile(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for CreateProfile
  media_create_profile_callback_data_t callback_data = {.profile = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &create_profile_operation, media_create_profile_response_callback,
                                           &callback_data);
}

static int handle_delete_profile(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for DeleteProfile
  media_delete_profile_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &delete_profile_operation, media_delete_profile_response_callback,
                                           &callback_data);
}

static int handle_set_video_source_configuration(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                                 onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetVideoSourceConfiguration
  media_set_video_source_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &set_video_source_configuration_operation,
                                           media_set_video_source_config_response_callback, &callback_data);
}

static int handle_set_video_encoder_configuration(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                                  onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetVideoEncoderConfiguration
  media_set_video_encoder_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &set_video_encoder_configuration_operation,
                                           media_set_video_encoder_config_response_callback, &callback_data);
}

static int handle_start_multicast_streaming(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for StartMulticastStreaming
  media_start_multicast_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &start_multicast_streaming_operation,
                                           media_start_multicast_response_callback, &callback_data);
}

static int handle_stop_multicast_streaming(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for StopMulticastStreaming
  media_stop_multicast_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &stop_multicast_streaming_operation,
                                           media_stop_multicast_response_callback, &callback_data);
}

static int handle_get_metadata_configurations(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                              onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for GetMetadataConfigurations
  media_get_metadata_configs_callback_data_t callback_data = {.configs = NULL, .config_count = 0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &get_metadata_configurations_operation,
                                           media_get_metadata_configs_response_callback, &callback_data);
}

static int handle_set_metadata_configuration(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                             onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for SetMetadataConfiguration
  media_set_metadata_config_callback_data_t callback_data = {.message = NULL};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx, &set_metadata_configuration_operation,
                                           media_set_metadata_config_response_callback, &callback_data);
}

/* ============================================================================
 * Business Logic Callback Functions
 * ============================================================================ */

/**
 * @brief Business logic callback for GetProfiles operation
 */
static int get_profiles_business_logic(const service_handler_config_t* config, // NOLINT
                                       const http_request_t* request,
                                       http_response_t* response, // NOLINT
                                       onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                       error_context_t* error_ctx, // NOLINT
                                       void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing GetProfiles request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse GetProfiles request (empty request structure)
  struct _trt__GetProfiles* profiles_req = NULL;
  result = onvif_gsoap_parse_get_profiles(gsoap_ctx, &profiles_req);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "parse_get_profiles", result, "Failed to parse GetProfiles request");
    return result;
  }

  media_profiles_callback_data_t* profile_data = (media_profiles_callback_data_t*)callback_data;

  /* Load profiles from runtime configuration */
  result = load_profiles_from_config();
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "load_profiles", result, "Failed to load profiles from configuration");
    return result;
  }

  // Set callback data with configuration-based profiles
  profile_data->profiles = g_media_profiles;
  profile_data->profile_count = get_active_profile_count();

  service_log_info(log_ctx, "GetProfiles request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetStreamUri operation
 */
static int get_stream_uri_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing GetStreamUri request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse GetStreamUri request using new gSOAP parsing function
  struct _trt__GetStreamUri* stream_uri_req = NULL;
  result = onvif_gsoap_parse_get_stream_uri(gsoap_ctx, &stream_uri_req);
  if (result != ONVIF_SUCCESS || !stream_uri_req) {
    service_log_operation_failure(log_ctx, "parse_get_stream_uri", result, "Failed to parse GetStreamUri request");
    return error_handle_parameter(error_ctx, "GetStreamUri", "parse_failed", response);
  }

  // Extract profile token from parsed structure
  if (!stream_uri_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = stream_uri_req->ProfileToken;

  // Extract protocol from StreamSetup
  if (!stream_uri_req->StreamSetup || !stream_uri_req->StreamSetup->Transport) {
    return error_handle_parameter(error_ctx, "StreamSetup.Transport", "missing", response);
  }

  // Convert protocol enum to string
  const char* protocol = NULL;
  switch (stream_uri_req->StreamSetup->Transport->Protocol) {
  case tt__TransportProtocol__UDP:
    protocol = "UDP";
    break;
  case tt__TransportProtocol__HTTP:
    protocol = "HTTP";
    break;
  case tt__TransportProtocol__RTSP:
  default:
    protocol = "RTSP"; // RTSP or default to RTSP
    break;
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
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, media_stream_uri_response_callback, callback_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result, "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1, "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  result = smart_response_build_with_dynamic_buffer(response, soap_response);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build_with_dynamic_buffer", result, "Failed to build smart response");
    return result;
  }

  service_log_info(log_ctx, "GetStreamUri request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for CreateProfile operation
 */
static int create_profile_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing CreateProfile request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse CreateProfile request using new gSOAP parsing function
  struct _trt__CreateProfile* create_profile_req = NULL;
  result = onvif_gsoap_parse_create_profile(gsoap_ctx, &create_profile_req);
  if (result != ONVIF_SUCCESS || !create_profile_req) {
    service_log_operation_failure(log_ctx, "parse_create_profile", result, "Failed to parse CreateProfile request");
    return error_handle_parameter(error_ctx, "CreateProfile", "parse_failed", response);
  }

  // Extract profile name and token from parsed structure
  const char* profile_name = create_profile_req->Name ? create_profile_req->Name : "Custom Profile";
  const char* profile_token = create_profile_req->Token ? *create_profile_req->Token : "CustomProfile";

  // Check current profile count before attempting creation
  int profile_count = get_active_profile_count();

  // Create profile
  struct media_profile profile;
  result = onvif_media_create_profile(profile_name, profile_token, &profile);
  if (result != ONVIF_SUCCESS) {
    // Provide specific error messages based on the failure reason
    if (result == ONVIF_ERROR_NOT_SUPPORTED && profile_count >= 4) {
      // Maximum profiles reached
      return error_handle_pattern(error_ctx, ERROR_PATTERN_INTERNAL_ERROR, "Maximum limit of 4 profiles reached", response);
    }
    if (result == ONVIF_ERROR_DUPLICATE) {
      // Token already exists
      char error_msg[MEDIA_ERROR_MESSAGE_SIZE];
      (void)snprintf(error_msg, sizeof(error_msg), "Profile token '%s' already exists", profile_token);
      return error_handle_pattern(error_ctx, ERROR_PATTERN_INTERNAL_ERROR, error_msg, response);
    }
    // Other errors - use generic handler
    return error_handle_system(error_ctx, result, "create_profile", response);
  }

  // Update callback data
  media_create_profile_callback_data_t* profile_data = (media_create_profile_callback_data_t*)callback_data;
  profile_data->profile = &profile;

  service_log_info(log_ctx, "CreateProfile request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for DeleteProfile operation
 */
static int delete_profile_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx, error_context_t* error_ctx,
                                         void* callback_data) {
  (void)config;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  service_log_info(log_ctx, "Processing DeleteProfile request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse DeleteProfile request using new gSOAP parsing function
  struct _trt__DeleteProfile* delete_profile_req = NULL;
  result = onvif_gsoap_parse_delete_profile(gsoap_ctx, &delete_profile_req);
  if (result != ONVIF_SUCCESS || !delete_profile_req) {
    service_log_operation_failure(log_ctx, "parse_delete_profile", result, "Failed to parse DeleteProfile request");
    return error_handle_parameter(error_ctx, "DeleteProfile", "parse_failed", response);
  }

  // Extract profile token from parsed structure
  if (!delete_profile_req->ProfileToken) {
    return error_handle_parameter(error_ctx, "ProfileToken", "missing", response);
  }
  const char* profile_token = delete_profile_req->ProfileToken;

  // Delete profile
  result = onvif_media_delete_profile(profile_token);
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "delete_profile", response);
  }

  // Update callback data
  media_delete_profile_callback_data_t* delete_data = (media_delete_profile_callback_data_t*)callback_data;
  delete_data->message = "Profile deleted successfully";

  service_log_info(log_ctx, "DeleteProfile request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SetVideoSourceConfiguration operation
 */
static int set_video_source_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                         http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                         error_context_t* error_ctx, void* callback_data) {
  (void)config;
  (void)error_ctx;
  (void)callback_data;

  service_log_info(log_ctx, "Processing SetVideoSourceConfiguration request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse SetVideoSourceConfiguration request using new gSOAP parsing function
  struct _trt__SetVideoSourceConfiguration* set_video_source_req = NULL;
  result = onvif_gsoap_parse_set_video_source_config(gsoap_ctx, &set_video_source_req);
  if (result != ONVIF_SUCCESS || !set_video_source_req) {
    service_log_operation_failure(log_ctx, "parse_set_video_source_config", result, "Failed to parse SetVideoSourceConfiguration request");
    return error_handle_parameter(error_ctx, "SetVideoSourceConfiguration", "parse_failed", response);
  }

  // Extract configuration from parsed structure
  if (!set_video_source_req->Configuration || !set_video_source_req->Configuration->token) {
    return error_handle_parameter(error_ctx, "Configuration.token", "missing", response);
  }
  const char* config_token = set_video_source_req->Configuration->token;

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
 *
 * This function implements T063: Runtime profile updates via SetVideoEncoderConfiguration.
 * It extracts encoder parameters from the ONVIF request, updates the runtime configuration,
 * and reloads the cached profile to ensure changes take effect immediately.
 */
static int set_video_encoder_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                          http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                          error_context_t* error_ctx, void* callback_data) {
  (void)config;
  (void)callback_data;

  service_log_info(log_ctx, "Processing SetVideoEncoderConfiguration request");

  // Initialize gSOAP context for request parsing
  int result = onvif_gsoap_init_request_parsing(gsoap_ctx, request->body, strlen(request->body));
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_request_parsing", result, "Failed to initialize gSOAP request parsing");
    return ONVIF_ERROR;
  }

  // Parse SetVideoEncoderConfiguration request using gSOAP parsing function
  struct _trt__SetVideoEncoderConfiguration* set_video_encoder_req = NULL;
  result = onvif_gsoap_parse_set_video_encoder_config(gsoap_ctx, &set_video_encoder_req);
  if (result != ONVIF_SUCCESS || !set_video_encoder_req) {
    service_log_operation_failure(log_ctx, "parse_set_video_encoder_config", result, "Failed to parse SetVideoEncoderConfiguration request");
    return error_handle_parameter(error_ctx, "SetVideoEncoderConfiguration", "parse_failed", response);
  }

  // Extract and validate configuration from parsed structure
  if (!set_video_encoder_req->Configuration || !set_video_encoder_req->Configuration->token) {
    return error_handle_parameter(error_ctx, "Configuration.token", "missing", response);
  }

  struct tt__VideoEncoderConfiguration* onvif_config = set_video_encoder_req->Configuration;
  const char* config_token = onvif_config->token;

  // Map encoder token to profile index (VideoEncoder0 -> 0, VideoEncoder1 -> 1, etc.)
  int profile_index = -1;
  if (strncmp(config_token, "VideoEncoder", MEDIA_VIDEO_ENCODER_PREFIX_LEN) == 0) {
    char* endptr;
    long profile_index_long = strtol(config_token + MEDIA_VIDEO_ENCODER_PREFIX_LEN, &endptr, 10);
    if (endptr != config_token + MEDIA_VIDEO_ENCODER_PREFIX_LEN && *endptr == '\0' && profile_index_long >= 0 && profile_index_long < INT_MAX) {
      profile_index = (int)profile_index_long;
    }
  }

  if (profile_index < 0 || profile_index >= get_active_profile_count()) {
    service_log_operation_failure(log_ctx, "map_encoder_token", profile_index, "Invalid encoder token or profile index");
    return error_handle_parameter(error_ctx, "Configuration.token", "invalid", response);
  }

  // Get current profile configuration from runtime
  video_config_t video_config;
  result = config_runtime_get_stream_profile(profile_index, &video_config);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "get_stream_profile", result, "Failed to retrieve current profile configuration");
    return error_handle_system(error_ctx, result, "get_stream_profile", response);
  }

  // Update video_config with ONVIF parameters (T063)

  // Update resolution if provided
  if (onvif_config->Resolution) {
    video_config.width = onvif_config->Resolution->Width;
    video_config.height = onvif_config->Resolution->Height;
    service_log_info(log_ctx, "Updated resolution: %dx%d", video_config.width, video_config.height);
  }

  // Update rate control parameters if provided
  if (onvif_config->RateControl) {
    video_config.fps = onvif_config->RateControl->FrameRateLimit;
    video_config.bitrate = onvif_config->RateControl->BitrateLimit;
    service_log_info(log_ctx, "Updated rate control: %d fps, %d kbps", video_config.fps, video_config.bitrate);
  }

  // Update GOP length from codec-specific configuration
  if (onvif_config->H264 && onvif_config->H264->GovLength > 0) {
    video_config.gop_size = onvif_config->H264->GovLength;
    video_config.codec_type = 0; // H.264
    service_log_info(log_ctx, "Updated H.264 GOP length: %d", video_config.gop_size);
  } else if (onvif_config->MPEG4 && onvif_config->MPEG4->GovLength > 0) {
    video_config.gop_size = onvif_config->MPEG4->GovLength;
    video_config.codec_type = 0; // Treat as H.264 (MPEG4 not widely used)
    service_log_info(log_ctx, "Updated MPEG4 GOP length: %d", video_config.gop_size);
  }

  // Map encoding type to codec_type
  switch (onvif_config->Encoding) {
  case tt__VideoEncoding__JPEG:
    video_config.codec_type = 2; // MJPEG
    break;
  case tt__VideoEncoding__MPEG4:
  case tt__VideoEncoding__H264:
  default:
    video_config.codec_type = 0; // H.264, MPEG4 (treated as H.264), or default to H.264
    break;
  }

  // Update runtime configuration with validation
  result = config_runtime_set_stream_profile(profile_index, &video_config);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "set_stream_profile", result, "Failed to update runtime profile configuration");
    return error_handle_system(error_ctx, result, "set_stream_profile", response);
  }

  // Reload cached profile to apply changes immediately
  result = build_media_profile_from_config(profile_index, &g_media_profiles[profile_index]);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "reload_cached_profile", result, "Failed to reload cached profile after update");
    return error_handle_system(error_ctx, result, "reload_cached_profile", response);
  }

  service_log_info(log_ctx, "SetVideoEncoderConfiguration completed: Profile %d updated successfully", profile_index + 1);

  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for StartMulticastStreaming operation
 */
static int start_multicast_streaming_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
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

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token, sizeof(profile_token)) != 0) {
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
static int stop_multicast_streaming_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
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

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ProfileToken", profile_token, sizeof(profile_token)) != 0) {
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
static int get_metadata_configurations_business_logic(const service_handler_config_t* config, const http_request_t* request,
                                                      http_response_t* response, onvif_gsoap_context_t* gsoap_ctx, service_log_context_t* log_ctx,
                                                      error_context_t* error_ctx, void* callback_data) {
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
  media_get_metadata_configs_callback_data_t* config_data = (media_get_metadata_configs_callback_data_t*)callback_data;
  config_data->configs = configs;
  config_data->config_count = count;

  service_log_info(log_ctx, "GetMetadataConfigurations request completed successfully");
  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SetMetadataConfiguration operation
 */
static int set_metadata_configuration_business_logic(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
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

  if (parse_value_from_request(request->body, gsoap_ctx, "//trt:ConfigurationToken", config_token, sizeof(config_token)) != 0) {
    return error_handle_parameter(error_ctx, "configuration_token", "missing", response);
  }

  // Parse metadata configuration parameters
  struct metadata_configuration metadata_config = {0};
  strncpy(metadata_config.token, config_token, sizeof(metadata_config.token) - 1);
  strncpy(metadata_config.name, "Updated Metadata Configuration", sizeof(metadata_config.name) - 1);

  // Parse analytics setting
  char analytics_str[MEDIA_ANALYTICS_STR_SIZE] = "";
  if (parse_value_from_request(request->body, gsoap_ctx, "//tt:Analytics", analytics_str, sizeof(analytics_str)) == 0) {
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
static const service_action_def_t media_actions[] = {{"GetProfiles", handle_get_profiles, 1},
                                                     {"GetStreamUri", handle_get_stream_uri, 1},
                                                     {"CreateProfile", handle_create_profile, 1},
                                                     {"DeleteProfile", handle_delete_profile, 1},
                                                     {"SetVideoSourceConfiguration", handle_set_video_source_configuration, 1},
                                                     {"SetVideoEncoderConfiguration", handle_set_video_encoder_configuration, 1},
                                                     {"StartMulticastStreaming", handle_start_multicast_streaming, 1},
                                                     {"StopMulticastStreaming", handle_stop_multicast_streaming, 1},
                                                     {"GetMetadataConfigurations", handle_get_metadata_configurations, 1},
                                                     {"SetMetadataConfiguration", handle_set_metadata_configuration, 1}};

/**
 * @brief Generate Media service capability structure
 * @param ctx gSOAP context for memory allocation
 * @param capabilities_ptr Output pointer to tt__MediaCapabilities*
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
static int media_service_get_capabilities(struct soap* ctx, void** capabilities_ptr) {
  if (!ctx || !capabilities_ptr) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate MediaCapabilities structure
  struct tt__MediaCapabilities* caps = soap_new_tt__MediaCapabilities(ctx, 1);
  if (!caps) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }
  soap_default_tt__MediaCapabilities(ctx, caps);

  // Get device IP and port from runtime config with defaults
  char device_ip[ONVIF_IP_BUFFER_SIZE] = "192.168.1.100";
  int http_port = HTTP_PORT_DEFAULT;
  config_runtime_get_string(CONFIG_SECTION_NETWORK, "device_ip", device_ip, sizeof(device_ip));
  config_runtime_get_int(CONFIG_SECTION_NETWORK, "http_port", &http_port);

  // Build XAddr
  char xaddr[ONVIF_XADDR_BUFFER_SIZE];
  (void)snprintf(xaddr, sizeof(xaddr), "http://%s:%d/onvif/media_service", device_ip, http_port);
  caps->XAddr = soap_strdup(ctx, xaddr);

  // StreamingCapabilities (REQUIRED for Media)
  caps->StreamingCapabilities = soap_new_tt__RealTimeStreamingCapabilities(ctx, 1);
  if (!caps->StreamingCapabilities) {
    return ONVIF_ERROR_MEMORY_ALLOCATION;
  }
  soap_default_tt__RealTimeStreamingCapabilities(ctx, caps->StreamingCapabilities);

  caps->StreamingCapabilities->RTPMulticast = (enum xsd__boolean*)soap_malloc(ctx, sizeof(enum xsd__boolean));
  if (caps->StreamingCapabilities->RTPMulticast) {
    *caps->StreamingCapabilities->RTPMulticast = xsd__boolean__false_;
  }
  caps->StreamingCapabilities->RTP_USCORETCP = (enum xsd__boolean*)soap_malloc(ctx, sizeof(enum xsd__boolean));
  if (caps->StreamingCapabilities->RTP_USCORETCP) {
    *caps->StreamingCapabilities->RTP_USCORETCP = xsd__boolean__true_;
  }
  caps->StreamingCapabilities->RTP_USCORERTSP_USCORETCP = (enum xsd__boolean*)soap_malloc(ctx, sizeof(enum xsd__boolean));
  if (caps->StreamingCapabilities->RTP_USCORERTSP_USCORETCP) {
    *caps->StreamingCapabilities->RTP_USCORERTSP_USCORETCP = xsd__boolean__true_;
  }

  *capabilities_ptr = (void*)caps;
  return ONVIF_SUCCESS;
}

int onvif_media_init(void) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  service_handler_config_t handler_config = {.service_type = ONVIF_SERVICE_MEDIA,
                                             .service_name = "Media",
                                             .config = NULL, // No longer using old config system
                                             .enable_validation = 1,
                                             .enable_logging = 1};

  int result = onvif_service_handler_init(&g_media_handler, &handler_config, media_actions, sizeof(media_actions) / sizeof(media_actions[0]));

  if (result == ONVIF_SUCCESS) {
    // Initialize buffer pool for medium-sized responses
    if (buffer_pool_init(&g_media_response_buffer_pool) != 0) {
      onvif_service_handler_cleanup(&g_media_handler);
      return ONVIF_ERROR;
    }

    // Set flag BEFORE registration (enables cleanup on failure)
    g_handler_initialized = 1;

    // Register with service dispatcher using standardized interface
    onvif_service_registration_t registration = {.service_name = "Media",
                                                 .namespace_uri = "http://www.onvif.org/ver10/media/wsdl",
                                                 .operation_handler = onvif_media_handle_request,
                                                 .init_handler = NULL,
                                                 .cleanup_handler = NULL,
                                                 .capabilities_handler = NULL,
                                                 .get_capabilities = media_service_get_capabilities,
                                                 .reserved = {NULL, NULL, NULL}};
#ifdef UNIT_TESTING
    int dispatch_result = onvif_service_unit_register(&registration, &g_handler_initialized, onvif_media_cleanup, "Media");
    if (dispatch_result != ONVIF_SUCCESS) {
      return dispatch_result;
    }
#else
    int dispatch_result = onvif_service_dispatcher_register_service(&registration);
    if (dispatch_result != ONVIF_SUCCESS) {
      platform_log_error("Failed to register media service with dispatcher: %d\n", dispatch_result);
      onvif_media_cleanup();
      return dispatch_result;
    }

    platform_log_info("Media service initialized and registered with dispatcher\n");
#endif
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

int onvif_media_handle_request(const char* action_name, const http_request_t* request, http_response_t* response) {
  if (!g_handler_initialized) {
#ifdef UNIT_TESTING
    platform_log_warning("Media service handle_request invoked before initialization; returning NOT_INITIALIZED\n");
    return ONVIF_ERROR_NOT_INITIALIZED;
#else
    return ONVIF_ERROR;
#endif
  }
  return onvif_service_handler_handle_request(&g_media_handler, action_name, request, response);
}

#ifdef UNIT_TESTING
int onvif_media_unit_init_cached_uris(void) {
  return init_cached_uris();
}

void onvif_media_unit_reset_cached_uris(void) {
  memset(g_cached_main_rtsp_uri, 0, sizeof(g_cached_main_rtsp_uri));
  memset(g_cached_sub_rtsp_uri, 0, sizeof(g_cached_sub_rtsp_uri));
  g_cached_uris_initialized = 0;
}

int onvif_media_unit_get_cached_rtsp_uri(const char* profile_token, char* uri_buffer, size_t buffer_size) {
  return get_cached_rtsp_uri(profile_token, uri_buffer, buffer_size);
}

const char* onvif_media_unit_get_main_profile_token(void) {
  return MEDIA_MAIN_PROFILE_TOKEN;
}

const char* onvif_media_unit_get_sub_profile_token(void) {
  return MEDIA_SUB_PROFILE_TOKEN;
}

int onvif_media_unit_validate_profile_token(const char* token) {
  return validate_profile_token(token);
}

int onvif_media_unit_validate_protocol(const char* protocol) {
  return validate_protocol(protocol);
}

int onvif_media_unit_parse_value_from_request(const char* request_body, onvif_gsoap_context_t* gsoap_ctx, const char* xpath, char* value,
                                              size_t value_size) {
  return parse_value_from_request(request_body, gsoap_ctx, xpath, value, value_size);
}
#endif
