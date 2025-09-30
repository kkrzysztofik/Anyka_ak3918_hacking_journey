/**
 * @file imaging_service_optimization_tests.h
 * @brief Integration tests header for imaging service optimization features
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef IMAGING_SERVICE_OPTIMIZATION_TESTS_H
#define IMAGING_SERVICE_OPTIMIZATION_TESTS_H

#include "cmocka_wrapper.h"

// Test function declarations
void test_integration_imaging_parameter_cache_efficiency(void** state);
void test_integration_imaging_bulk_settings_validation(void** state);
void test_integration_imaging_batch_parameter_update_optimization(void** state);
void test_integration_imaging_concurrent_access(void** state);
void test_integration_imaging_performance_regression(void** state);

// Setup and teardown function declarations
int setup_imaging_integration(void** state);
int teardown_imaging_integration(void** state);

// Test suite
extern const struct CMUnitTest imaging_service_optimization_tests[];

#endif // IMAGING_SERVICE_OPTIMIZATION_TESTS_H
