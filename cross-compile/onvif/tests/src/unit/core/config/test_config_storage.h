/**
 * @file test_config_storage.h
 * @brief Config storage unit tests header
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#ifndef TEST_CONFIG_STORAGE_H
#define TEST_CONFIG_STORAGE_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/**
 * @brief Get config_storage unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_storage_unit_tests(size_t* count);

#endif /* TEST_CONFIG_STORAGE_H */
