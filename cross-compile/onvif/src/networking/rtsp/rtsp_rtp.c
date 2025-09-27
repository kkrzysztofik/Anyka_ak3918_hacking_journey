/**
 * @file rtsp_rtp.c
 * @brief RTSP RTP Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all RTP (Real-time Transport Protocol) related functions
 * for the RTSP server, including RTP packet creation, transmission, and encoder
 * setup.
 */

#include "rtsp_rtp.h"

#include "platform/platform.h"
#include "rtsp_rtcp.h"
#include "rtsp_session.h"
#include "rtsp_types.h"

#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

/* ==================== RTP Functions ==================== */

/**
 * Setup video encoder
 */
int rtsp_setup_encoder(rtsp_server_t* server) {
  if (!server)
    return -1;

  // Create video encoder using platform abstraction with config from server
  platform_video_config_t venc_config;
  venc_config.width = server->config.video_config.width;
  venc_config.height = server->config.video_config.height;
  venc_config.fps = server->config.video_config.fps;
  venc_config.bitrate = server->config.video_config.bitrate; // bitrate is already in kbps
  venc_config.codec = PLATFORM_VIDEO_CODEC_H264;             // Default to H264

  if (platform_venc_init(&server->venc_handle, &venc_config) != PLATFORM_SUCCESS) {
    platform_log_error("Failed to create video encoder\n");
    return -1;
  }

  // Request stream binding between VI and VENC (video capture already started
  // globally)
  if (platform_venc_request_stream(server->vi_handle, server->venc_handle,
                                   &server->venc_stream_handle) != PLATFORM_SUCCESS) {
    platform_log_error("Failed to request video stream\n");
    platform_venc_cleanup(server->venc_handle);
    server->venc_handle = NULL;
    return -1;
  }

  server->encoder_initialized = true;
  platform_log_notice("Video encoder created and stream started: %dx%d@%dfps, %dkbps\n",
                      venc_config.width, venc_config.height, venc_config.fps, venc_config.bitrate);
  return 0;
}

/**
 * Cleanup video encoder
 */
void rtsp_cleanup_encoder(rtsp_server_t* server) {
  if (!server)
    return;

  if (server->encoder_initialized) {
    // Cancel stream first
    if (server->venc_stream_handle) {
      platform_venc_cancel_stream(server->venc_stream_handle);
      server->venc_stream_handle = NULL;
    }

    // Stop video capture
    if (server->vi_handle) {
      platform_vi_capture_off(server->vi_handle);
    }

    // Cleanup encoder
    if (server->venc_handle) {
      platform_venc_cleanup(server->venc_handle);
      server->venc_handle = NULL;
    }

    server->encoder_initialized = false;
  }
}

/**
 * Setup audio encoder
 */
int rtsp_setup_audio_encoder(rtsp_server_t* server) {
  if (!server || !server->config.audio_enabled) {
    platform_log_debug("Audio disabled for RTSP server\n");
    return 0;
  }

  platform_log_debug("Setting up audio encoder for RTSP server\n");

  // Create audio input using platform abstraction with proper error handling
  platform_result_t ai_result = platform_ai_open(&server->ai_handle);
  if (ai_result != PLATFORM_SUCCESS) {
    platform_log_warning("Failed to create audio input (error: %d) - continuing without audio\n",
                         ai_result);
    server->config.audio_enabled = false; // Disable audio for this server
    return 0;                             // Return success but without audio
  }

  platform_log_debug("Audio input created successfully\n");

  // Create audio encoder using platform abstraction
  platform_audio_config_t aenc_config;
  aenc_config.sample_rate = 16000;
  aenc_config.channels = 1;
  aenc_config.bits_per_sample = 16;
  aenc_config.codec = PLATFORM_AUDIO_CODEC_AAC;

  platform_result_t aenc_result = platform_aenc_init(&server->aenc_handle, &aenc_config);
  if (aenc_result != PLATFORM_SUCCESS) {
    platform_log_warning("Failed to create audio encoder (error: %d) - cleaning up audio "
                         "input\n",
                         aenc_result);
    platform_ai_close(server->ai_handle);
    server->ai_handle = NULL;
    server->config.audio_enabled = false; // Disable audio for this server
    return 0;                             // Return success but without audio
  }

  server->audio_encoder_initialized = true;
  platform_log_info("Audio encoder created successfully: %dHz, %d channels, %d bits\n",
                    aenc_config.sample_rate, aenc_config.channels, aenc_config.bits_per_sample);
  return 0;
}

/**
 * Cleanup audio encoder
 */
void rtsp_cleanup_audio_encoder(rtsp_server_t* server) {
  if (!server)
    return;

  if (server->aenc_handle) {
    platform_aenc_cleanup(server->aenc_handle);
    server->aenc_handle = NULL;
  }

  if (server->ai_handle) {
    platform_ai_close(server->ai_handle);
    server->ai_handle = NULL;
  }

  server->audio_encoder_initialized = false;
}

/**
 * Initialize RTP session
 */
int rtsp_init_rtp_session(rtsp_session_t* session) {
  if (!session)
    return -1;

  // Generate random SSRC
  srand((unsigned int)time(NULL));
  session->rtp_session.ssrc = rand();
  session->rtp_session.sequence = 0;
  session->rtp_session.seq_num = 0;
  session->rtp_session.timestamp = 0;

  // Create RTP socket
  session->rtp_session.rtp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
  if (session->rtp_session.rtp_sockfd < 0) {
    platform_log_error("Failed to create RTP socket: %s\n", strerror(errno));
    return -1;
  }

  // Create RTCP socket
  session->rtp_session.rtcp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
  if (session->rtp_session.rtcp_sockfd < 0) {
    platform_log_error("Failed to create RTCP socket: %s\n", strerror(errno));
    close(session->rtp_session.rtp_sockfd);
    session->rtp_session.rtp_sockfd = -1;
    return -1;
  }

  // Bind RTP socket
  struct sockaddr_in rtp_addr;
  memset(&rtp_addr, 0, sizeof(rtp_addr));
  rtp_addr.sin_family = AF_INET;
  rtp_addr.sin_addr.s_addr = INADDR_ANY;
  rtp_addr.sin_port = 0; // Let system choose port

  if (bind(session->rtp_session.rtp_sockfd, (struct sockaddr*)&rtp_addr, sizeof(rtp_addr)) < 0) {
    platform_log_error("Failed to bind RTP socket: %s\n", strerror(errno));
    close(session->rtp_session.rtp_sockfd);
    close(session->rtp_session.rtcp_sockfd);
    session->rtp_session.rtp_sockfd = -1;
    session->rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  // Get assigned port
  socklen_t addr_len = sizeof(rtp_addr);
  getsockname(session->rtp_session.rtp_sockfd, (struct sockaddr*)&rtp_addr, &addr_len);
  session->rtp_session.rtp_port = ntohs(rtp_addr.sin_port);
  session->rtp_session.rtcp_port = session->rtp_session.rtp_port + 1;

  // Bind RTCP socket
  rtp_addr.sin_port = htons(session->rtp_session.rtcp_port);
  if (bind(session->rtp_session.rtcp_sockfd, (struct sockaddr*)&rtp_addr, sizeof(rtp_addr)) < 0) {
    platform_log_error("Failed to bind RTCP socket: %s\n", strerror(errno));
    close(session->rtp_session.rtp_sockfd);
    close(session->rtp_session.rtcp_sockfd);
    session->rtp_session.rtp_sockfd = -1;
    session->rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  // Set client address for UDP transmission
  memcpy(&session->rtp_session.client_addr, &session->addr, sizeof(session->addr));
  session->rtp_session.client_addr.sin_port = htons(session->rtp_session.rtp_port);

  // Initialize RTCP
  if (rtcp_init_session(&session->rtp_session) < 0) {
    platform_log_error("Failed to initialize RTCP session\n");
    close(session->rtp_session.rtp_sockfd);
    close(session->rtp_session.rtcp_sockfd);
    session->rtp_session.rtp_sockfd = -1;
    session->rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  // Start RTCP thread for UDP transport
  if (session->rtp_session.transport == RTP_TRANSPORT_UDP) {
    if (pthread_create(&session->rtp_session.rtcp_thread, NULL, rtcp_thread,
                       &session->rtp_session) != 0) {
      platform_log_error("Failed to create RTCP thread: %s\n", strerror(errno));
      rtcp_cleanup_session(&session->rtp_session);
      close(session->rtp_session.rtp_sockfd);
      close(session->rtp_session.rtcp_sockfd);
      session->rtp_session.rtp_sockfd = -1;
      session->rtp_session.rtcp_sockfd = -1;
      return -1;
    }
  }

  platform_log_notice("RTP session initialized: RTP port %d, RTCP port %d, SSRC %u\n",
                      session->rtp_session.rtp_port, session->rtp_session.rtcp_port,
                      session->rtp_session.ssrc);
  return 0;
}

/**
 * Cleanup RTP session
 */
void rtsp_cleanup_rtp_session(rtsp_session_t* session) {
  if (!session)
    return;

  // Cleanup RTCP
  rtcp_cleanup_session(&session->rtp_session);

  if (session->rtp_session.rtp_sockfd >= 0) {
    close(session->rtp_session.rtp_sockfd);
    session->rtp_session.rtp_sockfd = -1;
  }

  if (session->rtp_session.rtcp_sockfd >= 0) {
    close(session->rtp_session.rtcp_sockfd);
    session->rtp_session.rtcp_sockfd = -1;
  }
}

/**
 * Initialize audio RTP session
 */
int rtsp_init_audio_rtp_session(rtsp_session_t* session) {
  if (!session || !session->audio_enabled)
    return 0;

  // Generate random SSRC for audio
  srand((unsigned int)time(NULL));
  session->audio_rtp_session.ssrc = rand();
  session->audio_rtp_session.sequence = 0;
  session->audio_rtp_session.timestamp = 0;

  // Create audio RTP socket
  session->audio_rtp_session.rtp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
  if (session->audio_rtp_session.rtp_sockfd < 0) {
    platform_log_error("Failed to create audio RTP socket: %s\n", strerror(errno));
    return -1;
  }

  // Create audio RTCP socket
  session->audio_rtp_session.rtcp_sockfd = socket(AF_INET, SOCK_DGRAM, 0);
  if (session->audio_rtp_session.rtcp_sockfd < 0) {
    platform_log_error("Failed to create audio RTCP socket: %s\n", strerror(errno));
    close(session->audio_rtp_session.rtp_sockfd);
    session->audio_rtp_session.rtp_sockfd = -1;
    return -1;
  }

  // Bind audio RTP socket
  struct sockaddr_in rtp_addr;
  memset(&rtp_addr, 0, sizeof(rtp_addr));
  rtp_addr.sin_family = AF_INET;
  rtp_addr.sin_addr.s_addr = INADDR_ANY;
  rtp_addr.sin_port = 0; // Let system choose port

  if (bind(session->audio_rtp_session.rtp_sockfd, (struct sockaddr*)&rtp_addr, sizeof(rtp_addr)) <
      0) {
    platform_log_error("Failed to bind audio RTP socket: %s\n", strerror(errno));
    close(session->audio_rtp_session.rtp_sockfd);
    close(session->audio_rtp_session.rtcp_sockfd);
    session->audio_rtp_session.rtp_sockfd = -1;
    session->audio_rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  // Get assigned port
  socklen_t addr_len = sizeof(rtp_addr);
  getsockname(session->audio_rtp_session.rtp_sockfd, (struct sockaddr*)&rtp_addr, &addr_len);
  session->audio_rtp_session.rtp_port = ntohs(rtp_addr.sin_port);
  session->audio_rtp_session.rtcp_port = session->audio_rtp_session.rtp_port + 1;

  // Bind audio RTCP socket
  rtp_addr.sin_port = htons(session->audio_rtp_session.rtcp_port);
  if (bind(session->audio_rtp_session.rtcp_sockfd, (struct sockaddr*)&rtp_addr, sizeof(rtp_addr)) <
      0) {
    platform_log_error("Failed to bind audio RTCP socket: %s\n", strerror(errno));
    close(session->audio_rtp_session.rtp_sockfd);
    close(session->audio_rtp_session.rtcp_sockfd);
    session->audio_rtp_session.rtp_sockfd = -1;
    session->audio_rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  // Set client address for UDP transmission
  memcpy(&session->audio_rtp_session.client_addr, &session->addr, sizeof(session->addr));
  session->audio_rtp_session.client_addr.sin_port = htons(session->audio_rtp_session.rtp_port);

  // Initialize audio RTCP
  if (rtcp_init_session((struct rtp_session*)&session->audio_rtp_session) < 0) {
    platform_log_error("Failed to initialize audio RTCP session\n");
    close(session->audio_rtp_session.rtp_sockfd);
    close(session->audio_rtp_session.rtcp_sockfd);
    session->audio_rtp_session.rtp_sockfd = -1;
    session->audio_rtp_session.rtcp_sockfd = -1;
    return -1;
  }

  platform_log_notice("Audio RTP session initialized: RTP port %d, RTCP port %d, SSRC %u\n",
                      session->audio_rtp_session.rtp_port, session->audio_rtp_session.rtcp_port,
                      session->audio_rtp_session.ssrc);
  return 0;
}

/**
 * Cleanup audio RTP session
 */
void rtsp_cleanup_audio_rtp_session(rtsp_session_t* session) {
  if (!session)
    return;

  // Cleanup audio RTCP
  rtcp_cleanup_session((struct rtp_session*)&session->audio_rtp_session);

  if (session->audio_rtp_session.rtp_sockfd >= 0) {
    close(session->audio_rtp_session.rtp_sockfd);
    session->audio_rtp_session.rtp_sockfd = -1;
  }

  if (session->audio_rtp_session.rtcp_sockfd >= 0) {
    close(session->audio_rtp_session.rtcp_sockfd);
    session->audio_rtp_session.rtcp_sockfd = -1;
  }
}

/**
 * Send RTP packet
 */
int rtsp_send_rtp_packet(rtsp_session_t* session, const uint8_t* data, size_t len,
                         uint32_t timestamp) {
  if (!session || !data || len == 0)
    return -1;

  if (session->rtp_session.transport == RTP_TRANSPORT_UDP) {
    return rtsp_send_rtp_packet_udp(session, data, len, timestamp);
  } else if (session->rtp_session.transport == RTP_TRANSPORT_TCP) {
    return rtsp_send_rtp_packet_tcp(session, data, len, timestamp);
  }

  return -1;
}

/**
 * Send RTP packet via UDP
 */
int rtsp_send_rtp_packet_udp(rtsp_session_t* session, const uint8_t* data, size_t len,
                             uint32_t timestamp) {
  if (!session || !data || len == 0)
    return -1;

  uint8_t rtp_packet[1500];
  int pos = 0;

  // RTP header
  rtp_packet[pos++] = 0x80;        // Version (2), Padding (0), Extension (0), CC (0)
  rtp_packet[pos++] = RTP_PT_H264; // Payload type
  rtp_packet[pos++] = (session->rtp_session.seq_num >> 8) & 0xFF; // Sequence number
  rtp_packet[pos++] = session->rtp_session.seq_num & 0xFF;
  rtp_packet[pos++] = (timestamp >> 24) & 0xFF; // Timestamp
  rtp_packet[pos++] = (timestamp >> 16) & 0xFF;
  rtp_packet[pos++] = (timestamp >> 8) & 0xFF;
  rtp_packet[pos++] = timestamp & 0xFF;
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 24) & 0xFF; // SSRC
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 16) & 0xFF;
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 8) & 0xFF;
  rtp_packet[pos++] = session->rtp_session.ssrc & 0xFF;

  // Copy payload
  size_t payload_len = len;
  if (pos + payload_len > sizeof(rtp_packet)) {
    payload_len = sizeof(rtp_packet) - pos;
  }
  memcpy(rtp_packet + pos, data, payload_len);
  pos += payload_len;

  // Send packet
  ssize_t sent = sendto(session->rtp_session.rtp_sockfd, rtp_packet, pos, 0,
                        (struct sockaddr*)&session->rtp_session.client_addr,
                        sizeof(session->rtp_session.client_addr));

  if (sent != pos) {
    platform_log_error("Failed to send RTP packet: %s\n", strerror(errno));
    return -1;
  }

  // Update sequence number and statistics
  session->rtp_session.seq_num++;
  session->rtp_session.stats.packets_sent++;
  session->rtp_session.stats.octets_sent += payload_len;

  return payload_len;
}

/**
 * Send RTP packet via TCP
 */
int rtsp_send_rtp_packet_tcp(rtsp_session_t* session, const uint8_t* data, size_t len,
                             uint32_t timestamp) {
  if (!session || !data || len == 0)
    return -1;

  uint8_t rtp_packet[1500];
  int pos = 0;

  // RTP header
  rtp_packet[pos++] = 0x80;        // Version (2), Padding (0), Extension (0), CC (0)
  rtp_packet[pos++] = RTP_PT_H264; // Payload type
  rtp_packet[pos++] = (session->rtp_session.seq_num >> 8) & 0xFF; // Sequence number
  rtp_packet[pos++] = session->rtp_session.seq_num & 0xFF;
  rtp_packet[pos++] = (timestamp >> 24) & 0xFF; // Timestamp
  rtp_packet[pos++] = (timestamp >> 16) & 0xFF;
  rtp_packet[pos++] = (timestamp >> 8) & 0xFF;
  rtp_packet[pos++] = timestamp & 0xFF;
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 24) & 0xFF; // SSRC
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 16) & 0xFF;
  rtp_packet[pos++] = (session->rtp_session.ssrc >> 8) & 0xFF;
  rtp_packet[pos++] = session->rtp_session.ssrc & 0xFF;

  // Copy payload
  size_t payload_len = len;
  if (pos + payload_len > sizeof(rtp_packet)) {
    payload_len = sizeof(rtp_packet) - pos;
  }
  memcpy(rtp_packet + pos, data, payload_len);
  pos += payload_len;

  // Send via TCP interleaved
  uint8_t tcp_packet[1504];
  tcp_packet[0] = '$';
  tcp_packet[1] = session->rtp_session.tcp_channel_rtp;
  tcp_packet[2] = (pos >> 8) & 0xFF;
  tcp_packet[3] = pos & 0xFF;
  memcpy(tcp_packet + 4, rtp_packet, pos);

  ssize_t sent = send(session->sockfd, tcp_packet, pos + 4, 0);
  if (sent != pos + 4) {
    platform_log_error("Failed to send RTP packet via TCP: %s\n", strerror(errno));
    return -1;
  }

  // Update sequence number and statistics
  session->rtp_session.seq_num++;
  session->rtp_session.stats.packets_sent++;
  session->rtp_session.stats.octets_sent += payload_len;

  return payload_len;
}

/**
 * Send audio RTP packet
 */
int rtsp_send_audio_rtp_packet(rtsp_session_t* session, const uint8_t* data, size_t len,
                               uint32_t timestamp) {
  if (!session || !data || len == 0 || !session->audio_enabled)
    return -1;

  uint8_t rtp_packet[1500];
  int pos = 0;

  // RTP header
  rtp_packet[pos++] = 0x80;       // Version (2), Padding (0), Extension (0), CC (0)
  rtp_packet[pos++] = RTP_PT_AAC; // Payload type
  rtp_packet[pos++] = (session->audio_rtp_session.sequence >> 8) & 0xFF; // Sequence number
  rtp_packet[pos++] = session->audio_rtp_session.sequence & 0xFF;
  rtp_packet[pos++] = (timestamp >> 24) & 0xFF; // Timestamp
  rtp_packet[pos++] = (timestamp >> 16) & 0xFF;
  rtp_packet[pos++] = (timestamp >> 8) & 0xFF;
  rtp_packet[pos++] = timestamp & 0xFF;
  rtp_packet[pos++] = (session->audio_rtp_session.ssrc >> 24) & 0xFF; // SSRC
  rtp_packet[pos++] = (session->audio_rtp_session.ssrc >> 16) & 0xFF;
  rtp_packet[pos++] = (session->audio_rtp_session.ssrc >> 8) & 0xFF;
  rtp_packet[pos++] = session->audio_rtp_session.ssrc & 0xFF;

  // Copy payload
  size_t payload_len = len;
  if (pos + payload_len > sizeof(rtp_packet)) {
    payload_len = sizeof(rtp_packet) - pos;
  }
  memcpy(rtp_packet + pos, data, payload_len);
  pos += payload_len;

  // Send packet
  ssize_t sent = sendto(session->audio_rtp_session.rtp_sockfd, rtp_packet, pos, 0,
                        (struct sockaddr*)&session->audio_rtp_session.client_addr,
                        sizeof(session->audio_rtp_session.client_addr));

  if (sent != pos) {
    platform_log_error("Failed to send audio RTP packet: %s\n", strerror(errno));
    return -1;
  }

  // Update sequence number and statistics
  session->audio_rtp_session.sequence++;
  session->audio_rtp_session.stats.packets_sent++;
  session->audio_rtp_session.stats.octets_sent += payload_len;

  return payload_len;
}
