/**
 * @file smart_response_mock.c
 * @brief Implementation of smart response mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "smart_response_mock.h"

// Mock state
static int g_smart_response_mock_initialized = 0; // NOLINT
static int g_smart_response_build_result = 0;     // NOLINT

// Call counters
static int g_smart_response_build_call_count = 0; // NOLINT

/**
 * @brief Set mock smart response build result
 * @param result Result to return
 * @return 0 on success
 */
int mock_smart_response_set_build_result(int result) {
  g_smart_response_build_result = result;
  return 0;
}

/**
 * @brief Get mock smart response build call count
 * @return Number of build calls
 */
int mock_smart_response_get_build_call_count(void) {
  return g_smart_response_build_call_count;
}

/**
 * @brief Initialize smart response mock
 */
void smart_response_mock_init(void) {
  g_smart_response_mock_initialized = 1;
  g_smart_response_build_result = 0;
  g_smart_response_build_call_count = 0;
}

/**
 * @brief Cleanup smart response mock
 */
void smart_response_mock_cleanup(void) {
  g_smart_response_mock_initialized = 0;
  g_smart_response_build_result = 0;
  g_smart_response_build_call_count = 0;
}
