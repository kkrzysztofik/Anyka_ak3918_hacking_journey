/**
 * @file thread_pool.h
 * @brief Thread pool management for concurrent request processing.
 * 
 * This module provides a pool of worker threads for processing
 * HTTP requests concurrently.
 */

#ifndef THREAD_POOL_H
#define THREAD_POOL_H

#include <stdint.h>
#include <pthread.h>

/* Thread pool configuration */
#define THREAD_POOL_SIZE 8

/* Forward declarations */
struct connection;
typedef struct connection connection_t;
typedef struct thread_pool thread_pool_t;

/* Thread pool structure */
struct thread_pool {
    pthread_t threads[THREAD_POOL_SIZE];
    pthread_mutex_t queue_mutex;
    pthread_cond_t queue_cond;
    connection_t *work_queue;
    int shutdown;
    int active_threads;
};

/* Thread pool functions */
int thread_pool_init(thread_pool_t *pool);
void thread_pool_cleanup(thread_pool_t *pool);
void thread_pool_add_work(thread_pool_t *pool, connection_t *conn);
void *thread_pool_worker(void *arg);

#endif /* THREAD_POOL_H */
