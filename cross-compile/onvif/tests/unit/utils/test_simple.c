/**
 * @file test_simple.c
 * @brief Simple unit test to verify basic framework functionality
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include <stdarg.h>
#include <setjmp.h>
#include <cmocka.h>

/**
 * @brief Simple test that always passes
 * @param state Test state (unused)
 */
void test_basic_pass(void** state) {
  (void)state;
  assert_int_equal(1, 1);
}

/**
 * @brief Test basic string operations
 * @param state Test state (unused)
 */
void test_basic_string(void** state) {
  (void)state;

  const char* test_str = "Hello, World!";
  assert_int_equal(strlen(test_str), 13);
  assert_string_equal(test_str, "Hello, World!");
}

/**
 * @brief Test basic memory operations
 * @param state Test state (unused)
 */
void test_basic_memory(void** state) {
  (void)state;

  void* ptr = malloc(100);
  assert_non_null(ptr);

  // Write something to memory
  char* char_ptr = (char*)ptr;
  strcpy(char_ptr, "test");
  assert_string_equal(char_ptr, "test");

  free(ptr);
}

// Test functions are called from test_runner.c