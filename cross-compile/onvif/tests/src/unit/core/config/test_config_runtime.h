/**
 * @file test_config_runtime.h
 * @brief Config runtime unit tests header
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#ifndef TEST_CONFIG_RUNTIME_H
#define TEST_CONFIG_RUNTIME_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/**
 * @brief Get config_runtime unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_runtime_unit_tests(size_t* count);

#endif /* TEST_CONFIG_RUNTIME_H */
