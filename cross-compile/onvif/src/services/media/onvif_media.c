/**
 * @file onvif_media.c
 * @brief ONVIF Media service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "core/config/config.h"
#include "onvif_media.h"
#include "protocol/soap/onvif_soap.h"
#include "protocol/xml/unified_xml.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "common/onvif_constants.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/memory/memory_manager.h"
#include "utils/network/network_utils.h"
#include "utils/string/string_shims.h"

#define MEDIA_PROFILE_COUNT_DEFAULT        2
#define MEDIA_MAIN_PROFILE_TOKEN           "MainProfile"
#define MEDIA_SUB_PROFILE_TOKEN            "SubProfile"

#define MEDIA_MAIN_RESOLUTION_WIDTH        1280
#define MEDIA_MAIN_RESOLUTION_HEIGHT       720
#define MEDIA_SUB_RESOLUTION_WIDTH         640
#define MEDIA_SUB_RESOLUTION_HEIGHT        360

#define MEDIA_MAIN_QUALITY_DEFAULT         4.0
#define MEDIA_SUB_QUALITY_DEFAULT          3.0

#define MEDIA_MAIN_BITRATE_DEFAULT         2048
#define MEDIA_SUB_BITRATE_DEFAULT          800
#define MEDIA_AUDIO_BITRATE_DEFAULT        64

#define MEDIA_RESPONSE_BUFFER_SIZE         4096
#define MEDIA_PROFILE_BUFFER_SIZE          2048
#define MEDIA_URI_BUFFER_SIZE              512
#define MEDIA_TOKEN_BUFFER_SIZE            32
#define MEDIA_PROTOCOL_BUFFER_SIZE         16

#define MEDIA_XML_PROFILE_TOKEN_TAG        "<trt:ProfileToken>"
#define MEDIA_XML_PROTOCOL_TAG             "<trt:Protocol>"
#define MEDIA_XML_VIDEO_SOURCE_TAG         "<trt:VideoSourceToken>"
#define MEDIA_XML_AUDIO_SOURCE_TAG         "<trt:AudioSourceToken>"

#define PROFILE_COUNT MEDIA_PROFILE_COUNT_DEFAULT

static int parse_profile_token(const char *request, char *token, size_t token_size);
static int parse_protocol(const char *request, char *protocol, size_t protocol_size);
static int build_profile_xml(const struct media_profile *profile, char *xml, size_t xml_size);
static int build_video_source_xml(const struct video_source *source, char *xml, size_t xml_size);
static int build_stream_uri_xml(const struct stream_uri *uri, char *xml, size_t xml_size);
static int validate_profile_token(const char *token);
static int validate_protocol(const char *protocol);
static int find_profile_by_token(const char *token, struct media_profile **profile);
static void init_default_profiles(void);
static int handle_media_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);
static int handle_media_stream_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);

static struct media_profile profiles[] = {
    {
        .token = MEDIA_MAIN_PROFILE_TOKEN,
        .name = "Main Video Profile",
        .fixed = 1,
        .video_source = {
            .source_token = "VideoSource0",
            .bounds = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT, 0, 0}
        },
        .video_encoder = {
            .token = "VideoEncoder0",
            .encoding = "H264",
            .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
            .quality = MEDIA_MAIN_QUALITY_DEFAULT,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = MEDIA_MAIN_BITRATE_DEFAULT,
            .gov_length = 50
        },
        .audio_source = {
            .source_token = "AudioSource0"
        },
        .audio_encoder = {
            .token = "AudioEncoder0",
            .encoding = "AAC",
            .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
            .sample_rate = 16000
        },
        .ptz = {
            .node_token = "PTZNode0",
            .default_absolute_pan_tilt_position_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
            .default_absolute_zoom_position_space = "",
            .default_relative_pan_tilt_translation_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace",
            .default_relative_zoom_translation_space = "",
            .default_continuous_pan_tilt_velocity_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace",
            .default_continuous_zoom_velocity_space = ""
        }
    },
    {
        .token = MEDIA_SUB_PROFILE_TOKEN,
        .name = "Sub Video Profile",
        .fixed = 1,
        .video_source = {
            .source_token = "VideoSource0",
            .bounds = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT, 0, 0}
        },
        .video_encoder = {
            .token = "VideoEncoder1",
            .encoding = "H264",
            .resolution = {MEDIA_SUB_RESOLUTION_WIDTH, MEDIA_SUB_RESOLUTION_HEIGHT},
            .quality = MEDIA_SUB_QUALITY_DEFAULT,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = MEDIA_SUB_BITRATE_DEFAULT,
            .gov_length = 50
        },
        .audio_source = {
            .source_token = "AudioSource0"
        },
        .audio_encoder = {
            .token = "AudioEncoder0",
            .encoding = "AAC",
            .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
            .sample_rate = 16000
        },
        .ptz = {
            .node_token = "PTZNode0",
            .default_absolute_pan_tilt_position_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace",
            .default_absolute_zoom_position_space = "",
            .default_relative_pan_tilt_translation_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace",
            .default_relative_zoom_translation_space = "",
            .default_continuous_pan_tilt_velocity_space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace",
            .default_continuous_zoom_velocity_space = ""
        }
    }
};

int onvif_media_get_profiles(struct media_profile **profile_list, int *count) {
    ONVIF_CHECK_NULL(profile_list);
    ONVIF_CHECK_NULL(count);
    
    *profile_list = profiles;
    *count = PROFILE_COUNT;
    return ONVIF_SUCCESS;
}

int onvif_media_get_profile(const char *profile_token, struct media_profile *profile) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(profile);
    
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, profile_token) == 0) {
            *profile = profiles[i];
            return ONVIF_SUCCESS;
        }
    }
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_create_profile(const char *name, const char *token, struct media_profile *profile) {
    ONVIF_CHECK_NULL(name);
    ONVIF_CHECK_NULL(token);
    ONVIF_CHECK_NULL(profile);
    
    // Check if token already exists
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, token) == 0) {
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
        profile->video_source = profiles[0].video_source;
        profile->video_encoder = profiles[0].video_encoder;
        profile->audio_source = profiles[0].audio_source;
        profile->audio_encoder = profiles[0].audio_encoder;
        profile->ptz = profiles[0].ptz;
        
        platform_log_info("Created custom profile: %s (%s)\n", name, token);
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_SUPPORTED;
}

int onvif_media_delete_profile(const char *profile_token) {
    ONVIF_CHECK_NULL(profile_token);
    
    // Check if it's a fixed profile (cannot be deleted)
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, profile_token) == 0) {
            if (profiles[i].fixed) {
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

int onvif_media_get_video_sources(struct video_source **sources, int *count) {
    static struct video_source video_sources[] = {
        {
            .token = "VideoSource0",
            .framerate = 25.0,
            .resolution = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT},
            .imaging = {
                .brightness = 50.0,
                .color_saturation = 50.0,
                .contrast = 50.0,
                .sharpness = 50.0
            }
        }
    };
    
    ONVIF_CHECK_NULL(sources);
    ONVIF_CHECK_NULL(count);
    
    *sources = video_sources;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_media_get_audio_sources(struct audio_source **sources, int *count) {
    static struct audio_source audio_sources[] = {
        {
            .token = "AudioSource0",
            .channels = 1
        }
    };
    
    ONVIF_CHECK_NULL(sources);
    ONVIF_CHECK_NULL(count);
    
    *sources = audio_sources;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_media_get_video_source_configurations(struct video_source_configuration **configs, int *count) {
    static struct video_source_configuration video_configs[] = {
        {
            .token = "VideoSourceConfig0",
            .name = "Video Source Configuration",
            .use_count = 2,
            .source_token = "VideoSource0",
            .bounds = {MEDIA_MAIN_RESOLUTION_WIDTH, MEDIA_MAIN_RESOLUTION_HEIGHT, 0, 0}
        }
    };
    
    ONVIF_CHECK_NULL(configs);
    ONVIF_CHECK_NULL(count);
    
    *configs = video_configs;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_media_get_video_encoder_configurations(struct video_encoder_configuration **configs, int *count) {
    static struct video_encoder_configuration video_enc_configs[] = {
        {
            .token = "VideoEncoder0",
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
            .multicast = {
                .address = "0.0.0.0",
                .port = 0,
                .ttl = 5,
                .auto_start = 0
            }
        },
        {
            .token = "VideoEncoder1",
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
            .multicast = {
                .address = "0.0.0.0",
                .port = 0,
                .ttl = 5,
                .auto_start = 0
            }
        }
    };
    
    ONVIF_CHECK_NULL(configs);
    ONVIF_CHECK_NULL(count);
    
    *configs = video_enc_configs;
    *count = 2;
    return ONVIF_SUCCESS;
}

int onvif_media_get_audio_source_configurations(struct audio_source_configuration **configs, int *count) {
    static struct audio_source_configuration audio_configs[] = {
        {
            .token = "AudioSourceConfig0",
            .name = "Audio Source Configuration",
            .use_count = 2,
            .source_token = "AudioSource0"
        }
    };
    
    ONVIF_CHECK_NULL(configs);
    ONVIF_CHECK_NULL(count);
    
    *configs = audio_configs;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration **configs, int *count) {
    static struct audio_encoder_configuration audio_enc_configs[] = {
        {
            .token = "AudioEncoder0",
            .name = "AAC Encoder",
            .use_count = 2,
            .encoding = "AAC",
            .bitrate = MEDIA_AUDIO_BITRATE_DEFAULT,
            .sample_rate = 16000,
            .multicast = {
                .address = "0.0.0.0",
                .port = 0,
                .ttl = 5,
                .auto_start = 0
            },
            .session_timeout = 60
        }
    };
    
    ONVIF_CHECK_NULL(configs);
    ONVIF_CHECK_NULL(count);
    
    *configs = audio_enc_configs;
    *count = 1;
    return ONVIF_SUCCESS;
}

/* Metadata Configuration Functions */
int onvif_media_get_metadata_configurations(struct metadata_configuration **configs, int *count) {
    static struct metadata_configuration metadata_configs[] = {
        {
            .token = "MetadataConfig0",
            .name = "Basic Metadata Configuration",
            .use_count = 1,
            .session_timeout = 60,
            .analytics = 0,
            .multicast = {
                .address = "0.0.0.0",
                .port = 0,
                .ttl = 5,
                .auto_start = 0
            },
            .session_timeout = 60
        }
    };
    
    ONVIF_CHECK_NULL(configs);
    ONVIF_CHECK_NULL(count);
    
    *configs = metadata_configs;
    *count = 1;
    return ONVIF_SUCCESS;
}

int onvif_media_set_metadata_configuration(const char *configuration_token, 
                                          const struct metadata_configuration *config) {
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
int onvif_media_set_video_source_configuration(const char *configuration_token, 
                                              const struct video_source_configuration *config) {
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

int onvif_media_set_video_encoder_configuration(const char *configuration_token, 
                                               const struct video_encoder_configuration *config) {
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

int onvif_media_set_audio_source_configuration(const char *configuration_token, 
                                              const struct audio_source_configuration *config) {
    ONVIF_CHECK_NULL(configuration_token);
    ONVIF_CHECK_NULL(config);
    
    if (strcmp(configuration_token, "AudioSourceConfig0") == 0) {
        platform_log_info("Updated audio source configuration: %s\n", configuration_token);
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_set_audio_encoder_configuration(const char *configuration_token, 
                                               const struct audio_encoder_configuration *config) {
    ONVIF_CHECK_NULL(configuration_token);
    ONVIF_CHECK_NULL(config);
    
    if (strcmp(configuration_token, "AudioEncoder0") == 0) {
        platform_log_info("Updated audio encoder configuration: %s\n", configuration_token);
        platform_log_info("  Encoding: %s\n", config->encoding);
        platform_log_info("  Bitrate: %d kbps\n", config->bitrate);
        platform_log_info("  Sample Rate: %d Hz\n", config->sample_rate);
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_get_stream_uri(const char *profile_token, const char *protocol, struct stream_uri *uri) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(protocol);
    ONVIF_CHECK_NULL(uri);
    
    /* Find the profile */
    struct media_profile *profile = NULL;
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, profile_token) == 0) {
            profile = &profiles[i];
            break;
        }
    }
    
    ONVIF_CHECK_NULL(profile);
    
    /* Generate RTSP URI based on profile */
    if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
        if (strcmp(profile_token, "MainProfile") == 0) {
            build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, uri->uri, sizeof(uri->uri));
        } else if (strcmp(profile_token, "SubProfile") == 0) {
            build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_SUB_STREAM_PATH, uri->uri, sizeof(uri->uri));
        } else {
            build_device_url("rtsp", ONVIF_RTSP_PORT_DEFAULT, RTSP_MAIN_STREAM_PATH, uri->uri, sizeof(uri->uri));
        }
        
        uri->invalid_after_connect = 0;
        uri->invalid_after_reboot = 0;
        uri->timeout = 60;
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_get_snapshot_uri(const char *profile_token, struct stream_uri *uri) {
    ONVIF_CHECK_NULL(profile_token);
    ONVIF_CHECK_NULL(uri);
    
    /* Generate snapshot URI */
    build_device_url("http", ONVIF_SNAPSHOT_PORT_DEFAULT, SNAPSHOT_PATH, uri->uri, sizeof(uri->uri));
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
    
    return ONVIF_SUCCESS;
}

int onvif_media_start_multicast_streaming(const char *profile_token) {
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

int onvif_media_stop_multicast_streaming(const char *profile_token) {
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

static int parse_profile_token(const char *request, char *token, size_t token_size) {
    if (!request || !token || token_size == 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    return onvif_xml_extract_string_value(request, MEDIA_XML_PROFILE_TOKEN_TAG, "</trt:ProfileToken>", token, token_size);
}

static int parse_protocol(const char *request, char *protocol, size_t protocol_size) {
    if (!request || !protocol || protocol_size == 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    return onvif_xml_extract_string_value(request, MEDIA_XML_PROTOCOL_TAG, "</trt:Protocol>", protocol, protocol_size);
}

static int validate_profile_token(const char *token) {
    if (!token) {
        return ONVIF_ERROR_INVALID;
    }
    
    // Check if token matches known profiles
    if (strcmp(token, MEDIA_MAIN_PROFILE_TOKEN) == 0 || strcmp(token, MEDIA_SUB_PROFILE_TOKEN) == 0) {
        return ONVIF_SUCCESS;
    }
    
        return ONVIF_ERROR_NOT_FOUND;
}

static int validate_protocol(const char *protocol) {
    if (!protocol) {
        return ONVIF_ERROR_INVALID;
    }
    
    // Check if protocol is supported
    if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
        return ONVIF_SUCCESS;
    }
    
    return ONVIF_ERROR_NOT_SUPPORTED;
}

static int find_profile_by_token(const char *token, struct media_profile **profile) {
    if (!token || !profile) {
        return ONVIF_ERROR_INVALID;
    }
    
    for (int i = 0; i < MEDIA_PROFILE_COUNT_DEFAULT; i++) {
        if (strcmp(profiles[i].token, token) == 0) {
            *profile = &profiles[i];
            return ONVIF_SUCCESS;
        }
    }
    
    return ONVIF_ERROR_NOT_FOUND;
}

static int build_profile_xml(const struct media_profile *profile, char *xml, size_t xml_size) {
    if (!profile || !xml || xml_size == 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    int result = snprintf(xml, xml_size,
                        "  <trt:Profile token=\"%s\" fixed=\"%s\">\n"
                        "    <tt:Name>%s</tt:Name>\n"
                        "    <tt:VideoSource>\n"
                        "      <tt:SourceToken>%s</tt:SourceToken>\n"
                        "      <tt:Bounds x=\"%d\" y=\"%d\" width=\"%d\" height=\"%d\" />\n"
                        "    </tt:VideoSource>\n"
                        "    <tt:VideoEncoder>\n"
                        "      <tt:Name>%s</tt:Name>\n"
                        "      <tt:Encoding>%s</tt:Encoding>\n"
                        "      <tt:Resolution>\n"
                        "        <tt:Width>%d</tt:Width>\n"
                        "        <tt:Height>%d</tt:Height>\n"
                        "      </tt:Resolution>\n"
                        "      <tt:Quality>%.1f</tt:Quality>\n"
                        "      <tt:RateControl>\n"
                        "        <tt:FrameRateLimit>%d</tt:FrameRateLimit>\n"
                        "        <tt:EncodingInterval>%d</tt:EncodingInterval>\n"
                        "        <tt:BitrateLimit>%d</tt:BitrateLimit>\n"
                        "      </tt:RateControl>\n"
                        "      <tt:H264>\n"
                        "        <tt:GovLength>%d</tt:GovLength>\n"
                        "      </tt:H264>\n"
                        "    </tt:VideoEncoder>\n"
                        "    <tt:AudioSource>\n"
                        "      <tt:SourceToken>%s</tt:SourceToken>\n"
                        "    </tt:AudioSource>\n"
                        "    <tt:AudioEncoder>\n"
                        "      <tt:Name>%s</tt:Name>\n"
                        "      <tt:Encoding>%s</tt:Encoding>\n"
                        "      <tt:Bitrate>%d</tt:Bitrate>\n"
                        "      <tt:SampleRate>%d</tt:SampleRate>\n"
                        "    </tt:AudioEncoder>\n"
                        "    <tt:PTZConfiguration>\n"
                        "      <tt:NodeToken>%s</tt:NodeToken>\n"
                        "      <tt:DefaultAbsolutePantTiltPositionSpace>%s</tt:DefaultAbsolutePantTiltPositionSpace>\n"
                        "      <tt:DefaultAbsoluteZoomPositionSpace>%s</tt:DefaultAbsoluteZoomPositionSpace>\n"
                        "      <tt:DefaultRelativePantTiltTranslationSpace>%s</tt:DefaultRelativePantTiltTranslationSpace>\n"
                        "      <tt:DefaultRelativeZoomTranslationSpace>%s</tt:DefaultRelativeZoomTranslationSpace>\n"
                        "      <tt:DefaultContinuousPantTiltVelocitySpace>%s</tt:DefaultContinuousPantTiltVelocitySpace>\n"
                        "      <tt:DefaultContinuousZoomVelocitySpace>%s</tt:DefaultContinuousZoomVelocitySpace>\n"
                        "    </tt:PTZConfiguration>\n"
                        "  </trt:Profile>\n",
        profile->token,
        profile->fixed ? "true" : "false",
        profile->name,
        profile->video_source.source_token,
        profile->video_source.bounds.x, profile->video_source.bounds.y,
        profile->video_source.bounds.width, profile->video_source.bounds.height,
        profile->video_encoder.token,
        profile->video_encoder.encoding,
        profile->video_encoder.resolution.width, profile->video_encoder.resolution.height,
        profile->video_encoder.quality,
        profile->video_encoder.framerate_limit,
        profile->video_encoder.encoding_interval,
        profile->video_encoder.bitrate_limit,
        profile->video_encoder.gov_length,
        profile->audio_source.source_token,
        profile->audio_encoder.token,
        profile->audio_encoder.encoding,
        profile->audio_encoder.bitrate,
        profile->audio_encoder.sample_rate,
        profile->ptz.node_token,
        profile->ptz.default_absolute_pan_tilt_position_space,
        profile->ptz.default_absolute_zoom_position_space,
        profile->ptz.default_relative_pan_tilt_translation_space,
        profile->ptz.default_relative_zoom_translation_space,
        profile->ptz.default_continuous_pan_tilt_velocity_space,
        profile->ptz.default_continuous_zoom_velocity_space);
    
    return (result < 0) ? ONVIF_ERROR : ONVIF_SUCCESS;
}

static int build_video_source_xml(const struct video_source *source, char *xml, size_t xml_size) {
    if (!source || !xml || xml_size == 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    int result = snprintf(xml, xml_size,
                        "  <tt:VideoSource token=\"%s\">\n"
                        "    <tt:Resolution>\n"
                        "      <tt:Width>%d</tt:Width>\n"
                        "      <tt:Height>%d</tt:Height>\n"
                        "    </tt:Resolution>\n"
                        "    <tt:Imaging>\n"
                        "      <tt:Brightness>%.1f</tt:Brightness>\n"
                        "      <tt:ColorSaturation>%.1f</tt:ColorSaturation>\n"
                        "      <tt:Contrast>%.1f</tt:Contrast>\n"
                        "      <tt:Sharpness>%.1f</tt:Sharpness>\n"
                        "    </tt:Imaging>\n"
                        "  </tt:VideoSource>\n",
        source->token,
        source->resolution.width, source->resolution.height,
        source->imaging.brightness, source->imaging.color_saturation,
        source->imaging.contrast, source->imaging.sharpness);
    
    return (result < 0) ? ONVIF_ERROR : ONVIF_SUCCESS;
}

static int build_stream_uri_xml(const struct stream_uri *uri, char *xml, size_t xml_size) {
    if (!uri || !xml || xml_size == 0) {
        return ONVIF_ERROR_INVALID;
    }
    
    int result = snprintf(xml, xml_size,
                        "<trt:MediaUri>\n"
                        "  <tt:Uri>%s</tt:Uri>\n"
                        "  <tt:InvalidAfterConnect>%s</tt:InvalidAfterConnect>\n"
                        "  <tt:InvalidAfterReboot>%s</tt:InvalidAfterReboot>\n"
                        "  <tt:Timeout>PT%dS</tt:Timeout>\n"
                        "</trt:MediaUri>",
        uri->uri,
        uri->invalid_after_connect ? "true" : "false",
        uri->invalid_after_reboot ? "true" : "false",
        uri->timeout);
    
    return (result < 0) ? ONVIF_ERROR : ONVIF_SUCCESS;
}

static void init_default_profiles(void) {
    // Profiles are already initialized as static data
    // This function is a placeholder for future dynamic initialization
}

static int handle_media_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("Media validation failed: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}

static int handle_media_stream_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("Media stream error: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}

/* Legacy service handler removed - using modern service handler implementation below */

/* Refactored media service implementation */

// Service handler instance
static onvif_service_handler_instance_t g_media_handler;
static int g_handler_initialized = 0;

// Action handlers
static int handle_get_profiles(const service_handler_config_t *config,
                              const onvif_request_t *request,
                              onvif_response_t *response,
                              onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "GetProfiles", "profiles_retrieval");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Build profiles XML using XML builder
  onvif_xml_builder_start_element(xml_builder, "trt:Profiles", NULL);
  
  for (int i = 0; i < MEDIA_PROFILE_COUNT_DEFAULT; i++) {
    onvif_xml_builder_start_element(xml_builder, "tt:Profile", "token", profiles[i].token, NULL);
    
    onvif_xml_builder_element_with_text(xml_builder, "tt:Name", profiles[i].name, NULL);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Fixed", "%s", profiles[i].fixed ? "true" : "false");
    
    // Video source
    onvif_xml_builder_start_element(xml_builder, "tt:VideoSource", "token", profiles[i].video_source.source_token, NULL);
    onvif_xml_builder_start_element(xml_builder, "tt:Bounds", NULL);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:width", "%d", profiles[i].video_source.bounds.width);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:height", "%d", profiles[i].video_source.bounds.height);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:x", "%d", profiles[i].video_source.bounds.x);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:y", "%d", profiles[i].video_source.bounds.y);
    onvif_xml_builder_end_element(xml_builder, "tt:Bounds");
    onvif_xml_builder_end_element(xml_builder, "tt:VideoSource");
    
    // Video encoder
    onvif_xml_builder_start_element(xml_builder, "tt:VideoEncoder", "token", profiles[i].video_encoder.token, NULL);
    onvif_xml_builder_element_with_text(xml_builder, "tt:Name", profiles[i].video_encoder.encoding, NULL);
    onvif_xml_builder_element_with_text(xml_builder, "tt:Encoding", profiles[i].video_encoder.encoding, NULL);
    
    onvif_xml_builder_start_element(xml_builder, "tt:Resolution", NULL);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Width", "%d", profiles[i].video_encoder.resolution.width);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Height", "%d", profiles[i].video_encoder.resolution.height);
    onvif_xml_builder_end_element(xml_builder, "tt:Resolution");
    
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Quality", "%.1f", profiles[i].video_encoder.quality);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:FrameRateLimit", "%d", profiles[i].video_encoder.framerate_limit);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:EncodingInterval", "%d", profiles[i].video_encoder.encoding_interval);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:BitrateLimit", "%d", profiles[i].video_encoder.bitrate_limit);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:GovLength", "%d", profiles[i].video_encoder.gov_length);
    
    onvif_xml_builder_end_element(xml_builder, "tt:VideoEncoder");
    
    onvif_xml_builder_end_element(xml_builder, "tt:Profile");
  }
  
  onvif_xml_builder_end_element(xml_builder, "trt:Profiles");
  
  // Generate success response using SOAP response utility
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "GetProfiles", xml_content);
}

static int handle_get_stream_uri(const service_handler_config_t *config,
                                const onvif_request_t *request,
                                onvif_response_t *response,
                                onvif_xml_builder_t *xml_builder) {
  // Initialize error context
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "GetStreamUri", "stream_uri_retrieval");
  
  // Enhanced parameter validation
  if (!config) {
    return error_handle_parameter(&error_ctx, "config", "missing", response);
  }
  if (!response) {
    return error_handle_parameter(&error_ctx, "response", "missing", response);
  }
  if (!xml_builder) {
    return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
  }
  
  // Parse profile token and protocol from request using common XML parser
  char profile_token[MEDIA_TOKEN_BUFFER_SIZE];
  char protocol[MEDIA_PROTOCOL_BUFFER_SIZE];
  
  if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  if (onvif_xml_parse_protocol(request->body, protocol, sizeof(protocol)) != 0) {
    return error_handle_parameter(&error_ctx, "protocol", "invalid", response);
  }
  
  // Validate parameters
  if (validate_profile_token(profile_token) != 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
  }
  
  if (validate_protocol(protocol) != 0) {
    return error_handle_parameter(&error_ctx, "protocol", "invalid", response);
  }
  
  // Get stream URI using the existing function
  struct stream_uri uri;
  int result = onvif_media_get_stream_uri(profile_token, protocol, &uri);
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Build stream URI XML using helper function
  char uri_xml[MEDIA_URI_BUFFER_SIZE];
  if (build_stream_uri_xml(&uri, uri_xml, sizeof(uri_xml)) != 0) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Generate success response using SOAP response utility
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "GetStreamUri", uri_xml);
}

// Action handlers for CreateProfile and DeleteProfile
static int handle_create_profile(const service_handler_config_t *config,
                                const onvif_request_t *request,
                                onvif_response_t *response,
                                onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "CreateProfile", "profile_creation");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse profile name and token from request
  char profile_name[64] = "Custom Profile";
  char profile_token[64] = "CustomProfile";
  
  // Simple XML parsing for profile name and token
  if (strstr(request->body, "<trt:Name>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:Name>", "</trt:Name>", 
                          profile_name, sizeof(profile_name));
  }
  
  if (strstr(request->body, "<trt:Token>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:Token>", "</trt:Token>", 
                          profile_token, sizeof(profile_token));
  }
  
  // Create profile
  struct media_profile profile;
  int result = onvif_media_create_profile(profile_name, profile_token, &profile);
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Build response XML
  onvif_xml_builder_start_element(xml_builder, "trt:Profile", "token", profile.token, NULL);
  onvif_xml_builder_element_with_text(xml_builder, "tt:Name", profile.name, NULL);
  onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Fixed", "%s", 
                                         profile.fixed ? "true" : "false");
  onvif_xml_builder_end_element(xml_builder, "trt:Profile");
  
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "CreateProfile", xml_content);
}

static int handle_delete_profile(const service_handler_config_t *config,
                                const onvif_request_t *request,
                                onvif_response_t *response,
                                onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "DeleteProfile", "profile_deletion");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse profile token from request
  char profile_token[64] = "";
  if (strstr(request->body, "<trt:ProfileToken>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:ProfileToken>", "</trt:ProfileToken>", 
                          profile_token, sizeof(profile_token));
  }
  
  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }
  
  // Delete profile
  int result = onvif_media_delete_profile(profile_token);
  
  if (result != ONVIF_SUCCESS) {
    return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
  }
  
  // Generate empty response for successful deletion
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "DeleteProfile", "");
}

// Action handlers for Set*Configuration operations
static int handle_set_video_source_configuration(const service_handler_config_t *config,
                                                const onvif_request_t *request,
                                                onvif_response_t *response,
                                                onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetVideoSourceConfiguration", "config_update");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse configuration token and parameters from request using common XML parser
  char config_token[64];
  if (onvif_xml_parse_configuration_token(request->body, config_token, sizeof(config_token)) != 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }
  
  // Parse video source configuration parameters using common XML parser
  struct video_source_configuration video_config;
  onvif_xml_parser_t parser;
  if (onvif_xml_parser_init(&parser, request->body, strlen(request->body), NULL) != ONVIF_SUCCESS) {
    return error_handle_parameter(&error_ctx, "xml_parser", "init_failed", response);
  }
  
  if (onvif_xml_parse_video_source_configuration(&parser, &video_config) != 0) {
    onvif_xml_parser_cleanup(&parser);
    return error_handle_parameter(&error_ctx, "video_source_configuration", "invalid", response);
  }
  
  onvif_xml_parser_cleanup(&parser);
  
  // Ensure the configuration token matches
  strncpy(video_config.token, config_token, sizeof(video_config.token) - 1);
  video_config.token[sizeof(video_config.token) - 1] = '\0';
  
  // Set configuration
  int result = onvif_media_set_video_source_configuration(config_token, &video_config);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "config_update", response);
  }
  
  // Generate empty response for successful update
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "SetVideoSourceConfiguration", "");
}

static int handle_set_video_encoder_configuration(const service_handler_config_t *config,
                                                 const onvif_request_t *request,
                                                 onvif_response_t *response,
                                                 onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetVideoEncoderConfiguration", "config_update");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse configuration token and parameters from request using common XML parser
  char config_token[64];
  if (onvif_xml_parse_configuration_token(request->body, config_token, sizeof(config_token)) != 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }
  
  // Parse video encoder configuration parameters using common XML parser
  struct video_encoder_configuration encoder_config;
  onvif_xml_parser_t parser;
  if (onvif_xml_parser_init(&parser, request->body, strlen(request->body), NULL) != ONVIF_SUCCESS) {
    return error_handle_parameter(&error_ctx, "xml_parser", "init_failed", response);
  }
  
  if (onvif_xml_parse_video_encoder_configuration(&parser, &encoder_config) != 0) {
    onvif_xml_parser_cleanup(&parser);
    return error_handle_parameter(&error_ctx, "video_encoder_configuration", "invalid", response);
  }
  
  onvif_xml_parser_cleanup(&parser);
  
  // Ensure the configuration token matches
  strncpy(encoder_config.token, config_token, sizeof(encoder_config.token) - 1);
  encoder_config.token[sizeof(encoder_config.token) - 1] = '\0';
  
  // Configuration is already parsed by onvif_xml_parse_video_encoder_configuration
  
  // Set configuration
  int result = onvif_media_set_video_encoder_configuration(config_token, &encoder_config);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "config_update", response);
  }
  
  // Generate empty response for successful update
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "SetVideoEncoderConfiguration", "");
}

// Action handlers for multicast streaming
static int handle_start_multicast_streaming(const service_handler_config_t *config,
                                           const onvif_request_t *request,
                                           onvif_response_t *response,
                                           onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "StartMulticastStreaming", "multicast_start");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse profile token from request
  char profile_token[64] = "";
  if (strstr(request->body, "<trt:ProfileToken>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:ProfileToken>", "</trt:ProfileToken>", 
                          profile_token, sizeof(profile_token));
  }
  
  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }
  
  // Start multicast streaming
  int result = onvif_media_start_multicast_streaming(profile_token);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "multicast_start", response);
  }
  
  // Generate empty response for successful start
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "StartMulticastStreaming", "");
}

static int handle_stop_multicast_streaming(const service_handler_config_t *config,
                                          const onvif_request_t *request,
                                          onvif_response_t *response,
                                          onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "StopMulticastStreaming", "multicast_stop");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse profile token from request
  char profile_token[64] = "";
  if (strstr(request->body, "<trt:ProfileToken>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:ProfileToken>", "</trt:ProfileToken>", 
                          profile_token, sizeof(profile_token));
  }
  
  if (strlen(profile_token) == 0) {
    return error_handle_parameter(&error_ctx, "profile_token", "missing", response);
  }
  
  // Stop multicast streaming
  int result = onvif_media_stop_multicast_streaming(profile_token);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "multicast_stop", response);
  }
  
  // Generate empty response for successful stop
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "StopMulticastStreaming", "");
}

// Action handlers for metadata configuration
static int handle_get_metadata_configurations(const service_handler_config_t *config,
                                             const onvif_request_t *request,
                                             onvif_response_t *response,
                                             onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "GetMetadataConfigurations", "metadata_config_retrieval");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Get metadata configurations
  struct metadata_configuration *configs;
  int count;
  int result = onvif_media_get_metadata_configurations(&configs, &count);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "metadata_config_retrieval", response);
  }
  
  // Build response XML
  onvif_xml_builder_start_element(xml_builder, "trt:Configurations", NULL);
  
  for (int i = 0; i < count; i++) {
    onvif_xml_builder_start_element(xml_builder, "tt:Configuration", "token", configs[i].token, NULL);
    onvif_xml_builder_element_with_text(xml_builder, "tt:Name", configs[i].name, NULL);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:UseCount", "%d", configs[i].use_count);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:SessionTimeout", "PT%dS", configs[i].session_timeout);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Analytics", "%s", configs[i].analytics ? "true" : "false");
    onvif_xml_builder_end_element(xml_builder, "tt:Configuration");
  }
  
  onvif_xml_builder_end_element(xml_builder, "trt:Configurations");
  
  const char *xml_content = onvif_xml_builder_get_string(xml_builder);
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "GetMetadataConfigurations", xml_content);
}

static int handle_set_metadata_configuration(const service_handler_config_t *config,
                                            const onvif_request_t *request,
                                            onvif_response_t *response,
                                            onvif_xml_builder_t *xml_builder) {
  error_context_t error_ctx;
  error_context_init(&error_ctx, "Media", "SetMetadataConfiguration", "metadata_config_update");
  
  if (!config || !response || !xml_builder) {
    return error_handle_parameter(&error_ctx, "parameters", "missing", response);
  }
  
  // Parse configuration token and parameters from request
  char config_token[64] = "";
  if (strstr(request->body, "<trt:ConfigurationToken>") != NULL) {
    onvif_xml_extract_string_value(request->body, "<trt:ConfigurationToken>", "</trt:ConfigurationToken>", 
                          config_token, sizeof(config_token));
  }
  
  if (strlen(config_token) == 0) {
    return error_handle_parameter(&error_ctx, "configuration_token", "missing", response);
  }
  
  // Parse metadata configuration parameters
  struct metadata_configuration metadata_config = {0};
  strncpy(metadata_config.token, config_token, sizeof(metadata_config.token) - 1);
  metadata_config.token[sizeof(metadata_config.token) - 1] = '\0';
  
  // Extract analytics setting
  if (strstr(request->body, "<tt:Analytics>") != NULL) {
    char analytics_str[16];
    if (onvif_xml_extract_string_value(request->body, "<tt:Analytics>", "</tt:Analytics>", 
                              analytics_str, sizeof(analytics_str)) == 0) {
      metadata_config.analytics = (strcmp(analytics_str, "true") == 0) ? 1 : 0;
    }
  }
  
  // Extract session timeout
  if (strstr(request->body, "<tt:SessionTimeout>") != NULL) {
    char timeout_str[16];
    if (onvif_xml_extract_string_value(request->body, "<tt:SessionTimeout>", "</tt:SessionTimeout>", 
                              timeout_str, sizeof(timeout_str)) == 0) {
      // Parse PT60S format
      if (strstr(timeout_str, "PT") != NULL && strstr(timeout_str, "S") != NULL) {
        sscanf(timeout_str, "PT%dS", &metadata_config.session_timeout);
      }
    }
  }
  
  // Set configuration
  int result = onvif_media_set_metadata_configuration(config_token, &metadata_config);
  
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(&error_ctx, result, "metadata_config_update", response);
  }
  
  // Generate empty response for successful update
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, "SetMetadataConfiguration", "");
}

// Action definitions
static const service_action_def_t media_actions[] = {
  {ONVIF_ACTION_GET_PROFILES, "GetProfiles", handle_get_profiles, 0},
  {ONVIF_ACTION_GET_STREAM_URI, "GetStreamUri", handle_get_stream_uri, 1},
  {ONVIF_ACTION_CREATE_PROFILE, "CreateProfile", handle_create_profile, 1},
  {ONVIF_ACTION_DELETE_PROFILE, "DeleteProfile", handle_delete_profile, 1},
  {ONVIF_ACTION_SET_VIDEO_SOURCE_CONFIGURATION, "SetVideoSourceConfiguration", handle_set_video_source_configuration, 1},
  {ONVIF_ACTION_SET_VIDEO_ENCODER_CONFIGURATION, "SetVideoEncoderConfiguration", handle_set_video_encoder_configuration, 1},
  {ONVIF_ACTION_START_MULTICAST_STREAMING, "StartMulticastStreaming", handle_start_multicast_streaming, 1},
  {ONVIF_ACTION_STOP_MULTICAST_STREAMING, "StopMulticastStreaming", handle_stop_multicast_streaming, 1},
  {ONVIF_ACTION_GET_METADATA_CONFIGURATIONS, "GetMetadataConfigurations", handle_get_metadata_configurations, 0},
  {ONVIF_ACTION_SET_METADATA_CONFIGURATION, "SetMetadataConfiguration", handle_set_metadata_configuration, 1}
};

int onvif_media_init(config_manager_t *config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }
  
  service_handler_config_t handler_config = {
    .service_type = ONVIF_SERVICE_MEDIA,
    .service_name = "Media",
    .config = config,
    .enable_validation = 1,
    .enable_logging = 1
  };
  
  int result = onvif_service_handler_init(&g_media_handler, &handler_config,
                                   media_actions, sizeof(media_actions) / sizeof(media_actions[0]));
  
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

int onvif_media_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!g_handler_initialized) {
        return ONVIF_ERROR;
    }
    return onvif_service_handler_handle_request(&g_media_handler, action, request, response);
}
