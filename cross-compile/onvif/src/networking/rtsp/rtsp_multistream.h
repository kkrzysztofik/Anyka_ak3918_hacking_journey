/**
 * @file rtsp_multistream.h
 * @brief Multi-stream RTSP server support
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides support for a single RTSP server that can handle
 * multiple video streams on different paths (e.g., /vs0, /vs1).
 */

#ifndef RTSP_MULTISTREAM_H
#define RTSP_MULTISTREAM_H

#include <bits/pthreadtypes.h>
#include <stdbool.h>
#include <stdint.h>

#include "platform/platform_common.h"
#include "rtsp_types.h"
#include "services/common/video_config_types.h"

/* Maximum number of streams supported by the multi-stream server */
#define RTSP_MAX_STREAMS 4

/* Stream information structure */
typedef struct {
  char path[64];                      /* Stream path (e.g., "/vs0") */
  char name[64];                      /* Stream name (e.g., "main") */
  bool enabled;                       /* Whether this stream is enabled */
  platform_venc_handle_t venc_handle; /* Video encoder handle */
  platform_venc_stream_handle_t
      venc_stream_handle;         /* Video stream handle for get/release */
  bool encoder_initialized;       /* Whether encoder is initialized */
  video_config_t video_config;    /* Video configuration */
  audio_config_t audio_config;    /* Audio configuration */
  bool audio_enabled;             /* Whether audio is enabled */
  platform_ai_handle_t ai_handle; /* Audio input handle */
  platform_aenc_stream_handle_t aenc_handle; /* Audio encoder stream handle */
  bool audio_encoder_initialized; /* Whether audio encoder is initialized */

  /* H.264 parameter sets for this stream */
  char h264_sps_b64[256];
  char h264_pps_b64[256];

  /* Statistics for this stream */
  uint64_t bytes_sent;
  uint64_t frames_sent;
  uint64_t audio_frames_sent;
} rtsp_stream_info_t;

/* Statistics structure to avoid parameter swapping */
typedef struct {
  uint64_t bytes_sent;
  uint64_t frames_sent;
  uint64_t audio_frames_sent;
} rtsp_stream_stats_t;

typedef struct {
  uint64_t total_bytes_sent;
  uint64_t total_frames_sent;
  uint64_t total_audio_frames_sent;
  int total_sessions;
} rtsp_total_stats_t;

/* Multi-stream RTSP server structure */
typedef struct {
  /* Basic server configuration */
  int port;
  bool running;
  int listen_sockfd;
  platform_vi_handle_t vi_handle;

  /* Threading */
  pthread_t accept_thread;
  pthread_t encoder_thread;
  pthread_t audio_thread;
  pthread_t timeout_thread;

  /* Sessions */
  rtsp_session_t *sessions;
  pthread_mutex_t sessions_mutex;
  int sessions_count;

  /* Streams */
  rtsp_stream_info_t streams[RTSP_MAX_STREAMS];
  int stream_count;
  pthread_mutex_t streams_mutex;

  /* Authentication */
  struct rtsp_auth_config auth_config;

  /* SDP session */
  struct sdp_session sdp_session;
} rtsp_multistream_server_t;

/* Multi-stream server functions */
rtsp_multistream_server_t *rtsp_multistream_server_create(
    int port, platform_vi_handle_t vi_handle);
int rtsp_multistream_server_add_stream(rtsp_multistream_server_t *server,
                                       const char *path, const char *name,
                                       const video_config_t *video_config,
                                       const audio_config_t *audio_config,
                                       bool audio_enabled);
int rtsp_multistream_server_start(rtsp_multistream_server_t *server);
int rtsp_multistream_server_stop(rtsp_multistream_server_t *server);
int rtsp_multistream_server_destroy(rtsp_multistream_server_t *server);

/* Stream management functions */
rtsp_stream_info_t *rtsp_multistream_get_stream(
    rtsp_multistream_server_t *server, const char *path);
int rtsp_multistream_remove_stream(rtsp_multistream_server_t *server,
                                   const char *path);
int rtsp_multistream_get_stream_count(rtsp_multistream_server_t *server);

/* Statistics functions */
int rtsp_multistream_get_stats(rtsp_multistream_server_t *server,
                               const char *path, rtsp_stream_stats_t *stats);
int rtsp_multistream_get_total_stats(rtsp_multistream_server_t *server,
                                     rtsp_total_stats_t *stats);

#endif /* RTSP_MULTISTREAM_H */
