/**
 * @file rtsp_session.c
 * @brief RTSP Session Management Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all session management related functions for the RTSP
 * server, including session creation, cleanup, timeout handling, and
 * statistics.
 */

#include "rtsp_session.h"

#include <arpa/inet.h>
#include <bits/types.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>

#include "platform.h"
#include "rtsp_types.h"

/* ==================== Utility Functions ==================== */

/**
 * @brief Safely validate a session pointer
 * @param session Session pointer to validate
 * @return true if valid, false if invalid
 * @note This function helps prevent crashes from corrupted pointers
 */
static bool rtsp_session_pointer_is_valid(rtsp_session_t* session) {
  if (!session) {
    return false;
  }

  // Check for obviously invalid pointers (below typical user space)
  if ((uintptr_t)session < 0x1000) {
    platform_log_error("rtsp_session_pointer_is_valid: Invalid session pointer (0x%p)\n", session);
    return false;
  }

  // Check for alignment (should be 4-byte aligned)
  if ((uintptr_t)session & 0x3) {
    platform_log_error("rtsp_session_pointer_is_valid: Misaligned session pointer (0x%p)\n",
                       session);
    return false;
  }

  return true;
}

/* ==================== Session Management Functions ==================== */

/**
 * Update session activity timestamp
 */
void rtsp_update_session_activity(rtsp_session_t* session) {
  if (!session) {
    return;
  }

  session->last_activity = time(NULL);
}

/**
 * Check if session has timed out
 */
bool rtsp_session_has_timed_out(rtsp_session_t* session) {
  if (!rtsp_session_pointer_is_valid(session)) {
    return false;
  }

  time_t now = time(NULL);
  return (now - session->last_activity) > session->timeout_seconds;
}

/**
 * Cleanup timeout sessions
 */
int rtsp_session_cleanup_timeout_sessions(rtsp_server_t* server) {
  if (!server) {
    return -1;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  rtsp_session_t* session = server->sessions;
  rtsp_session_t* prev = NULL;

  while (session) {
    // Store next pointer before any potential modifications
    rtsp_session_t* next = session->next;

    // Validate session pointer before using it
    if (!rtsp_session_pointer_is_valid(session)) {
      platform_log_error("rtsp_session_cleanup_timeout_sessions: Invalid session pointer "
                         "(0x%p), removing from list\n",
                         session);

      // Remove corrupted session from linked list
      if (prev) {
        prev->next = next;
      } else {
        server->sessions = next;
      }

      // Don't try to access the corrupted session
      session = next;
      continue;
    }

    if (rtsp_session_has_timed_out(session)) {
      platform_log_notice("Session %s timed out, cleaning up\n", session->session_id);

      // Remove from linked list
      if (prev) {
        prev->next = session->next;
      } else {
        server->sessions = session->next;
      }

      rtsp_session_t* to_free = session;

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
 * Set session timeout
 */
int rtsp_session_set_timeout(rtsp_session_t* session, int timeout_seconds) {
  if (!session) {
    return -1;
  }

  session->timeout_seconds = timeout_seconds;
  session->last_activity = time(NULL);
  session->created_time = time(NULL);

  return 0;
}

/**
 * Get session by ID
 */
rtsp_session_t* rtsp_session_get_by_id(rtsp_server_t* server, const char* session_id) {
  if (!server || !session_id) {
    return NULL;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  rtsp_session_t* session = server->sessions;
  while (session) {
    if (strcmp(session->session_id, session_id) == 0) {
      pthread_mutex_unlock(&server->sessions_mutex);
      return session;
    }
    session = session->next;
  }

  pthread_mutex_unlock(&server->sessions_mutex);
  return NULL;
}

/**
 * Get session count
 */
int rtsp_session_get_count(rtsp_server_t* server) {
  if (!server) {
    return -1;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  int count = 0;
  rtsp_session_t* session = server->sessions;
  while (session) {
    count++;
    session = session->next;
  }

  pthread_mutex_unlock(&server->sessions_mutex);
  return count;
}

/**
 * Cleanup all sessions
 */
void rtsp_session_cleanup_all(rtsp_server_t* server) {
  if (!server) {
    return;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  rtsp_session_t* session = server->sessions;
  while (session) {
    rtsp_session_t* next = session->next;
    rtsp_cleanup_session(session);
    free(session);
    session = next;
  }

  server->sessions = NULL;
  pthread_mutex_unlock(&server->sessions_mutex);
}

/**
 * Add session to server
 */
int rtsp_session_add(rtsp_server_t* server, rtsp_session_t* session) {
  if (!server || !session) {
    return -1;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  session->next = server->sessions;
  if (server->sessions) {
    server->sessions->prev = session;
  }
  server->sessions = session;
  session->prev = NULL;

  pthread_mutex_unlock(&server->sessions_mutex);
  return 0;
}

/**
 * Remove session from server
 */
int rtsp_session_remove(rtsp_server_t* server, rtsp_session_t* session) {
  if (!server || !session) {
    return -1;
  }

  pthread_mutex_lock(&server->sessions_mutex);

  if (session->prev) {
    session->prev->next = session->next;
  } else {
    server->sessions = session->next;
  }

  if (session->next) {
    session->next->prev = session->prev;
  }

  pthread_mutex_unlock(&server->sessions_mutex);
  return 0;
}

/**
 * Get session statistics
 */
int rtsp_session_get_stats(rtsp_session_t* session, uint64_t* bytes_sent, uint32_t* packets_sent,
                           time_t* last_activity) {
  if (!session) {
    return -1;
  }

  if (bytes_sent) {
    *bytes_sent = session->rtp_session.stats.octets_sent;
  }

  if (packets_sent) {
    *packets_sent = session->rtp_session.stats.packets_sent;
  }

  if (last_activity) {
    *last_activity = session->last_activity;
  }

  return 0;
}

/**
 * Set session state
 */
int rtsp_session_set_state(rtsp_session_t* session, enum rtsp_session_state state) {
  if (!session) {
    return -1;
  }

  session->state = state;
  rtsp_update_session_activity(session);

  return 0;
}

/**
 * Get session state
 */
enum rtsp_session_state rtsp_session_get_state(rtsp_session_t* session) {
  if (!session) {
    return RTSP_STATE_INVALID;
  }

  return session->state;
}

/**
 * Check if session is active
 */
bool rtsp_session_is_active(rtsp_session_t* session) {
  if (!rtsp_session_pointer_is_valid(session)) {
    return false;
  }

  return session->active && !rtsp_session_has_timed_out(session);
}

/**
 * Deactivate session
 */
void rtsp_session_deactivate(rtsp_session_t* session) {
  if (!session) {
    return;
  }

  session->active = false;
  rtsp_cleanup_session(session);
}

/**
 * Get session age in seconds
 */
time_t rtsp_session_get_age(rtsp_session_t* session) {
  if (!session) {
    return 0;
  }

  return time(NULL) - session->created_time;
}

/**
 * Get session idle time in seconds
 */
time_t rtsp_session_get_idle_time(rtsp_session_t* session) {
  if (!session) {
    return 0;
  }

  return time(NULL) - session->last_activity;
}

/**
 * Validate session
 */
bool rtsp_session_is_valid(rtsp_session_t* session) {
  if (!rtsp_session_pointer_is_valid(session)) {
    return false;
  }

  return session->sockfd >= 0 && session->active && !rtsp_session_has_timed_out(session);
}

/**
 * Get session info string
 */
int rtsp_session_get_info(rtsp_session_t* session, char* buffer, size_t buffer_size) {
  if (!session || !buffer || buffer_size < 64) {
    return -1;
  }

  char client_ip[INET_ADDRSTRLEN];
  inet_ntop(AF_INET, &session->addr.sin_addr, client_ip, INET_ADDRSTRLEN);

  const char* state_str = "UNKNOWN";
  switch (session->state) {
  case RTSP_STATE_INIT:
    state_str = "INIT";
    break;
  case RTSP_STATE_READY:
    state_str = "READY";
    break;
  case RTSP_STATE_PLAYING:
    state_str = "PLAYING";
    break;
  case RTSP_STATE_RECORDING:
    state_str = "RECORDING";
    break;
  default:
    state_str = "UNKNOWN";
    break;
  }

  return snprintf(buffer, buffer_size,
                  "Session %s: %s:%d, State: %s, Age: %lds, Idle: %lds, "
                  "Packets: %u, Bytes: %u",
                  session->session_id, client_ip, ntohs(session->addr.sin_port), state_str,
                  rtsp_session_get_age(session), rtsp_session_get_idle_time(session),
                  session->rtp_session.stats.packets_sent, session->rtp_session.stats.octets_sent);
}
