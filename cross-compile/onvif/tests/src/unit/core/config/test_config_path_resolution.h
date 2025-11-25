/**
 * @file test_config_path_resolution.h
 * @brief Unit tests for config path resolution
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_CONFIG_PATH_RESOLUTION_H
#define TEST_CONFIG_PATH_RESOLUTION_H

/* Test function declarations */
void test_unit_platform_get_executable_path_success(void** state);
void test_unit_platform_get_executable_path_null_buffer(void** state);
void test_unit_platform_get_executable_path_zero_size(void** state);
void test_unit_platform_get_executable_path_small_buffer(void** state);
void test_unit_config_path_resolution_integration_load(void** state);
void test_unit_config_path_resolution_fallback_behavior(void** state);

#endif /* TEST_CONFIG_PATH_RESOLUTION_H */
