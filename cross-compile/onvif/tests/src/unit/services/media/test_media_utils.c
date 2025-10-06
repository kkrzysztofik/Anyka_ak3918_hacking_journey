/**
 * @file test_media_utils.c
 * @brief Unit tests for ONVIF media service utility functions exercising real implementations
 *        with dependency-level mocks only where necessary.
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/media/onvif_media.h"
#include "services/media/onvif_media_unit_test.h"
#include "utils/error/error_handling.h"

// Test constants derived from media service defaults
#define TEST_MEDIA_PROFILE_COUNT        2
#define TEST_VIDEO_SOURCE_COUNT         1
#define TEST_AUDIO_SOURCE_COUNT         1
#define TEST_VIDEO_ENCODER_CONFIG_COUNT 2
#define TEST_AUDIO_ENCODER_CONFIG_COUNT 3
#define TEST_METADATA_CONFIG_COUNT      1

static void assert_string_prefix(const char* value, const char* prefix) {
  assert_non_null(value);
  assert_non_null(prefix);
  assert_true(strncmp(value, prefix, strlen(prefix)) == 0);
}

void test_unit_media_profile_functions(void** state) {
  (void)state;

  struct media_profile* profiles = NULL;
  int count = 0;
  int result = onvif_media_get_profiles(&profiles, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(profiles);
  assert_int_equal(TEST_MEDIA_PROFILE_COUNT, count);
  assert_string_equal("MainProfile", profiles[0].token);
  assert_string_equal("SubProfile", profiles[1].token);

  struct media_profile profile;
  memset(&profile, 0, sizeof(profile));
  result = onvif_media_get_profile("MainProfile", &profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("MainProfile", profile.token);
  assert_string_equal("Main Video Profile", profile.name);

  result = onvif_media_get_profile("NonExistentProfile", &profile);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  struct media_profile custom_profile;
  memset(&custom_profile, 0, sizeof(custom_profile));
  result = onvif_media_create_profile("New Profile", "CustomProfile", &custom_profile);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_equal("CustomProfile", custom_profile.token);
  assert_string_equal("New Profile", custom_profile.name);
  assert_int_equal(0, custom_profile.fixed);

  result = onvif_media_create_profile("Invalid", "OtherProfile", &custom_profile);
  assert_int_equal(ONVIF_ERROR_NOT_SUPPORTED, result);

  result = onvif_media_delete_profile("MainProfile");
  assert_int_equal(ONVIF_ERROR_NOT_SUPPORTED, result);
  result = onvif_media_delete_profile("CustomProfile");
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_video_source_functions(void** state) {
  (void)state;

  struct video_source* video_sources = NULL;
  int count = 0;
  int result = onvif_media_get_video_sources(&video_sources, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(video_sources);
  assert_int_equal(TEST_VIDEO_SOURCE_COUNT, count);
  assert_string_equal("VideoSource0", video_sources[0].token);
  assert_int_equal(1280, video_sources[0].resolution.width);
  assert_int_equal(720, video_sources[0].resolution.height);
}

void test_unit_media_audio_source_functions(void** state) {
  (void)state;

  struct audio_source* audio_sources = NULL;
  int count = 0;
  int result = onvif_media_get_audio_sources(&audio_sources, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(audio_sources);
  assert_int_equal(TEST_AUDIO_SOURCE_COUNT, count);
  assert_string_equal("AudioSource0", audio_sources[0].token);
}

void test_unit_media_video_configuration_functions(void** state) {
  (void)state;

  struct video_encoder_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_video_encoder_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_VIDEO_ENCODER_CONFIG_COUNT, count);
  assert_string_equal("VideoEncoder0", configs[0].token);
  assert_string_equal("VideoEncoder1", configs[1].token);

  struct video_source_configuration source_cfg;
  memset(&source_cfg, 0, sizeof(source_cfg));
  source_cfg.bounds.width = 640;
  source_cfg.bounds.height = 360;
  source_cfg.bounds.x = 0;
  source_cfg.bounds.y = 0;
  result = onvif_media_set_video_source_configuration("VideoSourceConfig0", &source_cfg);
  assert_int_equal(ONVIF_SUCCESS, result);
  result = onvif_media_set_video_source_configuration("UnknownConfig", &source_cfg);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  struct video_encoder_configuration encoder_cfg = configs[0];
  encoder_cfg.bitrate_limit = 1024;
  result = onvif_media_set_video_encoder_configuration("VideoEncoder0", &encoder_cfg);
  assert_int_equal(ONVIF_SUCCESS, result);
  result = onvif_media_set_video_encoder_configuration("UnknownEncoder", &encoder_cfg);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_audio_configuration_functions(void** state) {
  (void)state;

  struct audio_encoder_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_audio_encoder_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_AUDIO_ENCODER_CONFIG_COUNT, count);
  assert_string_equal("AudioEncoder0", configs[0].token);

  struct audio_source_configuration source_cfg;
  memset(&source_cfg, 0, sizeof(source_cfg));
  strncpy(source_cfg.source_token, "AudioSource0", sizeof(source_cfg.source_token) - 1);
  result = onvif_media_set_audio_source_configuration("AudioSourceConfig0", &source_cfg);
  assert_int_equal(ONVIF_SUCCESS, result);
  result = onvif_media_set_audio_source_configuration("UnknownConfig", &source_cfg);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  struct audio_encoder_configuration encoder_cfg = configs[0];
  encoder_cfg.bitrate = 64;
  encoder_cfg.sample_rate = 16000;
  result = onvif_media_set_audio_encoder_configuration("AudioEncoder0", &encoder_cfg);
  assert_int_equal(ONVIF_SUCCESS, result);

  encoder_cfg.bitrate = 16; // below AAC minimum bitrate
  result = onvif_media_set_audio_encoder_configuration("AudioEncoder0", &encoder_cfg);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

void test_unit_media_stream_uri_functions(void** state) {
  (void)state;

  struct stream_uri stream_uri;
  memset(&stream_uri, 0, sizeof(stream_uri));
  onvif_media_unit_reset_cached_uris();

  int result = onvif_media_get_stream_uri("MainProfile", "RTSP", &stream_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_prefix(stream_uri.uri, "rtsp://");
  assert_false(stream_uri.invalid_after_connect);
  assert_false(stream_uri.invalid_after_reboot);

  result = onvif_media_get_stream_uri("MainProfile", "HTTP", &stream_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_prefix(stream_uri.uri, "http://");

  result = onvif_media_get_stream_uri("UnknownProfile", "RTSP", &stream_uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);

  result = onvif_media_get_stream_uri("MainProfile", "SRT", &stream_uri);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_snapshot_uri_functions(void** state) {
  (void)state;

  struct stream_uri snapshot_uri;
  int result = onvif_media_get_snapshot_uri("MainProfile", &snapshot_uri);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_string_prefix(snapshot_uri.uri, "http://");
  assert_false(snapshot_uri.invalid_after_connect);
  assert_false(snapshot_uri.invalid_after_reboot);

  result = onvif_media_get_snapshot_uri(NULL, &snapshot_uri);
  assert_int_equal(ONVIF_ERROR_NULL, result);
}

void test_unit_media_multicast_functions(void** state) {
  (void)state;

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
  (void)state;

  struct metadata_configuration* configs = NULL;
  int count = 0;
  int result = onvif_media_get_metadata_configurations(&configs, &count);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_non_null(configs);
  assert_int_equal(TEST_METADATA_CONFIG_COUNT, count);
  assert_string_equal("MetadataConfig0", configs[0].token);

  struct metadata_configuration new_config;
  memset(&new_config, 0, sizeof(new_config));
  new_config.analytics = 1;
  new_config.session_timeout = 60;
  result = onvif_media_set_metadata_configuration("MetadataConfig0", &new_config);
  assert_int_equal(ONVIF_SUCCESS, result);

  result = onvif_media_set_metadata_configuration("NonExistentConfig", &new_config);
  assert_int_equal(ONVIF_ERROR_NOT_FOUND, result);
}

void test_unit_media_cached_uri_initialization(void** state) {
  (void)state;

  char uri_buffer[MEDIA_URI_BUFFER_SIZE];
  const char* main_token = onvif_media_unit_get_main_profile_token();

  onvif_media_unit_reset_cached_uris();

  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_init_cached_uris());
  assert_int_equal(
    ONVIF_SUCCESS,
    onvif_media_unit_get_cached_rtsp_uri(main_token, uri_buffer, sizeof(uri_buffer)));
  assert_string_prefix(uri_buffer, "rtsp://");

  // Second init should be idempotent
  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_init_cached_uris());
}

void test_unit_media_cached_uri_error_paths(void** state) {
  (void)state;

  char uri_buffer[MEDIA_URI_BUFFER_SIZE];
  const char* main_token = onvif_media_unit_get_main_profile_token();

  onvif_media_unit_reset_cached_uris();
  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_init_cached_uris());

  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_get_cached_rtsp_uri(NULL, uri_buffer, sizeof(uri_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_get_cached_rtsp_uri(main_token, NULL, sizeof(uri_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_get_cached_rtsp_uri(main_token, uri_buffer, 0));
  assert_int_equal(ONVIF_ERROR_NOT_FOUND,
                   onvif_media_unit_get_cached_rtsp_uri("UnknownProfile", uri_buffer,
                                                        sizeof(uri_buffer)));
}

void test_unit_media_validate_profile_token(void** state) {
  (void)state;

  const char* main_token = onvif_media_unit_get_main_profile_token();
  const char* sub_token = onvif_media_unit_get_sub_profile_token();

  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_validate_profile_token(main_token));
  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_validate_profile_token(sub_token));
  assert_int_equal(ONVIF_ERROR_INVALID, onvif_media_unit_validate_profile_token(NULL));
  assert_int_equal(ONVIF_ERROR_NOT_FOUND,
                   onvif_media_unit_validate_profile_token("UnknownProfile"));
}

void test_unit_media_validate_protocol(void** state) {
  (void)state;

  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_validate_protocol("RTSP"));
  assert_int_equal(ONVIF_SUCCESS, onvif_media_unit_validate_protocol("RTP-Unicast"));
  assert_int_equal(ONVIF_ERROR_INVALID, onvif_media_unit_validate_protocol(NULL));
  assert_int_equal(ONVIF_ERROR_NOT_SUPPORTED,
                   onvif_media_unit_validate_protocol("HTTP"));
}

void test_unit_media_parse_value_from_request(void** state) {
  (void)state;

  onvif_gsoap_context_t gsoap_ctx;
  memset(&gsoap_ctx, 0, sizeof(gsoap_ctx));

  char value_buffer[32];

  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_parse_value_from_request(NULL, &gsoap_ctx, "/xpath", value_buffer,
                                                             sizeof(value_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_parse_value_from_request("<xml>", NULL, "/xpath", value_buffer,
                                                             sizeof(value_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_parse_value_from_request("<xml>", &gsoap_ctx, NULL,
                                                             value_buffer, sizeof(value_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_parse_value_from_request("<xml>", &gsoap_ctx, "/xpath", NULL,
                                                             sizeof(value_buffer)));
  assert_int_equal(ONVIF_ERROR_INVALID,
                   onvif_media_unit_parse_value_from_request("<xml>", &gsoap_ctx, "/xpath",
                                                             value_buffer, 0));

  assert_int_equal(ONVIF_SUCCESS,
                   onvif_media_unit_parse_value_from_request("<xml>", &gsoap_ctx, "/xpath",
                                                             value_buffer, sizeof(value_buffer)));
  assert_string_equal("", value_buffer);
}

void test_unit_media_error_handling(void** state) {
  (void)state;

  int result = onvif_media_get_profiles(NULL, NULL);
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

// Additional test functions expected by legacy runners
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

static const struct CMUnitTest media_utils_tests[] = {
  cmocka_unit_test(test_unit_media_profile_functions),
  cmocka_unit_test(test_unit_media_video_source_functions),
  cmocka_unit_test(test_unit_media_audio_source_functions),
  cmocka_unit_test(test_unit_media_video_configuration_functions),
  cmocka_unit_test(test_unit_media_audio_configuration_functions),
  cmocka_unit_test(test_unit_media_stream_uri_functions),
  cmocka_unit_test(test_unit_media_snapshot_uri_functions),
  cmocka_unit_test(test_unit_media_multicast_functions),
  cmocka_unit_test(test_unit_media_metadata_functions),
  cmocka_unit_test(test_unit_media_cached_uri_initialization),
  cmocka_unit_test(test_unit_media_cached_uri_error_paths),
  cmocka_unit_test(test_unit_media_validate_profile_token),
  cmocka_unit_test(test_unit_media_validate_protocol),
  cmocka_unit_test(test_unit_media_parse_value_from_request),
  cmocka_unit_test(test_unit_media_error_handling),
};

const struct CMUnitTest* get_media_utils_unit_tests(size_t* count) {
  *count = sizeof(media_utils_tests) / sizeof(media_utils_tests[0]);
  return media_utils_tests;
}
