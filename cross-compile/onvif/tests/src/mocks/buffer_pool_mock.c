/**
 * @file buffer_pool_mock.c
 * @brief Implementation of buffer pool mock functions using generic mock framework
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool_mock.h"

#include <stdio.h>
#include <stdlib.h>

#include "../common/generic_mock_framework.h"

/* ============================================================================
 * Mock Operations
 * ============================================================================ */

enum buffer_pool_operations {
  BUFFER_POOL_OP_INIT = 0,
  BUFFER_POOL_OP_CLEANUP,
  BUFFER_POOL_OP_GET,
  BUFFER_POOL_OP_RETURN,
  BUFFER_POOL_OP_GET_STATS,
  BUFFER_POOL_OP_COUNT
};

/* ============================================================================
 * Mock Instance
 * ============================================================================ */

// Create mock instance using generic framework
GENERIC_MOCK_CREATE(buffer_pool, BUFFER_POOL_OP_COUNT);

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

int mock_buffer_pool_set_init_result(int result) {
  return generic_mock_set_operation_result(&buffer_pool_mock, BUFFER_POOL_OP_INIT, result);
}

int mock_buffer_pool_set_cleanup_result(int result) {
  return generic_mock_set_operation_result(&buffer_pool_mock, BUFFER_POOL_OP_CLEANUP, result);
}

int mock_buffer_pool_get_init_call_count(void) {
  return generic_mock_get_operation_call_count(&buffer_pool_mock, BUFFER_POOL_OP_INIT);
}

int mock_buffer_pool_get_cleanup_call_count(void) {
  return generic_mock_get_operation_call_count(&buffer_pool_mock, BUFFER_POOL_OP_CLEANUP);
}

void buffer_pool_mock_init(void) {
  generic_mock_init(&buffer_pool_mock);

  // Set default results
  mock_buffer_pool_set_init_result(0);
  mock_buffer_pool_set_cleanup_result(0);
  generic_mock_set_operation_result(&buffer_pool_mock, BUFFER_POOL_OP_GET, 0);
  generic_mock_set_operation_result(&buffer_pool_mock, BUFFER_POOL_OP_RETURN, 0);
  generic_mock_set_operation_result(&buffer_pool_mock, BUFFER_POOL_OP_GET_STATS, 0);
}

void buffer_pool_mock_cleanup(void) {
  generic_mock_cleanup(&buffer_pool_mock);
}

/* ============================================================================
 * Buffer Pool Actual Functions
 * ============================================================================ */

int buffer_pool_init(buffer_pool_t* pool) {
  (void)pool;
  return generic_mock_execute_operation(&buffer_pool_mock, BUFFER_POOL_OP_INIT, pool);
}

void buffer_pool_cleanup(buffer_pool_t* pool) {
  (void)pool;
  (void)generic_mock_execute_operation(&buffer_pool_mock, BUFFER_POOL_OP_CLEANUP, pool);
}

void* buffer_pool_get(buffer_pool_t* pool) {
  (void)pool;
  (void)generic_mock_execute_operation(&buffer_pool_mock, BUFFER_POOL_OP_GET, pool);
  return malloc(1024); // Mock buffer
}

void buffer_pool_return(buffer_pool_t* pool, void* buffer) {
  (void)pool;
  (void)generic_mock_execute_operation(&buffer_pool_mock, BUFFER_POOL_OP_RETURN, pool);
  if (buffer) {
    free(buffer);
  }
}

int buffer_pool_get_stats(buffer_pool_t* pool, int* used, int* total, size_t* memory_used) {
  (void)pool;
  (void)generic_mock_execute_operation(&buffer_pool_mock, BUFFER_POOL_OP_GET_STATS, pool);

  if (used)
    *used = 5;
  if (total)
    *total = 10;
  if (memory_used)
    *memory_used = 5120; // 5 * 1024

  return 0;
}
