/**
 * @file buffer_pool.c
 * @brief Buffer pool management implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool.h"

#include <pthread.h>
#include <stdlib.h>

#include "platform/platform.h"

/**
 * @brief Initialize buffer pool
 * @param pool Buffer pool to initialize
 * @return 0 on success, -1 on error
 */
int buffer_pool_init(buffer_pool_t* pool) {
  if (!pool) {
    platform_log_error("buffer_pool_init: NULL pool pointer\n");
    return -1;
  }

  platform_log_debug(
    "buffer_pool_init: Starting initialization (initialized=%d, mutex_initialized=%d)\n",
    pool->initialized, pool->mutex_initialized);

  // Check if already initialized - allow idempotent initialization
  if (pool->initialized) {
    platform_log_debug("Buffer pool already initialized, skipping reinitialization\n");
    return 0;
  }

  // Initialize mutex only if not already initialized
  if (!pool->mutex_initialized) {
    // First initialization - init the mutex
    platform_log_debug("Initializing buffer pool mutex...\n");
    int mutex_result = pthread_mutex_init(&pool->mutex, NULL);
    if (mutex_result != 0) {
      platform_log_error("Failed to initialize buffer pool mutex (errno=%d)\n", mutex_result);
      return -1;
    }
    pool->mutex_initialized = 1;
    platform_log_debug("Buffer pool mutex initialized successfully\n");
  } else {
    platform_log_debug("Buffer pool mutex already initialized, reusing\n");
  }

  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    pool->buffers[i] = malloc(BUFFER_SIZE);
    if (!pool->buffers[i]) {
      platform_log_error("Failed to allocate buffer %d\n", i);
      // Clean up already allocated buffers
      for (int j = 0; j < i; j++) {
        free(pool->buffers[j]);
        pool->buffers[j] = NULL;
      }
      // Note: Don't destroy mutex on error - it will be reused or cleaned up later
      return -1;
    }
    pool->available[i] = 1;
  }

  pool->count = BUFFER_POOL_SIZE;

  // Initialize statistics counters
  pool->hits = 0;
  pool->misses = 0;
  pool->peak_utilization = 0;

  // Mark as initialized
  pool->initialized = 1;

  platform_log_info("Buffer pool initialized with %d buffers\n", BUFFER_POOL_SIZE);
  return 0;
}

/**
 * @brief Cleanup buffer pool
 * @param pool Buffer pool to cleanup
 */
void buffer_pool_cleanup(buffer_pool_t* pool) {
  if (!pool) {
    return;
  }

  // Check if not initialized - allow idempotent cleanup
  if (!pool->initialized) {
    platform_log_debug("Buffer pool not initialized, skipping cleanup\n");
    return;
  }

  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (pool->buffers[i]) {
      free(pool->buffers[i]);
      pool->buffers[i] = NULL;
    }
  }

  // NOTE: Do NOT destroy the mutex to allow reinitialization
  // The mutex will be reused on next init call
  // pthread_mutex_destroy(&pool->mutex);  // Commented out for idempotent init/cleanup

  // Reset initialized flag to allow reinitialization
  pool->initialized = 0;

  platform_log_info("Buffer pool cleaned up (mutex preserved for reinitialization)\n");
}

/**
 * @brief Get buffer from pool
 * @param pool Buffer pool
 * @return Buffer pointer or NULL if none available
 */
char* buffer_pool_get(buffer_pool_t* pool) {
  if (!pool) {
    platform_log_debug("buffer_pool_get: NULL pool provided");
    return NULL;
  }

  char* buffer = NULL;
  int buffer_index = -1;
  int available_count = 0;

  pthread_mutex_lock(&pool->mutex);

  // Count available buffers and find first available
  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (pool->available[i]) {
      available_count++;
      if (buffer_index == -1) {
        buffer_index = i;
        pool->available[i] = 0;
        buffer = pool->buffers[i];
      }
    }
  }

  // Calculate current utilization percentage
  int current_utilization =
    ((BUFFER_POOL_SIZE - available_count) * BUFFER_POOL_UTILIZATION_MAX_PERCENT) / BUFFER_POOL_SIZE;

  // Update peak utilization if current is higher
  if (current_utilization > pool->peak_utilization) {
    pool->peak_utilization = current_utilization;
  }

  pthread_mutex_unlock(&pool->mutex);

  if (buffer) {
    pool->hits++; // Atomic increment for thread safety
    platform_log_debug("buffer_pool_get: SUCCESS - acquired buffer %d, %d buffers remaining "
                       "available (utilization: %d%%)",
                       buffer_index, available_count - 1, current_utilization);

    // Log warning if utilization exceeds threshold
    if (current_utilization > BUFFER_POOL_UTILIZATION_WARNING_THRESHOLD) {
      platform_log_warning("buffer_pool_get: HIGH UTILIZATION WARNING - Pool utilization at %d%% "
                           "(%d/%d buffers in use)",
                           current_utilization, BUFFER_POOL_SIZE - available_count + 1,
                           BUFFER_POOL_SIZE);
    }
  } else {
    pool->misses++; // Atomic increment for thread safety
    platform_log_debug("buffer_pool_get: MISS - no buffers available, all %d buffers in use "
                       "(utilization: %d%%)",
                       BUFFER_POOL_SIZE, BUFFER_POOL_UTILIZATION_MAX_PERCENT);

    // Log warning for pool exhaustion
    platform_log_warning(
      "buffer_pool_get: POOL EXHAUSTED - All %d buffers in use (%d%% utilization)",
      BUFFER_POOL_SIZE, BUFFER_POOL_UTILIZATION_MAX_PERCENT);
  }

  return buffer;
}

/**
 * @brief Return buffer to pool
 * @param pool Buffer pool
 * @param buffer Buffer to return
 */
void buffer_pool_return(buffer_pool_t* pool, const char* buffer) {
  if (!pool || !buffer) {
    platform_log_debug("buffer_pool_return: NULL pool or buffer provided");
    return;
  }

  int buffer_index = -1;
  int available_count = 0;

  pthread_mutex_lock(&pool->mutex);

  // Find the buffer and count available after return
  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (pool->buffers[i] == buffer) {
      pool->available[i] = 1;
      buffer_index = i;
    }
    if (pool->available[i]) {
      available_count++;
    }
  }

  // Calculate current utilization percentage after return
  int current_utilization =
    ((BUFFER_POOL_SIZE - available_count) * BUFFER_POOL_UTILIZATION_MAX_PERCENT) / BUFFER_POOL_SIZE;

  pthread_mutex_unlock(&pool->mutex);

  if (buffer_index != -1) {
    platform_log_debug("buffer_pool_return: SUCCESS - returned buffer %d, %d buffers now "
                       "available (utilization: %d%%)",
                       buffer_index, available_count, current_utilization);
  } else {
    platform_log_debug("buffer_pool_return: ERROR - buffer not found in pool");
  }
}

/**
 * @brief Get buffer pool statistics
 * @param pool Buffer pool
 * @return Buffer pool statistics structure
 */
buffer_pool_stats_t buffer_pool_get_stats(buffer_pool_t* pool) {
  buffer_pool_stats_t stats = {0};

  if (!pool) {
    return stats; // Return zero-initialized stats
  }

  pthread_mutex_lock(&pool->mutex);

  // Count currently used buffers
  int used_count = 0;
  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (!pool->available[i]) {
      used_count++;
    }
  }

  // Calculate statistics
  stats.hits = pool->hits;
  stats.misses = pool->misses;
  stats.current_used = used_count;
  stats.utilization_percent = (used_count * BUFFER_POOL_UTILIZATION_MAX_PERCENT) / BUFFER_POOL_SIZE;
  stats.total_requests = pool->hits + pool->misses;
  stats.peak_utilization = pool->peak_utilization;

  pthread_mutex_unlock(&pool->mutex);

  return stats;
}

/**
 * @brief Get comprehensive buffer pool utilization statistics
 * @param pool Buffer pool to get statistics from
 * @return Buffer pool statistics structure with real-time metrics
 * @note This function is thread-safe and provides real-time utilization data
 */
buffer_pool_stats_t get_buffer_pool_stats(buffer_pool_t* pool) {
  buffer_pool_stats_t stats = {0};

  if (!pool) {
    platform_log_debug("get_buffer_pool_stats: NULL pool provided");
    return stats; // Return zero-initialized stats
  }

  pthread_mutex_lock(&pool->mutex);

  // Count currently used buffers
  int used_count = 0;
  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (!pool->available[i]) {
      used_count++;
    }
  }

  // Calculate comprehensive statistics
  stats.hits = pool->hits;
  stats.misses = pool->misses;
  stats.current_used = used_count;
  stats.utilization_percent = (used_count * BUFFER_POOL_UTILIZATION_MAX_PERCENT) / BUFFER_POOL_SIZE;
  stats.total_requests = pool->hits + pool->misses;
  stats.peak_utilization = pool->peak_utilization;

  pthread_mutex_unlock(&pool->mutex);

  // Log utilization status for monitoring
  if (stats.utilization_percent > BUFFER_POOL_UTILIZATION_WARNING_THRESHOLD) {
    platform_log_warning("get_buffer_pool_stats: HIGH UTILIZATION - Pool at %d%% utilization "
                         "(%d/%d buffers in use, peak: %d%%)",
                         stats.utilization_percent, stats.current_used, BUFFER_POOL_SIZE,
                         stats.peak_utilization);
  } else {
    platform_log_debug("get_buffer_pool_stats: Pool utilization at %d%% (%d/%d buffers in use, "
                       "peak: %d%%, hits: %d, misses: %d)",
                       stats.utilization_percent, stats.current_used, BUFFER_POOL_SIZE,
                       stats.peak_utilization, stats.hits, stats.misses);
  }

  return stats;
}
