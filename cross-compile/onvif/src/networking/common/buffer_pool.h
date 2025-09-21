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
#define BUFFER_SIZE 32768

/* Buffer pool structure */
typedef struct {
  char *buffers[BUFFER_POOL_SIZE];
  int available[BUFFER_POOL_SIZE];
  int count;
  pthread_mutex_t mutex;  // Static mutex instead of dynamic allocation
} buffer_pool_t;

/* Buffer pool functions */
int buffer_pool_init(buffer_pool_t *pool);
void buffer_pool_cleanup(buffer_pool_t *pool);
char *buffer_pool_get(buffer_pool_t *pool);
void buffer_pool_return(buffer_pool_t *pool, const char *buffer);

#endif /* BUFFER_POOL_H */
