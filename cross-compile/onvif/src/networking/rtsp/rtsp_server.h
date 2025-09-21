/**
 * @file rtsp_server.h
 * @brief RTSP Server Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all RTSP server related function declarations
 * for creating, managing, and controlling RTSP server instances.
 */

#ifndef RTSP_SERVER_H
#define RTSP_SERVER_H

#include "rtsp_types.h"

/* RTSP server functions */
/**
 * @brief Allocate and initialize a server instance (not started)
 * @param config Desired stream configuration (copied internally)
 * @return Opaque server pointer or NULL on failure
 */
rtsp_server_t *rtsp_server_create(rtsp_stream_config_t *config);

/**
 * @brief Start the accept / encoding threads
 * @param server Instance created via rtsp_server_create
 * @return 0 on success, negative error code otherwise
 */
int rtsp_server_start(rtsp_server_t *server);

/**
 * @brief Stop all threads and close client sessions (server reusable after
 * start again)
 * @param server Server instance
 * @return 0 on success, negative error code
 */
int rtsp_server_stop(rtsp_server_t *server);

/**
 * @brief Destroy server and free all memory
 * @param server Server instance (may be NULL)
 * @return 0 always; provided for symmetry
 */
int rtsp_server_destroy(rtsp_server_t *server);

/**
 * @brief Get server statistics including RTCP data
 * @param server Server instance
 * @param bytes_sent Total bytes sent
 * @param frames_sent Total frames sent
 * @param sessions_count Number of active sessions
 * @return 0 on success, -1 on error
 */
int rtsp_server_get_stats(rtsp_server_t *server, uint64_t *bytes_sent,
                          uint32_t *frames_sent, uint32_t *sessions_count);

/**
 * @brief Get stream URL
 * @param server Server instance
 * @param url Buffer to store URL
 * @param url_size Size of URL buffer
 * @return 0 on success, -1 on error
 */
int rtsp_server_get_stream_url(rtsp_server_t *server, char *url,
                               size_t url_size);

#endif /* RTSP_SERVER_H */
