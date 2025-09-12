/**
 * @file buffer_pool.c
 * @brief Buffer pool management implementation.
 */

#include "buffer_pool.h"
#include "platform.h"
#include <stdlib.h>
#include <pthread.h>

/**
 * @brief Initialize buffer pool
 * @param pool Buffer pool to initialize
 * @return 0 on success, -1 on error
 */
int buffer_pool_init(buffer_pool_t *pool) {
    if (!pool) return -1;
    
    pool->mutex = malloc(sizeof(pthread_mutex_t));
    if (!pool->mutex) {
        platform_log_error("Failed to allocate mutex for buffer pool\n");
        return -1;
    }
    
    if (pthread_mutex_init((pthread_mutex_t*)pool->mutex, NULL) != 0) {
        platform_log_error("Failed to initialize buffer pool mutex\n");
        free(pool->mutex);
        return -1;
    }
    
    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        pool->buffers[i] = malloc(BUFFER_SIZE);
        if (!pool->buffers[i]) {
            platform_log_error("Failed to allocate buffer %d\n", i);
            // Clean up already allocated buffers
            for (int j = 0; j < i; j++) {
                free(pool->buffers[j]);
            }
            pthread_mutex_destroy((pthread_mutex_t*)pool->mutex);
            free(pool->mutex);
            return -1;
        }
        pool->available[i] = 1;
    }
    
    pool->count = BUFFER_POOL_SIZE;
    platform_log_info("Buffer pool initialized with %d buffers\n", BUFFER_POOL_SIZE);
    return 0;
}

/**
 * @brief Cleanup buffer pool
 * @param pool Buffer pool to cleanup
 */
void buffer_pool_cleanup(buffer_pool_t *pool) {
    if (!pool) return;
    
    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        if (pool->buffers[i]) {
            free(pool->buffers[i]);
            pool->buffers[i] = NULL;
        }
    }
    
    if (pool->mutex) {
        pthread_mutex_destroy((pthread_mutex_t*)pool->mutex);
        free(pool->mutex);
        pool->mutex = NULL;
    }
    
    platform_log_info("Buffer pool cleaned up\n");
}

/**
 * @brief Get buffer from pool
 * @param pool Buffer pool
 * @return Buffer pointer or NULL if none available
 */
char *buffer_pool_get(buffer_pool_t *pool) {
    if (!pool || !pool->mutex) return NULL;
    
    char *buffer = NULL;
    
    pthread_mutex_lock((pthread_mutex_t*)pool->mutex);
    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        if (pool->available[i]) {
            pool->available[i] = 0;
            buffer = pool->buffers[i];
            break;
        }
    }
    pthread_mutex_unlock((pthread_mutex_t*)pool->mutex);
    
    return buffer;
}

/**
 * @brief Return buffer to pool
 * @param pool Buffer pool
 * @param buffer Buffer to return
 */
void buffer_pool_return(buffer_pool_t *pool, char *buffer) {
    if (!pool || !pool->mutex || !buffer) return;
    
    pthread_mutex_lock((pthread_mutex_t*)pool->mutex);
    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        if (pool->buffers[i] == buffer) {
            pool->available[i] = 1;
            break;
        }
    }
    pthread_mutex_unlock((pthread_mutex_t*)pool->mutex);
}
