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
  if (!rtp_session) {
    return -1;
  }

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
  if (!rtp_session) {
    return;
  }

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
  if (!rtp_session || !rtp_session->rtcp_enabled) {
    return -1;
  }

  uint8_t packet[RTCP_SR_PACKET_SIZE];
  int pos = 0;

  // Version (2), Padding (0), RC (0), PT (200 = SR)
  packet[pos++] = RTCP_VERSION_BYTE;

  // Length (6 words = 24 bytes)
  packet[pos++] = 0x00;
  packet[pos++] = RTCP_SR_LENGTH_WORDS;

  // SSRC
  packet[pos++] = (rtp_session->ssrc >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->ssrc >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->ssrc >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = rtp_session->ssrc & RTP_BYTE_MASK;

  // NTP timestamp (simplified - use current time)
  uint64_t ntp_time = time(NULL) + NTP_OFFSET;
  packet[pos++] = (ntp_time >> NTP_FRAC_SHIFT_56) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> NTP_FRAC_SHIFT_48) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> NTP_FRAC_SHIFT_40) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> NTP_FRAC_SHIFT_32) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (ntp_time >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = ntp_time & RTP_BYTE_MASK;

  // RTP timestamp
  packet[pos++] = (rtp_session->timestamp >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->timestamp >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->timestamp >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = rtp_session->timestamp & RTP_BYTE_MASK;

  // Sender's packet count
  packet[pos++] = (rtp_session->stats.packets_sent >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->stats.packets_sent >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->stats.packets_sent >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = rtp_session->stats.packets_sent & RTP_BYTE_MASK;

  // Sender's octet count
  packet[pos++] = (rtp_session->stats.octets_sent >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->stats.octets_sent >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->stats.octets_sent >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = rtp_session->stats.octets_sent & RTP_BYTE_MASK;

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
  if (!rtp_session || !rtp_session->rtcp_enabled) {
    return -1;
  }

  uint8_t packet[RTCP_RR_PACKET_SIZE];
  int pos = 0;

  // Version (2), Padding (0), RC (1), PT (201 = RR)
  packet[pos++] = RTCP_RR_VERSION_RC1;

  // Length (7 words = 28 bytes)
  packet[pos++] = 0x00;
  packet[pos++] = RTCP_RR_LENGTH_WORDS;

  // SSRC
  packet[pos++] = (rtp_session->ssrc >> SHIFT_24_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->ssrc >> SHIFT_16_BITS) & RTP_BYTE_MASK;
  packet[pos++] = (rtp_session->ssrc >> SHIFT_8_BITS) & RTP_BYTE_MASK;
  packet[pos++] = rtp_session->ssrc & RTP_BYTE_MASK;

  // Report block (simplified - all zeros for now)
  memset(packet + pos, 0, RTCP_REPORT_BLOCK_SIZE);
  pos += RTCP_REPORT_BLOCK_SIZE;

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
  if (!rtp_session || !data || len < 4) {
    return -1;
  }

  // Parse RTCP header
  uint8_t version = (data[0] >> RTP_VERSION_SHIFT) & RTP_VERSION_BITS_MASK;
  uint8_t pt = data[1];  // NOLINT(readability-identifier-length) - standard RTCP payload type abbreviation

  if (version != RTCP_VERSION) {
    return -1; // Invalid version
  }

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
  if (!rtp_session) {
    return NULL;
  }

  platform_log_notice("RTCP thread started\n");

  uint8_t buffer[RTP_MAX_PACKET_SIZE];
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
    sleep_ms(RTCP_THREAD_POLL_DELAY_MS);
  }

  platform_log_notice("RTCP thread finished\n");
  return NULL;
}
