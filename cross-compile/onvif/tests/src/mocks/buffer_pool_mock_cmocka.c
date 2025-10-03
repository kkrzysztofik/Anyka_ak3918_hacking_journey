/**
 * @file buffer_pool_mock_cmocka.c
 * @brief Implementation of CMocka-based buffer pool mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool_mock_cmocka.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

/* ============================================================================
 * CMocka Wrapped Buffer Pool Functions
 * ============================================================================ */

int __wrap_buffer_pool_init(buffer_pool_t* pool) {
  check_expected_ptr(pool);
  return (int)mock();
}

void __wrap_buffer_pool_cleanup(buffer_pool_t* pool) {
  check_expected_ptr(pool);
}

void* __wrap_buffer_pool_get(buffer_pool_t* pool) {
  check_expected_ptr(pool);
  return (void*)mock();
}

void __wrap_buffer_pool_return(buffer_pool_t* pool, void* buffer) {
  check_expected_ptr(pool);
  check_expected_ptr(buffer);
}

int __wrap_buffer_pool_get_stats(buffer_pool_t* pool, buffer_pool_stats_t* stats) {
  check_expected_ptr(pool);
  check_expected_ptr(stats);
  return (int)mock();
}

int __wrap_get_buffer_pool_stats(buffer_pool_stats_t* stats) {
  check_expected_ptr(stats);
  return (int)mock();
}
