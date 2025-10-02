/**
 * @file media_service_tests.h
 * @brief Integration tests header for optimized ONVIF media service
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MEDIA_SERVICE_TESTS_H
#define MEDIA_SERVICE_TESTS_H

#include "cmocka_wrapper.h"

// Test function declarations
void test_integration_media_profile_operations(void** state);
void test_integration_media_stream_uri_generation_functionality(void** state);
void test_integration_optimized_profile_lookup_performance(void** state);
void test_integration_uri_caching_optimization(void** state);
void test_integration_media_memory_efficiency(void** state);
void test_integration_concurrent_stream_uri_access(void** state);
void test_integration_stress_test_optimization(void** state);
void test_integration_media_platform_integration(void** state);

// New test function declarations for enhanced coverage
void test_integration_media_delete_profile_operation(void** state);
void test_integration_media_error_invalid_profile_token(void** state);
void test_integration_media_concurrent_profile_operations(void** state);
void test_integration_media_request_response_validation(void** state);

// Test suite
extern const struct CMUnitTest media_service_optimization_tests[];

#endif // MEDIA_SERVICE_TESTS_H
