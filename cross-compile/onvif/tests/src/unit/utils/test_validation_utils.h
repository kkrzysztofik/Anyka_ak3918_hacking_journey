/**
 * @file test_validation_utils.h
 * @brief Header for validation utility unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_VALIDATION_UTILS_H
#define TEST_VALIDATION_UTILS_H

#include <cmocka.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

/**
 * @brief Test common validation functions
 * @param state Test state
 */
void test_common_validation(void** state);

/**
 * @brief Test input validation functions
 * @param state Test state
 */
void test_input_validation(void** state);

/**
 * @brief Test audio validation functions
 * @param state Test state
 */
void test_audio_validation(void** state);

#endif // TEST_VALIDATION_UTILS_H
