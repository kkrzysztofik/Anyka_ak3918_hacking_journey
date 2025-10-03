/**
 * @file memory_mock_cmocka.h
 * @brief CMocka-based memory allocation mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MEMORY_MOCK_CMOCKA_H
#define MEMORY_MOCK_CMOCKA_H

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * CMocka Wrapped Memory Functions
 * ============================================================================
 * All memory functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped malloc
 * @param size Size to allocate
 * @return Pointer to allocated memory (configured via will_return)
 */
void* __wrap_malloc(size_t size);

/**
 * @brief CMocka wrapped calloc
 * @param nmemb Number of elements
 * @param size Size of each element
 * @return Pointer to allocated memory (configured via will_return)
 */
void* __wrap_calloc(size_t nmemb, size_t size);

/**
 * @brief CMocka wrapped realloc
 * @param ptr Pointer to reallocate
 * @param size New size
 * @return Pointer to reallocated memory (configured via will_return)
 */
void* __wrap_realloc(void* ptr, size_t size);

/**
 * @brief CMocka wrapped free
 * @param ptr Pointer to free
 */
void __wrap_free(void* ptr);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful malloc
 * @param size Expected size to allocate
 * @param ptr Pointer to return
 */
#define EXPECT_MALLOC_SUCCESS(size, ptr)                                                           \
  expect_value(__wrap_malloc, size, size);                                                         \
  will_return(__wrap_malloc, ptr)

/**
 * @brief Set up expectations for malloc failure
 * @param size Expected size to allocate
 */
#define EXPECT_MALLOC_FAIL(size)                                                                   \
  expect_value(__wrap_malloc, size, size);                                                         \
  will_return(__wrap_malloc, NULL)

/**
 * @brief Set up expectations for malloc with any size
 * @param ptr Pointer to return
 */
#define EXPECT_MALLOC_ANY(ptr)                                                                     \
  expect_any(__wrap_malloc, size);                                                                 \
  will_return(__wrap_malloc, ptr)

/**
 * @brief Set up expectations for successful calloc
 * @param nmemb Expected number of elements
 * @param size Expected size of each element
 * @param ptr Pointer to return
 */
#define EXPECT_CALLOC_SUCCESS(nmemb, size, ptr)                                                    \
  expect_value(__wrap_calloc, nmemb, nmemb);                                                       \
  expect_value(__wrap_calloc, size, size);                                                         \
  will_return(__wrap_calloc, ptr)

/**
 * @brief Set up expectations for calloc failure
 * @param nmemb Expected number of elements
 * @param size Expected size of each element
 */
#define EXPECT_CALLOC_FAIL(nmemb, size)                                                            \
  expect_value(__wrap_calloc, nmemb, nmemb);                                                       \
  expect_value(__wrap_calloc, size, size);                                                         \
  will_return(__wrap_calloc, NULL)

/**
 * @brief Set up expectations for successful realloc
 * @param orig_ptr Expected original pointer
 * @param size Expected new size
 * @param new_ptr Pointer to return
 */
#define EXPECT_REALLOC_SUCCESS(orig_ptr, size, new_ptr)                                            \
  expect_value(__wrap_realloc, ptr, orig_ptr);                                                     \
  expect_value(__wrap_realloc, size, size);                                                        \
  will_return(__wrap_realloc, new_ptr)

/**
 * @brief Set up expectations for realloc failure
 * @param orig_ptr Expected original pointer
 * @param size Expected new size
 */
#define EXPECT_REALLOC_FAIL(orig_ptr, size)                                                        \
  expect_value(__wrap_realloc, ptr, orig_ptr);                                                     \
  expect_value(__wrap_realloc, size, size);                                                        \
  will_return(__wrap_realloc, NULL)

/**
 * @brief Set up expectations for free
 * @param ptr Expected pointer to free
 */
#define EXPECT_FREE(ptr)                                                                           \
  expect_value(__wrap_free, ptr, ptr)

/**
 * @brief Set up expectations for free with any pointer
 */
#define EXPECT_FREE_ANY()                                                                          \
  expect_any(__wrap_free, ptr)

#ifdef __cplusplus
}
#endif

#endif // MEMORY_MOCK_CMOCKA_H
