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
 * @brief HTTP performance metrics structure
 *
 * Thread-safe metrics collection for HTTP server performance monitoring
 */
typedef struct http_performance_metrics {
  uint64_t total_requests;       /**< Total requests processed */
  uint64_t successful_requests;  /**< Successful requests (2xx) */
  uint64_t client_errors;        /**< Client errors (4xx) */
  uint64_t server_errors;        /**< Server errors (5xx) */
  uint64_t total_response_bytes; /**< Total response bytes sent */
  uint64_t total_latency_ms;     /**< Total latency in milliseconds */
  uint64_t min_latency_ms;       /**< Minimum request latency */
  uint64_t max_latency_ms;       /**< Maximum request latency */
  uint64_t current_connections;  /**< Current active connections */
  uint64_t metrics_start_time;   /**< Metrics collection start time */
  pthread_mutex_t metrics_mutex; /**< Mutex for thread-safe access */
} http_performance_metrics_t;

/**
 * @brief HTTP server state structure
 *
 * Maintains server runtime state including socket, threads, and statistics
 */
typedef struct server_state {
  int running;                        /**< Server running flag */
  int socket;                         /**< Server socket file descriptor */
  pthread_t epoll_thread;             /**< Epoll event processing thread */
  thread_pool_t thread_pool;          /**< Thread pool for request processing */
  buffer_pool_t buffer_pool;          /**< Buffer pool for memory management */
  uint64_t connection_count;          /**< Total connections handled */
  uint64_t request_count;             /**< Total requests processed */
  http_performance_metrics_t metrics; /**< Performance metrics */
} server_state_t;

/* HTTP server state for shared access */
extern server_state_t g_http_server; // NOLINT

/**
 * @brief Initialize HTTP performance metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_init(void);

/**
 * @brief Cleanup HTTP performance metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_cleanup(void);

/**
 * @brief Get current HTTP performance metrics
 * @param metrics Output structure to fill with current metrics
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_get_current(http_performance_metrics_t* metrics);

/**
 * @brief Record HTTP request metrics
 * @param latency_ms Request latency in milliseconds
 * @param response_size Response size in bytes
 * @param status_code HTTP status code
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_record_request(uint64_t latency_ms, size_t response_size, int status_code);

/**
 * @brief Update current connection count
 * @param delta Change in connection count (+1 for new, -1 for closed)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int http_metrics_update_connections(int delta);

#endif
