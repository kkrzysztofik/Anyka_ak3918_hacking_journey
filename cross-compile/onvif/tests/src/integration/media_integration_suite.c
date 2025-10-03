/**
 * @file media_integration_suite.c
 * @brief Media integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from media_service_tests.c
void test_integration_optimized_profile_lookup_performance(void** state);
void test_integration_uri_caching_optimization(void** state);
void test_integration_media_memory_efficiency(void** state);
void test_integration_concurrent_stream_uri_access(void** state);
void test_integration_stress_test_optimization(void** state);
void test_integration_media_platform_integration(void** state);
void test_integration_media_error_invalid_profile_token(void** state);
void test_integration_media_concurrent_profile_operations(void** state);
void test_integration_media_request_response_validation(void** state);
void test_integration_media_get_profiles_soap(void** state);
void test_integration_media_get_stream_uri_soap(void** state);
void test_integration_media_delete_profile_soap(void** state);
void test_integration_media_set_video_source_config_soap(void** state);
void test_integration_media_set_video_encoder_config_soap(void** state);
void test_integration_media_get_metadata_configs_soap(void** state);
void test_integration_media_set_metadata_config_soap(void** state);
void test_integration_media_start_multicast_soap(void** state);
void test_integration_media_stop_multicast_soap(void** state);
void test_integration_media_create_profile_soap(void** state);

/**
 * @brief Get media integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_media_integration_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_integration_optimized_profile_lookup_performance),
    cmocka_unit_test(test_integration_uri_caching_optimization),
    cmocka_unit_test(test_integration_media_memory_efficiency),
    cmocka_unit_test(test_integration_concurrent_stream_uri_access),
    cmocka_unit_test(test_integration_stress_test_optimization),
    cmocka_unit_test(test_integration_media_platform_integration),
    cmocka_unit_test(test_integration_media_error_invalid_profile_token),
    cmocka_unit_test(test_integration_media_concurrent_profile_operations),
    cmocka_unit_test(test_integration_media_request_response_validation),
    cmocka_unit_test(test_integration_media_get_profiles_soap),
    cmocka_unit_test(test_integration_media_get_stream_uri_soap),
    cmocka_unit_test(test_integration_media_delete_profile_soap),
    cmocka_unit_test(test_integration_media_set_video_source_config_soap),
    cmocka_unit_test(test_integration_media_set_video_encoder_config_soap),
    cmocka_unit_test(test_integration_media_get_metadata_configs_soap),
    cmocka_unit_test(test_integration_media_set_metadata_config_soap),
    cmocka_unit_test(test_integration_media_start_multicast_soap),
    cmocka_unit_test(test_integration_media_stop_multicast_soap),
    cmocka_unit_test(test_integration_media_create_profile_soap),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
