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
#include "config.h"
#include "network_utils.h"
#include "../server/http/http_parser.h"
#include "../utils/xml_utils.h"
#include "../utils/logging_utils.h"
#include "../common/onvif_types.h"

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
    if (!profile_list || !count) return -1;
    
    *profile_list = profiles;
    *count = PROFILE_COUNT;
    return 0;
}

int onvif_media_get_profile(const char *profile_token, struct media_profile *profile) {
    if (!profile_token || !profile) return -1;
    
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, profile_token) == 0) {
            *profile = profiles[i];
            return 0;
        }
    }
    return -1;
}

int onvif_media_create_profile(const char *name, const char *token, struct media_profile *profile) {
    if (!name || !token || !profile) return -1;
    
    /* For simplicity, don't support dynamic profile creation */
    return -1;
}

int onvif_media_delete_profile(const char *profile_token) {
    if (!profile_token) return -1;
    
    /* For simplicity, don't support profile deletion */
    return -1;
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
    
    if (!sources || !count) return -1;
    
    *sources = video_sources;
    *count = 1;
    return 0;
}

int onvif_media_get_audio_sources(struct audio_source **sources, int *count) {
    static struct audio_source audio_sources[] = {
        {
            .token = "AudioSource0",
            .channels = 1
        }
    };
    
    if (!sources || !count) return -1;
    
    *sources = audio_sources;
    *count = 1;
    return 0;
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
    
    if (!configs || !count) return -1;
    
    *configs = video_configs;
    *count = 1;
    return 0;
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
    
    if (!configs || !count) return -1;
    
    *configs = video_enc_configs;
    *count = 2;
    return 0;
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
    
    if (!configs || !count) return -1;
    
    *configs = audio_configs;
    *count = 1;
    return 0;
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
    
    if (!configs || !count) return -1;
    
    *configs = audio_enc_configs;
    *count = 1;
    return 0;
}

int onvif_media_get_stream_uri(const char *profile_token, const char *protocol, struct stream_uri *uri) {
    if (!profile_token || !protocol || !uri) return -1;
    
    /* Find the profile */
    struct media_profile *profile = NULL;
    for (int i = 0; i < PROFILE_COUNT; i++) {
        if (strcmp(profiles[i].token, profile_token) == 0) {
            profile = &profiles[i];
            break;
        }
    }
    
    if (!profile) return -1;
    
    /* Generate RTSP URI based on profile */
    if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
        if (strcmp(profile_token, "MainProfile") == 0) {
            build_device_url("rtsp", 554, "/vs0", uri->uri, sizeof(uri->uri));
        } else if (strcmp(profile_token, "SubProfile") == 0) {
            build_device_url("rtsp", 554, "/vs1", uri->uri, sizeof(uri->uri));
        } else {
            build_device_url("rtsp", 554, "/vs0", uri->uri, sizeof(uri->uri));
        }
        
        uri->invalid_after_connect = 0;
        uri->invalid_after_reboot = 0;
        uri->timeout = 60;
        return 0;
    }
    
    return -1;
}

int onvif_media_get_snapshot_uri(const char *profile_token, struct stream_uri *uri) {
    if (!profile_token || !uri) return -1;
    
    /* Generate snapshot URI */
    build_device_url("http", 3000, "/snapshot.bmp", uri->uri, sizeof(uri->uri));
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
    
    return 0;
}

int onvif_media_start_multicast_streaming(const char *profile_token) {
    if (!profile_token) return -1;
    
    /* TODO: Implement multicast streaming */
    printf("StartMulticastStreaming for profile %s (not implemented)\n", profile_token);
    return -1;
}

int onvif_media_stop_multicast_streaming(const char *profile_token) {
    if (!profile_token) return -1;
    
    /* TODO: Implement multicast streaming */
    printf("StopMulticastStreaming for profile %s (not implemented)\n", profile_token);
    return -1;
}

/* SOAP XML generation helpers */
static void soap_fault_response(char *response, size_t response_size, const char *fault_code, const char *fault_string) {
    snprintf(response, response_size, 
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <soap:Fault>\n"
        "      <soap:Code>\n"
        "        <soap:Value>%s</soap:Value>\n"
        "      </soap:Code>\n"
        "      <soap:Reason>\n"
        "        <soap:Text>%s</soap:Text>\n"
        "      </soap:Reason>\n"
        "    </soap:Fault>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", fault_code, fault_string);
}

static void soap_success_response(char *response, size_t response_size, const char *action, const char *body_content) {
    snprintf(response, response_size,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <trt:%sResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
        "      %s\n"
        "    </trt:%sResponse>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", action, body_content, action);
}

/* XML parsing helpers - now using xml_utils module */


/**
 * @brief Handle ONVIF media service requests
 */
int onvif_media_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!request || !response) {
        return -1;
    }
    
    // Initialize response structure
    response->status_code = 200;
    response->content_type = "application/soap+xml";
    response->body = malloc(4096);
    if (!response->body) {
        return -1;
    }
    response->body_length = 0;
    
    switch (action) {
        case ONVIF_ACTION_GET_PROFILES: {
            struct media_profile *profiles = NULL;
            int count = 0;
            if (onvif_media_get_profiles(&profiles, &count) == 0) {
                char profiles_xml[4096];
                strcpy(profiles_xml, "<trt:Profiles>\n");
                
                for (int i = 0; i < count; i++) {
                    char profile_xml[1024];
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
                    strcat(profiles_xml, profile_xml);
                }
                strcat(profiles_xml, "</trt:Profiles>");
                
                snprintf(response->body, 4096, 
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <trt:GetProfilesResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
                    "      %s\n"
                    "    </trt:GetProfilesResponse>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>", profiles_xml);
                response->body_length = strlen(response->body);
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get profiles");
                response->body_length = strlen(response->body);
            }
            break;
        }
        
        case ONVIF_ACTION_GET_VIDEO_SOURCES: {
            struct video_source *sources = NULL;
            int count = 0;
            if (onvif_media_get_video_sources(&sources, &count) == 0) {
                char sources_xml[2048];
                strcpy(sources_xml, "<trt:VideoSources>\n");
                
                for (int i = 0; i < count; i++) {
                    char source_xml[512];
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
                    strcat(sources_xml, source_xml);
                }
                strcat(sources_xml, "</trt:VideoSources>");
                
                snprintf(response->body, 4096,
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                    "  <soap:Body>\n"
                    "    <trt:GetVideoSourcesResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
                    "      %s\n"
                    "    </trt:GetVideoSourcesResponse>\n"
                    "  </soap:Body>\n"
                    "</soap:Envelope>", sources_xml);
                response->body_length = strlen(response->body);
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get video sources");
                response->body_length = strlen(response->body);
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
                    
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <trt:GetStreamUriResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
                        "      %s\n"
                        "    </trt:GetStreamUriResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>", uri_xml);
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get stream URI");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing ProfileToken or Protocol");
                response->body_length = strlen(response->body);
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
                    
                    snprintf(response->body, 4096,
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
                        "  <soap:Body>\n"
                        "    <trt:GetSnapshotUriResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
                        "      %s\n"
                        "    </trt:GetSnapshotUriResponse>\n"
                        "  </soap:Body>\n"
                        "</soap:Envelope>", uri_xml);
                    response->body_length = strlen(response->body);
                } else {
                    soap_fault_response(response->body, 4096, "soap:Receiver", "Failed to get snapshot URI");
                    response->body_length = strlen(response->body);
                }
            } else {
                soap_fault_response(response->body, 4096, "soap:Receiver", "Missing ProfileToken");
                response->body_length = strlen(response->body);
            }
            
            if (profile_token) free(profile_token);
            break;
        }
        
        default:
            soap_fault_response(response->body, 4096, "soap:Receiver", "Unsupported action");
            response->body_length = strlen(response->body);
            break;
    }
    
    return response->body_length;
}
