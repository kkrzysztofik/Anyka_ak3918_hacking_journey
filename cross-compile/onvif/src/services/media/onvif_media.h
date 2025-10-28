/**
 * @file onvif_media.h
 * @brief ONVIF Media service structures & retrieval APIs
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef ONVIF_MEDIA_H
#define ONVIF_MEDIA_H

#include "networking/http/http_parser.h"

// Buffer size constants
#define MEDIA_URI_BUFFER_SIZE 256
#define MEDIA_TOKEN_SIZE      32
#define MEDIA_NAME_SIZE       64
#define MEDIA_ENCODING_SIZE   16
#define MEDIA_ADDRESS_SIZE    16
#define MEDIA_SPACE_SIZE      128

struct stream_uri {
  char uri[MEDIA_URI_BUFFER_SIZE];
  int invalid_after_connect;
  int invalid_after_reboot;
  int timeout;
};

struct video_source {
  char token[MEDIA_TOKEN_SIZE];
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
  char token[MEDIA_TOKEN_SIZE];
  int channels;
};

struct video_source_configuration {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int use_count;
  char source_token[MEDIA_TOKEN_SIZE];
  struct {
    int width;
    int height;
    int x;
    int y;
  } bounds;
};

struct multicast_config {
  char address[MEDIA_ADDRESS_SIZE];
  int port;
  int ttl;
  int auto_start;
};

struct video_encoder_configuration {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int use_count;
  char encoding[MEDIA_ENCODING_SIZE];
  struct {
    int width;
    int height;
  } resolution;
  float quality;
  int framerate_limit;
  int encoding_interval;
  int bitrate_limit;
  int gov_length;
  char profile[MEDIA_ENCODING_SIZE];
  int guaranteed_framerate;
  struct multicast_config multicast;
};

struct audio_source_configuration {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int use_count;
  char source_token[MEDIA_TOKEN_SIZE];
};

struct audio_encoder_configuration {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int use_count;
  char encoding[MEDIA_ENCODING_SIZE];
  int bitrate;
  int sample_rate;
  struct multicast_config multicast;
  int session_timeout;
};

struct metadata_configuration {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int use_count;
  int session_timeout;
  int analytics;
  struct multicast_config multicast;
};

struct media_profile {
  char token[MEDIA_TOKEN_SIZE];
  char name[MEDIA_NAME_SIZE];
  int fixed;
  struct {
    char source_token[MEDIA_TOKEN_SIZE];
    struct {
      int width;
      int height;
      int x;
      int y;
    } bounds;
  } video_source;
  struct {
    char token[MEDIA_TOKEN_SIZE];
    char encoding[MEDIA_ENCODING_SIZE];
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
    char source_token[MEDIA_TOKEN_SIZE];
  } audio_source;
  struct {
    char token[MEDIA_TOKEN_SIZE];
    char encoding[MEDIA_ENCODING_SIZE];
    int bitrate;
    int sample_rate;
  } audio_encoder;
  struct {
    char node_token[MEDIA_TOKEN_SIZE];
    char default_absolute_pan_tilt_position_space[MEDIA_SPACE_SIZE];
    char default_absolute_zoom_position_space[MEDIA_SPACE_SIZE];
    char default_relative_pan_tilt_translation_space[MEDIA_SPACE_SIZE];
    char default_relative_zoom_translation_space[MEDIA_SPACE_SIZE];
    char default_continuous_pan_tilt_velocity_space[MEDIA_SPACE_SIZE];
    char default_continuous_zoom_velocity_space[MEDIA_SPACE_SIZE];
  } ptz;
};

int onvif_media_get_profiles(struct media_profile** profile_list, int* count);                              /**< Enumerate media profiles. */
int onvif_media_get_profile(const char* profile_token, struct media_profile* profile);                      /**< Get single profile. */
int onvif_media_get_video_sources(struct video_source** sources, int* count);                               /**< Enumerate video sources. */
int onvif_media_get_audio_sources(struct audio_source** sources, int* count);                               /**< Enumerate audio sources. */
int onvif_media_get_video_source_configurations(struct video_source_configuration** configs, int* count);   /**< Get video source configs. */
int onvif_media_get_video_encoder_configurations(struct video_encoder_configuration** configs, int* count); /**< Video encoder configs. */
int onvif_media_get_audio_source_configurations(struct audio_source_configuration** configs, int* count);   /**< Audio source configs. */
int onvif_media_get_audio_encoder_configurations(struct audio_encoder_configuration** configs, int* count); /**< Audio encoder configs. */
int onvif_media_get_stream_uri(const char* profile_token, const char* protocol, struct stream_uri* uri);    /**< Build stream URI. */
int onvif_media_get_snapshot_uri(const char* profile_token, struct stream_uri* uri);                        /**< Build snapshot URI. */
int onvif_media_create_profile(const char* name, const char* token, struct media_profile* profile);         /**< Create a new media profile. */
int onvif_media_delete_profile(const char* profile_token);                                                  /**< Delete a media profile. */
int onvif_media_set_video_source_configuration(const char* configuration_token,
                                               const struct video_source_configuration* config); /**< Set video source configuration. */
int onvif_media_set_video_encoder_configuration(const char* configuration_token,
                                                const struct video_encoder_configuration* config); /**< Set video encoder configuration. */
int onvif_media_set_audio_source_configuration(const char* configuration_token,
                                               const struct audio_source_configuration* config); /**< Set audio source configuration. */
int onvif_media_set_audio_encoder_configuration(const char* configuration_token,
                                                const struct audio_encoder_configuration* config); /**< Set audio encoder configuration. */
int onvif_media_start_multicast_streaming(const char* profile_token);                              /**< Start multicast streaming. */
int onvif_media_stop_multicast_streaming(const char* profile_token);                               /**< Stop multicast streaming. */
int onvif_media_get_metadata_configurations(struct metadata_configuration** configs, int* count);  /**< Get metadata configurations. */
int onvif_media_set_metadata_configuration(const char* configuration_token,
                                           const struct metadata_configuration* config); /**< Set metadata configuration. */
int onvif_media_handle_request(const char* action_name, const http_request_t* request,
                               http_response_t* response); /**< Handle ONVIF media service requests. */

/* Media service functions */

/**
 * @brief Initialize media service
 * @return 0 on success, negative error code on failure
 * @note Uses config_runtime API for configuration access
 */
int onvif_media_init(void);

/**
 * @brief Clean up media service
 */
void onvif_media_cleanup(void);

#endif
