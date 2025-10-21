/**
 * @file connection_manager.h
 * @brief Connection management for HTTP server
 * @author kkrzysztofik
 * @date 2025
 *
 * This module handles connection lifecycle, state management,
 * and timeout handling.
 */

#ifndef CONNECTION_MANAGER_H
#define CONNECTION_MANAGER_H

#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>

/* HTTP field buffer sizes */
#define CONN_MGR_HTTP_METHOD_SIZE  16  /* HTTP method buffer size (GET, POST, etc.) */
#define CONN_MGR_HTTP_PATH_SIZE    256 /* HTTP path buffer size */
#define CONN_MGR_HTTP_VERSION_SIZE 16  /* HTTP version buffer size (HTTP/1.1) */

/* Connection states */
typedef enum {
  CONN_STATE_READING_HEADERS = 0,
  CONN_STATE_READING_BODY,
  CONN_STATE_PROCESSING,
  CONN_STATE_WRITING,
  CONN_STATE_KEEPALIVE,
  CONN_STATE_CLOSING
} connection_state_t;

/* Connection structure */
typedef struct connection {
  int fd;
  connection_state_t state;
  char* buffer;
  size_t buffer_size;
  size_t buffer_used;
  size_t content_length;
  size_t header_length;
  uint64_t last_activity;
  int keepalive_count;
  char method[CONN_MGR_HTTP_METHOD_SIZE];
  char path[CONN_MGR_HTTP_PATH_SIZE];
  char version[CONN_MGR_HTTP_VERSION_SIZE];
  char client_ip[INET_ADDRSTRLEN]; /* Client IP address */
  char* request_buffer;            /* Persistent 32KB buffer for HTTP request processing */
  struct connection* next;
  struct connection* prev;
} connection_t;

/* Connection manager functions */
int connection_manager_init(void);
void connection_manager_cleanup(void);
connection_t* connection_create(int socket_fd, char* buffer);
void connection_destroy(connection_t* conn);
int connection_is_timed_out(connection_t* conn);
void connection_cleanup_timed_out(void);
void connection_add_to_list(connection_t* conn);
void connection_remove_from_list(connection_t* conn);

/* Connection list accessor functions */
connection_t* connection_get_list_head(void);
void connection_lock_mutex(void);
void connection_unlock_mutex(void);

#endif /* CONNECTION_MANAGER_H */
