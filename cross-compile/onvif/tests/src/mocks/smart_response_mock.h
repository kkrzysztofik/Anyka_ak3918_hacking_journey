/**
 * @file smart_response_mock.h
 * @brief CMocka-based smart response builder mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SMART_RESPONSE_MOCK_H
#define SMART_RESPONSE_MOCK_H

#include <stdbool.h>
#include <stddef.h>

#include "cmocka_wrapper.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Forward declarations */
struct soap;

/* ============================================================================
 * CMocka Wrapped Smart Response Functions
 * ============================================================================
 * All smart response functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped smart response build with dynamic buffer
 * @param response HTTP response structure
 * @param soap_content SOAP content string
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build_with_dynamic_buffer(http_response_t* response, const char* soap_content);

/**
 * @brief CMocka wrapped smart response build with buffer pool
 * @param response HTTP response structure
 * @param soap_content SOAP content string
 * @param buffer_pool Buffer pool pointer
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build_with_buffer_pool(http_response_t* response, const char* soap_content, buffer_pool_t* buffer_pool);

/**
 * @brief CMocka wrapped smart response build
 * @param response HTTP response structure
 * @param soap_content SOAP content string
 * @param estimated_size Estimated response size
 * @param buffer_pool Buffer pool pointer
 * @return Result code (configured via will_return)
 */
int __wrap_smart_response_build(http_response_t* response, const char* soap_content, size_t estimated_size, buffer_pool_t* buffer_pool);

/**
 * @brief CMocka wrapped smart response estimate size
 * @param soap_content SOAP content string
 * @return Estimated size (configured via will_return)
 */
size_t __wrap_smart_response_estimate_size(const char* soap_content);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void smart_response_mock_use_real_function(bool use_real);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful smart response build with dynamic buffer
 */
#define EXPECT_SMART_RESPONSE_BUILD_DYNAMIC_SUCCESS()                                                                                                \
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, response);                                                                             \
  expect_any(__wrap_smart_response_build_with_dynamic_buffer, soap_content);                                                                         \
  will_return(__wrap_smart_response_build_with_dynamic_buffer, 0)

/**
 * @brief Set up expectations for successful smart response build with buffer pool
 */
#define EXPECT_SMART_RESPONSE_BUILD_POOL_SUCCESS()                                                                                                   \
  expect_any(__wrap_smart_response_build_with_buffer_pool, response);                                                                                \
  expect_any(__wrap_smart_response_build_with_buffer_pool, soap_content);                                                                            \
  expect_any(__wrap_smart_response_build_with_buffer_pool, buffer_pool);                                                                             \
  will_return(__wrap_smart_response_build_with_buffer_pool, 0)

/**
 * @brief Set up expectations for successful smart response build
 */
#define EXPECT_SMART_RESPONSE_BUILD_SUCCESS()                                                                                                        \
  expect_any(__wrap_smart_response_build, response);                                                                                                 \
  expect_any(__wrap_smart_response_build, soap_content);                                                                                             \
  expect_any(__wrap_smart_response_build, estimated_size);                                                                                           \
  expect_any(__wrap_smart_response_build, buffer_pool);                                                                                              \
  will_return(__wrap_smart_response_build, 0)

/**
 * @brief Set up expectations for smart response build failure
 * @param error_code Error code to return
 */
#define EXPECT_SMART_RESPONSE_BUILD_ERROR(error_code)                                                                                                \
  expect_any(__wrap_smart_response_build, response);                                                                                                 \
  expect_any(__wrap_smart_response_build, soap_content);                                                                                             \
  expect_any(__wrap_smart_response_build, estimated_size);                                                                                           \
  expect_any(__wrap_smart_response_build, buffer_pool);                                                                                              \
  will_return(__wrap_smart_response_build, error_code)

/**
 * @brief Set up expectations for smart response estimate size
 * @param size Size to return
 */
#define EXPECT_SMART_RESPONSE_ESTIMATE_SIZE(size)                                                                                                    \
  expect_any(__wrap_smart_response_estimate_size, soap_content);                                                                                     \
  will_return(__wrap_smart_response_estimate_size, size)

#ifdef __cplusplus
}
#endif

#endif // SMART_RESPONSE_MOCK_H
