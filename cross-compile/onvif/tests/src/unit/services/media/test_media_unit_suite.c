/**
 * @file test_media_unit_suite.c
 * @brief Media service unit test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Forward declarations from test_media_utils.c
void test_unit_media_profile_functions(void** state);
void test_unit_media_video_source_functions(void** state);
void test_unit_media_audio_source_functions(void** state);
void test_unit_media_video_configuration_functions(void** state);
void test_unit_media_audio_configuration_functions(void** state);
void test_unit_media_stream_uri_functions(void** state);
void test_unit_media_snapshot_uri_functions(void** state);
void test_unit_media_multicast_functions(void** state);
void test_unit_media_metadata_functions(void** state);

/**
 * @brief Get media utils unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_media_utils_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_media_profile_functions),
    cmocka_unit_test(test_unit_media_video_source_functions),
    cmocka_unit_test(test_unit_media_audio_source_functions),
    cmocka_unit_test(test_unit_media_video_configuration_functions),
    cmocka_unit_test(test_unit_media_audio_configuration_functions),
    cmocka_unit_test(test_unit_media_stream_uri_functions),
    cmocka_unit_test(test_unit_media_snapshot_uri_functions),
    cmocka_unit_test(test_unit_media_multicast_functions),
    cmocka_unit_test(test_unit_media_metadata_functions),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
