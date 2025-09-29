/**
 * @file buffer_pool_mock.c
 * @brief Implementation of buffer pool mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool_mock.h"

#include <stdio.h>
#include <stdlib.h>

// Mock state
static int g_buffer_pool_mock_initialized = 0; // NOLINT
static int g_buffer_pool_init_result = 0;      // NOLINT
static int g_buffer_pool_cleanup_result = 0;   // NOLINT

// Call counters
static int g_buffer_pool_init_call_count = 0;    // NOLINT
static int g_buffer_pool_cleanup_call_count = 0; // NOLINT

/**
 * @brief Set mock buffer pool init result
 * @param result Result to return
 * @return 0 on success
 */
int mock_buffer_pool_set_init_result(int result) {
  g_buffer_pool_init_result = result;
  return 0;
}

/**
 * @brief Set mock buffer pool cleanup result
 * @param result Result to return
 * @return 0 on success
 */
int mock_buffer_pool_set_cleanup_result(int result) {
  g_buffer_pool_cleanup_result = result;
  return 0;
}

/**
 * @brief Get mock buffer pool init call count
 * @return Number of init calls
 */
int mock_buffer_pool_get_init_call_count(void) {
  return g_buffer_pool_init_call_count;
}

/**
 * @brief Get mock buffer pool cleanup call count
 * @return Number of cleanup calls
 */
int mock_buffer_pool_get_cleanup_call_count(void) {
  return g_buffer_pool_cleanup_call_count;
}

/**
 * @brief Initialize buffer pool mock
 */
void buffer_pool_mock_init(void) {
  g_buffer_pool_mock_initialized = 1;
  g_buffer_pool_init_result = 0;
  g_buffer_pool_cleanup_result = 0;
  g_buffer_pool_init_call_count = 0;
  g_buffer_pool_cleanup_call_count = 0;
}

/**
 * @brief Cleanup buffer pool mock
 */
void buffer_pool_mock_cleanup(void) {
  g_buffer_pool_mock_initialized = 0;
  g_buffer_pool_init_result = 0;
  g_buffer_pool_cleanup_result = 0;
  g_buffer_pool_init_call_count = 0;
  g_buffer_pool_cleanup_call_count = 0;
}

/* ============================================================================
 * Buffer Pool Actual Functions
 * ============================================================================ */

/**
 * @brief Mock buffer pool initialization
 * @param pool Buffer pool to initialize
 * @return 0 on success
 */
int buffer_pool_init(buffer_pool_t* pool) {
  (void)pool;
  g_buffer_pool_init_call_count++;
  return g_buffer_pool_init_result;
}

/**
 * @brief Mock buffer pool cleanup
 * @param pool Buffer pool to cleanup
 */
void buffer_pool_cleanup(buffer_pool_t* pool) {
  (void)pool;
  g_buffer_pool_cleanup_call_count++;
}

/**
 * @brief Mock buffer pool get
 * @param pool Buffer pool
 * @return Mock buffer pointer
 */
void* buffer_pool_get(buffer_pool_t* pool) {
  (void)pool;
  return malloc(1024); // Mock buffer
}

/**
 * @brief Mock buffer pool return
 * @param pool Buffer pool
 * @param buffer Buffer to return
 */
void buffer_pool_return(buffer_pool_t* pool, void* buffer) {
  (void)pool;
  if (buffer) {
    free(buffer);
  }
}

/**
 * @brief Mock buffer pool get stats
 * @param pool Buffer pool
 * @param used Used buffer count
 * @param total Total buffer count
 * @param memory_used Memory used
 * @return 0 on success
 */
int buffer_pool_get_stats(buffer_pool_t* pool, int* used, int* total, size_t* memory_used) {
  (void)pool;
  if (used)
    *used = 5;
  if (total)
    *total = 10;
  if (memory_used)
    *memory_used = 5120; // 5 * 1024
  return 0;
}
