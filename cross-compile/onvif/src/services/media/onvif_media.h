/**
 * @file onvif_media.h
 * @brief ONVIF Media service structures & retrieval APIs.
 */
#ifndef ONVIF_MEDIA_H
#define ONVIF_MEDIA_H

#include "common/onvif_types.h"
#include "common/onvif_request.h"
#include "utils/centralized_config.h"

struct stream_uri {
    char uri[256];
    int invalid_after_connect;
    int invalid_after_reboot;
    int timeout;
};

struct video_source {
    char token[32];
    float framerate;
    struct {
        int width;
        int height;
    } resolution;
    struct {
        float brightness;
        float color_saturation;
        float contrast;
        float sharpness;
    } imaging;
};

struct audio_source {
    char token[32];
    int channels;
};

struct video_source_configuration {
    char token[32];
    char name[64];
    int use_count;
    char source_token[32];
    struct {
        int width;
        int height;
        int x;
        int y;
    } bounds;
};

struct multicast_config {
    char address[16];
    int port;
    int ttl;
    int auto_start;
};

struct video_encoder_configuration {
    char token[32];
    char name[64];
    int use_count;
    char encoding[16];
    struct {
        int width;
        int height;
    } resolution;
    float quality;
    int framerate_limit;
    int encoding_interval;
    int bitrate_limit;
    int gov_length;
    char profile[16];
    int guaranteed_framerate;
    struct multicast_config multicast;
};

struct audio_source_configuration {
    char token[32];
    char name[64];
    int use_count;
    char source_token[32];
};

struct audio_encoder_configuration {
    char token[32];
    char name[64];
    int use_count;
    char encoding[16];
    int bitrate;
    int sample_rate;
    struct multicast_config multicast;
    int session_timeout;
};

struct media_profile {
    char token[32];
    char name[64];
    int fixed;
    struct {
        char source_token[32];
        struct {
            int width;
            int height;
            int x;
            int y;
        } bounds;
    } video_source;
    struct {
        char token[32];
        char encoding[16];
        struct {
            int width;
            int height;
        } resolution;
        float quality;
        int framerate_limit;
        int encoding_interval;
        int bitrate_limit;
        int gov_length;
    } video_encoder;
    struct {
        char source_token[32];
    } audio_source;
    struct {
        char token[32];
        char encoding[16];
        int bitrate;
        int sample_rate;
    } audio_encoder;
    struct {
        char node_token[32];
        char default_absolute_pan_tilt_position_space[128];
        char default_absolute_zoom_position_space[128];
        char default_relative_pan_tilt_translation_space[128];
        char default_relative_zoom_translation_space[128];
        char default_continuous_pan_tilt_velocity_space[128];
        char default_continuous_zoom_velocity_space[128];
    } ptz;
};

int onvif_media_get_profiles(struct media_profile **profile_list, int *count); /**< Enumerate media profiles. */
int onvif_media_get_profile(const char *profile_token, struct media_profile *profile); /**< Get single profile. */
int onvif_media_get_video_sources(struct video_source **sources, int *count); /**< Enumerate video sources. */
int onvif_media_get_audio_sources(struct audio_source **sources, int *count); /**< Enumerate audio sources. */
int onvif_media_get_video_source_configurations(struct video_source_configuration **configs, int *count); /**< Get video source configs. */
int onvif_media_get_video_encoder_configurations(struct video_encoder_configuration **configs, int *count); /**< Video encoder configs. */
int onvif_media_get_audio_source_configurations(struct audio_source_configuration **configs, int *count); /**< Audio source configs. */
int onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration **configs, int *count); /**< Audio encoder configs. */
int onvif_media_get_stream_uri(const char *profile_token, const char *protocol, struct stream_uri *uri); /**< Build stream URI. */
int onvif_media_get_snapshot_uri(const char *profile_token, struct stream_uri *uri); /**< Build snapshot URI. */
int onvif_media_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response); /**< Handle ONVIF media service requests. */

/* Media service functions */

/**
 * @brief Initialize media service
 * @param config Centralized configuration
 * @return 0 on success, negative error code on failure
 */
int onvif_media_init(centralized_config_t *config);

/**
 * @brief Clean up media service
 */
void onvif_media_cleanup(void);

#endif
