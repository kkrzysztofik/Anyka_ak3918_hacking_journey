/**
 * @file epoll_server.h
 * @brief Epoll-based async I/O server module
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides async I/O handling using epoll for
 * high-performance concurrent connection management.
 */

#ifndef EPOLL_SERVER_H
#define EPOLL_SERVER_H

/* Epoll configuration */
#define EPOLL_MAX_EVENTS 100

/* Timeout configuration limits */
#define EPOLL_TIMEOUT_MIN_MS      100  /* Minimum epoll timeout in milliseconds */
#define EPOLL_TIMEOUT_MAX_MS      5000 /* Maximum epoll timeout in milliseconds */
#define CLEANUP_INTERVAL_MAX_SEC  60   /* Maximum cleanup interval in seconds */
#define MS_PER_SECOND             1000 /* Milliseconds per second conversion */
#define EPOLL_SHUTDOWN_DELAY_MS   100  /* Delay for epoll loop shutdown detection */

/* Forward declaration */
struct application_config;

/* Epoll server functions */
int epoll_server_init(int server_socket);
void epoll_server_cleanup(void);
void epoll_server_set_config(const struct application_config* config);
void* epoll_server_loop(void* arg);
int epoll_server_add_connection(int fd, void* data);    // NOLINT(readability-identifier-length) - standard POSIX file descriptor
int epoll_server_remove_connection(int fd);             // NOLINT(readability-identifier-length) - standard POSIX file descriptor

#endif /* EPOLL_SERVER_H */
