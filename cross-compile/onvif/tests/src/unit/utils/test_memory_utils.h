/**
 * @file test_memory_utils.h
 * @brief Header for memory utility unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_MEMORY_UTILS_H
#define TEST_MEMORY_UTILS_H

#include <cmocka.h>
#include <stddef.h>

/**
 * @brief Test memory manager initialization
 * @param state Test state
 */
void test_memory_manager_init(void** state);

/**
 * @brief Test memory allocation functionality
 * @param state Test state
 */
void test_memory_manager_alloc(void** state);

/**
 * @brief Test memory deallocation functionality
 * @param state Test state
 */
void test_memory_manager_free(void** state);

/**
 * @brief Test smart response builder functionality
 * @param state Test state
 */
void test_smart_response_builder(void** state);

/**
 * @brief Test memory manager statistics
 * @param state Test state
 */
void test_memory_manager_stats(void** state);

/**
 * @brief Test memory manager with stress allocation
 * @param state Test state
 */
void test_memory_manager_stress(void** state);

#endif // TEST_MEMORY_UTILS_H
