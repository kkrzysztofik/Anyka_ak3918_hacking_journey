/**
 * @file connection_manager.c
 * @brief Connection management implementation.
 */

#include "connection_manager.h"
#include "platform.h"
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/time.h>
#include <pthread.h>

/* Global connection list */
connection_t *g_connections = NULL;
pthread_mutex_t g_connections_mutex;

/* Timeout constants */
#define CONNECTION_TIMEOUT_MS 30000  // 30 seconds
#define KEEPALIVE_TIMEOUT_MS 5000   // 5 seconds

/**
 * @brief Get current time in milliseconds
 * @return Current time in milliseconds
 */
uint64_t get_time_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (uint64_t)tv.tv_sec * 1000 + (uint64_t)tv.tv_usec / 1000;
}

/**
 * @brief Initialize connection manager
 * @return 0 on success, -1 on error
 */
int connection_manager_init(void) {
    // Use static mutex instead of dynamic allocation
    if (pthread_mutex_init(&g_connections_mutex, NULL) != 0) {
        platform_log_error("Failed to initialize connections mutex\n");
        return -1;
    }
    
    g_connections = NULL;
    return 0;
}

/**
 * @brief Cleanup connection manager
 */
void connection_manager_cleanup(void) {
    pthread_mutex_lock(&g_connections_mutex);
    connection_t *conn = g_connections;
    while (conn) {
        connection_t *next = conn->next;
        connection_destroy(conn);
        conn = next;
    }
    g_connections = NULL;
    pthread_mutex_unlock(&g_connections_mutex);
    
    pthread_mutex_destroy(&g_connections_mutex);
}

/**
 * @brief Create new connection
 * @param fd Socket file descriptor
 * @param buffer Buffer for this connection
 * @return Connection pointer or NULL on error
 */
connection_t *connection_create(int fd, char *buffer) {
    connection_t *conn = malloc(sizeof(connection_t));
    if (!conn) {
        platform_log_error("Failed to allocate connection\n");
        return NULL;
    }
    
    conn->fd = fd;
    conn->state = CONN_STATE_READING_HEADERS;
    conn->buffer = buffer;
    conn->buffer_size = 32768; // BUFFER_SIZE
    conn->buffer_used = 0;
    conn->content_length = 0;
    conn->header_length = 0;
    conn->last_activity = get_time_ms();
    conn->keepalive_count = 0;
    conn->next = NULL;
    conn->prev = NULL;
    
    // Initialize method/path/version
    memset(conn->method, 0, sizeof(conn->method));
    memset(conn->path, 0, sizeof(conn->path));
    memset(conn->version, 0, sizeof(conn->version));
    
    platform_log_debug("Created connection %d\n", fd);
    return conn;
}

/**
 * @brief Destroy connection and cleanup resources
 * @param conn Connection to destroy
 */
void connection_destroy(connection_t *conn) {
    if (!conn) return;
    
    // Close socket
    if (conn->fd >= 0) {
        close(conn->fd);
    }
    
    platform_log_debug("Destroyed connection %d\n", conn->fd);
    free(conn);
}

/**
 * @brief Check if connection has timed out
 * @param conn Connection to check
 * @return 1 if timed out, 0 if still active
 */
int connection_is_timed_out(connection_t *conn) {
    if (!conn) return 1;
    
    uint64_t now = get_time_ms();
    uint64_t timeout = (conn->state == CONN_STATE_KEEPALIVE) ? 
                      KEEPALIVE_TIMEOUT_MS : CONNECTION_TIMEOUT_MS;
    
    return (now - conn->last_activity) > timeout;
}

/**
 * @brief Add connection to global list
 * @param conn Connection to add
 */
void connection_add_to_list(connection_t *conn) {
    if (!conn) return;
    
    pthread_mutex_lock(&g_connections_mutex);
    conn->next = g_connections;
    if (g_connections) {
        g_connections->prev = conn;
    }
    g_connections = conn;
    pthread_mutex_unlock(&g_connections_mutex);
}

/**
 * @brief Remove connection from global list
 * @param conn Connection to remove
 */
void connection_remove_from_list(connection_t *conn) {
    if (!conn) return;
    
    pthread_mutex_lock(&g_connections_mutex);
    if (conn->prev) {
        conn->prev->next = conn->next;
    } else {
        g_connections = conn->next;
    }
    if (conn->next) {
        conn->next->prev = conn->prev;
    }
    pthread_mutex_unlock(&g_connections_mutex);
}

/**
 * @brief Cleanup timed out connections
 */
void connection_cleanup_timed_out(void) {
    pthread_mutex_lock(&g_connections_mutex);
    
    connection_t *conn = g_connections;
    while (conn) {
        connection_t *next = conn->next;
        if (connection_is_timed_out(conn)) {
            platform_log_debug("Connection %d timed out\n", conn->fd);
            connection_remove_from_list(conn);
            connection_destroy(conn);
        }
        conn = next;
    }
    
    pthread_mutex_unlock(&g_connections_mutex);
}
