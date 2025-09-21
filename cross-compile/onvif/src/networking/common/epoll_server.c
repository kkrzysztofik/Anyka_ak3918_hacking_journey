/**
 * @file epoll_server.c
 * @brief Epoll-based async I/O server implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "epoll_server.h"

#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdint.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <unistd.h>

#include "networking/common/buffer_pool.h"
#include "networking/common/connection_manager.h"
#include "networking/common/thread_pool.h"
#include "networking/http/http_server.h"
#include "platform/platform.h"

/* Global epoll state */
static int g_epoll_fd = -1;       // NOLINT
static int g_server_socket = -1;  // NOLINT
static int g_running = 0;         // NOLINT

/* Forward declarations for helper functions */
static int handle_new_connection(void);
static void handle_client_event(struct epoll_event *event);
static void perform_periodic_cleanup(uint64_t *last_cleanup);

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
    platform_log_error("Failed to create epoll instance: %s\n",
                       strerror(errno));
    return -1;
  }

  // Add server socket to epoll
  struct epoll_event event;
  event.events = EPOLLIN;
  event.data.ptr = NULL;  // NULL indicates server socket
  if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, server_socket, &event) < 0) {
    platform_log_error("Failed to add server socket to epoll: %s\n",
                       strerror(errno));
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
 * @param socket_fd Socket file descriptor
 * @param data Connection data pointer
 * @return 0 on success, -1 on error
 */
int epoll_server_add_connection(int socket_fd, void *data) {
  if (g_epoll_fd < 0) {
    return -1;
  }

  struct epoll_event event;
  event.events = EPOLLIN | EPOLLET;
  event.data.ptr = data;

  if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, socket_fd, &event) < 0) {
    platform_log_error("Failed to add connection %d to epoll: %s\n", socket_fd,
                       strerror(errno));
    return -1;
  }

  platform_log_debug("Connection %d added to epoll\n", socket_fd);
  return 0;
}

/**
 * @brief Remove connection from epoll
 * @param socket_fd Socket file descriptor
 * @return 0 on success, -1 on error
 */
int epoll_server_remove_connection(int socket_fd) {
  if (g_epoll_fd < 0) {
    return -1;
  }

  if (epoll_ctl(g_epoll_fd, EPOLL_CTL_DEL, socket_fd, NULL) < 0) {
    platform_log_error("Failed to remove connection %d from epoll: %s\n",
                       socket_fd, strerror(errno));
    return -1;
  }

  platform_log_debug("Connection %d removed from epoll\n", socket_fd);
  return 0;
}

/**
 * @brief Handle new connection on server socket
 * @return 0 on success, -1 on error
 */
static int handle_new_connection(void) {
  int client = accept(g_server_socket, NULL, NULL);
  if (client < 0) {
    if (errno != EAGAIN && errno != EWOULDBLOCK) {
      platform_log_error("Accept failed: %s\n", strerror(errno));
    }
    return -1;
  }

  // Set socket to non-blocking
  int flags = fcntl(client, F_GETFL, 0);
  fcntl(client, F_SETFL, flags | O_NONBLOCK);

  // Get buffer from pool
  char *buffer = buffer_pool_get(&g_http_buffer_pool);
  if (!buffer) {
    platform_log_error("No buffers available, closing connection %d\n", client);
    close(client);
    return -1;
  }

  // Create connection
  connection_t *conn = connection_create(client, buffer);
  if (!conn) {
    buffer_pool_return(&g_http_buffer_pool, buffer);
    close(client);
    return -1;
  }

  // Add to epoll
  if (epoll_server_add_connection(client, conn) != 0) {
    connection_destroy(conn);
    buffer_pool_return(&g_http_buffer_pool, buffer);
    return -1;
  }

  // Add to connection list
  connection_add_to_list(conn);
  platform_log_debug("New connection %d accepted\n", client);
  return 0;
}

/**
 * @brief Handle client socket event
 * @param event Epoll event data
 */
static void handle_client_event(struct epoll_event *event) {
  connection_t *conn = (connection_t *)event->data.ptr;

  if (event->events & EPOLLIN) {
    thread_pool_add_work(&g_http_thread_pool, conn);
  } else if (event->events & (EPOLLHUP | EPOLLERR)) {
    platform_log_debug("Connection %d closed or error\n", conn->fd);
    epoll_server_remove_connection(conn->fd);
    connection_remove_from_list(conn);
    buffer_pool_return(&g_http_buffer_pool, conn->buffer);
    connection_destroy(conn);
  }
}

/**
 * @brief Perform periodic cleanup of timed out connections
 * @param last_cleanup Pointer to last cleanup timestamp
 */
static void perform_periodic_cleanup(uint64_t *last_cleanup) {
  uint64_t now = get_time_ms();
  if (now - *last_cleanup > 5000) {  // Every 5 seconds
    connection_cleanup_timed_out();
    *last_cleanup = now;
  }
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
    int nfds = epoll_wait(g_epoll_fd, events, EPOLL_MAX_EVENTS,
                          1000);  // 1 second timeout

    if (nfds < 0) {
      if (errno == EINTR) {
        continue;
      }
      platform_log_error("Epoll wait failed: %s\n", strerror(errno));
      break;
    }

    // Process events
    for (int i = 0; i < nfds; i++) {
      if (events[i].data.ptr == NULL) {
        // New connection on server socket
        handle_new_connection();
      } else {
        // Data available on client socket
        handle_client_event(&events[i]);
      }
    }

    // Periodic cleanup of timed out connections
    perform_periodic_cleanup(&last_cleanup);
  }

  platform_log_info("Epoll event loop stopped\n");
  return NULL;
}
