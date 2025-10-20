/**
 * @file rtsp_rtcp.c
 * @brief RTSP RTCP Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all RTCP (RTP Control Protocol) related functions
 * for the RTSP server, including statistics tracking and quality monitoring.
 */

#include "rtsp_rtcp.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <time.h>
#include <unistd.h>

#include "platform/platform.h"
#include "rtsp_types.h"
#include "utils/common/time_utils.h"

/* ==================== RTCP Functions ==================== */

/**
 * Initialize RTCP session
 */
int rtcp_init_session(struct rtp_session* rtp_session) {
  if (!rtp_session)
    return -1;

  // Initialize statistics
  memset(&rtp_session->stats, 0, sizeof(struct rtcp_stats));
  rtp_session->last_rtcp_sent = 0;
  rtp_session->last_rtcp_received = 0;
  rtp_session->rtcp_enabled = true;

  return 0;
}

/**
 * Cleanup RTCP session
 */
void rtcp_cleanup_session(struct rtp_session* rtp_session) {
  if (!rtp_session)
    return;

  rtp_session->rtcp_enabled = false;

  // Cancel RTCP thread if running
  if (rtp_session->rtcp_thread) {
    pthread_cancel(rtp_session->rtcp_thread);
    pthread_join(rtp_session->rtcp_thread, NULL);
    rtp_session->rtcp_thread = 0;
  }
}

/**
 * Send RTCP Sender Report (SR)
 */
int rtcp_send_sr(struct rtp_session* rtp_session) {
  if (!rtp_session || !rtp_session->rtcp_enabled)
    return -1;

  uint8_t packet[28]; // Minimum SR packet size
  int pos = 0;

  // Version (2), Padding (0), RC (0), PT (200 = SR)
  packet[pos++] = 0x80 | 0x00 | 0x00 | 0x00;

  // Length (6 words = 24 bytes)
  packet[pos++] = 0x00;
  packet[pos++] = 0x06;

  // SSRC
  packet[pos++] = (rtp_session->ssrc >> 24) & 0xFF;
  packet[pos++] = (rtp_session->ssrc >> 16) & 0xFF;
  packet[pos++] = (rtp_session->ssrc >> 8) & 0xFF;
  packet[pos++] = rtp_session->ssrc & 0xFF;

  // NTP timestamp (simplified - use current time)
  uint64_t ntp_time = time(NULL) + 2208988800ULL; // Convert to NTP epoch
  packet[pos++] = (ntp_time >> 56) & 0xFF;
  packet[pos++] = (ntp_time >> 48) & 0xFF;
  packet[pos++] = (ntp_time >> 40) & 0xFF;
  packet[pos++] = (ntp_time >> 32) & 0xFF;
  packet[pos++] = (ntp_time >> 24) & 0xFF;
  packet[pos++] = (ntp_time >> 16) & 0xFF;
  packet[pos++] = (ntp_time >> 8) & 0xFF;
  packet[pos++] = ntp_time & 0xFF;

  // RTP timestamp
  packet[pos++] = (rtp_session->timestamp >> 24) & 0xFF;
  packet[pos++] = (rtp_session->timestamp >> 16) & 0xFF;
  packet[pos++] = (rtp_session->timestamp >> 8) & 0xFF;
  packet[pos++] = rtp_session->timestamp & 0xFF;

  // Sender's packet count
  packet[pos++] = (rtp_session->stats.packets_sent >> 24) & 0xFF;
  packet[pos++] = (rtp_session->stats.packets_sent >> 16) & 0xFF;
  packet[pos++] = (rtp_session->stats.packets_sent >> 8) & 0xFF;
  packet[pos++] = rtp_session->stats.packets_sent & 0xFF;

  // Sender's octet count
  packet[pos++] = (rtp_session->stats.octets_sent >> 24) & 0xFF;
  packet[pos++] = (rtp_session->stats.octets_sent >> 16) & 0xFF;
  packet[pos++] = (rtp_session->stats.octets_sent >> 8) & 0xFF;
  packet[pos++] = rtp_session->stats.octets_sent & 0xFF;

  // Send packet
  if (rtp_session->rtcp_sockfd >= 0) {
    sendto(rtp_session->rtcp_sockfd, packet, pos, 0, (struct sockaddr*)&rtp_session->client_rtcp_addr, sizeof(rtp_session->client_rtcp_addr));
  }

  rtp_session->last_rtcp_sent = time(NULL);
  return 0;
}

/**
 * Send RTCP Receiver Report (RR)
 */
int rtcp_send_rr(struct rtp_session* rtp_session) {
  if (!rtp_session || !rtp_session->rtcp_enabled)
    return -1;

  uint8_t packet[32]; // Minimum RR packet size
  int pos = 0;

  // Version (2), Padding (0), RC (1), PT (201 = RR)
  packet[pos++] = 0x80 | 0x00 | 0x01 | 0x00;

  // Length (7 words = 28 bytes)
  packet[pos++] = 0x00;
  packet[pos++] = 0x07;

  // SSRC
  packet[pos++] = (rtp_session->ssrc >> 24) & 0xFF;
  packet[pos++] = (rtp_session->ssrc >> 16) & 0xFF;
  packet[pos++] = (rtp_session->ssrc >> 8) & 0xFF;
  packet[pos++] = rtp_session->ssrc & 0xFF;

  // Report block (simplified - all zeros for now)
  memset(packet + pos, 0, 24);
  pos += 24;

  // Send packet
  if (rtp_session->rtcp_sockfd >= 0) {
    sendto(rtp_session->rtcp_sockfd, packet, pos, 0, (struct sockaddr*)&rtp_session->client_rtcp_addr, sizeof(rtp_session->client_rtcp_addr));
  }

  rtp_session->last_rtcp_sent = time(NULL);
  return 0;
}

/**
 * Handle incoming RTCP packet
 */
int rtcp_handle_packet(struct rtp_session* rtp_session, const uint8_t* data, size_t len) {
  if (!rtp_session || !data || len < 4)
    return -1;

  // Parse RTCP header
  uint8_t version = (data[0] >> 6) & 0x03;
  uint8_t pt = data[1];

  if (version != 2)
    return -1; // Invalid version

  rtp_session->last_rtcp_received = time(NULL);

  // Handle different packet types
  switch (pt) {
  case RTCP_SR:
    // Sender Report - update statistics
    platform_log_debug("Received RTCP SR\n");
    break;

  case RTCP_RR:
    // Receiver Report - client feedback
    platform_log_debug("Received RTCP RR\n");
    break;

  case RTCP_SDES:
    // Source Description
    platform_log_debug("Received RTCP SDES\n");
    break;

  case RTCP_BYE:
    // Goodbye
    platform_log_debug("Received RTCP BYE\n");
    break;

  case RTCP_APP:
    // Application-defined
    platform_log_debug("Received RTCP APP\n");
    break;

  default:
    platform_log_debug("Received unknown RTCP packet type: %d\n", pt);
    break;
  }

  return 0;
}

/**
 * RTCP thread function
 */
void* rtcp_thread(void* arg) {
  struct rtp_session* rtp_session = (struct rtp_session*)arg;
  if (!rtp_session)
    return NULL;

  platform_log_notice("RTCP thread started\n");

  uint8_t buffer[1500];
  struct sockaddr_in client_addr;
  socklen_t addr_len = sizeof(client_addr);

  while (rtp_session->rtcp_enabled) {
    // Send periodic reports
    time_t now = time(NULL);
    if (now - rtp_session->last_rtcp_sent >= RTSP_RTCP_INTERVAL_SEC) {
      rtcp_send_sr(rtp_session);
    }

    // Check for incoming RTCP packets
    if (rtp_session->rtcp_sockfd >= 0) {
      fd_set readfds;
      struct timeval timeout;

      FD_ZERO(&readfds);
      FD_SET(rtp_session->rtcp_sockfd, &readfds);

      timeout.tv_sec = 1;
      timeout.tv_usec = 0;

      int result = select(rtp_session->rtcp_sockfd + 1, &readfds, NULL, NULL, &timeout);
      if (result > 0 && FD_ISSET(rtp_session->rtcp_sockfd, &readfds)) {
        ssize_t bytes = recvfrom(rtp_session->rtcp_sockfd, buffer, sizeof(buffer), 0, (struct sockaddr*)&client_addr, &addr_len);
        if (bytes > 0) {
          rtcp_handle_packet(rtp_session, buffer, bytes);
        }
      }
    }

    // Small delay to prevent busy waiting
    sleep_ms(100); // 100ms
  }

  platform_log_notice("RTCP thread finished\n");
  return NULL;
}
