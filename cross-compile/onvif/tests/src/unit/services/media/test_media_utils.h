/**
 * @file test_media_utils.h
 * @brief Unit tests for ONVIF media service utility functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_MEDIA_UTILS_H
#define TEST_MEDIA_UTILS_H

#include "cmocka_wrapper.h"

// Forward declarations of test functions
void test_media_profile_functions(void** state);
void test_media_video_source_functions(void** state);
void test_media_audio_source_functions(void** state);
void test_media_video_configuration_functions(void** state);
void test_media_audio_configuration_functions(void** state);
void test_media_stream_uri_functions(void** state);
void test_media_snapshot_uri_functions(void** state);
void test_media_multicast_functions(void** state);
void test_media_metadata_functions(void** state);
void test_media_error_handling(void** state);
void test_media_initialization(void** state);

// Test constants
#define TEST_MEDIA_PROFILE_COUNT        2
#define TEST_VIDEO_SOURCE_COUNT         1
#define TEST_AUDIO_SOURCE_COUNT         1
#define TEST_VIDEO_CONFIG_COUNT         1
#define TEST_VIDEO_ENCODER_CONFIG_COUNT 2
#define TEST_AUDIO_CONFIG_COUNT         1
#define TEST_AUDIO_ENCODER_CONFIG_COUNT 3
#define TEST_METADATA_CONFIG_COUNT      1

#endif // TEST_MEDIA_UTILS_H
