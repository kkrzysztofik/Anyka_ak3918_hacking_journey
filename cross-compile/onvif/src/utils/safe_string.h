/**
 * @file safe_string.h
 * @brief Safe string manipulation utilities to replace unsafe C functions.
 * 
 * This module provides safe alternatives to strcpy, strcat, sprintf, etc.
 * with proper bounds checking and error handling.
 */

#ifndef ONVIF_SAFE_STRING_H
#define ONVIF_SAFE_STRING_H

#include <stddef.h>
#include <stdarg.h>
#include <stdio.h>

/**
 * @brief Safe string copy with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_strcpy(char *dest, size_t dest_size, const char *src);

/**
 * @brief Safe string concatenation with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string to append
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_strcat(char *dest, size_t dest_size, const char *src);

/**
 * @brief Safe string copy with length limit
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string
 * @param max_len Maximum number of characters to copy
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_strncpy(char *dest, size_t dest_size, const char *src, size_t max_len);

/**
 * @brief Safe sprintf with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (excluding null terminator), -1 on error
 */
int safe_sprintf(char *dest, size_t dest_size, const char *format, ...);

/**
 * @brief Safe vsprintf with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param format Format string
 * @param args Variable arguments
 * @return Number of characters written (excluding null terminator), -1 on error
 */
int safe_vsprintf(char *dest, size_t dest_size, const char *format, va_list args);

/**
 * @brief Safe string concatenation with formatting
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param format Format string
 * @param ... Variable arguments
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_strcatf(char *dest, size_t dest_size, const char *format, ...);

/**
 * @brief Safe string length check
 * @param str String to check
 * @param max_len Maximum allowed length
 * @return 0 if string is within bounds, -1 if too long
 */
int safe_strlen_check(const char *str, size_t max_len);

/**
 * @brief Safe string duplication
 * @param src Source string
 * @return Newly allocated string copy, NULL on error
 */
char* safe_strdup(const char *src);

/**
 * @brief Safe string duplication with length limit
 * @param src Source string
 * @param max_len Maximum length to copy
 * @return Newly allocated string copy, NULL on error
 */
char* safe_strndup(const char *src, size_t max_len);

/**
 * @brief Safe string comparison with length limit
 * @param s1 First string
 * @param s2 Second string
 * @param max_len Maximum number of characters to compare
 * @return 0 if equal, negative if s1 < s2, positive if s1 > s2
 */
int safe_strncmp(const char *s1, const char *s2, size_t max_len);

/**
 * @brief Safe string search with bounds checking
 * @param haystack String to search in
 * @param haystack_size Size of haystack buffer
 * @param needle String to search for
 * @return Pointer to first occurrence, NULL if not found
 */
const char* safe_strstr(const char *haystack, size_t haystack_size, const char *needle);

/**
 * @brief Safe string tokenization
 * @param str String to tokenize (modified in place)
 * @param delim Delimiter characters
 * @param saveptr Pointer to save position for subsequent calls
 * @return Next token, NULL if no more tokens
 */
char* safe_strtok_r(char *str, const char *delim, char **saveptr);

/**
 * @brief Safe string trimming (remove leading/trailing whitespace)
 * @param str String to trim (modified in place)
 * @param str_size Size of string buffer
 * @return Trimmed string (same pointer)
 */
char* safe_strtrim(char *str, size_t str_size);

/**
 * @brief Safe string escape for XML
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_xml_escape(char *dest, size_t dest_size, const char *src);

/**
 * @brief Safe string unescape from XML
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string
 * @return 0 on success, -1 on error (buffer too small)
 */
int safe_xml_unescape(char *dest, size_t dest_size, const char *src);

#endif /* ONVIF_SAFE_STRING_H */
