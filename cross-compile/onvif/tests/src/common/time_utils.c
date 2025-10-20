/**
 * @file time_utils.c
 * @brief Monotonic time helpers for tests (microsecond precision)
 * @author kkrzysztofik
 * @date 2025
 */

#include "time_utils.h"

#include <bits/time.h>
#include <bits/types/struct_timeval.h>
#include <sys/time.h>
#include <time.h>

static const long MICROS_PER_SECOND = 1000000L;
static const long NANOS_PER_MICROSECOND = 1000L;

long test_get_time_microseconds(void) {
#if defined(CLOCK_MONOTONIC)
  struct timespec monotonic_time;
  if (clock_gettime(CLOCK_MONOTONIC, &monotonic_time) == 0) {
    return (long)(monotonic_time.tv_sec * MICROS_PER_SECOND + monotonic_time.tv_nsec / NANOS_PER_MICROSECOND);
  }
#endif

  struct timeval wall_time;
  gettimeofday(&wall_time, NULL);
  return (long)(wall_time.tv_sec * MICROS_PER_SECOND + wall_time.tv_usec);
}
