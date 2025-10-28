/**
 * @file test_validation_utils.h
 * @brief Header for validation utility unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_VALIDATION_UTILS_H
#define TEST_VALIDATION_UTILS_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/**
 * @brief Test common validation functions
 * @param state Test state
 */
void test_unit_common_validation(void** state);

/**
 * @brief Test input validation functions
 * @param state Test state
 */
void test_input_validation(void** state);

/**
 * @brief Test audio validation functions
 * @param state Test state
 */
void test_unit_audio_validation(void** state);

#endif // TEST_VALIDATION_UTILS_H
