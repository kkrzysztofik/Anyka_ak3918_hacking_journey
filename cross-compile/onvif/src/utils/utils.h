/**
 * @file utils.h
 * @brief Generic small utility helpers (timing etc.).
 */
#ifndef ONVIF_UTILS_H
#define ONVIF_UTILS_H

/**
 * @brief Sleep for a number of microseconds (best-effort).
 * @param microseconds Duration (<=0 is a no-op).
 */
void sleep_us(int microseconds);

#endif
