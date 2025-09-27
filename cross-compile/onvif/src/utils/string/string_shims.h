/**
 * @file string_shims.h
 * @brief String function shims for missing standard library functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_STRING_SHIMS_H
#define ONVIF_STRING_SHIMS_H

#include <stdarg.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Case-insensitive string comparison
 * @param s1 First string to compare
 * @param s2 Second string to compare
 * @return Negative if s1 < s2, 0 if equal, positive if s1 > s2
 * @note This function is not available on all platforms
 */
int strcasecmp(const char* s1, const char* s2);

/**
 * @brief Case-insensitive string search
 * @param haystack String to search in
 * @param needle String to search for
 * @return Pointer to first occurrence, NULL if not found
 * @note This function is not available on all platforms
 */
char* strcasestr(const char* haystack, const char* needle);

/**
 * @brief Get length of string with maximum limit
 * @param s String to measure
 * @param maxlen Maximum length to check
 * @return Length of string or maxlen, whichever is smaller
 * @note This function is not available on all platforms
 */
size_t strnlen(const char* s, size_t maxlen);

/**
 * @brief Trim leading and trailing whitespace from a string
 * @param s String to trim (modified in place)
 * @note Trims spaces, tabs, newlines, and carriage returns
 */
void trim_whitespace(char* s);

/**
 * @brief Safe version of vsnprintf with bounds checking
 * @param str Buffer to write to
 * @param size Size of the buffer
 * @param format Format string
 * @param args Variable arguments
 * @return Number of characters written (excluding null terminator), or -1 on
 * error
 * @note Ensures null termination and prevents buffer overflow
 */
int memory_safe_vsnprintf(char* str, size_t size, const char* format, va_list args);

#ifdef __cplusplus
}
#endif

#endif /* ONVIF_STRING_SHIMS_H */
