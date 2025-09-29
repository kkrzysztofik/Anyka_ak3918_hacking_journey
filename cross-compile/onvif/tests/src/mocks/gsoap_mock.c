/**
 * @file gsoap_mock.c
 * @brief Implementation of gsoap mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "gsoap_mock.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Mock state
static int g_gsoap_mock_initialized = 0;         // NOLINT
static int g_gsoap_init_result = 0;              // NOLINT
static int g_gsoap_cleanup_result = 0;           // NOLINT
static int g_gsoap_generate_response_result = 0; // NOLINT
static char* g_gsoap_response_data = NULL;       // NOLINT
static size_t g_gsoap_response_size = 0;         // NOLINT

// Call counters
static int g_gsoap_init_call_count = 0;              // NOLINT
static int g_gsoap_cleanup_call_count = 0;           // NOLINT
static int g_gsoap_generate_response_call_count = 0; // NOLINT
static int g_gsoap_get_response_data_call_count = 0; // NOLINT

/**
 * @brief Set mock gsoap init result
 * @param result Result to return
 * @return 0 on success
 */
int mock_gsoap_set_init_result(int result) {
  g_gsoap_init_result = result;
  return 0;
}

/**
 * @brief Set mock gsoap cleanup result
 * @param result Result to return
 * @return 0 on success
 */
int mock_gsoap_set_cleanup_result(int result) {
  g_gsoap_cleanup_result = result;
  return 0;
}

/**
 * @brief Set mock gsoap generate response result
 * @param result Result to return
 * @return 0 on success
 */
int mock_gsoap_set_generate_response_result(int result) {
  g_gsoap_generate_response_result = result;
  return 0;
}

/**
 * @brief Set mock gsoap get response data result
 * @param data Response data
 * @param size Size of data
 * @return 0 on success
 */
int mock_gsoap_set_get_response_data_result(const char* data, size_t size) {
  if (g_gsoap_response_data) {
    free(g_gsoap_response_data);
  }

  if (data && size > 0) {
    g_gsoap_response_data = malloc(size + 1);
    if (g_gsoap_response_data) {
      memcpy(g_gsoap_response_data, data, size);
      g_gsoap_response_data[size] = '\0';
      g_gsoap_response_size = size;
    }
  } else {
    g_gsoap_response_data = NULL;
    g_gsoap_response_size = 0;
  }

  return 0;
}

/**
 * @brief Get mock gsoap init call count
 * @return Number of init calls
 */
int mock_gsoap_get_init_call_count(void) {
  return g_gsoap_init_call_count;
}

/**
 * @brief Get mock gsoap cleanup call count
 * @return Number of cleanup calls
 */
int mock_gsoap_get_cleanup_call_count(void) {
  return g_gsoap_cleanup_call_count;
}

/**
 * @brief Get mock gsoap generate response call count
 * @return Number of generate response calls
 */
int mock_gsoap_get_generate_response_call_count(void) {
  return g_gsoap_generate_response_call_count;
}

/**
 * @brief Get mock gsoap get response data call count
 * @return Number of get response data calls
 */
int mock_gsoap_get_get_response_data_call_count(void) {
  return g_gsoap_get_response_data_call_count;
}

/**
 * @brief Initialize gsoap mock
 */
void gsoap_mock_init(void) {
  g_gsoap_mock_initialized = 1;
  g_gsoap_init_result = 0;
  g_gsoap_cleanup_result = 0;
  g_gsoap_generate_response_result = 0;
  g_gsoap_response_data = NULL;
  g_gsoap_response_size = 0;
  g_gsoap_init_call_count = 0;
  g_gsoap_cleanup_call_count = 0;
  g_gsoap_generate_response_call_count = 0;
  g_gsoap_get_response_data_call_count = 0;
}

/**
 * @brief Cleanup gsoap mock
 */
void gsoap_mock_cleanup(void) {
  g_gsoap_mock_initialized = 0;

  if (g_gsoap_response_data) {
    free(g_gsoap_response_data);
    g_gsoap_response_data = NULL;
  }

  g_gsoap_response_size = 0;
  g_gsoap_init_call_count = 0;
  g_gsoap_cleanup_call_count = 0;
  g_gsoap_generate_response_call_count = 0;
  g_gsoap_get_response_data_call_count = 0;
}
