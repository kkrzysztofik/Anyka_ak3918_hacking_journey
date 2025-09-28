/**
 * @file test_string_utils.c
 * @brief Unit tests for string manipulation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/string/string_shims.h"

/**
 * @brief Test string shim functions
 * @param state Test state (unused)
 */
static void test_string_shims(void** state) {
  (void)state;

  // Test safe string copy
  char dest[32];
  const char* src = "Hello, World!";

  int result = onvif_strncpy_safe(dest, sizeof(dest), src, strlen(src));
  assert_int_equal(result, 0);
  assert_string_equal(dest, src);

  // Test string copy with truncation
  char small_dest[5];
  result = onvif_strncpy_safe(small_dest, sizeof(small_dest), src, strlen(src));
  assert_int_equal(result, 0);
  assert_string_equal(small_dest, "Hell");

  // Test string copy with NULL inputs
  result = onvif_strncpy_safe(NULL, sizeof(dest), src, strlen(src));
  assert_int_equal(result, -1);

  result = onvif_strncpy_safe(dest, sizeof(dest), NULL, 0);
  assert_int_equal(result, -1);

  // Test string copy with zero size
  result = onvif_strncpy_safe(dest, 0, src, strlen(src));
  assert_int_equal(result, -1);
}

/**
 * @brief Test string validation functions
 * @param state Test state (unused)
 */
static void test_string_validation(void** state) {
  (void)state;

  // Test valid strings
  assert_true(onvif_is_valid_string("Hello"));
  assert_true(onvif_is_valid_string(""));
  assert_true(onvif_is_valid_string("123"));

  // Test invalid strings
  assert_false(onvif_is_valid_string(NULL));

  // Test string length validation
  assert_true(onvif_is_valid_string_length("Hello", 5));
  assert_true(onvif_is_valid_string_length("Hello", 10));
  assert_false(onvif_is_valid_string_length("Hello", 3));
  assert_false(onvif_is_valid_string_length(NULL, 5));

  // Test string format validation
  assert_true(onvif_is_valid_token("abc123"));
  assert_true(onvif_is_valid_token("ABC_123"));
  assert_false(onvif_is_valid_token("abc-123")); // Invalid character
  assert_false(onvif_is_valid_token(""));        // Empty
  assert_false(onvif_is_valid_token(NULL));      // NULL
}

/**
 * @brief Test string manipulation functions
 * @param state Test state (unused)
 */
static void test_string_manipulation(void** state) {
  (void)state;

  // Test string trimming
  char str1[] = "  Hello, World!  ";
  onvif_trim_string(str1);
  assert_string_equal(str1, "Hello, World!");

  char str2[] = "NoSpaces";
  onvif_trim_string(str2);
  assert_string_equal(str2, "NoSpaces");

  char str3[] = "   ";
  onvif_trim_string(str3);
  assert_string_equal(str3, "");

  // Test string case conversion
  char str4[] = "Hello, World!";
  onvif_to_lowercase(str4);
  assert_string_equal(str4, "hello, world!");

  char str5[] = "HELLO, WORLD!";
  onvif_to_uppercase(str5);
  assert_string_equal(str5, "HELLO, WORLD!");

  // Test string concatenation
  char dest[64] = "Hello";
  const char* src = ", World!";

  int result = onvif_strcat_safe(dest, sizeof(dest), src);
  assert_int_equal(result, 0);
  assert_string_equal(dest, "Hello, World!");

  // Test concatenation with overflow
  char small_dest[8] = "Hello";
  result = onvif_strcat_safe(small_dest, sizeof(small_dest), ", World!");
  assert_int_equal(result, -1); // Should fail due to overflow
}

/**
 * @brief Test string search and comparison functions
 * @param state Test state (unused)
 */
static void test_string_search(void** state) {
  (void)state;

  const char* haystack = "Hello, World!";

  // Test string contains
  assert_true(onvif_string_contains(haystack, "Hello"));
  assert_true(onvif_string_contains(haystack, "World"));
  assert_false(onvif_string_contains(haystack, "Goodbye"));
  assert_false(onvif_string_contains(NULL, "Hello"));
  assert_false(onvif_string_contains(haystack, NULL));

  // Test case-insensitive comparison
  assert_true(onvif_strcasecmp("Hello", "HELLO") == 0);
  assert_true(onvif_strcasecmp("Hello", "hello") == 0);
  assert_false(onvif_strcasecmp("Hello", "World") == 0);

  // Test string starts with
  assert_true(onvif_string_starts_with(haystack, "Hello"));
  assert_false(onvif_string_starts_with(haystack, "World"));
  assert_false(onvif_string_starts_with(NULL, "Hello"));
  assert_false(onvif_string_starts_with(haystack, NULL));

  // Test string ends with
  assert_true(onvif_string_ends_with(haystack, "World!"));
  assert_false(onvif_string_ends_with(haystack, "Hello"));
  assert_false(onvif_string_ends_with(NULL, "World!"));
  assert_false(onvif_string_ends_with(haystack, NULL));
}

/**
 * @brief Test string formatting functions
 * @param state Test state (unused)
 */
static void test_string_formatting(void** state) {
  (void)state;

  char buffer[64];

  // Test safe sprintf
  int result = onvif_snprintf_safe(buffer, sizeof(buffer), "Hello, %s!", "World");
  assert_int_equal(result, 0);
  assert_string_equal(buffer, "Hello, World!");

  // Test sprintf with overflow
  result = onvif_snprintf_safe(buffer, 5, "Hello, %s!", "World");
  assert_int_equal(result, -1); // Should fail due to overflow

  // Test sprintf with NULL inputs
  result = onvif_snprintf_safe(NULL, sizeof(buffer), "Hello, %s!", "World");
  assert_int_equal(result, -1);

  // Test string replacement
  char str[] = "Hello, World! Hello, Universe!";
  result = onvif_string_replace(str, sizeof(str), "Hello", "Hi");
  assert_int_equal(result, 0);
  assert_string_equal(str, "Hi, World! Hi, Universe!");

  // Test string replacement with no matches
  char str2[] = "Goodbye, World!";
  result = onvif_string_replace(str2, sizeof(str2), "Hello", "Hi");
  assert_int_equal(result, 0);
  assert_string_equal(str2, "Goodbye, World!"); // Should be unchanged
}
