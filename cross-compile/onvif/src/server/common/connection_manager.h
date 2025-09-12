/**
 * @file connection_manager.h
 * @brief Connection management for HTTP server.
 * 
 * This module handles connection lifecycle, state management,
 * and timeout handling.
 */

#ifndef CONNECTION_MANAGER_H
#define CONNECTION_MANAGER_H

#include <stdint.h>
#include <stddef.h>
#include <pthread.h>

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
    char *buffer;
    size_t buffer_size;
    size_t buffer_used;
    size_t content_length;
    size_t header_length;
    uint64_t last_activity;
    int keepalive_count;
    char method[16];
    char path[256];
    char version[16];
    struct connection *next;
    struct connection *prev;
} connection_t;

/* Connection manager functions */
int connection_manager_init(void);
void connection_manager_cleanup(void);
connection_t *connection_create(int fd, char *buffer);
void connection_destroy(connection_t *conn);
int connection_is_timed_out(connection_t *conn);
void connection_cleanup_timed_out(void);
void connection_add_to_list(connection_t *conn);
void connection_remove_from_list(connection_t *conn);
uint64_t get_time_ms(void);

/* Global connection list management */
extern connection_t *g_connections;
extern pthread_mutex_t g_connections_mutex;

#endif /* CONNECTION_MANAGER_H */
