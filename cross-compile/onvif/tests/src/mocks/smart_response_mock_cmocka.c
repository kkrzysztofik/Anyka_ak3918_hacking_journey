/**
 * @file smart_response_mock_cmocka.c
 * @brief Implementation of CMocka-based smart response builder mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "smart_response_mock_cmocka.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

/* ============================================================================
 * CMocka Wrapped Smart Response Functions
 * ============================================================================ */

int __wrap_smart_response_build_with_dynamic_buffer(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response) {
  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  return (int)mock();
}

int __wrap_smart_response_build_with_buffer_pool(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response, struct buffer_pool_t* pool) {
  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  check_expected_ptr(pool);
  return (int)mock();
}

int __wrap_smart_response_build(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response) {
  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  return (int)mock();
}

size_t __wrap_smart_response_estimate_size(struct soap* soap, int (*response_func)(struct soap*, const void*), const void* response) {
  check_expected_ptr(soap);
  check_expected_ptr(response_func);
  check_expected_ptr(response);
  return (size_t)mock();
}
