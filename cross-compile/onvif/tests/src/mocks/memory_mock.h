/**
 * @file memory_mock.h
 * @brief Mock functions for memory management operations
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MEMORY_MOCK_H
#define MEMORY_MOCK_H

#include <stddef.h>

/* ============================================================================
 * Memory Mock State Management
 * ============================================================================ */

/**
 * @brief Initialize memory mock
 */
void memory_mock_init(void);

/**
 * @brief Cleanup memory mock
 */
void memory_mock_cleanup(void);

/**
 * @brief Reset memory mock state
 */
void memory_mock_reset(void);

/* ============================================================================
 * Memory Allocation Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock malloc result
 * @param result Pointer to return (or NULL for failure)
 */
void memory_mock_set_malloc_result(void* result);

/**
 * @brief Set mock calloc result
 * @param result Pointer to return (or NULL for failure)
 */
void memory_mock_set_calloc_result(void* result);

/**
 * @brief Set mock realloc result
 * @param result Pointer to return (or NULL for failure)
 */
void memory_mock_set_realloc_result(void* result);

/**
 * @brief Get malloc call count
 * @return Number of times malloc was called
 */
size_t memory_mock_get_malloc_call_count(void);

/**
 * @brief Get calloc call count
 * @return Number of times calloc was called
 */
size_t memory_mock_get_calloc_call_count(void);

/**
 * @brief Get realloc call count
 * @return Number of times realloc was called
 */
size_t memory_mock_get_realloc_call_count(void);

/**
 * @brief Get free call count
 * @return Number of times free was called
 */
size_t memory_mock_get_free_call_count(void);

/* ============================================================================
 * Memory Tracking Mock Functions
 * ============================================================================ */

/**
 * @brief Get total allocated memory
 * @return Total bytes allocated
 */
size_t memory_mock_get_total_allocated(void);

/**
 * @brief Get total freed memory
 * @return Total bytes freed
 */
size_t memory_mock_get_total_freed(void);

/**
 * @brief Get current memory usage
 * @return Current allocated bytes
 */
size_t memory_mock_get_current_usage(void);

/**
 * @brief Check for memory leaks
 * @return 1 if leaks detected, 0 if no leaks
 */
int memory_mock_has_leaks(void);

/* ============================================================================
 * Memory Error Simulation
 * ============================================================================ */

/**
 * @brief Enable memory allocation failure
 * @param fail_after Number of successful allocations before failure
 */
void memory_mock_enable_allocation_failure(size_t fail_after);

/**
 * @brief Disable memory allocation failure
 */
void memory_mock_disable_allocation_failure(void);

/**
 * @brief Check if allocation failure is enabled
 * @return 1 if enabled, 0 if disabled
 */
int memory_mock_is_allocation_failure_enabled(void);

#endif // MEMORY_MOCK_H
