/**
 * @file time_utils.h
 * @brief Time utility functions with microsecond precision
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides time-related utility functions for performance
 * measurement and timestamp generation with microsecond precision.
 */

#ifndef UTILS_TIME_UTILS_H
#define UTILS_TIME_UTILS_H

#include <stdint.h>

/**
 * @brief Get current timestamp in microseconds
 * @return Timestamp in microseconds since epoch
 * @note Uses gettimeofday() for compatibility with embedded systems
 */
uint64_t get_timestamp_us(void);

/**
 * @brief Get current monotonic time in milliseconds
 * @return Monotonic time in milliseconds
 * @note Uses CLOCK_MONOTONIC for consistent time measurements
 */
uint64_t get_time_ms(void);

/**
 * @brief Get elapsed time in microseconds between two timestamps
 * @param start_time Start timestamp in microseconds
 * @param end_time End timestamp in microseconds
 * @return Elapsed time in microseconds
 * @note Returns 0 if end_time is less than start_time
 */
uint64_t get_elapsed_time_us(uint64_t start_time, uint64_t end_time);

/**
 * @brief Sleep for specified milliseconds
 * @param milliseconds Number of milliseconds to sleep
 */
void sleep_ms(uint32_t milliseconds);

/**
 * @brief Sleep for specified microseconds
 * @param microseconds Number of microseconds to sleep
 */
void sleep_us(uint32_t microseconds);

#endif /* UTILS_TIME_UTILS_H */
