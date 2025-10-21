/**
 * @file thread_pool.c
 * @brief Thread pool management implementation
 * @author kkrzysztofik
 * @date 2025
 */

#define _GNU_SOURCE
#include "thread_pool.h"

#include <bits/pthreadtypes.h>
#include <errno.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "core/lifecycle/signal_lifecycle.h"
#include "networking/common/connection_manager.h"
#include "platform/platform.h"
#include "utils/common/time_utils.h"
#include "utils/memory/memory_manager.h"

/* Forward declaration for connection processing */
extern void process_connection(void* conn);

/**
 * @brief Initialize thread pool
 * @param pool Thread pool to initialize
 * @param thread_count Number of worker threads to create
 * @return 0 on success, -1 on error
 */
int thread_pool_init(thread_pool_t* pool, int thread_count) {
  if (!pool) {
    platform_log_error("Thread pool pointer is NULL\n");
    return -1;
  }

  if (thread_count <= 0) {
    platform_log_error("Thread count must be positive, got %d\n", thread_count);
    return -1;
  }

  if (thread_count > THREAD_POOL_MAX_THREADS) {
    platform_log_error("Thread count exceeds maximum (%d), got %d\n", THREAD_POOL_MAX_THREADS, thread_count);
    return -1;
  }

  // Use static mutex and condition variable instead of dynamic allocation
  if (pthread_mutex_init(&pool->queue_mutex, NULL) != 0) {
    platform_log_error("Failed to initialize thread pool mutex\n");
    return -1;
  }

  if (pthread_cond_init(&pool->queue_cond, NULL) != 0) {
    platform_log_error("Failed to initialize thread pool condition\n");
    pthread_mutex_destroy(&pool->queue_mutex);
    return -1;
  }

  pool->work_queue = NULL;
  pool->shutdown = 0;
  pool->active_threads = 0;
  pool->thread_count = thread_count;

  // Allocate memory for thread handles
  pool->threads = ONVIF_MALLOC(thread_count * sizeof(pthread_t));
  if (!pool->threads) {
    platform_log_error("Failed to allocate memory for thread handles\n");
    pthread_cond_destroy(&pool->queue_cond);
    pthread_mutex_destroy(&pool->queue_mutex);
    return -1;
  }

  // Create worker threads
  for (int i = 0; i < thread_count; i++) {
    if (pthread_create(&pool->threads[i], NULL, thread_pool_worker, pool) != 0) {
      platform_log_error("Failed to create worker thread %d\n", i);
      // Clean up already created threads
      pool->shutdown = 1;
      pthread_cond_broadcast(&pool->queue_cond);
      for (int j = 0; j < i; j++) {
        pthread_join(pool->threads[j], NULL);
      }
      ONVIF_FREE(pool->threads);
      pool->threads = NULL;
      pthread_cond_destroy(&pool->queue_cond);
      pthread_mutex_destroy(&pool->queue_mutex);
      return -1;
    }
  }

  platform_log_info("Thread pool initialized with %d worker threads\n", thread_count);
  return 0;
}

/**
 * @brief Cleanup thread pool
 * @param pool Thread pool to cleanup
 */
void thread_pool_cleanup(thread_pool_t* pool) {
  if (!pool) {
    return;
  }

  platform_log_info("Shutting down thread pool...\n");

  // Signal all worker threads to shutdown
  platform_log_debug("Signaling worker threads to shutdown...\n");
  pthread_mutex_lock(&pool->queue_mutex);
  pool->shutdown = 1;
  pthread_cond_broadcast(&pool->queue_cond);
  pthread_mutex_unlock(&pool->queue_mutex);
  platform_log_debug("Shutdown signal sent to all worker threads\n");

  // Give threads a moment to process the shutdown signal
  platform_log_debug("Waiting for threads to process shutdown signal...\n");
  sleep_ms(THREAD_POOL_SHUTDOWN_DELAY_MS);
  platform_log_debug("Starting thread join process...\n");

  // Wait for all threads to finish with timeout
  for (int i = 0; i < pool->thread_count; i++) {
    platform_log_debug("Waiting for worker thread %d to finish...\n", i);

    struct timespec timeout;
    clock_gettime(CLOCK_REALTIME, &timeout);
    timeout.tv_sec += 2; // 2 second timeout per thread

    int join_result = pthread_timedjoin_np(pool->threads[i], NULL, &timeout);
    if (join_result == ETIMEDOUT) {
      platform_log_warning("Worker thread %d did not finish within timeout, continuing...\n", i);
      // Try to cancel the thread as a last resort
      pthread_cancel(pool->threads[i]);
      // Wait a bit more for cancellation to take effect
      struct timespec cancel_timeout;
      clock_gettime(CLOCK_REALTIME, &cancel_timeout);
      cancel_timeout.tv_sec += 1;
      pthread_timedjoin_np(pool->threads[i], NULL, &cancel_timeout);
    } else if (join_result != 0) {
      platform_log_warning("Failed to join worker thread %d: %s\n", i, strerror(join_result));
    } else {
      platform_log_debug("Worker thread %d finished successfully\n", i);
    }
  }

  platform_log_debug("Destroying thread pool synchronization objects...\n");
  pthread_cond_destroy(&pool->queue_cond);
  pthread_mutex_destroy(&pool->queue_mutex);

  // Free thread handles array
  if (pool->threads) {
    ONVIF_FREE(pool->threads);
    pool->threads = NULL;
  }

  platform_log_info("Thread pool cleaned up\n");
}

/**
 * @brief Add connection to work queue
 * @param pool Thread pool
 * @param conn Connection to add
 */
void thread_pool_add_work(thread_pool_t* pool, connection_t* conn) {
  if (!pool || !conn) {
    return;
  }

  pthread_mutex_lock(&pool->queue_mutex);

  conn->next = pool->work_queue;
  conn->prev = NULL;
  if (pool->work_queue) {
    pool->work_queue->prev = conn;
  }
  pool->work_queue = conn;

  pthread_cond_signal(&pool->queue_cond);
  pthread_mutex_unlock(&pool->queue_mutex);
}

/**
 * @brief Check if pool should shutdown
 */
static bool should_shutdown(thread_pool_t* pool) {
  return pool->shutdown || !signal_lifecycle_should_continue();
}

/**
 * @brief Wait for work to become available in queue
 * @return true if work is available, false if should shutdown
 */
static bool wait_for_work(thread_pool_t* pool) {
  while (pool->work_queue == NULL && !should_shutdown(pool)) {
    struct timespec timeout;
    clock_gettime(CLOCK_REALTIME, &timeout);
    timeout.tv_sec += 1; // 1 second timeout for responsive shutdown

    int wait_result = pthread_cond_timedwait(&pool->queue_cond, &pool->queue_mutex, &timeout);

    if (wait_result == ETIMEDOUT) {
      if (should_shutdown(pool)) {
        return false;
      }
      continue;
    }

    if (wait_result != 0) {
      platform_log_warning("Worker thread cond_wait failed: %s\n", strerror(wait_result));
      return false;
    }
  }

  return !should_shutdown(pool);
}

/**
 * @brief Dequeue connection from work queue
 */
static connection_t* dequeue_work(thread_pool_t* pool) {
  connection_t* conn = pool->work_queue;
  if (!conn) {
    return NULL;
  }

  pool->work_queue = conn->next;
  if (pool->work_queue) {
    pool->work_queue->prev = NULL;
  }

  pool->active_threads++;
  return conn;
}

/**
 * @brief Worker thread function
 * @param arg Thread pool pointer
 * @return NULL
 */
void* thread_pool_worker(void* arg) {
  thread_pool_t* pool = (thread_pool_t*)arg;

  platform_log_debug("Worker thread started\n");

  while (1) {
    pthread_mutex_lock(&pool->queue_mutex);

    // Wait for work or shutdown
    if (!wait_for_work(pool)) {
      pthread_mutex_unlock(&pool->queue_mutex);
      break;
    }

    // Dequeue work
    connection_t* conn = dequeue_work(pool);
    pthread_mutex_unlock(&pool->queue_mutex);

    // Process connection if we got one
    if (conn) {
      process_connection(conn);

      pthread_mutex_lock(&pool->queue_mutex);
      pool->active_threads--;
      pthread_mutex_unlock(&pool->queue_mutex);
    }

    // Check for shutdown signal after processing
    if (!signal_lifecycle_should_continue()) {
      platform_log_debug("Worker thread received shutdown signal, exiting\n");
      break;
    }
  }

  platform_log_debug("Worker thread stopped\n");
  return NULL;
}
