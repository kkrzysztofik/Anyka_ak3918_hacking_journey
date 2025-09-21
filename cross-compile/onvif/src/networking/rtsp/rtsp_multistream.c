/**
 * @file rtsp_multistream.c
 * @brief Multi-stream RTSP server implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "rtsp_multistream.h"

#include <arpa/inet.h>
#include <asm/socket.h>
#include <bits/pthreadtypes.h>
#include <errno.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#include "platform/platform.h"
#include "platform/platform_common.h"
#include "rtsp_auth.h"
#include "rtsp_rtp.h"
#include "rtsp_sdp.h"
#include "rtsp_session.h"
#include "rtsp_types.h"
#include "services/common/video_config_types.h"

/* Forward declarations */
static void *rtsp_multistream_accept_thread(void *arg);
static void *rtsp_multistream_encoder_thread(void *arg);
static void *rtsp_multistream_audio_thread(void *arg);
static void *rtsp_multistream_timeout_thread(void *arg);
static int rtsp_multistream_handle_request(rtsp_session_t *session,
                                           const char *request);
static int rtsp_multistream_send_response(rtsp_session_t *session, int code,
                                          const char *headers,
                                          const char *body);
static int rtsp_multistream_parse_method(const char *line);
static void rtsp_multistream_h264_extract_sps_pps(rtsp_stream_info_t *stream,
                                                  const uint8_t *buf,
                                                  size_t len);
static void rtsp_multistream_base64_encode(const uint8_t *input,
                                           size_t input_len, char *output,
                                           size_t output_len);
static void rtsp_multistream_get_local_ip_address(char *ip_str,
                                                  size_t ip_str_size);
static void rtsp_multistream_cleanup_video_encoder(
    rtsp_stream_info_t *stream, platform_vi_handle_t vi_handle);
static void rtsp_multistream_cleanup_audio_encoder(rtsp_stream_info_t *stream);
static int rtsp_multistream_process_video_stream(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream,
    int stream_index);
static int rtsp_multistream_get_video_stream_with_retry(
    rtsp_stream_info_t *stream, platform_venc_stream_t *venc_stream);
static void rtsp_multistream_send_to_sessions(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream,
    platform_venc_stream_t *venc_stream);
static int rtsp_multistream_process_audio_stream(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream);

/* Global session counter for unique session IDs */
static uint32_t g_rtsp_session_counter = 1;            // NOLINT
static pthread_mutex_t g_rtsp_session_counter_mutex =  // NOLINT
    PTHREAD_MUTEX_INITIALIZER;

/**
 * Create multi-stream RTSP server
 */
rtsp_multistream_server_t *rtsp_multistream_server_create(
    int port, platform_vi_handle_t vi_handle) {
  rtsp_multistream_server_t *server =
      calloc(1, sizeof(rtsp_multistream_server_t));
  if (!server) {
    platform_log_error("Failed to allocate multi-stream server memory\n");
    return NULL;
  }

  server->port = port;
  server->vi_handle = vi_handle;
  server->listen_sockfd = -1;
  server->running = false;
  server->sessions = NULL;
  server->stream_count = 0;

  // Initialize mutexes
  if (pthread_mutex_init(&server->sessions_mutex, NULL) != 0) {
    platform_log_error("Failed to initialize sessions mutex\n");
    free(server);
    return NULL;
  }

  if (pthread_mutex_init(&server->streams_mutex, NULL) != 0) {
    platform_log_error("Failed to initialize streams mutex\n");
    pthread_mutex_destroy(&server->sessions_mutex);
    free(server);
    return NULL;
  }

  // Initialize authentication
  rtsp_auth_init(&server->auth_config);

  // Initialize SDP session
  sdp_init_session(&server->sdp_session, "Multi-Stream RTSP Server", NULL);

  platform_log_notice("Multi-stream RTSP server created on port %d\n", port);

  return server;
}

/**
 * Add a stream to the multi-stream server
 */
int rtsp_multistream_server_add_stream(rtsp_multistream_server_t *server,
                                       const char *path, const char *name,
                                       const video_config_t *video_config,
                                       const audio_config_t *audio_config,
                                       bool audio_enabled) {
  if (!server || !path || !name || !video_config) {
    platform_log_error("Invalid parameters for stream addition\n");
    return -1;
  }

  pthread_mutex_lock(&server->streams_mutex);

  if (server->stream_count >= RTSP_MAX_STREAMS) {
    platform_log_error("Maximum number of streams reached\n");
    pthread_mutex_unlock(&server->streams_mutex);
    return -1;
  }

  // Find an empty slot
  int stream_index = -1;
  for (int i = 0; i < RTSP_MAX_STREAMS; i++) {
    if (!server->streams[i].enabled) {
      stream_index = i;
      break;
    }
  }

  if (stream_index == -1) {
    platform_log_error("No available stream slots\n");
    pthread_mutex_unlock(&server->streams_mutex);
    return -1;
  }

  rtsp_stream_info_t *stream = &server->streams[stream_index];

  // Initialize stream
  strncpy(stream->path, path, sizeof(stream->path) - 1);
  stream->path[sizeof(stream->path) - 1] = '\0';

  strncpy(stream->name, name, sizeof(stream->name) - 1);
  stream->name[sizeof(stream->name) - 1] = '\0';

  stream->enabled = true;
  stream->encoder_initialized = false;
  stream->audio_enabled = audio_enabled;
  stream->audio_encoder_initialized = false;

  // Copy video configuration
  memcpy(&stream->video_config, video_config, sizeof(video_config_t));

  // Copy audio configuration if provided
  if (audio_config) {
    memcpy(&stream->audio_config, audio_config, sizeof(audio_config_t));
  } else {
    memset(&stream->audio_config, 0, sizeof(audio_config_t));
  }

  // Initialize H.264 parameter sets
  stream->h264_sps_b64[0] = '\0';
  stream->h264_pps_b64[0] = '\0';

  // Initialize statistics
  stream->bytes_sent = 0;
  stream->frames_sent = 0;
  stream->audio_frames_sent = 0;

  // Initialize video encoder with proper configuration mapping
  platform_video_config_t venc_config = {0};

  // Map video configuration to platform format
  venc_config.width = video_config->width;
  venc_config.height = video_config->height;
  venc_config.fps = video_config->fps;
  venc_config.bitrate = video_config->bitrate;
  venc_config.codec = (platform_video_codec_t)video_config->codec_type;

  // Map bitrate mode from video config
  venc_config.br_mode = video_config->br_mode;

  // Map profile from video config
  venc_config.profile = video_config->profile;

  // Additional configuration for smart encoding (if needed)
  // Note: Smart encoding configuration would be applied after encoder
  // initialization

  platform_log_debug(
      "rtsp_multistream_server_add_stream: Creating encoder for stream %s "
      "(%dx%d@%dfps, %dkbps, codec=%d, br_mode=%d, profile=%d)\n",
      path, venc_config.width, venc_config.height, venc_config.fps,
      venc_config.bitrate, venc_config.codec, venc_config.br_mode,
      venc_config.profile);

  if (platform_venc_init(&stream->venc_handle, &venc_config) !=
      PLATFORM_SUCCESS) {
    platform_log_error("Failed to create video encoder for stream %s\n", path);
    stream->enabled = false;
    pthread_mutex_unlock(&server->streams_mutex);
    return -1;
  }

  // Request stream binding between VI and VENC (video capture already started
  // globally) Note: This should be called after video capture is started
  if (platform_venc_request_stream(server->vi_handle, stream->venc_handle,
                                   &stream->venc_stream_handle) !=
      PLATFORM_SUCCESS) {
    platform_log_error("Failed to request video stream for stream %s\n", path);
    platform_venc_cleanup(stream->venc_handle);
    stream->venc_handle = NULL;
    stream->enabled = false;
    pthread_mutex_unlock(&server->streams_mutex);
    return -1;
  }

  stream->encoder_initialized = true;

  // Smart encoding configuration would be added here if supported by the
  // platform For now, VBR mode uses the platform's default smart encoding
  // settings
  if (video_config->br_mode == PLATFORM_BR_MODE_VBR) {
    platform_log_debug(
        "rtsp_multistream_server_add_stream: VBR mode enabled for stream %s, "
        "using platform defaults\n",
        path);
  }

  platform_log_debug(
      "rtsp_multistream_server_add_stream: Stream %s initialized "
      "successfully\n",
      path);

  // Audio encoder completely disabled to prevent segmentation fault
  // if (audio_enabled && audio_config) {
  //     platform_log_debug("Initializing audio for stream %s (rate=%d,
  //     channels=%d, bits=%d)\n",
  //                        path, audio_config->sample_rate,
  //                        audio_config->channels,
  //                        audio_config->bits_per_sample);
  //
  //     // Create audio input with proper error handling
  //     platform_result_t ai_result = platform_ai_open(&stream->ai_handle);
  //     if (ai_result != PLATFORM_SUCCESS) {
  //         platform_log_warning("Failed to create audio input for stream %s
  //         (error: %d) - continuing without audio\n",
  //                             path, ai_result);
  //         stream->audio_enabled = false;
  //         stream->ai_handle = NULL;
  //     } else {
  //         platform_log_debug("Audio input created successfully for stream
  //         %s\n", path);
  //
  //         // Create audio encoder with proper error handling
  //         platform_audio_config_t aenc_config;
  //         aenc_config.sample_rate = audio_config->sample_rate;
  //         aenc_config.channels = audio_config->channels;
  //         aenc_config.bits_per_sample = audio_config->bits_per_sample;
  //         aenc_config.codec = audio_config->codec_type;
  //
  //         platform_result_t aenc_result =
  //         platform_aenc_init(&stream->aenc_handle, &aenc_config); if
  //         (aenc_result != PLATFORM_SUCCESS) {
  //             platform_log_warning("Failed to create audio encoder for stream
  //             %s (error: %d) - cleaning up audio input\n",
  //                                 path, aenc_result);
  //             platform_ai_close(stream->ai_handle);
  //             stream->ai_handle = NULL;
  //             stream->audio_enabled = false;
  //         } else {
  //             stream->audio_encoder_initialized = true;
  //             platform_log_info("Audio encoder created successfully for
  //             stream %s\n", path);
  //         }
  //     }
  // } else {
  //     platform_log_debug("Audio disabled for stream %s\n", path);
  //     stream->audio_enabled = false;
  // }

  // Force audio to be disabled
  platform_log_debug("Audio completely disabled for stream %s\n", path);
  stream->audio_enabled = false;
  stream->ai_handle = NULL;
  stream->aenc_handle = NULL;
  stream->audio_encoder_initialized = false;

  server->stream_count++;

  pthread_mutex_unlock(&server->streams_mutex);

  platform_log_notice(
      "Stream %s (%s) added: %dx%d@%dfps, %dkbps (Audio: disabled)\n", path,
      name, video_config->width, video_config->height, video_config->fps,
      video_config->bitrate);

  return 0;
}

/**
 * Start multi-stream RTSP server
 */
int rtsp_multistream_server_start(rtsp_multistream_server_t *server) {
  if (!server) {
    platform_log_error("Invalid server parameter\n");
    return -1;
  }

  if (server->running) {
    platform_log_warning("Server is already running\n");
    return 0;
  }

  // Create socket
  server->listen_sockfd = socket(AF_INET, SOCK_STREAM, 0);
  if (server->listen_sockfd < 0) {
    platform_log_error("Failed to create socket: %s\n", strerror(errno));
    return -1;
  }

  // Set socket options
  int opt = 1;
  if (setsockopt(server->listen_sockfd, SOL_SOCKET, SO_REUSEADDR, &opt,
                 sizeof(opt)) < 0) {
    platform_log_error("Failed to set socket options: %s\n", strerror(errno));
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  // Bind socket
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = INADDR_ANY;
  addr.sin_port = htons(server->port);

  if (bind(server->listen_sockfd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
    platform_log_error("Failed to bind socket: %s\n", strerror(errno));
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  // Listen for connections
  if (listen(server->listen_sockfd, 10) < 0) {
    platform_log_error("Failed to listen on socket: %s\n", strerror(errno));
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  // Start threads
  server->running = true;

  if (pthread_create(&server->accept_thread, NULL,
                     rtsp_multistream_accept_thread, server) != 0) {
    platform_log_error("Failed to create accept thread: %s\n", strerror(errno));
    server->running = false;
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  if (pthread_create(&server->encoder_thread, NULL,
                     rtsp_multistream_encoder_thread, server) != 0) {
    platform_log_error("Failed to create encoder thread: %s\n",
                       strerror(errno));
    server->running = false;
    (void)pthread_cancel(server->accept_thread);
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  // Audio thread disabled to prevent segmentation fault
  // if (pthread_create(&server->audio_thread, NULL,
  // rtsp_multistream_audio_thread, server) != 0) {
  //     platform_log_error("Failed to create audio thread: %s\n",
  //     strerror(errno)); server->running = false;
  //     (void)pthread_cancel(server->accept_thread);
  //     (void)pthread_cancel(server->encoder_thread);
  //     close(server->listen_sockfd);
  //     server->listen_sockfd = -1;
  //     return -1;
  // }

  if (pthread_create(&server->timeout_thread, NULL,
                     rtsp_multistream_timeout_thread, server) != 0) {
    platform_log_error("Failed to create timeout thread: %s\n",
                       strerror(errno));
    server->running = false;
    (void)pthread_cancel(server->accept_thread);
    (void)pthread_cancel(server->encoder_thread);
    // pthread_cancel(server->audio_thread);  // Audio thread disabled
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
    return -1;
  }

  platform_log_notice("Multi-stream RTSP server started on port %d\n",
                      server->port);
  return 0;
}

/**
 * Stop multi-stream RTSP server
 */
int rtsp_multistream_server_stop(rtsp_multistream_server_t *server) {
  if (!server) {
    return -1;
  }

  if (!server->running) {
    return 0;
  }

  server->running = false;

  // Close listening socket
  if (server->listen_sockfd >= 0) {
    close(server->listen_sockfd);
    server->listen_sockfd = -1;
  }

  // Wait for threads to finish
  pthread_join(server->accept_thread, NULL);
  pthread_join(server->encoder_thread, NULL);
  // pthread_join(server->audio_thread, NULL);  // Audio thread disabled
  pthread_join(server->timeout_thread, NULL);

  // Cleanup all sessions
  rtsp_session_cleanup_all((rtsp_server_t *)server);

  platform_log_notice("Multi-stream RTSP server stopped\n");
  return 0;
}

/**
 * Cleanup video encoder for a stream
 */
static void rtsp_multistream_cleanup_video_encoder(
    rtsp_stream_info_t *stream, platform_vi_handle_t vi_handle) {
  if (!stream || !stream->encoder_initialized) {
    return;
  }

  platform_log_debug(
      "rtsp_multistream_cleanup_video_encoder: Cleaning up video encoder\n");

  // Cancel stream first
  if (stream->venc_stream_handle) {
    platform_venc_cancel_stream(stream->venc_stream_handle);
    stream->venc_stream_handle = NULL;
  }

  // Stop video capture
  if (vi_handle) {
    platform_vi_capture_off(vi_handle);
  }

  // Cleanup encoder
  if (stream->venc_handle) {
    platform_venc_cleanup(stream->venc_handle);
    stream->venc_handle = NULL;
  }

  stream->encoder_initialized = false;
}

/**
 * Cleanup audio encoder for a stream
 */
static void rtsp_multistream_cleanup_audio_encoder(rtsp_stream_info_t *stream) {
  if (!stream || !stream->audio_encoder_initialized) {
    return;
  }

  platform_log_debug(
      "rtsp_multistream_cleanup_audio_encoder: Cleaning up audio encoder\n");

  if (stream->aenc_handle) {
    platform_aenc_cleanup(stream->aenc_handle);
    stream->aenc_handle = NULL;
  }
  if (stream->ai_handle) {
    platform_ai_close(stream->ai_handle);
    stream->ai_handle = NULL;
  }
  stream->audio_encoder_initialized = false;
}

/**
 * Get video stream with retry mechanism
 */
static int rtsp_multistream_get_video_stream_with_retry(
    rtsp_stream_info_t *stream, platform_venc_stream_t *venc_stream) {
  platform_result_t stream_result = PLATFORM_ERROR;
  int retry_count = 0;
  const int max_retries = 3;

  // Retry mechanism for getting video stream
  while (retry_count < max_retries && stream_result != PLATFORM_SUCCESS) {
    stream_result = platform_venc_get_stream_by_handle(
        stream->venc_stream_handle, venc_stream, 100);

    if (stream_result == PLATFORM_SUCCESS) {
      break;  // Success
    }
    if (stream_result == PLATFORM_ERROR) {
      // Log error and retry with delay
      platform_log_debug(
          "Multi-stream encoder: Failed to get stream (attempt %d/%d, "
          "result=%d)\n",
          retry_count + 1, max_retries, stream_result);

      if (retry_count < max_retries - 1) {
        platform_sleep_ms(20);  // Wait before retry
      }
    }

    retry_count++;
  }

  return (stream_result == PLATFORM_SUCCESS) ? 0 : -1;
}

/**
 * Send video stream to active sessions
 */
static void rtsp_multistream_send_to_sessions(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream,
    platform_venc_stream_t *venc_stream) {
  // Send to all active sessions for this stream with safe mutex handling
  if (pthread_mutex_lock(&server->sessions_mutex) == 0) {
    rtsp_session_t *session = server->sessions;
    while (session) {
      if (session->active && session->state == RTSP_STATE_PLAYING) {
        // Check if this session is for this stream
        if (strstr(session->uri, stream->path)) {
          rtsp_send_rtp_packet(session, venc_stream->data, venc_stream->len,
                               venc_stream->timestamp);
        }
      }
      session = session->next;
    }
    pthread_mutex_unlock(&server->sessions_mutex);
  } else {
    platform_log_debug(
        "Multi-stream encoder: sessions_mutex lock failed, skipping "
        "session processing\n");
  }
}

/**
 * Process a single video stream
 */
static int rtsp_multistream_process_video_stream(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream,
    int stream_index) {
  if (!stream->enabled || !stream->encoder_initialized) {
    return 0;
  }

  // Validate stream handle before use
  if (!stream->venc_stream_handle) {
    platform_log_error(
        "Multi-stream encoder: Stream %d has NULL stream handle\n",
        stream_index);
    return -1;
  }

  // Get video stream using stream handle with error handling and retry
  // mechanism
  platform_venc_stream_t venc_stream;
  if (rtsp_multistream_get_video_stream_with_retry(stream, &venc_stream) != 0) {
    platform_log_debug(
        "Multi-stream encoder: Failed to get stream for stream %d after "
        "retries\n",
        stream_index);
    return -1;
  }

  // Validate stream data before processing
  if (!venc_stream.data || venc_stream.len == 0) {
    platform_log_warning(
        "Multi-stream encoder: Empty stream data for stream %d\n",
        stream_index);
    platform_venc_release_stream_by_handle(stream->venc_stream_handle,
                                           &venc_stream);
    return -1;
  }

  // Extract SPS/PPS from keyframes (only for H.264 streams)
  if (stream->video_config.codec_type == PLATFORM_H264_ENC_TYPE) {
    if (!stream->h264_sps_b64[0] || !stream->h264_pps_b64[0]) {
      rtsp_multistream_h264_extract_sps_pps(stream, venc_stream.data,
                                            venc_stream.len);
    }
  }

  // Send to all active sessions for this stream
  rtsp_multistream_send_to_sessions(server, stream, &venc_stream);

  // Update statistics
  stream->bytes_sent += venc_stream.len;
  stream->frames_sent++;

  // Release stream using stream handle
  platform_venc_release_stream_by_handle(stream->venc_stream_handle,
                                         &venc_stream);

  return 0;
}

/**
 * Process a single audio stream
 */
static int rtsp_multistream_process_audio_stream(
    rtsp_multistream_server_t *server, rtsp_stream_info_t *stream) {
  if (!stream->enabled || !stream->audio_enabled ||
      !stream->audio_encoder_initialized) {
    return 0;
  }

  // Get audio stream using platform abstraction
  platform_aenc_stream_t aenc_stream;
  if (platform_aenc_get_stream(stream->aenc_handle, &aenc_stream, 100) !=
      PLATFORM_SUCCESS) {
    return -1;
  }

  // Send to all active sessions for this stream
  pthread_mutex_lock(&server->sessions_mutex);
  rtsp_session_t *session = server->sessions;
  while (session) {
    if (session->active && session->state == RTSP_STATE_PLAYING &&
        session->audio_enabled) {
      // Check if this session is for this stream
      if (strstr(session->uri, stream->path)) {
        rtsp_send_audio_rtp_packet(session, aenc_stream.data, aenc_stream.len,
                                   aenc_stream.timestamp);
      }
    }
    session = session->next;
  }
  pthread_mutex_unlock(&server->sessions_mutex);

  // Update statistics
  stream->audio_frames_sent++;

  // Release stream using platform abstraction
  platform_aenc_release_stream(stream->aenc_handle, &aenc_stream);

  return 0;
}

/**
 * Destroy multi-stream RTSP server
 */
int rtsp_multistream_server_destroy(rtsp_multistream_server_t *server) {
  if (!server) {
    return -1;
  }

  rtsp_multistream_server_stop(server);

  // Cleanup all streams with proper error handling
  pthread_mutex_lock(&server->streams_mutex);
  for (int stream_index = 0; stream_index < RTSP_MAX_STREAMS; stream_index++) {
    rtsp_stream_info_t *stream = &server->streams[stream_index];
    if (stream->enabled) {
      // Cleanup video encoder
      rtsp_multistream_cleanup_video_encoder(stream, server->vi_handle);

      // Cleanup audio encoder
      rtsp_multistream_cleanup_audio_encoder(stream);

      // Reset stream state
      stream->enabled = false;
    }
  }
  pthread_mutex_unlock(&server->streams_mutex);

  // Cleanup authentication
  rtsp_auth_cleanup(&server->auth_config);

  // Cleanup SDP session
  sdp_cleanup_session(&server->sdp_session);

  pthread_mutex_destroy(&server->sessions_mutex);
  pthread_mutex_destroy(&server->streams_mutex);
  free(server);
  return 0;
}

/**
 * Get stream by path
 */
rtsp_stream_info_t *rtsp_multistream_get_stream(
    rtsp_multistream_server_t *server, const char *path) {
  if (!server || !path) {
    return NULL;
  }

  pthread_mutex_lock(&server->streams_mutex);

  for (int i = 0; i < RTSP_MAX_STREAMS; i++) {
    rtsp_stream_info_t *stream = &server->streams[i];
    if (stream->enabled && strcmp(stream->path, path) == 0) {
      pthread_mutex_unlock(&server->streams_mutex);
      return stream;
    }
  }

  pthread_mutex_unlock(&server->streams_mutex);
  return NULL;
}

/**
 * Get stream count
 */
int rtsp_multistream_get_stream_count(rtsp_multistream_server_t *server) {
  if (!server) {
    return 0;
  }

  pthread_mutex_lock(&server->streams_mutex);
  int count = server->stream_count;
  pthread_mutex_unlock(&server->streams_mutex);

  return count;
}

/**
 * Get stream statistics
 */
int rtsp_multistream_get_stats(rtsp_multistream_server_t *server,
                               const char *path, rtsp_stream_stats_t *stats) {
  if (!server || !path || !stats) {
    return -1;
  }

  rtsp_stream_info_t *stream = rtsp_multistream_get_stream(server, path);
  if (!stream) {
    return -1;
  }

  stats->bytes_sent = stream->bytes_sent;
  stats->frames_sent = stream->frames_sent;
  stats->audio_frames_sent = stream->audio_frames_sent;

  return 0;
}

/**
 * Get total statistics
 */
int rtsp_multistream_get_total_stats(rtsp_multistream_server_t *server,
                                     rtsp_total_stats_t *stats) {
  if (!server || !stats) {
    return -1;
  }

  uint64_t bytes = 0;
  uint64_t frames = 0;
  uint64_t audio_frames = 0;

  pthread_mutex_lock(&server->streams_mutex);
  for (int stream_index = 0; stream_index < RTSP_MAX_STREAMS; stream_index++) {
    rtsp_stream_info_t *stream = &server->streams[stream_index];
    if (stream->enabled) {
      bytes += stream->bytes_sent;
      frames += stream->frames_sent;
      audio_frames += stream->audio_frames_sent;
    }
  }
  pthread_mutex_unlock(&server->streams_mutex);

  stats->total_bytes_sent = bytes;
  stats->total_frames_sent = frames;
  stats->total_audio_frames_sent = audio_frames;
  stats->total_sessions = server->sessions_count;

  return 0;
}

/* ==================== Thread Functions ==================== */

/**
 * Accept thread - handles new connections
 */
static void *rtsp_multistream_accept_thread(void *arg) {
  rtsp_multistream_server_t *server = (rtsp_multistream_server_t *)arg;

  platform_log_notice("Multi-stream RTSP accept thread started\n");

  while (server->running) {
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);

    int client_sockfd = accept(server->listen_sockfd,
                               (struct sockaddr *)&client_addr, &client_len);
    if (client_sockfd < 0) {
      if (server->running) {
        platform_log_error("Failed to accept connection: %s\n",
                           strerror(errno));
      }
      continue;
    }

    // Create new session
    rtsp_session_t *session = calloc(1, sizeof(rtsp_session_t));
    if (!session) {
      platform_log_error("Failed to allocate session memory\n");
      close(client_sockfd);
      continue;
    }

    session->sockfd = client_sockfd;
    memcpy(&session->addr, &client_addr, sizeof(client_addr));
    session->state = RTSP_STATE_INIT;
    session->active = true;
    session->audio_enabled = false;  // Will be set based on stream

    // Initialize session timeout
    session->timeout_seconds = RTSP_SESSION_TIMEOUT_SEC;
    session->last_activity = time(NULL);
    session->created_time = time(NULL);

    // Initialize authentication
    session->authenticated = false;
    session->auth_username[0] = '\0';
    session->auth_nonce[0] = '\0';

    // Initialize headers
    session->headers = NULL;

    // Thread-safe session ID generation
    pthread_mutex_lock(&g_rtsp_session_counter_mutex);
    uint32_t session_id = g_rtsp_session_counter++;
    pthread_mutex_unlock(&g_rtsp_session_counter_mutex);
    (void)snprintf(session->session_id, sizeof(session->session_id), "%u",
                   session_id);

    // Allocate buffers
    session->recv_buffer = malloc(RTSP_BUFFER_SIZE);
    session->send_buffer = malloc(RTSP_BUFFER_SIZE);
    if (!session->recv_buffer || !session->send_buffer) {
      platform_log_error("Failed to allocate session buffers\n");
      free(session->recv_buffer);
      free(session->send_buffer);
      free(session);
      close(client_sockfd);
      continue;
    }

    session->recv_pos = 0;
    session->server = (rtsp_server_t *)server;  // Cast for compatibility

    // Add to server sessions
    rtsp_session_add((rtsp_server_t *)server, session);

    char client_ip[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &client_addr.sin_addr, client_ip, INET_ADDRSTRLEN);
    platform_log_notice("New RTSP connection from %s:%d (Session: %s)\n",
                        client_ip, ntohs(client_addr.sin_port),
                        session->session_id);
  }

  platform_log_notice("Multi-stream RTSP accept thread finished\n");
  return NULL;
}

/**
 * Encoder thread - handles video encoding and RTP transmission
 */
static void *rtsp_multistream_encoder_thread(void *arg) {
  rtsp_multistream_server_t *server = (rtsp_multistream_server_t *)arg;

  platform_log_notice("Multi-stream RTSP encoder thread started\n");

  while (server->running) {
    // Use safe mutex lock with timeout and corruption recovery
    if (pthread_mutex_lock(&server->streams_mutex) != 0) {
      // Mutex lock failed, wait and continue
      platform_sleep_ms(10);
      continue;
    }

    // Process each enabled stream
    for (int stream_index = 0; stream_index < RTSP_MAX_STREAMS;
         stream_index++) {
      rtsp_stream_info_t *stream = &server->streams[stream_index];
      rtsp_multistream_process_video_stream(server, stream, stream_index);
    }

    pthread_mutex_unlock(&server->streams_mutex);

    // Small delay to prevent busy waiting (similar to reference implementation)
    platform_sleep_ms(10);  // 10ms
  }

  platform_log_notice("Multi-stream RTSP encoder thread finished\n");
  return NULL;
}

/**
 * Audio thread - handles audio encoding and RTP transmission
 */
static void *rtsp_multistream_audio_thread(void *arg) {
  rtsp_multistream_server_t *server = (rtsp_multistream_server_t *)arg;

  platform_log_notice("Multi-stream RTSP audio thread started\n");

  while (server->running) {
    pthread_mutex_lock(&server->streams_mutex);

    // Process each enabled stream with audio
    for (int stream_index = 0; stream_index < RTSP_MAX_STREAMS;
         stream_index++) {
      rtsp_stream_info_t *stream = &server->streams[stream_index];
      rtsp_multistream_process_audio_stream(server, stream);
    }

    pthread_mutex_unlock(&server->streams_mutex);

    // Small delay to prevent busy waiting
    platform_sleep_ms(10);  // 10ms
  }

  platform_log_notice("Multi-stream RTSP audio thread finished\n");
  return NULL;
}

/**
 * Cleanup timeout sessions for multistream server
 */
static int rtsp_multistream_session_cleanup_timeout_sessions(
    rtsp_multistream_server_t *server) {
  if (!server) {
    return -1;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  rtsp_session_t *session = server->sessions;
  rtsp_session_t *prev = NULL;

  while (session) {
    // Store next pointer before any potential modifications
    rtsp_session_t *next = session->next;

    // Check if session has timed out
    if (rtsp_session_has_timed_out(session)) {
      platform_log_notice("Session %s timed out, cleaning up\n",
                          session->session_id);

      // Remove from linked list
      if (prev) {
        prev->next = session->next;
      } else {
        server->sessions = session->next;
      }

      rtsp_session_t *to_free = session;

      // Cleanup session
      rtsp_cleanup_session(to_free);
      free(to_free);

      // Move to next session (already stored in 'next')
      session = next;
    } else {
      prev = session;
      session = next;
    }
  }

  pthread_mutex_unlock(&server->sessions_mutex);
  return 0;
}

/**
 * Timeout thread - cleans up timed out sessions
 */
static void *rtsp_multistream_timeout_thread(void *arg) {
  rtsp_multistream_server_t *server = (rtsp_multistream_server_t *)arg;

  platform_log_notice("Multi-stream RTSP timeout thread started\n");

  while (server->running) {
    // Cleanup timeout sessions every 10 seconds
    rtsp_multistream_session_cleanup_timeout_sessions(server);
    platform_sleep_ms(10000);
  }

  platform_log_notice("Multi-stream RTSP timeout thread finished\n");
  return NULL;
}

/* ==================== RTSP Utility Functions ==================== */

/**
 * Validate RTSP request
 */
int rtsp_validate_request(const char *request, size_t request_len) {
  if (!request || request_len < 4) {
    return -1;
  }

  // Check for basic RTSP request format
  if (strncmp(request, "RTSP/", 5) == 0) {
    // Response format
    return 0;
  }

  // Check for method format (METHOD URI RTSP/1.0)
  const char *space1 = strchr(request, ' ');
  if (!space1) {
    return -1;
  }

  const char *space2 = strchr(space1 + 1, ' ');
  if (!space2) {
    return -1;
  }

  if (strncmp(space2 + 1, "RTSP/1.0", 8) != 0) {
    return -1;
  }

  return 0;
}

/**
 * Parse headers enhanced
 */
int rtsp_parse_headers_enhanced(const char *request,
                                struct rtsp_header **headers) {
  if (!request || !headers) {
    return -1;
  }

  *headers = NULL;
  struct rtsp_header *last = NULL;

  const char *line_start = strstr(request, "\r\n");
  if (!line_start) {
    return -1;
  }
  line_start += 2;  // Skip \r\n

  while (line_start && *line_start != '\0') {
    const char *line_end = strstr(line_start, "\r\n");
    if (!line_end) {
      break;
    }

    if (line_end == line_start) {
      break;  // Empty line, end of headers
    }

    // Find colon separator
    const char *colon = strchr(line_start, ':');
    if (!colon || colon >= line_end) {
      line_start = line_end + 2;
      continue;
    }

    // Create new header
    struct rtsp_header *header = malloc(sizeof(struct rtsp_header));
    if (!header) {
      return -1;
    }

    // Copy name
    size_t name_len = colon - line_start;
    if (name_len >= sizeof(header->name)) {
      name_len = sizeof(header->name) - 1;
    }
    strncpy(header->name, line_start, name_len);
    header->name[name_len] = '\0';

    // Copy value (skip colon and whitespace)
    const char *value_start = colon + 1;
    while (*value_start == ' ' || *value_start == '\t') {
      value_start++;
    }

    size_t value_len = line_end - value_start;
    if (value_len >= sizeof(header->value)) {
      value_len = sizeof(header->value) - 1;
    }
    strncpy(header->value, value_start, value_len);
    header->value[value_len] = '\0';

    header->next = NULL;

    // Add to list
    if (last) {
      last->next = header;
    } else {
      *headers = header;
    }
    last = header;

    line_start = line_end + 2;
  }

  return 0;
}

/**
 * Send RTSP error response
 */
int rtsp_send_error_response(rtsp_session_t *session, enum rtsp_error_code code,
                             const char *reason) {
  if (!session || !reason) {
    return -1;
  }

  char response[RTSP_BUFFER_SIZE];
  (void)snprintf(response, sizeof(response),
                 "RTSP/1.0 %d %s\r\n"
                 "CSeq: %d\r\n"
                 "Server: Multi-Stream RTSP Server\r\n"
                 "Content-Length: 0\r\n"
                 "\r\n",
                 code, reason, session->cseq);

  size_t response_len = strlen(response);
  int len = (int)response_len;
  if (send(session->sockfd, response, len, 0) != len) {
    platform_log_error("Failed to send error response: %s\n", strerror(errno));
    return -1;
  }

  return 0;
}

/**
 * Free RTSP headers
 */
void rtsp_free_headers(struct rtsp_header *headers) {
  while (headers) {
    struct rtsp_header *next = headers->next;
    free(headers);
    headers = next;
  }
}

/**
 * Cleanup RTSP session
 */
void rtsp_cleanup_session(rtsp_session_t *session) {
  if (!session) {
    return;
  }

  // Close socket
  if (session->sockfd >= 0) {
    close(session->sockfd);
    session->sockfd = -1;
  }

  // Cleanup RTP session
  rtsp_cleanup_rtp_session(session);

  // Cleanup audio RTP session
  rtsp_cleanup_audio_rtp_session(session);

  // Free buffers
  if (session->recv_buffer) {
    free(session->recv_buffer);
    session->recv_buffer = NULL;
  }

  if (session->send_buffer) {
    free(session->send_buffer);
    session->send_buffer = NULL;
  }

  // Free headers
  rtsp_free_headers(session->headers);
  session->headers = NULL;

  // Mark as inactive
  session->active = false;
  session->state = RTSP_STATE_INVALID;
}

/* ==================== Utility Functions ==================== */

/**
 * Extract H.264 SPS/PPS from stream
 */
static void rtsp_multistream_h264_extract_sps_pps(rtsp_stream_info_t *stream,
                                                  const uint8_t *buf,
                                                  size_t len) {
  for (size_t buf_index = 0; buf_index < len - 4; buf_index++) {
    if (buf[buf_index] == 0x00 && buf[buf_index + 1] == 0x00 &&
        buf[buf_index + 2] == 0x00 && buf[buf_index + 3] == 0x01) {
      uint8_t nal_type = buf[buf_index + 4] & 0x1F;
      size_t nal_start = buf_index + 4;
      size_t search_index = buf_index + 4;
      while (search_index < len - 4) {
        if (buf[search_index] == 0x00 && buf[search_index + 1] == 0x00 &&
            buf[search_index + 2] == 0x00 && buf[search_index + 3] == 0x01) {
          break;
        }
        search_index++;
      }
      size_t nal_end = search_index;
      if (nal_type == 7 && stream->h264_sps_b64[0] == '\0') {
        rtsp_multistream_base64_encode(buf + nal_start, nal_end - nal_start,
                                       stream->h264_sps_b64,
                                       sizeof(stream->h264_sps_b64));
      } else if (nal_type == 8 && stream->h264_pps_b64[0] == '\0') {
        rtsp_multistream_base64_encode(buf + nal_start, nal_end - nal_start,
                                       stream->h264_pps_b64,
                                       sizeof(stream->h264_pps_b64));
      }
      if (stream->h264_sps_b64[0] && stream->h264_pps_b64[0]) {
        return;
      }
      buf_index = search_index;
    }
  }
}

/**
 * Base64 encode
 */
static void rtsp_multistream_base64_encode(const uint8_t *input,
                                           size_t input_len, char *output,
                                           size_t output_len) {
  const char base64_chars[] =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  size_t input_index = 0;
  size_t output_index = 0;

  while (input_index < input_len && output_index < output_len - 1) {
    uint32_t byte_a = input_index < input_len ? input[input_index++] : 0;
    uint32_t byte_b = input_index < input_len ? input[input_index++] : 0;
    uint32_t byte_c = input_index < input_len ? input[input_index++] : 0;

    uint32_t triple = (byte_a << 16) | (byte_b << 8) | byte_c;

    if (output_index < output_len - 1) {
      output[output_index++] = base64_chars[(triple >> 18) & 0x3F];
    }
    if (output_index < output_len - 1) {
      output[output_index++] = base64_chars[(triple >> 12) & 0x3F];
    }
    if (output_index < output_len - 1) {
      output[output_index++] = base64_chars[(triple >> 6) & 0x3F];
    }
    if (output_index < output_len - 1) {
      output[output_index++] = base64_chars[triple & 0x3F];
    }
  }

  // Add padding
  while (output_index < output_len - 1 && (input_len * 4) % 3 != 0) {
    output[output_index++] = '=';
  }

  output[output_index] = '\0';
}

/**
 * Get local IP address
 */
static void rtsp_multistream_get_local_ip_address(char *ip_str,
                                                  size_t ip_str_size) {
  int sock = socket(AF_INET, SOCK_DGRAM, 0);
  if (sock < 0) {
    (void)strcpy(ip_str, "127.0.0.1");
    return;
  }

  struct sockaddr_in addr;
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = inet_addr("8.8.8.8");
  addr.sin_port = htons(80);

  if (connect(sock, (struct sockaddr *)&addr, sizeof(addr)) == 0) {
    socklen_t len = sizeof(addr);
    getsockname(sock, (struct sockaddr *)&addr, &len);
    inet_ntop(AF_INET, &addr.sin_addr, ip_str, ip_str_size);
  } else {
    (void)strcpy(ip_str, "127.0.0.1");
  }

  close(sock);
}
