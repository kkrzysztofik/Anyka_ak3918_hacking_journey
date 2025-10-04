/**
 * @file buffer_pool_mock.c
 * @brief Implementation of CMocka-based buffer pool mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "buffer_pool_mock.h"

#include <stdbool.h>
#include <stddef.h>

#include "cmocka_wrapper.h"


/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void buffer_pool_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * CMocka Wrapped Buffer Pool Functions
 * ============================================================================ */

int __wrap_buffer_pool_init(buffer_pool_t* pool) {
  if (g_use_real_functions) {
    return __real_buffer_pool_init(pool);
  }

  check_expected_ptr(pool);
  return (int)mock();
}

void __wrap_buffer_pool_cleanup(buffer_pool_t* pool) {
  if (g_use_real_functions) {
    __real_buffer_pool_cleanup(pool);
    return;
  }

  check_expected_ptr(pool);
}

void* __wrap_buffer_pool_get(buffer_pool_t* pool) {
  if (g_use_real_functions) {
    return __real_buffer_pool_get(pool);
  }

  check_expected_ptr(pool);
  return (void*)mock();
}

void __wrap_buffer_pool_return(buffer_pool_t* pool, void* buffer) {
  if (g_use_real_functions) {
    __real_buffer_pool_return(pool, buffer);
    return;
  }

  check_expected_ptr(pool);
  check_expected_ptr(buffer);
}

int __wrap_buffer_pool_get_stats(buffer_pool_t* pool, buffer_pool_stats_t* stats) {
  if (g_use_real_functions) {
    return __real_buffer_pool_get_stats(pool, stats);
  }

  check_expected_ptr(pool);
  check_expected_ptr(stats);
  return (int)mock();
}

int __wrap_get_buffer_pool_stats(buffer_pool_stats_t* stats) {
  if (g_use_real_functions) {
    return __real_get_buffer_pool_stats(stats);
  }

  check_expected_ptr(stats);
  return (int)mock();
}
