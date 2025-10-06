/**
 * @file test_memory_utils.c
 * @brief Unit tests for memory management utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/memory/memory_manager.h"
#include "utils/memory/smart_response_builder.h"

// Include mock control for conditional real/mock function usage
#include "mocks/smart_response_mock.h"

// Test constants for memory allocation sizes
#define TEST_ALLOC_SIZE           1024
#define TEST_ALLOC_LAST_INDEX     (TEST_ALLOC_SIZE - 1)
#define TEST_SMALL_ALLOC_SIZE     512
#define TEST_LARGE_ALLOC_SIZE     ((size_t)1024 * 1024)
#define TEST_MEDIUM_ALLOC_SIZE_1  100
#define TEST_MEDIUM_ALLOC_SIZE_2  200
#define TEST_ITERATION_ALLOC_SIZE 64
#define TEST_ITERATION_COUNT      10

/**
 * @brief Test memory manager initialization
 * @param state Test state (unused)
 */
void test_unit_memory_manager_init(void** state) {
  (void)state;

  // Test that memory manager can be initialized
  int result = memory_manager_init();
  assert_int_equal(result, 0);

  // Test that initialization is idempotent
  result = memory_manager_init();
  assert_int_equal(result, 0);

  // Cleanup
  memory_manager_cleanup();
}

/**
 * @brief Test memory allocation and deallocation
 * @param state Test state (unused)
 */
void test_unit_memory_manager_alloc(void** state) {
  (void)state;

  // Initialize memory manager
  memory_manager_init();

  // Test normal allocation using the actual API
  void* ptr = ONVIF_MALLOC(TEST_ALLOC_SIZE);
  assert_non_null(ptr);

  // Test that we can write to allocated memory
  char* char_ptr = (char*)ptr;
  char_ptr[0] = 'A';
  char_ptr[TEST_ALLOC_LAST_INDEX] = 'Z';
  assert_int_equal(char_ptr[0], 'A');
  assert_int_equal(char_ptr[TEST_ALLOC_LAST_INDEX], 'Z');

  // Test zero size allocation (behavior may vary)
  void* zero_ptr = ONVIF_MALLOC(0);
  if (zero_ptr) {
    ONVIF_FREE(zero_ptr);
  }

  // Test large allocation (should succeed on most systems)
  void* large_ptr = ONVIF_MALLOC(TEST_LARGE_ALLOC_SIZE);
  assert_non_null(large_ptr);
  ONVIF_FREE(large_ptr);

  // Cleanup
  ONVIF_FREE(ptr);
  memory_manager_cleanup();
}

/**
 * @brief Test memory manager free functionality
 * @param state Test state (unused)
 */
void test_unit_memory_manager_free(void** state) {
  (void)state;

  // Initialize memory manager
  memory_manager_init();

  // Test that free works
  void* ptr = ONVIF_MALLOC(TEST_SMALL_ALLOC_SIZE);
  assert_non_null(ptr);
  ONVIF_FREE(ptr);

  // Test free with NULL pointer (should not crash)
  ONVIF_FREE(NULL);

  // Cleanup
  memory_manager_cleanup();
}

/**
 * @brief Test smart response builder functionality
 * @param state Test state (unused)
 */
void test_unit_smart_response_builder(void** state) {
  (void)state;

  // Enable real functions for this utility behavior test
  // We're testing the actual implementation logic (NULL handling, strlen behavior)
  smart_response_mock_use_real_function(true);

  // Initialize for testing
  memory_manager_init();

  // Test smart response size estimation (basic functionality test)
  const char* soap_content = "<test>content</test>";
  size_t estimated_size = smart_response_estimate_size(soap_content);
  // Just test that the function doesn't crash and returns some value
  assert_true(estimated_size > 0); // Should return positive size for valid content

  // Test with empty content (should return 0 for empty string)
  size_t empty_size = smart_response_estimate_size("");
  assert_int_equal(empty_size, 0); // Empty string has length 0

  // Test with NULL content (should handle gracefully and return 0)
  size_t null_size = smart_response_estimate_size(NULL);
  assert_int_equal(null_size, 0); // NULL handling returns 0

  // Cleanup
  memory_manager_cleanup();

  // Restore mock behavior for other tests
  smart_response_mock_use_real_function(false);
}

/**
 * @brief Test memory manager statistics and logging
 * @param state Test state (unused)
 */
void test_unit_memory_manager_stats(void** state) {
  (void)state;

  // Initialize for testing
  memory_manager_init();

  // Test basic memory allocation and logging
  void* ptr1 = ONVIF_MALLOC(TEST_MEDIUM_ALLOC_SIZE_1);
  void* ptr2 = ONVIF_MALLOC(TEST_MEDIUM_ALLOC_SIZE_2);
  assert_non_null(ptr1);
  assert_non_null(ptr2);

  // Test logging stats (should not crash)
  memory_manager_log_stats();

  // Cleanup first
  ONVIF_FREE(ptr1);
  ONVIF_FREE(ptr2);

  // Test leak checking after cleanup (should be 0 leaks)
  int leak_result = memory_manager_check_leaks();
  // Verify no memory leaks after proper cleanup
  assert_int_equal(leak_result, 0); // Should return 0 when no leaks detected

  memory_manager_cleanup();
}

/**
 * @brief Test memory manager under stress conditions
 * @param state Test state (unused)
 */
void test_unit_memory_manager_stress(void** state) {
  (void)state;

  // Initialize for testing
  memory_manager_init();

  const int num_allocations = TEST_ITERATION_COUNT; // Very small for unit testing
  void* pointers[num_allocations];
  // Allocate many blocks
  for (int i = 0; i < num_allocations; i++) {
    pointers[i] = ONVIF_MALLOC(TEST_ITERATION_ALLOC_SIZE);
    assert_non_null(pointers[i]);
  }

  // Log stats (should handle many allocations)
  memory_manager_log_stats();

  // Free all blocks
  for (int i = 0; i < num_allocations; i++) {
    ONVIF_FREE(pointers[i]);
  }

  // Final check - verify no memory leaks after stress test
  int leak_result = memory_manager_check_leaks();
  assert_int_equal(leak_result, 0); // Should return 0 when all memory is properly freed

  // Cleanup
  memory_manager_cleanup();
}

/**
 * @brief Test dynamic buffer functionality
 * @param state Test state (unused)
 */
void test_unit_dynamic_buffer(void** state) {
  (void)state;

  dynamic_buffer_t buffer;

  // Test buffer initialization
  int result = dynamic_buffer_init(&buffer, 0);
  assert_int_equal(result, 0);

  // Test buffer operations
  const char* test_data = "Hello, World!";
  result = dynamic_buffer_append_string(&buffer, test_data);
  assert_int_equal(result, 0);

  // Test buffer data retrieval
  const char* data = dynamic_buffer_data(&buffer);
  assert_non_null(data);
  assert_string_equal(data, test_data);

  // Test buffer length
  size_t length = dynamic_buffer_length(&buffer);
  assert_int_equal(length, strlen(test_data));

  // Test buffer capacity
  size_t capacity = dynamic_buffer_capacity(&buffer);
  assert_true(capacity >= length);

  // Test buffer cleanup
  dynamic_buffer_cleanup(&buffer);
}

/**
 * @brief Get memory utils unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_memory_utils_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_memory_manager_init),
    cmocka_unit_test(test_unit_memory_manager_alloc),
    cmocka_unit_test(test_unit_memory_manager_free),
    cmocka_unit_test(test_unit_smart_response_builder),
    cmocka_unit_test(test_unit_memory_manager_stats),
    cmocka_unit_test(test_unit_memory_manager_stress),
    cmocka_unit_test(test_unit_dynamic_buffer),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}
