/**
 * @file smart_response_mock.h
 * @brief CMocka-based smart response builder mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SMART_RESPONSE_MOCK_H
#define SMART_RESPONSE_MOCK_H

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Forward declarations */
struct soap;
struct buffer_pool_t;

/* ============================================================================
 * CMocka Wrapped Smart Response Functions
 * ============================================================================
 * All smart response functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped smart response build with dynamic buffer
 * @param soap SOAP context pointer
 * @param response_func Response function pointer
 * @param response Response data pointer
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build_with_dynamic_buffer(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response);

/**
 * @brief CMocka wrapped smart response build with buffer pool
 * @param soap SOAP context pointer
 * @param response_func Response function pointer
 * @param response Response data pointer
 * @param pool Buffer pool pointer
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build_with_buffer_pool(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response, struct buffer_pool_t* pool);

/**
 * @brief CMocka wrapped smart response build
 * @param soap SOAP context pointer
 * @param response_func Response function pointer
 * @param response Response data pointer
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response);

/**
 * @brief CMocka wrapped smart response estimate size
 * @param soap SOAP context pointer
 * @param response_func Response function pointer
 * @param response Response data pointer
 * @return Estimated size (configured via will_return)
 */
size_t __wrap_smart_response_estimate_size(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful smart response build with dynamic buffer
 */
#define EXPECT_SMART_RESPONSE_BUILD_DYNAMIC_SUCCESS()                                              \
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, soap);                              \
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, response_func);                     \
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, response);                          \
  will_return(__wrap_smart_response_build_with_dynamic_buffer, 0)

/**
 * @brief Set up expectations for successful smart response build with buffer pool
 */
#define EXPECT_SMART_RESPONSE_BUILD_POOL_SUCCESS()                                                 \
  expect_any(__wrap_smart_response_build_with_buffer_pool, soap);                                 \
  expect_any(__wrap_smart_response_build_with_buffer_pool, response_func);                        \
  expect_any(__wrap_smart_response_build_with_buffer_pool, response);                             \
  expect_any(__wrap_smart_response_build_with_buffer_pool, pool);                                 \
  will_return(__wrap_smart_response_build_with_buffer_pool, 0)

/**
 * @brief Set up expectations for successful smart response build
 */
#define EXPECT_SMART_RESPONSE_BUILD_SUCCESS()                                                      \
  expect_any(__wrap_smart_response_build, soap);                                                  \
  expect_any(__wrap_smart_response_build, response_func);                                         \
  expect_any(__wrap_smart_response_build, response);                                              \
  will_return(__wrap_smart_response_build, 0)

/**
 * @brief Set up expectations for smart response build failure
 * @param error_code Error code to return
 */
#define EXPECT_SMART_RESPONSE_BUILD_ERROR(error_code)                                              \
  expect_any(__wrap_smart_response_build, soap);                                                  \
  expect_any(__wrap_smart_response_build, response_func);                                         \
  expect_any(__wrap_smart_response_build, response);                                              \
  will_return(__wrap_smart_response_build, error_code)

/**
 * @brief Set up expectations for smart response estimate size
 * @param size Size to return
 */
#define EXPECT_SMART_RESPONSE_ESTIMATE_SIZE(size)                                                  \
  expect_any(__wrap_smart_response_estimate_size, soap);                                          \
  expect_any(__wrap_smart_response_estimate_size, response_func);                                 \
  expect_any(__wrap_smart_response_estimate_size, response);                                      \
  will_return(__wrap_smart_response_estimate_size, size)

#ifdef __cplusplus
}
#endif

#endif // SMART_RESPONSE_MOCK_H
