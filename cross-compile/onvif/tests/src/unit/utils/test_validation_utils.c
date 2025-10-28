/**
 * @file test_validation_utils.c
 * @brief Unit tests for validation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <stdint.h>

#include "cmocka_wrapper.h"
#include "common/onvif_constants.h"
#include "platform/platform_common.h"
#include "utils/error/error_handling.h"
#include "utils/security/security_hardening.h"
#include "utils/validation/audio_validation.h"
#include "utils/validation/common_validation.h"
#include "utils/validation/input_validation.h"

#define TEST_HTTP_PORT_MIN          1
#define TEST_HTTP_PORT_INVALID_HIGH 70000
#define TEST_DECODE_BUFFER_SIZE     64U
#define TEST_STRING_MIN_LENGTH      1U
#define TEST_STRING_MAX_LENGTH      32U

/**
 * @brief Test common validation helper routines
 * @param state Test state (unused)
 */
static void test_common_validation_case(void** state) {
  (void)state;

  validation_result_t result = validate_onvif_token("ValidToken_1", "token");
  assert_true(validation_is_valid(&result));

  result = validate_onvif_token("invalid token!", "token");
  assert_false(validation_is_valid(&result));

  result = validate_profile_token("Profile_1", "profile");
  assert_true(validation_is_valid(&result));

  result = validate_profile_token("Profile token with spaces", "profile");
  assert_false(validation_is_valid(&result));

  result = validate_string("Manufacturer", "Anyka", TEST_STRING_MIN_LENGTH, TEST_STRING_MAX_LENGTH, 0);
  assert_true(validation_is_valid(&result));

  result = validate_string("Manufacturer", "", TEST_STRING_MIN_LENGTH, TEST_STRING_MAX_LENGTH, 0);
  assert_false(validation_is_valid(&result));

  result = validate_int("HTTP Port", ONVIF_HTTP_STANDARD_PORT, TEST_HTTP_PORT_MIN, (int)UINT16_MAX);
  assert_true(validation_is_valid(&result));

  result = validate_int("HTTP Port", -1, TEST_HTTP_PORT_MIN, (int)UINT16_MAX);
  assert_false(validation_is_valid(&result));

  result = validate_int("HTTP Port", TEST_HTTP_PORT_INVALID_HIGH, TEST_HTTP_PORT_MIN, (int)UINT16_MAX);
  assert_false(validation_is_valid(&result));
}

/**
 * @brief Test input validation APIs
 * @param state Test state (unused)
 */
static void test_input_validation_case(void** state) {
  (void)state;

  int status = validate_username_input("ValidUser1");
  assert_int_equal(ONVIF_VALIDATION_SUCCESS, status);

  status = validate_username_input("!");
  assert_int_equal(ONVIF_VALIDATION_FAILED, status);

  status = validate_password_input("Password123!");
  assert_int_equal(ONVIF_VALIDATION_SUCCESS, status);

  status = validate_password_input("short");
  assert_int_equal(ONVIF_VALIDATION_FAILED, status);

  status = validate_auth_header_input("Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==");
  assert_int_equal(ONVIF_VALIDATION_SUCCESS, status);

  status = validate_auth_header_input("Basic invalid!");
  assert_int_equal(ONVIF_VALIDATION_FAILED, status);

  char decoded[TEST_DECODE_BUFFER_SIZE];
  status = validate_and_decode_base64("QWxhZGRpbjpvcGVuIHNlc2FtZQ==", decoded, sizeof(decoded));
  assert_int_equal(ONVIF_VALIDATION_SUCCESS, status);

  status = validate_and_decode_base64("not_base64", decoded, sizeof(decoded));
  assert_int_equal(ONVIF_VALIDATION_FAILED, status);

  assert_int_equal(1, security_is_valid_ip("192.168.1.1"));
  assert_int_equal(0, security_is_valid_ip("256.256.256.256"));
}

/**
 * @brief Test audio validation entry points
 * @param state Test state (unused)
 */
static void test_audio_validation_case(void** state) {
  (void)state;

  assert_int_equal(1, audio_validation_validate_sample_rate(16000));
  assert_int_equal(0, audio_validation_validate_sample_rate(12345));

  assert_int_equal(1, audio_validation_validate_channels(2));
  assert_int_equal(0, audio_validation_validate_channels(3));

  assert_int_equal(1, audio_validation_validate_bits_per_sample(16));
  assert_int_equal(0, audio_validation_validate_bits_per_sample(20));

  assert_int_equal(1, audio_validation_validate_codec(PLATFORM_AUDIO_CODEC_PCM));
  assert_int_equal(0, audio_validation_validate_codec((platform_audio_codec_t)99));
}

/**
 * @brief Register validation utility unit tests
 * @param state CMocka state (unused)
 */
void test_unit_common_validation(void** state) {
  (void)state;

  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_common_validation_case),
  };

  cmocka_run_group_tests_name("common_validation_utils", tests, NULL, NULL);
}

/**
 * @brief Register input validation tests
 * @param state CMocka state (unused)
 */
void test_input_validation(void** state) {
  (void)state;

  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_input_validation_case),
  };

  cmocka_run_group_tests_name("input_validation_utils", tests, NULL, NULL);
}

/**
 * @brief Register audio validation tests
 * @param state CMocka state (unused)
 */
void test_unit_audio_validation(void** state) {
  (void)state;

  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_audio_validation_case),
  };

  cmocka_run_group_tests_name("audio_validation_utils", tests, NULL, NULL);
}
