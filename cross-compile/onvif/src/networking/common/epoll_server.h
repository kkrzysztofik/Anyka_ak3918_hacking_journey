/**
 * @file epoll_server.h
 * @brief Epoll-based async I/O server module.
 * 
 * This module provides async I/O handling using epoll for
 * high-performance concurrent connection management.
 */

#ifndef EPOLL_SERVER_H
#define EPOLL_SERVER_H

#include <stdint.h>

/* Epoll configuration */
#define EPOLL_MAX_EVENTS 100

/* Epoll server functions */
int epoll_server_init(int server_socket);
void epoll_server_cleanup(void);
void *epoll_server_loop(void *arg);
int epoll_server_add_connection(int fd, void *data);
int epoll_server_remove_connection(int fd);

#endif /* EPOLL_SERVER_H */
