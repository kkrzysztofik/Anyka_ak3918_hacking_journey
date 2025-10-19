/**
 * @file time_utils.c
 * @brief Time utility functions implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "utils/common/time_utils.h"

#include <stddef.h>
#include <sys/time.h>
#include <time.h>

/* Time conversion constants */
#define TIME_MS_PER_SECOND  1000      /* Milliseconds per second */
#define TIME_US_PER_SECOND  1000000ULL /* Microseconds per second */
#define TIME_NS_PER_MS      1000000   /* Nanoseconds per millisecond */
#define TIME_NS_PER_US      1000      /* Nanoseconds per microsecond */

/* ============================================================================
 * PUBLIC API - Time Utility Functions
 * ============================================================================ */

/**
 * @brief Get current timestamp in microseconds
 * @return Timestamp in microseconds since epoch
 */
uint64_t get_timestamp_us(void) {
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return (uint64_t)tv.tv_sec * TIME_US_PER_SECOND + (uint64_t)tv.tv_usec;
}

/**
 * @brief Get current monotonic time in milliseconds
 * @return Monotonic time in milliseconds
 */
uint64_t get_time_ms(void) {
  struct timespec time_spec;
  clock_gettime(CLOCK_MONOTONIC, &time_spec);
  return (uint64_t)time_spec.tv_sec * TIME_MS_PER_SECOND + time_spec.tv_nsec / TIME_NS_PER_MS;
}

/**
 * @brief Get elapsed time in microseconds between two timestamps
 * @param start_time Start timestamp in microseconds
 * @param end_time End timestamp in microseconds
 * @return Elapsed time in microseconds
 */
uint64_t get_elapsed_time_us(uint64_t start_time, uint64_t end_time) {
  if (end_time < start_time) {
    return 0;
  }
  return end_time - start_time;
}

/**
 * @brief Sleep for specified milliseconds
 * @param milliseconds Number of milliseconds to sleep
 */
void sleep_ms(uint32_t milliseconds) {
  struct timespec time_spec;
  time_spec.tv_sec = (time_t)(milliseconds / TIME_MS_PER_SECOND);
  time_spec.tv_nsec = (long)((milliseconds % TIME_MS_PER_SECOND) * TIME_NS_PER_MS);
  nanosleep(&time_spec, NULL);
}

/**
 * @brief Sleep for specified microseconds
 * @param microseconds Number of microseconds to sleep
 */
void sleep_us(uint32_t microseconds) {
  struct timespec time_spec;
  time_spec.tv_sec = (time_t)(microseconds / TIME_US_PER_SECOND);
  time_spec.tv_nsec = (long)((microseconds % TIME_US_PER_SECOND) * TIME_NS_PER_US);
  nanosleep(&time_spec, NULL);
}
