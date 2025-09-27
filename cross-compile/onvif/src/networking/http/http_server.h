/**
 * @file http_server.h
 * @brief Minimal blocking HTTP server exposing ONVIF SOAP endpoints
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef HTTP_SERVER_H
#define HTTP_SERVER_H

#include "networking/common/buffer_pool.h"
#include "networking/common/thread_pool.h"

/* Forward declaration */
struct application_config;

/**
 * @brief Start the ONVIF HTTP/SOAP server.
 * @param port TCP port to bind.
 * @param config Application configuration (can be NULL for defaults).
 * @return 0 on success, negative on failure.
 */
int http_server_start(int port, const struct application_config* config);

/**
 * @brief Stop the HTTP server and release resources.
 * @return 0 on success, negative on failure.
 */
int http_server_stop(void);

/**
 * @brief Process a single connection (used by thread pool)
 * @param conn Connection to process
 */
void process_connection(void* conn);

/**
 * @brief HTTP server state structure
 *
 * Maintains server runtime state including socket, threads, and statistics
 */
typedef struct server_state {
  int running;               /**< Server running flag */
  int socket;                /**< Server socket file descriptor */
  pthread_t epoll_thread;    /**< Epoll event processing thread */
  thread_pool_t thread_pool; /**< Thread pool for request processing */
  buffer_pool_t buffer_pool; /**< Buffer pool for memory management */
  uint64_t connection_count; /**< Total connections handled */
  uint64_t request_count;    /**< Total requests processed */
} server_state_t;

/* HTTP server state for shared access */
extern server_state_t g_http_server; // NOLINT

#endif
