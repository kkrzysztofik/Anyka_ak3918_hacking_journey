/**
 * @file test_media_utils.c
 * @brief Unit tests for ONVIF media service utility functions
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"

// Test constants
#define TEST_MEDIA_PROFILE_COUNT        2
#define TEST_VIDEO_SOURCE_COUNT         1
#define TEST_AUDIO_SOURCE_COUNT         1
#define TEST_VIDEO_CONFIG_COUNT         1
#define TEST_VIDEO_ENCODER_CONFIG_COUNT 2
#define TEST_AUDIO_CONFIG_COUNT         1
#define TEST_AUDIO_ENCODER_CONFIG_COUNT 3
#define TEST_METADATA_CONFIG_COUNT      1

// Test functions
void test_unit_media_profile_functions(void** state) {
  (void)state; // Unused parameter

  struct media_profile* profiles = NULL;
  int count = 0;
  int result = onvif_media_get_profiles(&profiles, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles);
  assert_int_equal(TEST_MEDIA_PROFILE_COUNT, count);

  struct media_profile profile;
  result = onvif_media_get_profile("MainProfile", &profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("MainProfile", profile.token);

  result = onvif_media_get_profile("NonExistentProfile", &profile);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  struct media_profile new_profile;
  result = onvif_media_create_profile("New Profile", "CustomProfile", &new_profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("New Profile", new_profile.name);

  result = onvif_media_delete_profile("CustomProfile");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_video_source_functions(void** state) {
  (void)state; // Unused parameter

  struct video_source* video_sources = NULL;
  int count = 0;
  int result = onvif_media_get_video_sources(&video_sources, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(video_sources);
  assert_int_equal(TEST_VIDEO_SOURCE_COUNT, count);
}

void test_unit_media_audio_source_functions(void** state) {
  (void)state; // Unused parameter

  struct audio_source* audio_sources = NULL;
  int count = 0;
  int result = onvif_media_get_audio_sources(&audio_sources, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(audio_sources);
  assert_int_equal(TEST_AUDIO_SOURCE_COUNT, count);
}

void test_unit_media_video_configuration_functions(void** state) {
  (void)state; // Unused parameter

  struct video_encoder_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_video_encoder_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_VIDEO_ENCODER_CONFIG_COUNT, count);
}

void test_unit_media_audio_configuration_functions(void** state) {
  (void)state; // Unused parameter

  struct audio_encoder_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_audio_encoder_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_AUDIO_ENCODER_CONFIG_COUNT, count);
}

void test_unit_media_stream_uri_functions(void** state) {
  (void)state; // Unused parameter

  struct stream_uri stream_uri;
  int result = onvif_media_get_stream_uri("MainProfile", "RTSP", &stream_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("rtsp://192.168.1.10:554/main", stream_uri.uri);
  assert_false(stream_uri.invalid_after_connect);
  assert_false(stream_uri.invalid_after_reboot);

  result = onvif_media_get_stream_uri("NonExistentProfile", "RTSP", &stream_uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_snapshot_uri_functions(void** state) {
  (void)state; // Unused parameter

  struct stream_uri snapshot_uri;
  int result = onvif_media_get_snapshot_uri("MainProfile", &snapshot_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("http://192.168.1.10:80/snapshot", snapshot_uri.uri);
  assert_false(snapshot_uri.invalid_after_connect);
  assert_false(snapshot_uri.invalid_after_reboot);

  result = onvif_media_get_snapshot_uri("NonExistentProfile", &snapshot_uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_multicast_functions(void** state) {
  (void)state; // Unused parameter

  int result = onvif_media_start_multicast_streaming("MainProfile");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = onvif_media_stop_multicast_streaming("MainProfile");
  assert_int_equal(ONVIF_SUCCESS, result);

  result = onvif_media_start_multicast_streaming("NonExistentProfile");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  result = onvif_media_stop_multicast_streaming("NonExistentProfile");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_metadata_functions(void** state) {
  (void)state; // Unused parameter

  struct metadata_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_metadata_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_METADATA_CONFIG_COUNT, count);

  struct metadata_configuration new_config = {.name = "New Metadata", .use_count = 1};
  result = onvif_media_set_metadata_configuration("MetadataConfig0", &new_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = onvif_media_set_metadata_configuration("NonExistentConfig", &new_config);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_error_handling(void** state) {
  (void)state; // Unused parameter

  int result;

  // Test null pointer handling
  result = onvif_media_get_profiles(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_profile(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_create_profile(NULL, NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_delete_profile(NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_video_sources(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_audio_sources(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_video_encoder_configurations(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_audio_encoder_configurations(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_stream_uri(NULL, NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_snapshot_uri(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_start_multicast_streaming(NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_stop_multicast_streaming(NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_get_metadata_configurations(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
  result = onvif_media_set_metadata_configuration(NULL, NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
}

void test_unit_media_initialization(void** state) {
  (void)state; // Unused parameter

  config_manager_t mock_config;
  int result = onvif_media_init(&mock_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = onvif_media_init(NULL);
  assert_int_equal(ONVIF_ERROR_NULL, result);
}

// Additional test functions expected by test_runner.c
void test_unit_media_basic_functions(void** state) {
  test_unit_media_profile_functions(state);
}

void test_unit_media_video_sources(void** state) {
  test_unit_media_video_source_functions(state);
}

void test_unit_media_audio_sources(void** state) {
  test_unit_media_audio_source_functions(state);
}

void test_unit_media_video_configurations(void** state) {
  test_unit_media_video_configuration_functions(state);
}

void test_unit_media_audio_configurations(void** state) {
  test_unit_media_audio_configuration_functions(state);
}

void test_unit_media_metadata_configurations(void** state) {
  test_unit_media_metadata_functions(state);
}
