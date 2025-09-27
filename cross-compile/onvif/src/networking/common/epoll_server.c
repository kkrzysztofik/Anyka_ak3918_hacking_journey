/**
 * @file epoll_server.c
 * @brief Epoll-based async I/O server implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "epoll_server.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdint.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/lifecycle/signal_lifecycle.h"
#include "networking/common/buffer_pool.h"
#include "networking/common/connection_manager.h"
#include "networking/common/thread_pool.h"
#include "networking/http/http_server.h"
#include "platform/platform.h"

/* Global epoll state */
static int g_epoll_fd = -1;      // NOLINT
static int g_server_socket = -1; // NOLINT
static int g_running = 0;        // NOLINT

/* Global configuration */
static int g_epoll_timeout = 500;     // NOLINT - Default 500ms
static int g_cleanup_interval = 5000; // NOLINT - Default 5 seconds

/* Forward declarations for helper functions */
static int handle_new_connection(void);
static void handle_client_event(struct epoll_event* event);
static void perform_periodic_cleanup(uint64_t* last_cleanup);

/**
 * @brief Set epoll server configuration
 * @param config Application configuration (can be NULL for defaults)
 */
void epoll_server_set_config(const struct application_config* config) {
  if (!config || !config->server) {
    platform_log_debug("Using default epoll server configuration\n");
    return;
  }

  // Validate and set epoll timeout
  if (config->server->epoll_timeout >= 100 && config->server->epoll_timeout <= 5000) {
    g_epoll_timeout = config->server->epoll_timeout;
    platform_log_debug("Epoll timeout set to %d ms\n", g_epoll_timeout);
  } else {
    platform_log_warning("Invalid epoll timeout %d, using default 500 ms\n",
                         config->server->epoll_timeout);
  }

  // Validate and set cleanup interval
  if (config->server->cleanup_interval >= 1 && config->server->cleanup_interval <= 60) {
    g_cleanup_interval = config->server->cleanup_interval * 1000; // Convert to ms
    platform_log_debug("Cleanup interval set to %d seconds\n", config->server->cleanup_interval);
  } else {
    platform_log_warning("Invalid cleanup interval %d, using default 5 seconds\n",
                         config->server->cleanup_interval);
  }
}

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
  struct epoll_event event;
  event.events = EPOLLIN;
  event.data.ptr = NULL; // NULL indicates server socket
  if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, server_socket, &event) < 0) {
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
  platform_log_info("Stopping epoll server...\n");
  g_running = 0;

  if (g_epoll_fd >= 0) {
    close(g_epoll_fd);
    g_epoll_fd = -1;
  }

  // Give the epoll loop a moment to detect the shutdown signal
  platform_sleep_ms(100); // 100ms

  platform_log_info("Epoll server cleaned up\n");
}

/**
 * @brief Add connection to epoll
 * @param socket_fd Socket file descriptor
 * @param data Connection data pointer
 * @return 0 on success, -1 on error
 */
int epoll_server_add_connection(int socket_fd, void* data) {
  if (g_epoll_fd < 0) {
    return -1;
  }

  struct epoll_event event;
  event.events = EPOLLIN | EPOLLET;
  event.data.ptr = data;

  if (epoll_ctl(g_epoll_fd, EPOLL_CTL_ADD, socket_fd, &event) < 0) {
    platform_log_error("Failed to add connection %d to epoll: %s\n", socket_fd, strerror(errno));
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
    platform_log_error("Failed to remove connection %d from epoll: %s\n", socket_fd,
                       strerror(errno));
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
  struct sockaddr_in client_addr;
  socklen_t client_len = sizeof(client_addr);

  int client = accept(g_server_socket, (struct sockaddr*)&client_addr, &client_len);
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
  char* buffer = buffer_pool_get(&g_http_server.buffer_pool);
  if (!buffer) {
    platform_log_error("No buffers available, closing connection %d\n", client);
    close(client);
    return -1;
  }

  // Create connection
  connection_t* conn = connection_create(client, buffer);
  if (!conn) {
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    close(client);
    return -1;
  }

  // Store client IP address
  if (inet_ntop(AF_INET, &client_addr.sin_addr, conn->client_ip, INET_ADDRSTRLEN) == NULL) {
    platform_log_warning("Failed to convert client IP address\n");
    strncpy(conn->client_ip, "unknown", INET_ADDRSTRLEN - 1);
    conn->client_ip[INET_ADDRSTRLEN - 1] = '\0';
  }

  // Add to epoll
  if (epoll_server_add_connection(client, conn) != 0) {
    connection_destroy(conn);
    buffer_pool_return(&g_http_server.buffer_pool, buffer);
    return -1;
  }

  // Add to connection list
  connection_add_to_list(conn);

  // Update connection statistics
  g_http_server.connection_count++;

  platform_log_info("New connection %d accepted (total: %llu)\n", client,
                    (unsigned long long)g_http_server.connection_count);
  platform_log_debug("Connection %d: buffer allocated, added to epoll\n", client);
  return 0;
}

/**
 * @brief Handle client socket event
 * @param event Epoll event data
 */
static void handle_client_event(struct epoll_event* event) {
  connection_t* conn = (connection_t*)event->data.ptr;

  if (event->events & EPOLLIN) {
    // Remove from epoll immediately to prevent double processing
    epoll_server_remove_connection(conn->fd);
    thread_pool_add_work(&g_http_server.thread_pool, conn);
  } else if (event->events & (EPOLLHUP | EPOLLERR)) {
    platform_log_info("Connection %d closed or error (events: 0x%x)\n", conn->fd, event->events);
    platform_log_debug("Connection %d: keepalive count was %d\n", conn->fd, conn->keepalive_count);
    epoll_server_remove_connection(conn->fd);
    connection_remove_from_list(conn);
    buffer_pool_return(&g_http_server.buffer_pool, conn->buffer);
    g_http_server.connection_count--; // Decrement connection count
    connection_destroy(conn);
  }
}

/**
 * @brief Perform periodic cleanup of timed out connections
 * @param last_cleanup Pointer to last cleanup timestamp
 */
static void perform_periodic_cleanup(uint64_t* last_cleanup) {
  uint64_t now = platform_get_time_ms();
  if (now - *last_cleanup > g_cleanup_interval) { // Use configured interval
    connection_cleanup_timed_out();

    // Log periodic statistics
    platform_log_info("HTTP Server Stats: %llu connections, %llu requests processed\n",
                      (unsigned long long)g_http_server.connection_count,
                      (unsigned long long)g_http_server.request_count);

    *last_cleanup = now;
  }
}

/**
 * @brief Main epoll event loop
 * @param arg Thread argument (unused)
 * @return NULL
 */
void* epoll_server_loop(void* arg) {
  (void)arg;
  struct epoll_event events[EPOLL_MAX_EVENTS];
  uint64_t last_cleanup = platform_get_time_ms();

  platform_log_info("Epoll event loop started\n");

  while (g_running && signal_lifecycle_should_continue()) {
    int nfds = epoll_wait(g_epoll_fd, events, EPOLL_MAX_EVENTS,
                          g_epoll_timeout); // Use configured timeout

    if (nfds < 0) {
      if (errno == EINTR) {
        platform_log_debug("Epoll wait interrupted by signal\n");
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

    // Check for shutdown signal after each iteration
    if (!signal_lifecycle_should_continue()) {
      platform_log_info("Shutdown signal received, exiting epoll loop\n");
      break;
    }
  }

  platform_log_info("Epoll event loop stopped\n");
  return NULL;
}
