/**
 * @file test_basic.c
 * @brief Basic framework validation tests
 * @author kkrzysztofik
 * @date 2025
 */

#include <string.h>

#include "cmocka_wrapper.h"

/**
 * @brief Test that always passes
 * @param state Test state (unused)
 */
void test_unit_basic_pass(void** state) {
  (void)state;
  assert_true(1);
}

/**
 * @brief Test basic string operations
 * @param state Test state (unused)
 */
void test_unit_basic_string(void** state) {
  (void)state;
  const char* test_str = "Hello, World!";
  assert_string_equal(test_str, "Hello, World!");
  assert_int_equal(strlen(test_str), 13);
}

/**
 * @brief Test basic memory operations
 * @param state Test state (unused)
 */
void test_unit_basic_memory(void** state) {
  (void)state;
  char buffer[10];
  memset(buffer, 0, sizeof(buffer));
  assert_int_equal(buffer[0], 0);
  assert_int_equal(buffer[9], 0);
}

/**
 * @brief Get basic unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_basic_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_basic_pass),
    cmocka_unit_test(test_unit_basic_string),
    cmocka_unit_test(test_unit_basic_memory),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
