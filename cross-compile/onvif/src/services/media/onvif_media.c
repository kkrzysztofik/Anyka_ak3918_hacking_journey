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
#include "onvif_config.h"
#include "network_utils.h"

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
