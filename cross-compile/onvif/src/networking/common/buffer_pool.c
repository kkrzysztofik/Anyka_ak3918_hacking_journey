/**
 * @file buffer_pool.c
 * @brief Buffer pool management implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool.h"

#include "platform/platform.h"

#include <pthread.h>
#include <stdlib.h>

/**
 * @brief Initialize buffer pool
 * @param pool Buffer pool to initialize
 * @return 0 on success, -1 on error
 */
int buffer_pool_init(buffer_pool_t* pool) {
  if (!pool) {
    return -1;
  }

  // Use static mutex instead of dynamic allocation
  if (pthread_mutex_init(&pool->mutex, NULL) != 0) {
    platform_log_error("Failed to initialize buffer pool mutex\n");
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
      pthread_mutex_destroy(&pool->mutex);
      return -1;
    }
    pool->available[i] = 1;
  }

  pool->count = BUFFER_POOL_SIZE;

  // Initialize statistics counters
  pool->hits = 0;
  pool->misses = 0;

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

  for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
    if (pool->buffers[i]) {
      free(pool->buffers[i]);
      pool->buffers[i] = NULL;
    }
  }

  pthread_mutex_destroy(&pool->mutex);

  platform_log_info("Buffer pool cleaned up\n");
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

  pthread_mutex_unlock(&pool->mutex);

  if (buffer) {
    pool->hits++; // Atomic increment for thread safety
    platform_log_debug("buffer_pool_get: SUCCESS - acquired buffer %d, %d buffers remaining "
                       "available",
                       buffer_index, available_count - 1);
  } else {
    pool->misses++; // Atomic increment for thread safety
    platform_log_debug("buffer_pool_get: MISS - no buffers available, all %d buffers in use",
                       BUFFER_POOL_SIZE);
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

  pthread_mutex_unlock(&pool->mutex);

  if (buffer_index != -1) {
    platform_log_debug("buffer_pool_return: SUCCESS - returned buffer %d, %d buffers now "
                       "available",
                       buffer_index, available_count);
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
  stats.utilization_percent =
    (used_count * 100) / // NOLINT(cppcoreguidelines-avoid-magic-numbers,readability-magic-numbers)
    BUFFER_POOL_SIZE;

  pthread_mutex_unlock(&pool->mutex);

  return stats;
}
