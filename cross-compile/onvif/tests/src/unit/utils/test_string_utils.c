/**
 * @file test_string_utils.c
 * @brief Unit tests for string shim utilities
 */

#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"

#include "utils/string/string_shims.h"

#define TEST_SMALL_BUFFER_SIZE 8U
#define TEST_LARGE_BUFFER_SIZE 64U
#define TEST_BUILD_YEAR 2025

static int call_memory_safe_vsnprintf(char* buffer, size_t buffer_size, const char* format, ...) {
  va_list args;
  va_start(args, format);
  const int result = memory_safe_vsnprintf(buffer, buffer_size, format, args);
  va_end(args);
  return result;
}

static void test_case_insensitive_helpers(void** state) {
  (void)state;

  assert_int_equal(0, strcasecmp("hello", "HELLO"));
  assert_true(strcasecmp("apple", "Banana") < 0);
  assert_true(strcasecmp("Cherry", "banana") > 0);

  const char* haystack = "Anyka Embedded Platform";
  const char* match = strcasestr(haystack, "PLATFORM");
  assert_non_null(match);
  assert_string_equal("Platform", match);

  assert_null(strcasestr(haystack, "Firmware"));
  assert_int_equal(5, (int)strnlen("Hello", 10));
  assert_int_equal(3, (int)strnlen("Hi", 3));
}

static void test_trim_whitespace_helpers(void** state) {
  (void)state;

  char padded[] = "  Anyka SDK  ";
  trim_whitespace(padded);
  assert_string_equal("Anyka SDK", padded);

  char no_padding[] = "Camera";
  trim_whitespace(no_padding);
  assert_string_equal("Camera", no_padding);

  char only_spaces[] = "   ";
  trim_whitespace(only_spaces);
  assert_string_equal("", only_spaces);
}

static void test_memory_safe_formatting(void** state) {
  (void)state;

  char buffer[TEST_LARGE_BUFFER_SIZE];
  int written = call_memory_safe_vsnprintf(buffer, sizeof(buffer), "Hello, %s!", "World");
  assert_int_equal(13, written);
  assert_string_equal("Hello, World!", buffer);

  char small_buffer[TEST_SMALL_BUFFER_SIZE];
  written =
    call_memory_safe_vsnprintf(small_buffer, sizeof(small_buffer), "Serial:%s", "ABC12345");
  assert_int_equal(-1, written);
  assert_string_equal("Serial:", small_buffer);

  written = call_memory_safe_vsnprintf(buffer, sizeof(buffer), "%d-%s", TEST_BUILD_YEAR, "ONVIF");
  assert_int_equal(9, written);
  assert_string_equal("2025-ONVIF", buffer);
}

void test_unit_string_shims(void** state) {
  (void)state;
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_case_insensitive_helpers),
  };
  cmocka_run_group_tests_name("string_shims_case_insensitive", tests, NULL, NULL);
}

void test_unit_string_validation(void** state) {
  (void)state;
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_trim_whitespace_helpers),
  };
  cmocka_run_group_tests_name("string_validation_helpers", tests, NULL, NULL);
}

void test_unit_string_manipulation(void** state) {
  (void)state;
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_memory_safe_formatting),
  };
  cmocka_run_group_tests_name("string_manipulation_helpers", tests, NULL, NULL);
}

void test_unit_string_search(void** state) {
  (void)state;
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_case_insensitive_helpers),
  };
  cmocka_run_group_tests_name("string_search_helpers", tests, NULL, NULL);
}

void test_unit_string_formatting(void** state) {
  (void)state;
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_memory_safe_formatting),
  };
  cmocka_run_group_tests_name("string_formatting_helpers", tests, NULL, NULL);
}

