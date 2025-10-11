/**
 * @file test_config_integration.h
 * @brief Config integration tests header
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#ifndef TEST_CONFIG_INTEGRATION_H
#define TEST_CONFIG_INTEGRATION_H

#include <stddef.h>

#include "cmocka_wrapper.h"


/**
 * @brief Get config integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_integration_tests(size_t* count);

#endif /* TEST_CONFIG_INTEGRATION_H */
