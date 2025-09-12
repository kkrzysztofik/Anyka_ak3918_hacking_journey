/*
 * onvif_media.c - ONVIF Media service implementation
 * 
 * This file implements the ONVIF Media Web Service endpoints including
 * video/audio profiles, stream URIs, and configuration.
 */

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "onvif_media.h"
#include "utils/config.h"
#include "utils/network_utils.h"
#include "utils/xml_utils.h"
#include "utils/logging_utils.h"
#include "utils/error_handling.h"
#include "utils/constants_clean.h"
#include "utils/unified_soap_generator.h"
#include "utils/response_helpers.h"
#include "utils/service_handler.h"
#include "utils/xml_builder.h"
#include "utils/common_error_handling.h"
#include "utils/centralized_config.h"
#include "common/onvif_types.h"
#include "utils/safe_string.h"
#include "utils/memory_manager.h"

/* Static media profiles */
static struct media_profile profiles[] = {
    {
        .token = "MainProfile",
        .name = "Main Video Profile",
        .fixed = 1,
        .video_source = {
            .source_token = "VideoSource0",
            .bounds = {1280, 720, 0, 0}
        },
        .video_encoder = {
            .token = "VideoEncoder0",
            .encoding = "H264",
            .resolution = {1280, 720},
            .quality = 4.0,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = 2048,
            .gov_length = 50
        },
        .audio_source = {
            .source_token = "AudioSource0"
        },
        .audio_encoder = {
            .token = "AudioEncoder0",
            .encoding = "AAC",
            .bitrate = 64,
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
        .token = "SubProfile",
        .name = "Sub Video Profile",
        .fixed = 1,
        .video_source = {
            .source_token = "VideoSource0",
            .bounds = {640, 360, 0, 0}
        },
        .video_encoder = {
            .token = "VideoEncoder1",
            .encoding = "H264",
            .resolution = {640, 360},
            .quality = 3.0,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = 800,
            .gov_length = 50
        },
        .audio_source = {
            .source_token = "AudioSource0"
        },
        .audio_encoder = {
            .token = "AudioEncoder0",
            .encoding = "AAC",
            .bitrate = 64,
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

#define PROFILE_COUNT 2

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
    
    /* For simplicity, don't support dynamic profile creation */
    return ONVIF_ERROR_NOT_SUPPORTED;
}

int onvif_media_delete_profile(const char *profile_token) {
    ONVIF_CHECK_NULL(profile_token);
    
    /* For simplicity, don't support profile deletion */
    return ONVIF_ERROR_NOT_SUPPORTED;
}

int onvif_media_get_video_sources(struct video_source **sources, int *count) {
    static struct video_source video_sources[] = {
        {
            .token = "VideoSource0",
            .framerate = 25.0,
            .resolution = {1280, 720},
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
            .bounds = {1280, 720, 0, 0}
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
            .resolution = {1280, 720},
            .quality = 4.0,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = 2048,
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
            .resolution = {640, 360},
            .quality = 3.0,
            .framerate_limit = 25,
            .encoding_interval = 1,
            .bitrate_limit = 800,
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
            .bitrate = 64,
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
    
    /* TODO: Implement multicast streaming */
    platform_log_info("StartMulticastStreaming for profile %s (not implemented)\n", profile_token);
    return ONVIF_ERROR_NOT_FOUND;
}

int onvif_media_stop_multicast_streaming(const char *profile_token) {
    ONVIF_CHECK_NULL(profile_token);
    
    /* TODO: Implement multicast streaming */
    platform_log_info("StopMulticastStreaming for profile %s (not implemented)\n", profile_token);
    return ONVIF_ERROR_NOT_FOUND;
}

/* SOAP XML generation helpers - now using common utilities */

/* XML parsing helpers - now using xml_utils module */


/**
 * @brief Internal media service handler
 */
static int media_service_handler(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    // Initialize response structure
    response->status_code = 200;
    response->content_type = "application/soap+xml";
    response->body = malloc(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
        return ONVIF_ERROR_NOT_FOUND;
    }
    response->body_length = 0;
    
    switch (action) {
        case ONVIF_ACTION_GET_PROFILES: {
            struct media_profile *profiles = NULL;
            int count = 0;
            if (onvif_media_get_profiles(&profiles, &count) == 0) {
                char profiles_xml[ONVIF_RESPONSE_BUFFER_SIZE];
                if (safe_strcpy(profiles_xml, sizeof(profiles_xml), "<trt:Profiles>\n") != 0) {
                    onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for profiles XML");
                    break;
                }
                
                for (int i = 0; i < count; i++) {
                    char profile_xml[ONVIF_XML_BUFFER_SIZE];
                    snprintf(profile_xml, sizeof(profile_xml),
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
                        profiles[i].token,
                        profiles[i].fixed ? "true" : "false",
                        profiles[i].name,
                        profiles[i].video_source.source_token,
                        profiles[i].video_source.bounds.x, profiles[i].video_source.bounds.y,
                        profiles[i].video_source.bounds.width, profiles[i].video_source.bounds.height,
                        profiles[i].video_encoder.token,
                        profiles[i].video_encoder.encoding,
                        profiles[i].video_encoder.resolution.width, profiles[i].video_encoder.resolution.height,
                        profiles[i].video_encoder.quality,
                        profiles[i].video_encoder.framerate_limit,
                        profiles[i].video_encoder.encoding_interval,
                        profiles[i].video_encoder.bitrate_limit,
                        profiles[i].video_encoder.gov_length,
                        profiles[i].audio_source.source_token,
                        profiles[i].audio_encoder.token,
                        profiles[i].audio_encoder.encoding,
                        profiles[i].audio_encoder.bitrate,
                        profiles[i].audio_encoder.sample_rate,
                        profiles[i].ptz.node_token,
                        profiles[i].ptz.default_absolute_pan_tilt_position_space,
                        profiles[i].ptz.default_absolute_zoom_position_space,
                        profiles[i].ptz.default_relative_pan_tilt_translation_space,
                        profiles[i].ptz.default_relative_zoom_translation_space,
                        profiles[i].ptz.default_continuous_pan_tilt_velocity_space,
                        profiles[i].ptz.default_continuous_zoom_velocity_space);
                    if (safe_strcat(profiles_xml, sizeof(profiles_xml), profile_xml) != 0) {
                        onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for profiles XML");
                        break;
                    }
                }
                if (safe_strcat(profiles_xml, sizeof(profiles_xml), "</trt:Profiles>") != 0) {
                    onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for profiles XML");
                    break;
                }
                
                onvif_response_media_success(response, "GetProfiles", profiles_xml);
            } else {
                onvif_response_soap_fault(response, "soap:Receiver", "Failed to get profiles");
            }
            break;
        }
        
        case ONVIF_ACTION_GET_VIDEO_SOURCES: {
            struct video_source *sources = NULL;
            int count = 0;
            if (onvif_media_get_video_sources(&sources, &count) == 0) {
                char sources_xml[ONVIF_RESPONSE_BUFFER_SIZE];
                if (safe_strcpy(sources_xml, sizeof(sources_xml), "<trt:VideoSources>\n") != 0) {
                    onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for sources XML");
                    break;
                }
                
                for (int i = 0; i < count; i++) {
                    char source_xml[ONVIF_XML_BUFFER_SIZE];
                    snprintf(source_xml, sizeof(source_xml),
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
                        sources[i].token,
                        sources[i].resolution.width, sources[i].resolution.height,
                        sources[i].imaging.brightness, sources[i].imaging.color_saturation,
                        sources[i].imaging.contrast, sources[i].imaging.sharpness);
                    if (safe_strcat(sources_xml, sizeof(sources_xml), source_xml) != 0) {
                        onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for sources XML");
                        break;
                    }
                }
                if (safe_strcat(sources_xml, sizeof(sources_xml), "</trt:VideoSources>") != 0) {
                    onvif_response_soap_fault(response, "soap:Receiver", "Buffer too small for sources XML");
                    break;
                }
                
                onvif_response_media_success(response, "GetVideoSources", sources_xml);
            } else {
                onvif_response_soap_fault(response, "soap:Receiver", "Failed to get video sources");
            }
            break;
        }
        
        case ONVIF_ACTION_GET_STREAM_URI: {
            char *profile_token = xml_extract_value(request->body, "<trt:ProfileToken>", "</trt:ProfileToken>");
            char *protocol = xml_extract_value(request->body, "<trt:Protocol>", "</trt:Protocol>");
            
            if (profile_token && protocol) {
                struct stream_uri uri;
                if (onvif_media_get_stream_uri(profile_token, protocol, &uri) == 0) {
                    char uri_xml[512];
                    snprintf(uri_xml, sizeof(uri_xml),
                        "<trt:MediaUri>\n"
                        "  <tt:Uri>%s</tt:Uri>\n"
                        "  <tt:InvalidAfterConnect>%s</tt:InvalidAfterConnect>\n"
                        "  <tt:InvalidAfterReboot>%s</tt:InvalidAfterReboot>\n"
                        "  <tt:Timeout>PT%dS</tt:Timeout>\n"
                        "</trt:MediaUri>",
                        uri.uri,
                        uri.invalid_after_connect ? "true" : "false",
                        uri.invalid_after_reboot ? "true" : "false",
                        uri.timeout);
                    
                    onvif_response_media_success(response, "GetStreamUri", uri_xml);
                } else {
                    onvif_response_soap_fault(response, "soap:Receiver", "Failed to get stream URI");
                }
            } else {
                onvif_handle_missing_parameter(response, "ProfileToken or Protocol");
            }
            
            if (profile_token) free(profile_token);
            if (protocol) free(protocol);
            break;
        }
        
        case ONVIF_ACTION_GET_SNAPSHOT_URI: {
            char *profile_token = xml_extract_value(request->body, "<trt:ProfileToken>", "</trt:ProfileToken>");
            
            if (profile_token) {
                struct stream_uri uri;
                if (onvif_media_get_snapshot_uri(profile_token, &uri) == 0) {
                    char uri_xml[512];
                    snprintf(uri_xml, sizeof(uri_xml),
                        "<trt:MediaUri>\n"
                        "  <tt:Uri>%s</tt:Uri>\n"
                        "  <tt:InvalidAfterConnect>%s</tt:InvalidAfterConnect>\n"
                        "  <tt:InvalidAfterReboot>%s</tt:InvalidAfterReboot>\n"
                        "  <tt:Timeout>PT%dS</tt:Timeout>\n"
                        "</trt:MediaUri>",
                        uri.uri,
                        uri.invalid_after_connect ? "true" : "false",
                        uri.invalid_after_reboot ? "true" : "false",
                        uri.timeout);
                    
                    onvif_response_media_success(response, "GetSnapshotUri", uri_xml);
                } else {
                    onvif_response_soap_fault(response, "soap:Receiver", "Failed to get snapshot URI");
                }
            } else {
                onvif_handle_missing_parameter(response, "ProfileToken");
            }
            
            if (profile_token) free(profile_token);
            break;
        }
        
        default:
            onvif_handle_unsupported_action(response);
            break;
    }
    
    return response->body_length;
}

/**
 * @brief Handle ONVIF media service requests
 */
int onvif_media_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    return onvif_handle_service_request(action, request, response, media_service_handler);
}

/* Refactored media service implementation */

// Service handler instance
static service_handler_t g_media_handler;
static int g_handler_initialized = 0;

// Action handlers
static int handle_get_profiles(const service_handler_config_t *config,
                              const onvif_request_t *request,
                              onvif_response_t *response,
                              xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Build profiles XML using XML builder
  xml_builder_start_element(xml_builder, "trt:Profiles", NULL);
  
  for (int i = 0; i < 2; i++) {
    xml_builder_start_element(xml_builder, "tt:Profile", "token", profiles[i].token, NULL);
    
    xml_builder_element_with_text(xml_builder, "tt:Name", profiles[i].name, NULL);
    xml_builder_element_with_formatted_text(xml_builder, "tt:Fixed", "%s", profiles[i].fixed ? "true" : "false");
    
    // Video source
    xml_builder_start_element(xml_builder, "tt:VideoSource", "token", profiles[i].video_source.source_token, NULL);
    xml_builder_start_element(xml_builder, "tt:Bounds", NULL);
    xml_builder_element_with_formatted_text(xml_builder, "tt:width", "%d", profiles[i].video_source.bounds.width);
    xml_builder_element_with_formatted_text(xml_builder, "tt:height", "%d", profiles[i].video_source.bounds.height);
    xml_builder_element_with_formatted_text(xml_builder, "tt:x", "%d", profiles[i].video_source.bounds.x);
    xml_builder_element_with_formatted_text(xml_builder, "tt:y", "%d", profiles[i].video_source.bounds.y);
    xml_builder_end_element(xml_builder, "tt:Bounds");
    xml_builder_end_element(xml_builder, "tt:VideoSource");
    
    // Video encoder
    xml_builder_start_element(xml_builder, "tt:VideoEncoder", "token", profiles[i].video_encoder.token, NULL);
    xml_builder_element_with_text(xml_builder, "tt:Name", profiles[i].video_encoder.encoding, NULL);
    xml_builder_element_with_text(xml_builder, "tt:Encoding", profiles[i].video_encoder.encoding, NULL);
    
    xml_builder_start_element(xml_builder, "tt:Resolution", NULL);
    xml_builder_element_with_formatted_text(xml_builder, "tt:Width", "%d", profiles[i].video_encoder.resolution.width);
    xml_builder_element_with_formatted_text(xml_builder, "tt:Height", "%d", profiles[i].video_encoder.resolution.height);
    xml_builder_end_element(xml_builder, "tt:Resolution");
    
    xml_builder_element_with_formatted_text(xml_builder, "tt:Quality", "%.1f", profiles[i].video_encoder.quality);
    xml_builder_element_with_formatted_text(xml_builder, "tt:FrameRateLimit", "%d", profiles[i].video_encoder.framerate_limit);
    xml_builder_element_with_formatted_text(xml_builder, "tt:EncodingInterval", "%d", profiles[i].video_encoder.encoding_interval);
    xml_builder_element_with_formatted_text(xml_builder, "tt:BitrateLimit", "%d", profiles[i].video_encoder.bitrate_limit);
    xml_builder_element_with_formatted_text(xml_builder, "tt:GovLength", "%d", profiles[i].video_encoder.gov_length);
    
    xml_builder_end_element(xml_builder, "tt:VideoEncoder");
    
    xml_builder_end_element(xml_builder, "tt:Profile");
  }
  
  xml_builder_end_element(xml_builder, "trt:Profiles");
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_media_handler, "GetProfiles", xml_content, response);
}

static int handle_get_stream_uri(const service_handler_config_t *config,
                                const onvif_request_t *request,
                                onvif_response_t *response,
                                xml_builder_t *xml_builder) {
  if (!config || !response || !xml_builder) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse profile token and protocol from request
  char profile_token[32] = "MainProfile";
  char protocol[16] = "RTSP";
  
  // Simple XML parsing for required parameters
  char *profile_start = strstr(request->body, "<ProfileToken>");
  if (profile_start) {
    char *profile_end = strstr(profile_start, "</ProfileToken>");
    if (profile_end) {
      size_t len = profile_end - (profile_start + 13);
      if (len < sizeof(profile_token)) {
        strncpy(profile_token, profile_start + 13, len);
        profile_token[len] = '\0';
      }
    }
  }
  
  char *protocol_start = strstr(request->body, "<Protocol>");
  if (protocol_start) {
    char *protocol_end = strstr(protocol_start, "</Protocol>");
    if (protocol_end) {
      size_t len = protocol_end - (protocol_start + 10);
      if (len < sizeof(protocol)) {
        strncpy(protocol, protocol_start + 10, len);
        protocol[len] = '\0';
      }
    }
  }
  
  // Build stream URI XML
  xml_builder_start_element(xml_builder, "trt:MediaUri", NULL);
  xml_builder_element_with_formatted_text(xml_builder, "tt:Uri", "rtsp://[IP]:554/vs0");
  xml_builder_element_with_formatted_text(xml_builder, "tt:InvalidAfterConnect", "false");
  xml_builder_element_with_formatted_text(xml_builder, "tt:InvalidAfterReboot", "false");
  xml_builder_element_with_formatted_text(xml_builder, "tt:Timeout", "PT60S");
  xml_builder_end_element(xml_builder, "trt:MediaUri");
  
  // Generate success response
  const char *xml_content = xml_builder_get_string(xml_builder);
  return service_handler_generate_success(&g_media_handler, "GetStreamUri", xml_content, response);
}

// Action definitions
static const service_action_def_t media_actions[] = {
  {ONVIF_ACTION_GET_PROFILES, "GetProfiles", handle_get_profiles, 0},
  {ONVIF_ACTION_GET_STREAM_URI, "GetStreamUri", handle_get_stream_uri, 1}
};

int onvif_media_init(centralized_config_t *config) {
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
  
  int result = service_handler_init(&g_media_handler, &handler_config,
                                   media_actions, sizeof(media_actions) / sizeof(media_actions[0]));
  
  if (result == ONVIF_SUCCESS) {
    g_handler_initialized = 1;
  }
  
  return result;
}

void onvif_media_cleanup(void) {
  if (g_handler_initialized) {
    service_handler_cleanup(&g_media_handler);
    g_handler_initialized = 0;
  }
}
