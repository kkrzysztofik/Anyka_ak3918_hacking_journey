/**
 * @file safe_string.c
 * @brief Safe string manipulation utilities implementation.
 */

#include "safe_string.h"
#include "platform/platform.h"
#include <string.h>
#include <stdlib.h>
#include <ctype.h>

int safe_strcpy(char *dest, size_t dest_size, const char *src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }
  
  size_t src_len = strlen(src);
  if (src_len >= dest_size) {
    platform_log_error("safe_strcpy: source string too long (%zu >= %zu)\n", src_len, dest_size);
    return -1;
  }
  
  strcpy(dest, src);
  return 0;
}

int safe_strcat(char *dest, size_t dest_size, const char *src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }
  
  size_t dest_len = strlen(dest);
  size_t src_len = strlen(src);
  
  if (dest_len + src_len >= dest_size) {
    platform_log_error("safe_strcat: result would exceed buffer size (%zu + %zu >= %zu)\n", 
                       dest_len, src_len, dest_size);
    return -1;
  }
  
  strcat(dest, src);
  return 0;
}

int safe_strncpy(char *dest, size_t dest_size, const char *src, size_t max_len) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }
  
  size_t copy_len = max_len;
  if (copy_len >= dest_size) {
    copy_len = dest_size - 1;
  }
  
  strncpy(dest, src, copy_len);
  dest[copy_len] = '\0';
  return 0;
}

int safe_sprintf(char *dest, size_t dest_size, const char *format, ...) {
  if (!dest || !format || dest_size == 0) {
    return -1;
  }
  
  va_list args;
  va_start(args, format);
  int result = safe_vsprintf(dest, dest_size, format, args);
  va_end(args);
  return result;
}

int safe_vsprintf(char *dest, size_t dest_size, const char *format, va_list args) {
  if (!dest || !format || dest_size == 0) {
    return -1;
  }
  
  int len = vsnprintf(dest, dest_size, format, args);
  if (len < 0) {
    platform_log_error("safe_vsprintf: vsnprintf failed\n");
    return -1;
  }
  
  if ((size_t)len >= dest_size) {
    platform_log_error("safe_vsprintf: formatted string too long (%d >= %zu)\n", len, dest_size);
    return -1;
  }
  
  return len;
}

int safe_strcatf(char *dest, size_t dest_size, const char *format, ...) {
  if (!dest || !format || dest_size == 0) {
    return -1;
  }
  
  size_t dest_len = strlen(dest);
  if (dest_len >= dest_size) {
    platform_log_error("safe_strcatf: destination buffer already full\n");
    return -1;
  }
  
  va_list args;
  va_start(args, format);
  int len = vsnprintf(dest + dest_len, dest_size - dest_len, format, args);
  va_end(args);
  
  if (len < 0) {
    platform_log_error("safe_strcatf: vsnprintf failed\n");
    return -1;
  }
  
  if ((size_t)len >= dest_size - dest_len) {
    platform_log_error("safe_strcatf: formatted string too long (%d >= %zu)\n", 
                       len, dest_size - dest_len);
    return -1;
  }
  
  return 0;
}

int safe_strlen_check(const char *str, size_t max_len) {
  if (!str) {
    return -1;
  }
  
  size_t len = strlen(str);
  if (len > max_len) {
    platform_log_error("safe_strlen_check: string too long (%zu > %zu)\n", len, max_len);
    return -1;
  }
  
  return 0;
}

char* safe_strdup(const char *src) {
  if (!src) {
    return NULL;
  }
  
  size_t len = strlen(src) + 1;
  char *dest = malloc(len);
  if (!dest) {
    platform_log_error("safe_strdup: malloc failed for %zu bytes\n", len);
    return NULL;
  }
  
  strcpy(dest, src);
  return dest;
}

char* safe_strndup(const char *src, size_t max_len) {
  if (!src) {
    return NULL;
  }
  
  size_t src_len = strlen(src);
  size_t copy_len = (src_len < max_len) ? src_len : max_len;
  
  char *dest = malloc(copy_len + 1);
  if (!dest) {
    platform_log_error("safe_strndup: malloc failed for %zu bytes\n", copy_len + 1);
    return NULL;
  }
  
  strncpy(dest, src, copy_len);
  dest[copy_len] = '\0';
  return dest;
}

int safe_strncmp(const char *s1, const char *s2, size_t max_len) {
  if (!s1 || !s2) {
    return -1;
  }
  
  return strncmp(s1, s2, max_len);
}

const char* safe_strstr(const char *haystack, size_t haystack_size, const char *needle) {
  if (!haystack || !needle || haystack_size == 0) {
    return NULL;
  }
  
  size_t needle_len = strlen(needle);
  if (needle_len == 0) {
    return haystack;
  }
  
  if (needle_len > haystack_size) {
    return NULL;
  }
  
  for (size_t i = 0; i <= haystack_size - needle_len; i++) {
    if (strncmp(haystack + i, needle, needle_len) == 0) {
      return haystack + i;
    }
  }
  
  return NULL;
}

char* safe_strtok_r(char *str, const char *delim, char **saveptr) {
  if (!delim || !saveptr) {
    return NULL;
  }
  
  return strtok_r(str, delim, saveptr);
}

char* safe_strtrim(char *str, size_t str_size) {
  if (!str || str_size == 0) {
    return str;
  }
  
  // Trim leading whitespace
  char *start = str;
  while (*start && isspace((unsigned char)*start)) {
    start++;
  }
  
  // Trim trailing whitespace
  char *end = str + str_size - 1;
  while (end > start && isspace((unsigned char)*end)) {
    *end = '\0';
    end--;
  }
  
  // Move trimmed string to beginning if needed
  if (start != str) {
    size_t len = strlen(start) + 1;
    memmove(str, start, len);
  }
  
  return str;
}

int safe_xml_escape(char *dest, size_t dest_size, const char *src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }
  
  size_t dest_pos = 0;
  const char *src_pos = src;
  
  while (*src_pos && dest_pos < dest_size - 1) {
    switch (*src_pos) {
      case '<':
        if (dest_pos + 4 < dest_size) {
          strcpy(dest + dest_pos, "&lt;");
          dest_pos += 4;
        } else {
          goto buffer_full;
        }
        break;
      case '>':
        if (dest_pos + 4 < dest_size) {
          strcpy(dest + dest_pos, "&gt;");
          dest_pos += 4;
        } else {
          goto buffer_full;
        }
        break;
      case '&':
        if (dest_pos + 5 < dest_size) {
          strcpy(dest + dest_pos, "&amp;");
          dest_pos += 5;
        } else {
          goto buffer_full;
        }
        break;
      case '"':
        if (dest_pos + 6 < dest_size) {
          strcpy(dest + dest_pos, "&quot;");
          dest_pos += 6;
        } else {
          goto buffer_full;
        }
        break;
      case '\'':
        if (dest_pos + 6 < dest_size) {
          strcpy(dest + dest_pos, "&apos;");
          dest_pos += 6;
        } else {
          goto buffer_full;
        }
        break;
      default:
        dest[dest_pos++] = *src_pos;
        break;
    }
    src_pos++;
  }
  
  dest[dest_pos] = '\0';
  return 0;
  
buffer_full:
  platform_log_error("safe_xml_escape: buffer too small for escaped string\n");
  return -1;
}

int safe_xml_unescape(char *dest, size_t dest_size, const char *src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }
  
  size_t dest_pos = 0;
  const char *src_pos = src;
  
  while (*src_pos && dest_pos < dest_size - 1) {
    if (*src_pos == '&') {
      if (strncmp(src_pos, "&lt;", 4) == 0) {
        dest[dest_pos++] = '<';
        src_pos += 4;
      } else if (strncmp(src_pos, "&gt;", 4) == 0) {
        dest[dest_pos++] = '>';
        src_pos += 4;
      } else if (strncmp(src_pos, "&amp;", 5) == 0) {
        dest[dest_pos++] = '&';
        src_pos += 5;
      } else if (strncmp(src_pos, "&quot;", 6) == 0) {
        dest[dest_pos++] = '"';
        src_pos += 6;
      } else if (strncmp(src_pos, "&apos;", 6) == 0) {
        dest[dest_pos++] = '\'';
        src_pos += 6;
      } else {
        dest[dest_pos++] = *src_pos++;
      }
    } else {
      dest[dest_pos++] = *src_pos++;
    }
  }
  
  dest[dest_pos] = '\0';
  return 0;
}
