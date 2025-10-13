/**
 * @file smart_response_mock.c
 * @brief Implementation of CMocka-based smart response builder mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "smart_response_mock.h"

#include <stdbool.h>
#include <stddef.h>

#include "cmocka_wrapper.h"
#include "networking/http/http_parser.h"


/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void smart_response_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * CMocka Wrapped Smart Response Functions
 * ============================================================================ */

int __wrap_smart_response_build_with_dynamic_buffer(http_response_t* response,
                                                    const char* soap_content) {
  if (g_use_real_functions) {
    return __real_smart_response_build_with_dynamic_buffer(response, soap_content);
  }

  check_expected_ptr(response);
  check_expected_ptr(soap_content);
  return (int)mock();
}

int __wrap_smart_response_build_with_buffer_pool(struct soap* soap,
                                                 int (*response_func)(struct soap*, const void*),
                                                 const void* response, struct buffer_pool_t* pool) {
  if (g_use_real_functions) {
    return __real_smart_response_build_with_buffer_pool(soap, response_func, response, pool);
  }

  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  check_expected_ptr(pool);
  return (int)mock();
}

int __wrap_smart_response_build(struct soap* soap, int (*response_func)(struct soap*, const void*),
                                const void* response) {
  if (g_use_real_functions) {
    return __real_smart_response_build(soap, response_func, response);
  }

  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  return (int)mock();
}

size_t __wrap_smart_response_estimate_size(struct soap* soap,
                                           int (*response_func)(struct soap*, const void*),
                                           const void* response) {
  if (g_use_real_functions) {
    return __real_smart_response_estimate_size(soap, response_func, response);
  }

  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  return (size_t)mock();
}
