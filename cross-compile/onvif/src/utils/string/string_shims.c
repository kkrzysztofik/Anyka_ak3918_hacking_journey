/**
 * @file string_shims.c
 * @brief String function shims implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "string_shims.h"

#include <ctype.h>
#include <string.h>


/* Fallback implementation for missing strcasecmp function */
#ifndef strcasecmp
int strcasecmp(const char *s1, const char *s2) {
  if (!s1 || !s2) {
    if (s1 == s2) return 0;
    return s1 ? 1 : -1;
  }

  while (*s1 && *s2) {
    int c1 = tolower((unsigned char)*s1);
    int c2 = tolower((unsigned char)*s2);

    if (c1 != c2) {
      return c1 - c2;
    }

    s1++;
    s2++;
  }

  return tolower((unsigned char)*s1) - tolower((unsigned char)*s2);
}
#endif

/* Fallback implementation for missing strcasestr function */
#ifndef strcasestr
char *strcasestr(const char *haystack, const char *needle) {
  if (!haystack || !needle) {
    return NULL;
  }

  if (*needle == '\0') {
    return (char *)haystack;
  }

  for (; *haystack; haystack++) {
    const char *h = haystack;
    const char *n = needle;

    while (*h && *n &&
           tolower((unsigned char)*h) == tolower((unsigned char)*n)) {
      h++;
      n++;
    }

    if (*n == '\0') {
      return (char *)haystack;
    }
  }

  return NULL;
}
#endif

/* Fallback implementation for missing strnlen function */
#ifndef strnlen
size_t strnlen(const char *s, size_t maxlen) {
  if (!s) {
    return 0;
  }

  const char *p = s;
  size_t len = 0;

  while (len < maxlen && *p) {
    p++;
    len++;
  }

  return len;
}
#endif

/* Trim leading and trailing whitespace from a string */
void trim_whitespace(char *s) {
  if (!s || *s == '\0') {
    return;
  }

  char *start = s;
  char *end = s + strlen(s) - 1;

  // Trim leading whitespace
  while (*start && (*start == ' ' || *start == '\t' || *start == '\n' ||
                    *start == '\r')) {
    start++;
  }

  // If the string was all whitespace, return empty string
  if (*start == '\0') {
    s[0] = '\0';
    return;
  }

  // Trim trailing whitespace
  while (end >= start &&
         (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
    *end = '\0';
    end--;
  }

  // Move trimmed string to beginning if needed
  if (start != s) {
    size_t new_len = (size_t)(end - start + 1);
    memmove(s, start, new_len);
    s[new_len] = '\0';
  } else {
    // Ensure null-termination after trimming
    s[end - start + 1] = '\0';
  }
}
