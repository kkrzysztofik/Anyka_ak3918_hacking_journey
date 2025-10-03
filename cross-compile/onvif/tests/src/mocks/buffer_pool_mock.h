/**
 * @file buffer_pool_mock.h
 * @brief CMocka-based buffer pool mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef BUFFER_POOL_MOCK_H
#define BUFFER_POOL_MOCK_H

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#include "networking/common/buffer_pool.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * CMocka Wrapped Buffer Pool Functions
 * ============================================================================
 * All buffer pool functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped buffer pool initialization
 * @param pool Buffer pool pointer
 * @return Result code (configured via will_return)
 */
int __wrap_buffer_pool_init(buffer_pool_t* pool);

/**
 * @brief CMocka wrapped buffer pool cleanup
 * @param pool Buffer pool pointer
 */
void __wrap_buffer_pool_cleanup(buffer_pool_t* pool);

/**
 * @brief CMocka wrapped buffer pool get
 * @param pool Buffer pool pointer
 * @return Buffer pointer (configured via will_return)
 */
void* __wrap_buffer_pool_get(buffer_pool_t* pool);

/**
 * @brief CMocka wrapped buffer pool return
 * @param pool Buffer pool pointer
 * @param buffer Buffer to return
 */
void __wrap_buffer_pool_return(buffer_pool_t* pool, void* buffer);

/**
 * @brief CMocka wrapped buffer pool get stats
 * @param pool Buffer pool pointer
 * @param stats Statistics structure pointer
 * @return Result code (configured via will_return)
 */
int __wrap_buffer_pool_get_stats(buffer_pool_t* pool, buffer_pool_stats_t* stats);

/**
 * @brief CMocka wrapped get buffer pool stats (global)
 * @param stats Statistics structure pointer
 * @return Result code (configured via will_return)
 */
int __wrap_get_buffer_pool_stats(buffer_pool_stats_t* stats);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful buffer pool initialization
 */
#define EXPECT_BUFFER_POOL_INIT_SUCCESS()                                                          \
  expect_any(__wrap_buffer_pool_init, pool);                                                       \
  will_return(__wrap_buffer_pool_init, 0)

/**
 * @brief Set up expectations for buffer pool initialization failure
 * @param error_code Error code to return
 */
#define EXPECT_BUFFER_POOL_INIT_ERROR(error_code)                                                  \
  expect_any(__wrap_buffer_pool_init, pool);                                                       \
  will_return(__wrap_buffer_pool_init, error_code)

/**
 * @brief Set up expectations for buffer pool cleanup
 */
#define EXPECT_BUFFER_POOL_CLEANUP()                                                               \
  expect_any(__wrap_buffer_pool_cleanup, pool)

/**
 * @brief Set up expectations for successful buffer pool get
 * @param buffer_ptr Buffer pointer to return
 */
#define EXPECT_BUFFER_POOL_GET_SUCCESS(buffer_ptr)                                                 \
  expect_any(__wrap_buffer_pool_get, pool);                                                        \
  will_return(__wrap_buffer_pool_get, buffer_ptr)

/**
 * @brief Set up expectations for buffer pool get failure (NULL)
 */
#define EXPECT_BUFFER_POOL_GET_NULL()                                                              \
  expect_any(__wrap_buffer_pool_get, pool);                                                        \
  will_return(__wrap_buffer_pool_get, NULL)

/**
 * @brief Set up expectations for buffer pool return
 */
#define EXPECT_BUFFER_POOL_RETURN()                                                                \
  expect_any(__wrap_buffer_pool_return, pool);                                                     \
  expect_any(__wrap_buffer_pool_return, buffer)

/**
 * @brief Set up expectations for successful buffer pool get stats
 */
#define EXPECT_BUFFER_POOL_GET_STATS_SUCCESS()                                                     \
  expect_any(__wrap_buffer_pool_get_stats, pool);                                                  \
  expect_any(__wrap_buffer_pool_get_stats, stats);                                                 \
  will_return(__wrap_buffer_pool_get_stats, 0)

/**
 * @brief Set up expectations for successful get buffer pool stats (global)
 */
#define EXPECT_GET_BUFFER_POOL_STATS_SUCCESS()                                                     \
  expect_any(__wrap_get_buffer_pool_stats, stats);                                                 \
  will_return(__wrap_get_buffer_pool_stats, 0)

#ifdef __cplusplus
}
#endif

#endif // BUFFER_POOL_MOCK_H
