/**
 * @file rtsp_rtp.h
 * @brief RTSP RTP Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all RTP (Real-time Transport Protocol) related
 * function declarations for the RTSP server, including RTP packet
 * creation, transmission, and encoder setup.
 */

#ifndef RTSP_RTP_H
#define RTSP_RTP_H

#include "rtsp_types.h"

/* RTP functions */
/**
 * @brief Setup video encoder for RTP streaming
 * @param server RTSP server instance
 * @return 0 on success, -1 on error
 */
int rtsp_setup_encoder(rtsp_server_t* server);

/**
 * @brief Cleanup video encoder
 * @param server RTSP server instance
 */
void rtsp_cleanup_encoder(rtsp_server_t* server);

/**
 * @brief Setup audio encoder for RTP streaming
 * @param server RTSP server instance
 * @return 0 on success, -1 on error
 */
int rtsp_setup_audio_encoder(rtsp_server_t* server);

/**
 * @brief Cleanup audio encoder
 * @param server RTSP server instance
 */
void rtsp_cleanup_audio_encoder(rtsp_server_t* server);

/**
 * @brief Initialize RTP session for video
 * @param session RTSP session
 * @return 0 on success, -1 on error
 */
int rtsp_init_rtp_session(rtsp_session_t* session);

/**
 * @brief Cleanup RTP session for video
 * @param session RTSP session
 */
void rtsp_cleanup_rtp_session(rtsp_session_t* session);

/**
 * @brief Initialize RTP session for audio
 * @param session RTSP session
 * @return 0 on success, -1 on error
 */
int rtsp_init_audio_rtp_session(rtsp_session_t* session);

/**
 * @brief Cleanup RTP session for audio
 * @param session RTSP session
 */
void rtsp_cleanup_audio_rtp_session(rtsp_session_t* session);

/**
 * @brief Send RTP packet for video
 * @param session RTSP session
 * @param data Video data to send
 * @param len Length of data
 * @param timestamp RTP timestamp
 * @return 0 on success, -1 on error
 */
int rtsp_send_rtp_packet(rtsp_session_t* session, const uint8_t* data, size_t len, uint32_t timestamp);

/**
 * @brief Send RTP packet for audio
 * @param session RTSP session
 * @param data Audio data to send
 * @param len Length of data
 * @param timestamp RTP timestamp
 * @return 0 on success, -1 on error
 */
int rtsp_send_audio_rtp_packet(rtsp_session_t* session, const uint8_t* data, size_t len, uint32_t timestamp);

/**
 * @brief Send RTP packet over UDP
 * @param session RTSP session
 * @param data Data to send
 * @param len Length of data
 * @param timestamp RTP timestamp
 * @return 0 on success, -1 on error
 */
int rtsp_send_rtp_packet_udp(rtsp_session_t* session, const uint8_t* data, size_t len, uint32_t timestamp);

/**
 * @brief Send RTP packet over TCP (interleaved)
 * @param session RTSP session
 * @param data Data to send
 * @param len Length of data
 * @param timestamp RTP timestamp
 * @return 0 on success, -1 on error
 */
int rtsp_send_rtp_packet_tcp(rtsp_session_t* session, const uint8_t* data, size_t len, uint32_t timestamp);

#endif /* RTSP_RTP_H */
