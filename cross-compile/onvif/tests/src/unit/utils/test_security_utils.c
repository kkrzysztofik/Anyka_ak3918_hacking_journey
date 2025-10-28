/**
 * @file test_security_utils.c
 * @brief Unit tests for security utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/security/base64_utils.h"
#include "utils/security/security_hardening.h"

/**
 * @brief Test security hardening initialization
 * @param state Test state (unused)
 */
static void test_security_init(void** state) {
  (void)state;

  // Test security hardening initialization
  int result = onvif_security_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test that initialization is idempotent
  result = onvif_security_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_security_cleanup();
}

/**
 * @brief Test security hardening cleanup
 * @param state Test state (unused)
 */
static void test_security_cleanup(void** state) {
  (void)state;

  // Initialize security first
  onvif_security_init();

  // Test cleanup (should not crash)
  onvif_security_cleanup();

  // Test multiple cleanups (should not crash)
  onvif_security_cleanup();
}

/**
 * @brief Test input sanitization
 * @param state Test state (unused)
 */
static void test_input_sanitization(void** state) {
  (void)state;

  onvif_security_init();

  // Test basic string sanitization
  char input1[] = "normal_input";
  int result = onvif_sanitize_input(input1, sizeof(input1));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(input1, "normal_input");

  // Test sanitization with special characters
  char input2[] = "input<script>alert('xss')</script>";
  result = onvif_sanitize_input(input2, sizeof(input2));
  assert_int_equal(result, ONVIF_SUCCESS);
  // Input should be sanitized (exact behavior depends on implementation)

  // Test with NULL input
  result = onvif_sanitize_input(NULL, 100);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with zero length
  result = onvif_sanitize_input(input1, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}

/**
 * @brief Test XML sanitization
 * @param state Test state (unused)
 */
static void test_xml_sanitization(void** state) {
  (void)state;

  onvif_security_init();

  // Test basic XML sanitization
  char xml1[] = "<valid>content</valid>";
  int result = onvif_sanitize_xml_input(xml1, sizeof(xml1));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test XML with potentially malicious content
  char xml2[] = "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "
                "\"file:///etc/passwd\">]><root>&xxe;</root>";
  result = onvif_sanitize_xml_input(xml2, sizeof(xml2));
  // Should either succeed (if sanitized) or fail (if rejected)
  assert_true(result == ONVIF_SUCCESS || result == ONVIF_ERROR_INVALID);

  // Test with NULL XML input
  result = onvif_sanitize_xml_input(NULL, 100);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}

/**
 * @brief Test Base64 encoding
 * @param state Test state (unused)
 */
static void test_base64_encode(void** state) {
  (void)state;

  char output[256];
  size_t output_len;

  // Test basic encoding
  const char* input1 = "Hello, World!";
  int result = onvif_base64_encode((const unsigned char*)input1, strlen(input1), output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(output_len > 0);
  assert_string_equal(output, "SGVsbG8sIFdvcmxkIQ==");

  // Test empty input
  result = onvif_base64_encode((const unsigned char*)"", 0, output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(output_len, 0);

  // Test NULL input
  result = onvif_base64_encode(NULL, 5, output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL output buffer
  result = onvif_base64_encode((const unsigned char*)input1, strlen(input1), NULL, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test insufficient output buffer
  char small_output[4];
  result = onvif_base64_encode((const unsigned char*)input1, strlen(input1), small_output, sizeof(small_output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_BUFFER_TOO_SMALL);
}

/**
 * @brief Test Base64 decoding
 * @param state Test state (unused)
 */
static void test_base64_decode(void** state) {
  (void)state;

  unsigned char output[256];
  size_t output_len;

  // Test basic decoding
  const char* input1 = "SGVsbG8sIFdvcmxkIQ==";
  int result = onvif_base64_decode(input1, strlen(input1), output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(output_len, 13);
  assert_memory_equal(output, "Hello, World!", 13);

  // Test empty input
  result = onvif_base64_decode("", 0, output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(output_len, 0);

  // Test invalid Base64 input
  const char* invalid_input = "Invalid@Base64!";
  result = onvif_base64_decode(invalid_input, strlen(invalid_input), output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL input
  result = onvif_base64_decode(NULL, 5, output, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL output buffer
  result = onvif_base64_decode(input1, strlen(input1), NULL, sizeof(output), &output_len);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test Base64 roundtrip encoding/decoding
 * @param state Test state (unused)
 */
static void test_base64_roundtrip(void** state) {
  (void)state;

  const char* original = "This is a test message for Base64 roundtrip encoding!";
  char encoded[256];
  unsigned char decoded[256];
  size_t encoded_len, decoded_len;

  // Encode
  int result = onvif_base64_encode((const unsigned char*)original, strlen(original), encoded, sizeof(encoded), &encoded_len);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Decode
  result = onvif_base64_decode(encoded, encoded_len, decoded, sizeof(decoded), &decoded_len);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify roundtrip
  assert_int_equal(decoded_len, strlen(original));
  assert_memory_equal(decoded, original, strlen(original));
}

/**
 * @brief Test password hashing
 * @param state Test state (unused)
 */
static void test_password_hashing(void** state) {
  (void)state;

  onvif_security_init();

  char hash1[256], hash2[256];

  // Test password hashing
  const char* password = "test_password";
  int result = onvif_hash_password(password, hash1, sizeof(hash1));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(strlen(hash1) > 0);

  // Test that same password produces same hash
  result = onvif_hash_password(password, hash2, sizeof(hash2));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(hash1, hash2);

  // Test different password produces different hash
  result = onvif_hash_password("different_password", hash2, sizeof(hash2));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(hash1, hash2);

  // Test NULL password
  result = onvif_hash_password(NULL, hash1, sizeof(hash1));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL hash buffer
  result = onvif_hash_password(password, NULL, sizeof(hash1));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}

/**
 * @brief Test password verification
 * @param state Test state (unused)
 */
static void test_password_verification(void** state) {
  (void)state;

  onvif_security_init();

  const char* password = "test_password";
  char hash[256];

  // Hash the password
  int result = onvif_hash_password(password, hash, sizeof(hash));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify correct password
  result = onvif_verify_password(password, hash);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify incorrect password
  result = onvif_verify_password("wrong_password", hash);
  assert_int_equal(result, ONVIF_ERROR_AUTH_FAILED);

  // Test NULL password
  result = onvif_verify_password(NULL, hash);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL hash
  result = onvif_verify_password(password, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}

/**
 * @brief Test security token generation
 * @param state Test state (unused)
 */
static void test_token_generation(void** state) {
  (void)state;

  onvif_security_init();

  char token1[128], token2[128];

  // Test token generation
  int result = onvif_generate_security_token(token1, sizeof(token1));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_true(strlen(token1) > 0);

  // Test that different calls produce different tokens
  result = onvif_generate_security_token(token2, sizeof(token2));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(token1, token2);

  // Test NULL token buffer
  result = onvif_generate_security_token(NULL, sizeof(token1));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test zero buffer size
  result = onvif_generate_security_token(token1, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}

/**
 * @brief Test security token validation
 * @param state Test state (unused)
 */
static void test_token_validation(void** state) {
  (void)state;

  onvif_security_init();

  char token[128];

  // Generate a token
  int result = onvif_generate_security_token(token, sizeof(token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate the token
  result = onvif_validate_security_token(token);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid token
  result = onvif_validate_security_token("invalid_token");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test NULL token
  result = onvif_validate_security_token(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test empty token
  result = onvif_validate_security_token("");
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_security_cleanup();
}
