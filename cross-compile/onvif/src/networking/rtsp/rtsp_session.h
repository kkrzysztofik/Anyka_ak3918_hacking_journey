/**
 * @file rtsp_session.h
 * @brief RTSP Session Management Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all session management related function declarations
 * for the RTSP server, including session creation, cleanup, timeout
 * handling, and statistics.
 */

#ifndef RTSP_SESSION_H
#define RTSP_SESSION_H

#include <bits/pthreadtypes.h>
#include <bits/types.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <time.h>

#include "rtsp_types.h"

/* RTSP Session Constants */
#define RTSP_MIN_USERSPACE_ADDRESS 0x1000 /**< Minimum valid userspace memory address */
#define RTSP_MIN_BUFFER_SIZE       64     /**< Minimum buffer size for session info */

/* RTSP session structure definition */
struct rtsp_session {
  int sockfd;
  struct sockaddr_in addr;
  enum rtsp_session_state state;
  bool active;
  pthread_t thread;
  char session_id[RTSP_SESSION_ID_SIZE];
  int cseq;                   /* RTSP sequence number */
  char uri[RTSP_MAX_URI_LEN]; /* Request URI */

  /* Session timeout */
  time_t last_activity;
  time_t created_time;
  int timeout_seconds;

  /* Headers */
  struct rtsp_header* headers;

  /* Authentication */
  bool authenticated;
  char auth_username[RTSP_MAX_USERNAME_LEN];
  char auth_nonce[RTSP_MAX_NONCE_LEN];

  /* Buffers */
  char* recv_buffer;
  char* send_buffer;
  size_t recv_pos;

  /* RTP session */
  struct rtp_session rtp_session;

  /* Audio RTP session */
  struct audio_rtp_session audio_rtp_session;
  bool audio_enabled;

  /* Linked list */
  struct rtsp_session* next;
  struct rtsp_session* prev;
  struct rtsp_server* server; /* back-pointer */
};

/* Enhanced RTSP functions */
/**
 * @brief Parse RTSP headers from request
 * @param request RTSP request string
 * @param headers Pointer to store parsed headers
 * @return 0 on success, -1 on error
 */
int rtsp_parse_headers(const char* request, struct rtsp_header** headers);

/**
 * @brief Free RTSP headers
 * @param headers Headers to free
 */
void rtsp_free_headers(struct rtsp_header* headers);

/**
 * @brief Get header value by name
 * @param headers Headers list
 * @param name Header name to find
 * @return Header value or NULL if not found
 */
const char* rtsp_get_header(struct rtsp_header* headers, const char* name);

/**
 * @brief Validate RTSP request
 * @param request Request string
 * @param len Length of request
 * @return 0 if valid, -1 if invalid
 */
int rtsp_validate_request(const char* request, size_t len);

/**
 * @brief Send error response
 * @param session RTSP session
 * @param code Error code
 * @param reason Error reason
 * @return 0 on success, -1 on error
 */
int rtsp_send_error_response(rtsp_session_t* session, enum rtsp_error_code code, const char* reason);

/**
 * @brief Check if session has timed out
 * @param session RTSP session
 * @return 0 if not timed out, 1 if timed out
 */
int rtsp_check_session_timeout(rtsp_session_t* session);

/**
 * @brief Update session activity timestamp
 * @param session RTSP session
 */
void rtsp_update_session_activity(rtsp_session_t* session);

/**
 * @brief Enhanced header parsing
 * @param request RTSP request string
 * @param headers Pointer to store parsed headers
 * @return 0 on success, -1 on error
 */
int rtsp_parse_headers_enhanced(const char* request, struct rtsp_header** headers);

/**
 * @brief Handle authentication required response
 * @param session RTSP session
 * @return 0 on success, -1 on error
 */
int rtsp_handle_auth_required(rtsp_session_t* session);

/**
 * @brief Generate WWW-Authenticate header
 * @param session RTSP session
 * @param header Buffer to store header
 * @param header_size Size of header buffer
 * @return 0 on success, -1 on error
 */
int rtsp_generate_www_authenticate_header(rtsp_session_t* session, char* header, size_t header_size);

/* Session management functions */
/**
 * @brief Cleanup timeout sessions
 * @param server RTSP server
 * @return Number of sessions cleaned up
 */
int rtsp_session_cleanup_timeout_sessions(rtsp_server_t* server);

/**
 * @brief Set session timeout
 * @param session RTSP session
 * @param timeout_seconds Timeout in seconds
 * @return 0 on success, -1 on error
 */
int rtsp_session_set_timeout(rtsp_session_t* session, int timeout_seconds);

/**
 * @brief Cleanup all sessions
 * @param server RTSP server
 */
void rtsp_session_cleanup_all(rtsp_server_t* server);

/**
 * @brief Get session count
 * @param server RTSP server
 * @return Number of active sessions
 */
int rtsp_session_get_count(rtsp_server_t* server);

/**
 * @brief Add session to server
 * @param server RTSP server
 * @param session Session to add
 * @return 0 on success, -1 on error
 */
int rtsp_session_add(rtsp_server_t* server, rtsp_session_t* session);

/**
 * @brief Remove session from server
 * @param server RTSP server
 * @param session Session to remove
 * @return 0 on success, -1 on error
 */
int rtsp_session_remove(rtsp_server_t* server, rtsp_session_t* session);

/**
 * @brief Check if session has timed out
 * @param session RTSP session
 * @return true if timed out, false otherwise
 */
bool rtsp_session_has_timed_out(rtsp_session_t* session);

/**
 * @brief Cleanup session resources
 * @param session RTSP session
 */
void rtsp_cleanup_session(rtsp_session_t* session);

#endif /* RTSP_SESSION_H */
