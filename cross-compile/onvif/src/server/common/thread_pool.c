/**
 * @file thread_pool.c
 * @brief Thread pool management implementation.
 */

#include "thread_pool.h"
#include "connection_manager.h"
#include "platform.h"
#include <stdlib.h>
#include <pthread.h>

/* Forward declaration for connection processing */
extern void process_connection(connection_t *conn);

/**
 * @brief Initialize thread pool
 * @param pool Thread pool to initialize
 * @return 0 on success, -1 on error
 */
int thread_pool_init(thread_pool_t *pool) {
    if (!pool) return -1;
    
    pool->queue_mutex = malloc(sizeof(pthread_mutex_t));
    pool->queue_cond = malloc(sizeof(pthread_cond_t));
    
    if (!pool->queue_mutex || !pool->queue_cond) {
        platform_log_error("Failed to allocate thread pool resources\n");
        if (pool->queue_mutex) free(pool->queue_mutex);
        if (pool->queue_cond) free(pool->queue_cond);
        return -1;
    }
    
    if (pthread_mutex_init((pthread_mutex_t*)pool->queue_mutex, NULL) != 0) {
        platform_log_error("Failed to initialize thread pool mutex\n");
        free(pool->queue_mutex);
        free(pool->queue_cond);
        return -1;
    }
    
    if (pthread_cond_init((pthread_cond_t*)pool->queue_cond, NULL) != 0) {
        platform_log_error("Failed to initialize thread pool condition\n");
        pthread_mutex_destroy((pthread_mutex_t*)pool->queue_mutex);
        free(pool->queue_mutex);
        free(pool->queue_cond);
        return -1;
    }
    
    pool->work_queue = NULL;
    pool->shutdown = 0;
    pool->active_threads = 0;
    
    // Create worker threads
    for (int i = 0; i < THREAD_POOL_SIZE; i++) {
        if (pthread_create((pthread_t*)&pool->threads[i], NULL, thread_pool_worker, pool) != 0) {
            platform_log_error("Failed to create worker thread %d\n", i);
            // Clean up already created threads
            pool->shutdown = 1;
            pthread_cond_broadcast((pthread_cond_t*)pool->queue_cond);
            for (int j = 0; j < i; j++) {
                pthread_join((pthread_t)pool->threads[j], NULL);
            }
            pthread_cond_destroy((pthread_cond_t*)pool->queue_cond);
            pthread_mutex_destroy((pthread_mutex_t*)pool->queue_mutex);
            free(pool->queue_cond);
            free(pool->queue_mutex);
            return -1;
        }
    }
    
    platform_log_info("Thread pool initialized with %d worker threads\n", THREAD_POOL_SIZE);
    return 0;
}

/**
 * @brief Cleanup thread pool
 * @param pool Thread pool to cleanup
 */
void thread_pool_cleanup(thread_pool_t *pool) {
    if (!pool) return;
    
    platform_log_info("Shutting down thread pool...\n");
    
    pthread_mutex_lock((pthread_mutex_t*)pool->queue_mutex);
    pool->shutdown = 1;
    pthread_cond_broadcast((pthread_cond_t*)pool->queue_cond);
    pthread_mutex_unlock((pthread_mutex_t*)pool->queue_mutex);
    
    // Wait for all threads to finish
    for (int i = 0; i < THREAD_POOL_SIZE; i++) {
        pthread_join((pthread_t)pool->threads[i], NULL);
    }
    
    pthread_cond_destroy((pthread_cond_t*)pool->queue_cond);
    pthread_mutex_destroy((pthread_mutex_t*)pool->queue_mutex);
    free(pool->queue_cond);
    free(pool->queue_mutex);
    
    platform_log_info("Thread pool cleaned up\n");
}

/**
 * @brief Add connection to work queue
 * @param pool Thread pool
 * @param conn Connection to add
 */
void thread_pool_add_work(thread_pool_t *pool, connection_t *conn) {
    if (!pool || !conn) return;
    
    pthread_mutex_lock((pthread_mutex_t*)pool->queue_mutex);
    
    conn->next = pool->work_queue;
    conn->prev = NULL;
    if (pool->work_queue) {
        pool->work_queue->prev = conn;
    }
    pool->work_queue = conn;
    
    pthread_cond_signal((pthread_cond_t*)pool->queue_cond);
    pthread_mutex_unlock((pthread_mutex_t*)pool->queue_mutex);
}

/**
 * @brief Worker thread function
 * @param arg Thread pool pointer
 * @return NULL
 */
void *thread_pool_worker(void *arg) {
    thread_pool_t *pool = (thread_pool_t *)arg;
    
    platform_log_debug("Worker thread started\n");
    
    while (1) {
        connection_t *conn = NULL;
        
        pthread_mutex_lock((pthread_mutex_t*)pool->queue_mutex);
        
        // Wait for work or shutdown
        while (pool->work_queue == NULL && !pool->shutdown) {
            pthread_cond_wait((pthread_cond_t*)pool->queue_cond, (pthread_mutex_t*)pool->queue_mutex);
        }
        
        if (pool->shutdown) {
            pthread_mutex_unlock((pthread_mutex_t*)pool->queue_mutex);
            break;
        }
        
        // Get work from queue
        conn = pool->work_queue;
        if (conn) {
            pool->work_queue = conn->next;
            if (pool->work_queue) {
                pool->work_queue->prev = NULL;
            }
        }
        
        pool->active_threads++;
        pthread_mutex_unlock((pthread_mutex_t*)pool->queue_mutex);
        
        if (conn) {
            // Process the connection
            process_connection(conn);
        }
        
        pthread_mutex_lock((pthread_mutex_t*)pool->queue_mutex);
        pool->active_threads--;
        pthread_mutex_unlock((pthread_mutex_t*)pool->queue_mutex);
    }
    
    platform_log_debug("Worker thread stopped\n");
    return NULL;
}
