/**
 * @file buffer_pool.h
 * @brief Buffer pool management for efficient memory allocation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides a pool of pre-allocated buffers to eliminate
 * malloc/free overhead during runtime.
 */

#ifndef BUFFER_POOL_H
#define BUFFER_POOL_H

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stddef.h>

/* Buffer pool configuration */
#define BUFFER_POOL_SIZE 50
#define BUFFER_SIZE      32768

/* Buffer pool monitoring constants */
#define BUFFER_POOL_UTILIZATION_WARNING_THRESHOLD 80
#define BUFFER_POOL_UTILIZATION_MAX_PERCENT       100

/* Buffer pool statistics structure */
typedef struct {
  int hits;                // Successful buffer acquisitions
  int misses;              // Failed buffer acquisitions (pool exhausted)
  int current_used;        // Currently allocated buffers
  int utilization_percent; // Current utilization percentage
  int total_requests;      // Total buffer requests (hits + misses)
  int peak_utilization;    // Peak utilization percentage reached
} buffer_pool_stats_t;

/* Buffer pool structure */
typedef struct {
  char* buffers[BUFFER_POOL_SIZE];
  int available[BUFFER_POOL_SIZE];
  int count;
  int initialized;       // Flag to track buffer pool initialization state
  int mutex_initialized; // Flag to track mutex initialization state
  pthread_mutex_t mutex; // Static mutex instead of dynamic allocation
  // Thread-safe statistics counters
  volatile int hits;             // Use volatile for thread safety
  volatile int misses;           // Use volatile for thread safety
  volatile int peak_utilization; // Peak utilization percentage reached
} buffer_pool_t;

/* Buffer pool functions */
int buffer_pool_init(buffer_pool_t* pool);
void buffer_pool_cleanup(buffer_pool_t* pool);
char* buffer_pool_get(buffer_pool_t* pool);
void buffer_pool_return(buffer_pool_t* pool, const char* buffer);
buffer_pool_stats_t buffer_pool_get_stats(buffer_pool_t* pool);

/* Buffer pool monitoring functions */
/**
 * @brief Get comprehensive buffer pool utilization statistics
 * @param pool Buffer pool to get statistics from
 * @return Buffer pool statistics structure with real-time metrics
 * @note This function is thread-safe and provides real-time utilization data
 */
buffer_pool_stats_t get_buffer_pool_stats(buffer_pool_t* pool);

#endif /* BUFFER_POOL_H */
