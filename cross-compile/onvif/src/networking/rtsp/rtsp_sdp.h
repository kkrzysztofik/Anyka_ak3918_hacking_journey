/**
 * @file rtsp_sdp.h
 * @brief RTSP SDP Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all SDP (Session Description Protocol) related
 * function declarations for the RTSP server, including SDP generation,
 * parsing, and validation.
 */

#ifndef RTSP_SDP_H
#define RTSP_SDP_H

#include "rtsp_types.h"

/* SDP functions */
/**
 * @brief Initialize SDP session
 * @param sdp SDP session to initialize
 * @param session_name Name of the session
 * @param origin Origin information
 * @return 0 on success, -1 on error
 */
int sdp_init_session(struct sdp_session *sdp, const char *session_name,
                     const char *origin);

/**
 * @brief Cleanup SDP session
 * @param sdp SDP session to cleanup
 */
void sdp_cleanup_session(struct sdp_session *sdp);

/**
 * @brief Add media description to SDP session
 * @param sdp SDP session
 * @param type Media type (video, audio, application)
 * @param port Port number
 * @param protocol Protocol (RTP/AVP, RTP/AVPF)
 * @param payload_type RTP payload type
 * @param encoding Encoding name (H264, PCMU, etc.)
 * @param clock_rate Clock rate in Hz
 * @param channels Number of channels (for audio)
 * @return 0 on success, -1 on error
 */
int sdp_add_media(struct sdp_session *sdp, sdp_media_type_t type, int port,
                  const char *protocol, int payload_type, const char *encoding,
                  int clock_rate, int channels);

/**
 * @brief Set media direction
 * @param sdp SDP session
 * @param type Media type
 * @param direction Direction (sendrecv, sendonly, recvonly, inactive)
 * @return 0 on success, -1 on error
 */
int sdp_set_media_direction(struct sdp_session *sdp, sdp_media_type_t type,
                            sdp_direction_t direction);

/**
 * @brief Set media control URL
 * @param sdp SDP session
 * @param type Media type
 * @param control Control URL
 * @return 0 on success, -1 on error
 */
int sdp_set_media_control(struct sdp_session *sdp, sdp_media_type_t type,
                          const char *control);

/**
 * @brief Set media format parameters
 * @param sdp SDP session
 * @param type Media type
 * @param fmtp Format parameters
 * @return 0 on success, -1 on error
 */
int sdp_set_media_fmtp(struct sdp_session *sdp, sdp_media_type_t type,
                       const char *fmtp);

/**
 * @brief Set media RTCP feedback parameters
 * @param sdp SDP session
 * @param type Media type
 * @param rtcp_fb RTCP feedback parameters
 * @return 0 on success, -1 on error
 */
int sdp_set_media_rtcp_fb(struct sdp_session *sdp, sdp_media_type_t type,
                          const char *rtcp_fb);

/**
 * @brief Set media extension map
 * @param sdp SDP session
 * @param type Media type
 * @param extmap Extension map
 * @return 0 on success, -1 on error
 */
int sdp_set_media_extmap(struct sdp_session *sdp, sdp_media_type_t type,
                         const char *extmap);

/**
 * @brief Set media identification
 * @param sdp SDP session
 * @param type Media type
 * @param mid Media identification
 * @return 0 on success, -1 on error
 */
int sdp_set_media_mid(struct sdp_session *sdp, sdp_media_type_t type,
                      const char *mid);

/**
 * @brief Set media SSRC
 * @param sdp SDP session
 * @param type Media type
 * @param ssrc SSRC value
 * @return 0 on success, -1 on error
 */
int sdp_set_media_ssrc(struct sdp_session *sdp, sdp_media_type_t type,
                       const char *ssrc);

/**
 * @brief Generate SDP text from session
 * @param sdp SDP session
 * @param buffer Buffer to store SDP text
 * @param buffer_size Size of buffer
 * @return 0 on success, -1 on error
 */
int sdp_generate(struct sdp_session *sdp, char *buffer, size_t buffer_size);

/**
 * @brief Parse SDP text into session
 * @param sdp SDP session to populate
 * @param sdp_text SDP text to parse
 * @return 0 on success, -1 on error
 */
int sdp_parse(struct sdp_session *sdp, const char *sdp_text);

/**
 * @brief Validate SDP text
 * @param sdp_text SDP text to validate
 * @return 0 if valid, -1 if invalid
 */
int sdp_validate(const char *sdp_text);

#endif /* RTSP_SDP_H */
