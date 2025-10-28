/**
 * @file test_config_auth.h
 * @brief Unit tests for authentication configuration functionality
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_CONFIG_AUTH_H
#define TEST_CONFIG_AUTH_H

#include <stddef.h>

#include "cmocka_wrapper.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Test Function Declarations
 * ============================================================================ */

/**
 * @brief Test that auth_enabled field exists in onvif_settings structure
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_field_exists(void** state);

/**
 * @brief Test that auth_enabled field is properly positioned in structure
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_field_position(void** state);

/**
 * @brief Test that auth_enabled parameter is properly defined in config.c
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_parameter_definition(void** state);

/**
 * @brief Test auth_enabled parameter validation
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_parameter_validation(void** state);

/**
 * @brief Test loading auth_enabled from INI file
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_ini_loading(void** state);

/**
 * @brief Test default value for auth_enabled when not specified in INI
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_default_value(void** state);

/**
 * @brief Test that auth_enabled appears in configuration summary
 * @param state Test state (unused)
 */
void test_unit_config_auth_enabled_summary(void** state);

/* ============================================================================
 * Test Suite Access
 * ============================================================================ */

/**
 * @brief Get config auth unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_config_auth_unit_tests(size_t* count);

#ifdef __cplusplus
}
#endif

#endif // TEST_CONFIG_AUTH_H
