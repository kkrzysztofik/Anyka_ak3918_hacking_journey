/**
 * @file memory_mock_cmocka.c
 * @brief Implementation of CMocka-based memory allocation mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "memory_mock_cmocka.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>

#include <cmocka.h>

/* ============================================================================
 * CMocka Wrapped Memory Functions
 * ============================================================================ */

void* __wrap_malloc(size_t size) {
  check_expected(size);
  return (void*)mock();
}

void* __wrap_calloc(size_t nmemb, size_t size) {
  check_expected(nmemb);
  check_expected(size);
  return (void*)mock();
}

void* __wrap_realloc(void* ptr, size_t size) {
  check_expected_ptr(ptr);
  check_expected(size);
  return (void*)mock();
}

void __wrap_free(void* ptr) {
  check_expected_ptr(ptr);
}
