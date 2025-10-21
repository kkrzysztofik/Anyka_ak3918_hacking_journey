/**
 * @file rtsp_rtcp.h
 * @brief RTSP RTCP Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all RTCP (RTP Control Protocol) related function
 * declarations for the RTSP server, including statistics tracking and
 * quality monitoring.
 */

#ifndef RTSP_RTCP_H
#define RTSP_RTCP_H

#include <bits/types.h>
#include <stdint.h>

#include "rtsp_types.h"

/* RTCP functions */
/**
 * @brief Initialize RTCP session
 * @param rtp_session RTP session to initialize RTCP for
 * @return 0 on success, -1 on error
 */
int rtcp_init_session(struct rtp_session* rtp_session);

/**
 * @brief Cleanup RTCP session
 * @param rtp_session RTP session to cleanup RTCP for
 */
void rtcp_cleanup_session(struct rtp_session* rtp_session);

/**
 * @brief Send RTCP Sender Report (SR)
 * @param rtp_session RTP session
 * @return 0 on success, -1 on error
 */
int rtcp_send_sr(struct rtp_session* rtp_session);

/**
 * @brief Send RTCP Receiver Report (RR)
 * @param rtp_session RTP session
 * @return 0 on success, -1 on error
 */
int rtcp_send_rr(struct rtp_session* rtp_session);

/**
 * @brief Handle incoming RTCP packet
 * @param rtp_session RTP session
 * @param data RTCP packet data
 * @param len Length of packet data
 * @return 0 on success, -1 on error
 */
int rtcp_handle_packet(struct rtp_session* rtp_session, const uint8_t* data, size_t len);

/**
 * @brief RTCP thread function
 * @param arg RTP session pointer
 * @return NULL
 */
void* rtcp_thread(void* arg);

#endif /* RTSP_RTCP_H */
