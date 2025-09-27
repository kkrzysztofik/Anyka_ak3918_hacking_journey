/**
 * @file test_validation_utils.c
 * @brief Unit tests for validation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include <cmocka.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

// Include the actual source files we're testing
#include "utils/validation/audio_validation.h"
#include "utils/validation/common_validation.h"
#include "utils/validation/input_validation.h"

/**
 * @brief Test common validation functions
 * @param state Test state (unused)
 */
static void test_common_validation(void** state) {
  (void)state;

  // Test token validation
  assert_true(onvif_util_validate_token("valid_token"));
  assert_true(onvif_util_validate_token("abc123"));
  assert_false(onvif_util_validate_token(""));
  assert_false(onvif_util_validate_token(NULL));
  assert_false(onvif_util_validate_token("token_with_invalid_chars!"));

  // Test profile token validation
  assert_true(onvif_util_validate_profile_token("Profile_1"));
  assert_false(onvif_util_validate_profile_token(""));
  assert_false(onvif_util_validate_profile_token(NULL));

  // Test encoder token validation
  assert_true(onvif_util_validate_encoder_token("Encoder_1"));
  assert_false(onvif_util_validate_encoder_token(""));
  assert_false(onvif_util_validate_encoder_token(NULL));
}

/**
 * @brief Test input validation functions
 * @param state Test state (unused)
 */
static void test_input_validation(void** state) {
  (void)state;

  // Test string input validation
  assert_true(onvif_util_validate_string_input("valid_string"));
  assert_false(onvif_util_validate_string_input(""));
  assert_false(onvif_util_validate_string_input(NULL));

  // Test numeric input validation
  assert_true(onvif_util_validate_numeric_input(0, 0, 100));
  assert_true(onvif_util_validate_numeric_input(50, 0, 100));
  assert_true(onvif_util_validate_numeric_input(100, 0, 100));
  assert_false(onvif_util_validate_numeric_input(-1, 0, 100));
  assert_false(onvif_util_validate_numeric_input(101, 0, 100));

  // Test IP address validation
  assert_true(onvif_util_validate_ip_address("192.168.1.1"));
  assert_true(onvif_util_validate_ip_address("127.0.0.1"));
  assert_false(onvif_util_validate_ip_address("256.256.256.256"));
  assert_false(onvif_util_validate_ip_address("192.168.1"));
  assert_false(onvif_util_validate_ip_address(""));
  assert_false(onvif_util_validate_ip_address(NULL));
}

/**
 * @brief Test audio validation functions
 * @param state Test state (unused)
 */
static void test_audio_validation(void** state) {
  (void)state;

  // Test audio encoding validation
  assert_true(onvif_util_validate_audio_encoding("G711"));
  assert_true(onvif_util_validate_audio_encoding("AAC"));
  assert_false(onvif_util_validate_audio_encoding("INVALID"));
  assert_false(onvif_util_validate_audio_encoding(""));
  assert_false(onvif_util_validate_audio_encoding(NULL));

  // Test sample rate validation
  assert_true(onvif_util_validate_sample_rate(8000));
  assert_true(onvif_util_validate_sample_rate(16000));
  assert_true(onvif_util_validate_sample_rate(44100));
  assert_false(onvif_util_validate_sample_rate(0));
  assert_false(onvif_util_validate_sample_rate(-1));

  // Test bitrate validation
  assert_true(onvif_util_validate_bitrate(64000));
  assert_true(onvif_util_validate_bitrate(128000));
  assert_false(onvif_util_validate_bitrate(0));
  assert_false(onvif_util_validate_bitrate(-1));
}
