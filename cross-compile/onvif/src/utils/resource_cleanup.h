/**
 * @file resource_cleanup.h
 * @brief RAII-style resource cleanup utilities.
 */

#ifndef ONVIF_RESOURCE_CLEANUP_H
#define ONVIF_RESOURCE_CLEANUP_H

#include <stdlib.h>
#include <unistd.h>
#include <pthread.h>

/* RAII-style cleanup structures */
typedef struct {
    void **ptrs;
    int count;
    int capacity;
} cleanup_list_t;

typedef struct {
    int *fds;
    int count;
    int capacity;
} cleanup_fd_list_t;

/* Cleanup list management */
int cleanup_list_init(cleanup_list_t *list);
int cleanup_list_add_ptr(cleanup_list_t *list, void *ptr);
void cleanup_list_cleanup(cleanup_list_t *list);

int cleanup_fd_list_init(cleanup_fd_list_t *list);
int cleanup_fd_list_add(cleanup_fd_list_t *list, int fd);
void cleanup_fd_list_cleanup(cleanup_fd_list_t *list);

/* Thread-safe cleanup */
typedef struct {
    pthread_mutex_t mutex;
    cleanup_list_t ptrs;
    cleanup_fd_list_t fds;
} thread_cleanup_t;

int thread_cleanup_init(thread_cleanup_t *cleanup);
int thread_cleanup_add_ptr(thread_cleanup_t *cleanup, void *ptr);
int thread_cleanup_add_fd(thread_cleanup_t *cleanup, int fd);
void thread_cleanup_cleanup(thread_cleanup_t *cleanup);

#endif /* ONVIF_RESOURCE_CLEANUP_H */
