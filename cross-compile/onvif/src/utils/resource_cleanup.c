/**
 * @file resource_cleanup.c
 * @brief Implementation of RAII-style resource cleanup utilities.
 */

#include "resource_cleanup.h"
#include "error_handling.h"
#include "ak_common.h"

#define CLEANUP_INITIAL_CAPACITY 16

int cleanup_list_init(cleanup_list_t *list) {
    ONVIF_CHECK_NULL(list);
    
    list->ptrs = malloc(CLEANUP_INITIAL_CAPACITY * sizeof(void*));
    if (!list->ptrs) {
        return ONVIF_ERROR_MEMORY;
    }
    
    list->count = 0;
    list->capacity = CLEANUP_INITIAL_CAPACITY;
    return ONVIF_SUCCESS;
}

int cleanup_list_add_ptr(cleanup_list_t *list, void *ptr) {
    ONVIF_CHECK_NULL(list);
    
    if (list->count >= list->capacity) {
        void **new_ptrs = realloc(list->ptrs, list->capacity * 2 * sizeof(void*));
        if (!new_ptrs) {
            return ONVIF_ERROR_MEMORY;
        }
        list->ptrs = new_ptrs;
        list->capacity *= 2;
    }
    
    list->ptrs[list->count++] = ptr;
    return ONVIF_SUCCESS;
}

void cleanup_list_cleanup(cleanup_list_t *list) {
    if (!list) return;
    
    for (int i = 0; i < list->count; i++) {
        if (list->ptrs[i]) {
            free(list->ptrs[i]);
        }
    }
    
    free(list->ptrs);
    list->ptrs = NULL;
    list->count = 0;
    list->capacity = 0;
}

int cleanup_fd_list_init(cleanup_fd_list_t *list) {
    ONVIF_CHECK_NULL(list);
    
    list->fds = malloc(CLEANUP_INITIAL_CAPACITY * sizeof(int));
    if (!list->fds) {
        return ONVIF_ERROR_MEMORY;
    }
    
    list->count = 0;
    list->capacity = CLEANUP_INITIAL_CAPACITY;
    return ONVIF_SUCCESS;
}

int cleanup_fd_list_add(cleanup_fd_list_t *list, int fd) {
    ONVIF_CHECK_NULL(list);
    
    if (fd < 0) return ONVIF_ERROR_INVALID;
    
    if (list->count >= list->capacity) {
        int *new_fds = realloc(list->fds, list->capacity * 2 * sizeof(int));
        if (!new_fds) {
            return ONVIF_ERROR_MEMORY;
        }
        list->fds = new_fds;
        list->capacity *= 2;
    }
    
    list->fds[list->count++] = fd;
    return ONVIF_SUCCESS;
}

void cleanup_fd_list_cleanup(cleanup_fd_list_t *list) {
    if (!list) return;
    
    for (int i = 0; i < list->count; i++) {
        if (list->fds[i] >= 0) {
            close(list->fds[i]);
        }
    }
    
    free(list->fds);
    list->fds = NULL;
    list->count = 0;
    list->capacity = 0;
}

int thread_cleanup_init(thread_cleanup_t *cleanup) {
    ONVIF_CHECK_NULL(cleanup);
    
    if (pthread_mutex_init(&cleanup->mutex, NULL) != 0) {
        return ONVIF_ERROR;
    }
    
    int ret = cleanup_list_init(&cleanup->ptrs);
    if (ret != ONVIF_SUCCESS) {
        pthread_mutex_destroy(&cleanup->mutex);
        return ret;
    }
    
    ret = cleanup_fd_list_init(&cleanup->fds);
    if (ret != ONVIF_SUCCESS) {
        cleanup_list_cleanup(&cleanup->ptrs);
        pthread_mutex_destroy(&cleanup->mutex);
        return ret;
    }
    
    return ONVIF_SUCCESS;
}

int thread_cleanup_add_ptr(thread_cleanup_t *cleanup, void *ptr) {
    ONVIF_CHECK_NULL(cleanup);
    
    pthread_mutex_lock(&cleanup->mutex);
    int ret = cleanup_list_add_ptr(&cleanup->ptrs, ptr);
    pthread_mutex_unlock(&cleanup->mutex);
    
    return ret;
}

int thread_cleanup_add_fd(thread_cleanup_t *cleanup, int fd) {
    ONVIF_CHECK_NULL(cleanup);
    
    pthread_mutex_lock(&cleanup->mutex);
    int ret = cleanup_fd_list_add(&cleanup->fds, fd);
    pthread_mutex_unlock(&cleanup->mutex);
    
    return ret;
}

void thread_cleanup_cleanup(thread_cleanup_t *cleanup) {
    if (!cleanup) return;
    
    pthread_mutex_lock(&cleanup->mutex);
    cleanup_list_cleanup(&cleanup->ptrs);
    cleanup_fd_list_cleanup(&cleanup->fds);
    pthread_mutex_unlock(&cleanup->mutex);
    
    pthread_mutex_destroy(&cleanup->mutex);
}
