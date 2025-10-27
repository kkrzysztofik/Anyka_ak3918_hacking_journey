/**
 * @file thread_mock.c
 * @brief CMocka-based pthread mock implementation for deterministic unit tests
 *
 * These wrappers execute the provided start routine synchronously to avoid
 * concurrency side effects during unit testing while still validating that the
 * deferred work is triggered.
 */

#include "thread_mock.h"

#include <stdbool.h>

#include "cmocka_wrapper.h"

static bool g_use_real_functions = true;

void thread_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

extern int __real_pthread_create(pthread_t* thread, const pthread_attr_t* attr, void* (*start_routine)(void*), void* arg);
extern int __real_pthread_detach(pthread_t thread);

int __wrap_pthread_create(pthread_t* thread, const pthread_attr_t* attr, void* (*start_routine)(void*), void* arg) {
  if (g_use_real_functions) {
    return __real_pthread_create(thread, attr, start_routine, arg);
  }

  (void)attr;

  if (thread != NULL) {
    *thread = 0;
  }

  if (start_routine != NULL) {
    (void)start_routine(arg);
  }

  return 0;
}

int __wrap_pthread_detach(pthread_t thread) {
  if (g_use_real_functions) {
    return __real_pthread_detach(thread);
  }

  (void)thread;
  return 0;
}
