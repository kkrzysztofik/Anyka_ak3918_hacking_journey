/**
 * @file utils.c
 * @brief Utility function implementations.
 */

#include "utils.h"
#include <time.h>

/**
 * @brief Sleep for provided microseconds using nanosleep.
 * @param microseconds Duration (<=0 => return immediately).
 */
void sleep_us(int microseconds) {
    if (microseconds <= 0) return;
    struct timespec ts;
    ts.tv_sec = microseconds / 1000000;
    ts.tv_nsec = (long)(microseconds % 1000000) * 1000L;
    nanosleep(&ts, NULL);
}
