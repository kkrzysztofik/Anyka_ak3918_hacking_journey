/**
 * @file test_hash_utils.c
 * @brief Unit tests for hash utilities (SHA256, password hashing)
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "cmocka_wrapper.h"

/* Code under test */
#include "utils/error/error_handling.h"
#include "utils/security/hash_utils.h"
#include "utils/security/sha256.h"

/* Mocks */
#include "mocks/network_mock.h"

// NOLINTBEGIN(cppcoreguidelines-avoid-magic-number,readability-magic-numbers)

/**
 * @brief Setup function for password hashing tests
 * @param state Test state (unused)
 * @return 0 on success
 */
static int setup_password_tests(void** state) {
  (void)state;
  network_mock_use_real_function(true);
  return 0;
}

/**
 * @brief Teardown function for password hashing tests
 * @param state Test state (unused)
 * @return 0 on success
 */
static int teardown_password_tests(void** state) {
  (void)state;
  network_mock_use_real_function(false);
  return 0;
}

/**
 * @brief Test SHA256 with empty input
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_empty(void** state) {
  (void)state;

  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];
  int result = onvif_sha256_compute((const uint8_t*)"", 0, digest);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Known SHA256 of empty string: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
   */
  const uint8_t expected[ONVIF_SHA256_DIGEST_SIZE] = {0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
                                                      0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55};

  assert_memory_equal(digest, expected, ONVIF_SHA256_DIGEST_SIZE);
}

/**
 * @brief Test SHA256 with known test vector
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_known_vector(void** state) {
  (void)state;

  const char* input = "abc";
  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];
  int result = onvif_sha256_compute((const uint8_t*)input, strlen(input), digest);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Known SHA256 of "abc": ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad */
  const uint8_t expected[ONVIF_SHA256_DIGEST_SIZE] = {0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
                                                      0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad};

  assert_memory_equal(digest, expected, ONVIF_SHA256_DIGEST_SIZE);
}

/**
 * @brief Test SHA256 with longer input
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_longer_input(void** state) {
  (void)state;

  const char* input = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];
  int result = onvif_sha256_compute((const uint8_t*)input, strlen(input), digest);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Known SHA256 of this string: 248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1
   */
  const uint8_t expected[ONVIF_SHA256_DIGEST_SIZE] = {0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e, 0x60, 0x39,
                                                      0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4, 0x19, 0xdb, 0x06, 0xc1};

  assert_memory_equal(digest, expected, ONVIF_SHA256_DIGEST_SIZE);
}

/**
 * @brief Test SHA256 to hex conversion
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_to_hex(void** state) {
  (void)state;

  const uint8_t digest[ONVIF_SHA256_DIGEST_SIZE] = {0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
                                                    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad};

  char hex_output[ONVIF_SHA256_HEX_SIZE];
  int result = onvif_sha256_to_hex(digest, hex_output, sizeof(hex_output));
  assert_int_equal(result, ONVIF_SUCCESS);

  const char* expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
  assert_string_equal(hex_output, expected);
}

/**
 * @brief Test SHA256 compute and convert to hex in one call
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_compute_hex(void** state) {
  (void)state;

  const char* input = "abc";
  char hex_output[ONVIF_SHA256_HEX_SIZE];
  int result = onvif_sha256_compute_hex((const uint8_t*)input, strlen(input), hex_output, sizeof(hex_output));
  assert_int_equal(result, ONVIF_SUCCESS);

  const char* expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
  assert_string_equal(hex_output, expected);
}

/**
 * @brief Test SHA256 with NULL pointer
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_null_pointer(void** state) {
  (void)state;

  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];

  /* NULL data pointer */
  int result = onvif_sha256_compute(NULL, 10, digest);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  /* NULL digest pointer */
  result = onvif_sha256_compute((const uint8_t*)"test", 4, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test SHA256 to hex with buffer too small
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_to_hex_buffer_too_small(void** state) {
  (void)state;

  const uint8_t digest[ONVIF_SHA256_DIGEST_SIZE] = {0};
  char hex_output[32]; /* Too small - needs 65 bytes */

  int result = onvif_sha256_to_hex(digest, hex_output, sizeof(hex_output));
  assert_int_equal(result, ONVIF_ERROR_BUFFER_TOO_SMALL);
}

/**
 * @brief Test password hashing
 * @param state Test state (unused)
 */
static void test_unit_hash_password_hashing(void** state) {
  (void)state;

  char hash1[ONVIF_PASSWORD_HASH_SIZE];
  char hash2[ONVIF_PASSWORD_HASH_SIZE];

  /* Test password hashing */
  const char* password = "test_password";
  int result = onvif_hash_password(password, hash1, sizeof(hash1));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(strlen(hash1) > 0);

  /* Verify hash contains salt separator */
  assert_non_null(strchr(hash1, '$'));

  /* Test that same password produces different hash (due to different salt) */
  result = onvif_hash_password(password, hash2, sizeof(hash2));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(hash1, hash2);

  /* Test different password produces different hash */
  result = onvif_hash_password("different_password", hash2, sizeof(hash2));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(hash1, hash2);
}

/**
 * @brief Test password verification
 * @param state Test state (unused)
 */
static void test_unit_hash_password_verification(void** state) {
  (void)state;

  const char* password = "test_password";
  char hash[ONVIF_PASSWORD_HASH_SIZE];

  /* Hash the password */
  int result = onvif_hash_password(password, hash, sizeof(hash));
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify correct password */
  result = onvif_verify_password(password, hash);
  assert_int_equal(result, ONVIF_SUCCESS);

  /* Verify incorrect password */
  result = onvif_verify_password("wrong_password", hash);
  assert_int_equal(result, ONVIF_ERROR_AUTH_FAILED);
}

/**
 * @brief Test password hashing with NULL pointers
 * @param state Test state (unused)
 */
static void test_unit_hash_password_null_pointers(void** state) {
  (void)state;

  char hash[ONVIF_PASSWORD_HASH_SIZE];

  /* NULL password */
  int result = onvif_hash_password(NULL, hash, sizeof(hash));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  /* NULL hash buffer */
  result = onvif_hash_password("password", NULL, sizeof(hash));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  /* NULL password for verification */
  result = onvif_verify_password(NULL, hash);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  /* NULL hash for verification */
  result = onvif_verify_password("password", NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test password hashing with buffer too small
 * @param state Test state (unused)
 */
static void test_unit_hash_password_buffer_too_small(void** state) {
  (void)state;

  char hash[32]; /* Too small */

  int result = onvif_hash_password("password", hash, sizeof(hash));
  assert_int_equal(result, ONVIF_ERROR_BUFFER_TOO_SMALL);
}

/**
 * @brief Test password with invalid length
 * @param state Test state (unused)
 */
static void test_unit_hash_password_invalid_length(void** state) {
  (void)state;

  char hash[ONVIF_PASSWORD_HASH_SIZE];

  /* Empty password */
  int result = onvif_hash_password("", hash, sizeof(hash));
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test multi-block SHA256 hashing (incremental)
 * @param state Test state (unused)
 */
static void test_unit_hash_sha256_incremental(void** state) {
  (void)state;

  sha256_ctx_t ctx;
  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];

  /* Hash "abc" incrementally */
  sha256_begin(&ctx);
  sha256_hash(&ctx, (const uint8_t*)"a", 1);
  sha256_hash(&ctx, (const uint8_t*)"b", 1);
  sha256_hash(&ctx, (const uint8_t*)"c", 1);
  sha256_end(&ctx, digest);

  /* Should match single-block hash of "abc" */
  const uint8_t expected[ONVIF_SHA256_DIGEST_SIZE] = {0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
                                                      0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad};

  assert_memory_equal(digest, expected, ONVIF_SHA256_DIGEST_SIZE);
}

/**
 * @brief Get hash utils unit tests
 * @param count Output parameter for number of tests
 * @return Array of unit tests
 */
const struct CMUnitTest* get_hash_utils_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_hash_sha256_empty),
    cmocka_unit_test(test_unit_hash_sha256_known_vector),
    cmocka_unit_test(test_unit_hash_sha256_longer_input),
    cmocka_unit_test(test_unit_hash_sha256_to_hex),
    cmocka_unit_test(test_unit_hash_sha256_compute_hex),
    cmocka_unit_test(test_unit_hash_sha256_null_pointer),
    cmocka_unit_test(test_unit_hash_sha256_to_hex_buffer_too_small),
    cmocka_unit_test_setup_teardown(test_unit_hash_password_hashing, setup_password_tests, teardown_password_tests),
    cmocka_unit_test_setup_teardown(test_unit_hash_password_verification, setup_password_tests, teardown_password_tests),
    cmocka_unit_test(test_unit_hash_password_null_pointers),
    cmocka_unit_test(test_unit_hash_password_buffer_too_small),
    cmocka_unit_test(test_unit_hash_password_invalid_length),
    cmocka_unit_test(test_unit_hash_sha256_incremental),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
// NOLINTEND(cppcoreguidelines-avoid-magic-number,readability-magic-numbers)
