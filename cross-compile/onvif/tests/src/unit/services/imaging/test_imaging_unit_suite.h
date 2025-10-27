/**
 * @file test_imaging_unit_suite.h
 * @brief Imaging service unit test suite registration
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_IMAGING_UNIT_SUITE_H
#define TEST_IMAGING_UNIT_SUITE_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/**
 * @brief Get imaging service unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_imaging_service_unit_tests(size_t* count);

#endif /* TEST_IMAGING_UNIT_SUITE_H */
