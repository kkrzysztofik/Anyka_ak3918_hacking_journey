/**
 * @file test_string_utils.h
 * @brief Header for string utility unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_STRING_UTILS_H
#define TEST_STRING_UTILS_H

#include <cmocka.h>
#include <stddef.h>

/**
 * @brief Test string shim functions
 * @param state Test state
 */
void test_unit_string_shims(void** state);

/**
 * @brief Test string validation functions
 * @param state Test state
 */
void test_unit_string_validation(void** state);

/**
 * @brief Test string manipulation functions
 * @param state Test state
 */
void test_unit_string_manipulation(void** state);

/**
 * @brief Test string search and comparison functions
 * @param state Test state
 */
void test_unit_string_search(void** state);

/**
 * @brief Test string formatting functions
 * @param state Test state
 */
void test_unit_string_formatting(void** state);

#endif // TEST_STRING_UTILS_H
