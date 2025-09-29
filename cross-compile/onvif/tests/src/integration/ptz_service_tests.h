/**
 * @file ptz_service_tests.h
 * @brief Integration tests header for optimized ONVIF PTZ service
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PTZ_SERVICE_TESTS_H
#define PTZ_SERVICE_TESTS_H

#include "cmocka_wrapper.h"

// Test function declarations for PTZ movement operations
void test_integration_ptz_absolute_move_functionality(void** state);
void test_integration_ptz_relative_move_functionality(void** state);
void test_integration_ptz_continuous_move_functionality(void** state);
void test_integration_ptz_stop_functionality(void** state);

// Test function declarations for PTZ preset management
void test_integration_ptz_preset_creation(void** state);
void test_integration_ptz_preset_retrieval(void** state);
void test_integration_ptz_preset_goto(void** state);
void test_integration_ptz_preset_removal(void** state);
void test_integration_ptz_preset_memory_optimization(void** state);

// Test function declarations for PTZ service optimization validation
void test_integration_ptz_memory_usage_improvements(void** state);
void test_integration_ptz_buffer_pool_usage(void** state);
void test_integration_ptz_string_operations_optimization(void** state);
void test_integration_ptz_error_handling_robustness(void** state);

// Test function declarations for PTZ service performance
void test_integration_ptz_concurrent_operations(void** state);
void test_integration_ptz_stress_testing(void** state);
void test_integration_ptz_memory_leak_detection(void** state);

// Test suite
extern const struct CMUnitTest ptz_service_optimization_tests[];

#endif // PTZ_SERVICE_TESTS_H
