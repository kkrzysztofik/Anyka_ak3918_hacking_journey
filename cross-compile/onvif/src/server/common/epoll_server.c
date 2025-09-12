/**
 * @file epoll_server.c
 * @brief Epoll-based async I/O server implementation.
 */

#include "epoll_server.h"
#include "server/common/connection_manager.h"
#include "server/common/thread_pool.h"
#include "server/common/buffer_pool.h"
#include "platform/platform.h"
#include <sys/epoll.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <pthread.h>
#include <string.h>
#include <sys/socket.h>

/* Global epoll state */
static int g_epoll_fd = -1;
static int g_server_socket = -1;
static int g_running = 0;

/* Forward declarations */
extern thread_pool_t g_thread_pool;
extern buffer_pool_t g_buffer_pool;

/**
 * @brief Initialize epoll server
 * @param server_socket Server socket file descriptor
 * @return 0 on success, -1 on error
 */
int epoll_server_init(int server_socket) {
    g_server_socket = server_socket;
    
    // Create epoll instance
    g_epoll_fd = epoll_create1(EPOLL_CLOEXEC);
    if (g_epoll_fd < 0) {
        platform_log_error("Failed to create epoll instance: %s\n", strerror(errno));
        return -1;
    }
    
    // Add server socket to epoll
    struct epoll_event ev;
    ev.events = EPOLLIN;
    ev.data.ptr = NULL; // NULL indicates server socket
    if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, server_socket, &ev) < 0) {
        platform_log_error("Failed to add server socket to epoll: %s\n", strerror(errno));
        close(g_epoll_fd);
        g_epoll_fd = -1;
        return -1;
    }
    
    g_running = 1;
    platform_log_info("Epoll server initialized\n");
    return 0;
}

/**
 * @brief Cleanup epoll server
 */
void epoll_server_cleanup(void) {
    g_running = 0;
    
    if (g_epoll_fd >= 0) {
        close(g_epoll_fd);
        g_epoll_fd = -1;
    }
    
    platform_log_info("Epoll server cleaned up\n");
}

/**
 * @brief Add connection to epoll
 * @param fd Socket file descriptor
 * @param data Connection data pointer
 * @return 0 on success, -1 on error
 */
int epoll_server_add_connection(int fd, void *data) {
    if (g_epoll_fd < 0) return -1;
    
    struct epoll_event ev;
    ev.events = EPOLLIN | EPOLLET;
    ev.data.ptr = data;
    
    if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, fd, &ev) < 0) {
        platform_log_error("Failed to add connection %d to epoll: %s\n", fd, strerror(errno));
        return -1;
    }
    
    platform_log_debug("Connection %d added to epoll\n", fd);
    return 0;
}

/**
 * @brief Remove connection from epoll
 * @param fd Socket file descriptor
 * @return 0 on success, -1 on error
 */
int epoll_server_remove_connection(int fd) {
    if (g_epoll_fd < 0) return -1;
    
    if (epoll_ctl(g_epoll_fd, EPOLL_CTL_DEL, fd, NULL) < 0) {
        platform_log_error("Failed to remove connection %d from epoll: %s\n", fd, strerror(errno));
        return -1;
    }
    
    platform_log_debug("Connection %d removed from epoll\n", fd);
    return 0;
}

/**
 * @brief Main epoll event loop
 * @param arg Thread argument (unused)
 * @return NULL
 */
void *epoll_server_loop(void *arg) {
    (void)arg;
    struct epoll_event events[EPOLL_MAX_EVENTS];
    uint64_t last_cleanup = get_time_ms();
    
    platform_log_info("Epoll event loop started\n");
    
    while (g_running) {
        int nfds = epoll_wait(g_epoll_fd, events, EPOLL_MAX_EVENTS, 1000); // 1 second timeout
        
        if (nfds < 0) {
            if (errno == EINTR) continue;
            platform_log_error("Epoll wait failed: %s\n", strerror(errno));
            break;
        }
        
        // Process events
        for (int i = 0; i < nfds; i++) {
            if (events[i].data.ptr == NULL) {
                // New connection on server socket
                int client = accept(g_server_socket, NULL, NULL);
                if (client < 0) {
                    if (errno != EAGAIN && errno != EWOULDBLOCK) {
                        platform_log_error("Accept failed: %s\n", strerror(errno));
                    }
                    continue;
                }
                
                // Set socket to non-blocking
                int flags = fcntl(client, F_GETFL, 0);
                fcntl(client, F_SETFL, flags | O_NONBLOCK);
                
                // Get buffer from pool
                char *buffer = buffer_pool_get(&g_buffer_pool);
                if (!buffer) {
                    platform_log_error("No buffers available, closing connection %d\n", client);
                    close(client);
                    continue;
                }
                
                // Create connection
                connection_t *conn = connection_create(client, buffer);
                if (!conn) {
                    buffer_pool_return(&g_buffer_pool, buffer);
                    close(client);
                    continue;
                }
                
                // Add to epoll
                if (epoll_server_add_connection(client, conn) != 0) {
                    connection_destroy(conn);
                    buffer_pool_return(&g_buffer_pool, buffer);
                    continue;
                }
                
                // Add to connection list
                connection_add_to_list(conn);
                
                platform_log_debug("New connection %d accepted\n", client);
            } else {
                // Data available on client socket
                connection_t *conn = (connection_t *)events[i].data.ptr;
                if (events[i].events & EPOLLIN) {
                    thread_pool_add_work(&g_thread_pool, conn);
                } else if (events[i].events & (EPOLLHUP | EPOLLERR)) {
                    platform_log_debug("Connection %d closed or error\n", conn->fd);
                    epoll_server_remove_connection(conn->fd);
                    connection_remove_from_list(conn);
                    buffer_pool_return(&g_buffer_pool, conn->buffer);
                    connection_destroy(conn);
                }
            }
        }
        
        // Periodic cleanup of timed out connections
        uint64_t now = get_time_ms();
        if (now - last_cleanup > 5000) { // Every 5 seconds
            connection_cleanup_timed_out();
            last_cleanup = now;
        }
    }
    
    platform_log_info("Epoll event loop stopped\n");
    return NULL;
}
