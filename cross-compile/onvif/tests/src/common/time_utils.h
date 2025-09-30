/**
 * @file time_utils.h
 * @brief Monotonic time helpers for tests (microsecond precision)
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_TIME_UTILS_H
#define TEST_TIME_UTILS_H

/**
 * @brief Get current monotonic time in microseconds
 * @return Current monotonic time in microseconds
 * @note Uses CLOCK_MONOTONIC when available, falls back to gettimeofday()
 */
long test_get_time_microseconds(void);

#endif /* TEST_TIME_UTILS_H */
