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
  return (uint64_t)tv.tv_sec * 1000000ULL + (uint64_t)tv.tv_usec;
}

/**
 * @brief Get current monotonic time in milliseconds
 * @return Monotonic time in milliseconds
 */
uint64_t get_time_ms(void) {
  struct timespec time_spec;
  clock_gettime(CLOCK_MONOTONIC, &time_spec);
  return (uint64_t)time_spec.tv_sec * 1000 + time_spec.tv_nsec / 1000000;
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
  time_spec.tv_sec = (time_t)(milliseconds / 1000);
  time_spec.tv_nsec = (long)((milliseconds % 1000) * 1000000);
  nanosleep(&time_spec, NULL);
}

/**
 * @brief Sleep for specified microseconds
 * @param microseconds Number of microseconds to sleep
 */
void sleep_us(uint32_t microseconds) {
  struct timespec time_spec;
  time_spec.tv_sec = (time_t)(microseconds / 1000000);
  time_spec.tv_nsec = (long)((microseconds % 1000000) * 1000);
  nanosleep(&time_spec, NULL);
}
