/**
 * @file test_stream_utils.c
 * @brief Unit tests for stream configuration utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

#include <stddef.h>
#include <string.h>

#include "services/common/video_config_types.h"
// Include the actual source files we're testing
#include "utils/stream/stream_config_utils.h"

/**
 * @brief Test stream configuration initialization
 * @param state Test state (unused)
 */
static void test_stream_config_init(void** state) {
  (void)state;

  stream_config_t config;
  memset(&config, 0, sizeof(config));

  // Test stream configuration initialization
  int result = stream_config_init(&config);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL config
  result = stream_config_init(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup
  stream_config_cleanup(&config);
}

/**
 * @brief Test stream configuration cleanup
 * @param state Test state (unused)
 */
static void test_stream_config_cleanup(void** state) {
  (void)state;

  stream_config_t config;
  memset(&config, 0, sizeof(config));

  // Initialize first
  stream_config_init(&config);

  // Test cleanup (should not crash)
  stream_config_cleanup(&config);

  // Test cleanup with NULL config (should not crash)
  stream_config_cleanup(NULL);
}

/**
 * @brief Test video configuration setting
 * @param state Test state (unused)
 */
static void test_video_config_set(void** state) {
  (void)state;

  stream_config_t config;
  video_config_t video_cfg;

  memset(&config, 0, sizeof(config));
  memset(&video_cfg, 0, sizeof(video_cfg));

  stream_config_init(&config);

  // Set up valid video configuration
  video_cfg.width = 1920;
  video_cfg.height = 1080;
  video_cfg.framerate = 30;
  video_cfg.bitrate = 2000000;
  strcpy(video_cfg.encoding, "H264");

  // Test setting video configuration
  int result = stream_config_set_video(&config, &video_cfg);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL config
  result = stream_config_set_video(NULL, &video_cfg);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL video config
  result = stream_config_set_video(&config, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test audio configuration setting
 * @param state Test state (unused)
 */
static void test_audio_config_set(void** state) {
  (void)state;

  stream_config_t config;
  audio_config_t audio_cfg;

  memset(&config, 0, sizeof(config));
  memset(&audio_cfg, 0, sizeof(audio_cfg));

  stream_config_init(&config);

  // Set up valid audio configuration
  audio_cfg.sample_rate = 16000;
  audio_cfg.bitrate = 64000;
  audio_cfg.channels = 1;
  strcpy(audio_cfg.encoding, "G711");

  // Test setting audio configuration
  int result = stream_config_set_audio(&config, &audio_cfg);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL config
  result = stream_config_set_audio(NULL, &audio_cfg);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL audio config
  result = stream_config_set_audio(&config, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test video configuration getting
 * @param state Test state (unused)
 */
static void test_video_config_get(void** state) {
  (void)state;

  stream_config_t config;
  video_config_t video_cfg, retrieved_cfg;

  memset(&config, 0, sizeof(config));
  memset(&video_cfg, 0, sizeof(video_cfg));
  memset(&retrieved_cfg, 0, sizeof(retrieved_cfg));

  stream_config_init(&config);

  // Set up and set video configuration
  video_cfg.width = 1280;
  video_cfg.height = 720;
  video_cfg.framerate = 25;
  video_cfg.bitrate = 1500000;
  strcpy(video_cfg.encoding, "H264");

  stream_config_set_video(&config, &video_cfg);

  // Test getting video configuration
  int result = stream_config_get_video(&config, &retrieved_cfg);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(retrieved_cfg.width, 1280);
  assert_int_equal(retrieved_cfg.height, 720);
  assert_int_equal(retrieved_cfg.framerate, 25);
  assert_int_equal(retrieved_cfg.bitrate, 1500000);
  assert_string_equal(retrieved_cfg.encoding, "H264");

  // Test with NULL config
  result = stream_config_get_video(NULL, &retrieved_cfg);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL video config
  result = stream_config_get_video(&config, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test audio configuration getting
 * @param state Test state (unused)
 */
static void test_audio_config_get(void** state) {
  (void)state;

  stream_config_t config;
  audio_config_t audio_cfg, retrieved_cfg;

  memset(&config, 0, sizeof(config));
  memset(&audio_cfg, 0, sizeof(audio_cfg));
  memset(&retrieved_cfg, 0, sizeof(retrieved_cfg));

  stream_config_init(&config);

  // Set up and set audio configuration
  audio_cfg.sample_rate = 8000;
  audio_cfg.bitrate = 32000;
  audio_cfg.channels = 1;
  strcpy(audio_cfg.encoding, "G711");

  stream_config_set_audio(&config, &audio_cfg);

  // Test getting audio configuration
  int result = stream_config_get_audio(&config, &retrieved_cfg);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(retrieved_cfg.sample_rate, 8000);
  assert_int_equal(retrieved_cfg.bitrate, 32000);
  assert_int_equal(retrieved_cfg.channels, 1);
  assert_string_equal(retrieved_cfg.encoding, "G711");

  // Test with NULL config
  result = stream_config_get_audio(NULL, &retrieved_cfg);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL audio config
  result = stream_config_get_audio(&config, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test stream configuration validation
 * @param state Test state (unused)
 */
static void test_stream_config_validate(void** state) {
  (void)state;

  stream_config_t config;
  video_config_t video_cfg;
  audio_config_t audio_cfg;

  memset(&config, 0, sizeof(config));
  memset(&video_cfg, 0, sizeof(video_cfg));
  memset(&audio_cfg, 0, sizeof(audio_cfg));

  stream_config_init(&config);

  // Set up valid configurations
  video_cfg.width = 1920;
  video_cfg.height = 1080;
  video_cfg.framerate = 30;
  video_cfg.bitrate = 2000000;
  strcpy(video_cfg.encoding, "H264");

  audio_cfg.sample_rate = 16000;
  audio_cfg.bitrate = 64000;
  audio_cfg.channels = 1;
  strcpy(audio_cfg.encoding, "G711");

  stream_config_set_video(&config, &video_cfg);
  stream_config_set_audio(&config, &audio_cfg);

  // Test validation with valid configuration
  int result = stream_config_validate(&config);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL config
  result = stream_config_validate(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test stream configuration copying
 * @param state Test state (unused)
 */
static void test_stream_config_copy(void** state) {
  (void)state;

  stream_config_t src_config, dst_config;
  video_config_t video_cfg;
  audio_config_t audio_cfg;

  memset(&src_config, 0, sizeof(src_config));
  memset(&dst_config, 0, sizeof(dst_config));
  memset(&video_cfg, 0, sizeof(video_cfg));
  memset(&audio_cfg, 0, sizeof(audio_cfg));

  stream_config_init(&src_config);
  stream_config_init(&dst_config);

  // Set up source configuration
  video_cfg.width = 640;
  video_cfg.height = 480;
  video_cfg.framerate = 15;
  video_cfg.bitrate = 500000;
  strcpy(video_cfg.encoding, "H264");

  audio_cfg.sample_rate = 8000;
  audio_cfg.bitrate = 32000;
  audio_cfg.channels = 1;
  strcpy(audio_cfg.encoding, "G711");

  stream_config_set_video(&src_config, &video_cfg);
  stream_config_set_audio(&src_config, &audio_cfg);

  // Test copying configuration
  int result = stream_config_copy(&dst_config, &src_config);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify copied configuration
  video_config_t dst_video;
  audio_config_t dst_audio;
  memset(&dst_video, 0, sizeof(dst_video));
  memset(&dst_audio, 0, sizeof(dst_audio));

  stream_config_get_video(&dst_config, &dst_video);
  stream_config_get_audio(&dst_config, &dst_audio);

  assert_int_equal(dst_video.width, 640);
  assert_int_equal(dst_video.height, 480);
  assert_int_equal(dst_audio.sample_rate, 8000);

  // Test with NULL configs
  result = stream_config_copy(NULL, &src_config);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = stream_config_copy(&dst_config, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&src_config);
  stream_config_cleanup(&dst_config);
}

/**
 * @brief Test stream configuration serialization
 * @param state Test state (unused)
 */
static void test_stream_config_serialize(void** state) {
  (void)state;

  stream_config_t config;
  video_config_t video_cfg;
  char buffer[1024];

  memset(&config, 0, sizeof(config));
  memset(&video_cfg, 0, sizeof(video_cfg));

  stream_config_init(&config);

  // Set up configuration
  video_cfg.width = 1280;
  video_cfg.height = 720;
  video_cfg.framerate = 30;
  video_cfg.bitrate = 1000000;
  strcpy(video_cfg.encoding, "H264");

  stream_config_set_video(&config, &video_cfg);

  // Test serialization
  int result = stream_config_serialize(&config, buffer, sizeof(buffer));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(strlen(buffer) > 0);

  // Test with NULL config
  result = stream_config_serialize(NULL, buffer, sizeof(buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL buffer
  result = stream_config_serialize(&config, NULL, sizeof(buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with zero buffer size
  result = stream_config_serialize(&config, buffer, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&config);
}

/**
 * @brief Test stream configuration deserialization
 * @param state Test state (unused)
 */
static void test_stream_config_deserialize(void** state) {
  (void)state;

  stream_config_t src_config, dst_config;
  video_config_t video_cfg;
  char buffer[1024];

  memset(&src_config, 0, sizeof(src_config));
  memset(&dst_config, 0, sizeof(dst_config));
  memset(&video_cfg, 0, sizeof(video_cfg));

  stream_config_init(&src_config);
  stream_config_init(&dst_config);

  // Set up and serialize configuration
  video_cfg.width = 800;
  video_cfg.height = 600;
  video_cfg.framerate = 20;
  video_cfg.bitrate = 800000;
  strcpy(video_cfg.encoding, "H264");

  stream_config_set_video(&src_config, &video_cfg);
  stream_config_serialize(&src_config, buffer, sizeof(buffer));

  // Test deserialization
  int result = stream_config_deserialize(&dst_config, buffer, strlen(buffer));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify deserialized configuration
  video_config_t dst_video;
  memset(&dst_video, 0, sizeof(dst_video));
  stream_config_get_video(&dst_config, &dst_video);
  assert_int_equal(dst_video.width, 800);
  assert_int_equal(dst_video.height, 600);

  // Test with NULL config
  result = stream_config_deserialize(NULL, buffer, strlen(buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL buffer
  result = stream_config_deserialize(&dst_config, NULL, 100);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with zero buffer size
  result = stream_config_deserialize(&dst_config, buffer, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  stream_config_cleanup(&src_config);
  stream_config_cleanup(&dst_config);
}
