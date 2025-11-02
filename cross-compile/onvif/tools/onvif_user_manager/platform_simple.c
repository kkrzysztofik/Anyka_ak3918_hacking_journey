/**
 * @file platform_simple.c
 * @brief Simplified platform implementation for command-line tools
 * @author kkrzysztofik
 * @date 2025
 */

#define _GNU_SOURCE

#include <limits.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#include "platform/platform.h"
#include "platform/platform_common.h"

platform_result_t platform_init(void) {
  // Simple initialization - just return success
  return PLATFORM_SUCCESS;
}

void platform_cleanup(void) {
  // Simple cleanup - no return value
}

int platform_log_error(const char* format, ...) {
  va_list args;
  va_start(args, format);
  (void)vfprintf(stderr, format, args);
  va_end(args);
  return 0;
}

int platform_log_warning(const char* format, ...) {
  va_list args;
  va_start(args, format);
  (void)vfprintf(stderr, format, args);
  va_end(args);
  return 0;
}

int platform_log_notice(const char* format, ...) {
  va_list args;
  va_start(args, format);
  (void)vfprintf(stdout, format, args);
  va_end(args);
  return 0;
}

int platform_log_info(const char* format, ...) {
  va_list args;
  va_start(args, format);
  (void)vfprintf(stdout, format, args);
  va_end(args);
  return 0;
}

int platform_log_debug(const char* format, ...) {
  // Debug logging disabled for simple tool
  (void)format;
  return 0;
}

platform_result_t platform_get_executable_path(char* path_buffer, size_t buffer_size) {
  if (path_buffer == NULL || buffer_size == 0) {
    return PLATFORM_ERROR;
  }

  // Try to read from /proc/self/exe (Linux)
  ssize_t len = readlink("/proc/self/exe", path_buffer, buffer_size - 1);
  if (len > 0) {
    path_buffer[len] = '\0';
    return PLATFORM_SUCCESS;
  }

  // Fallback: use current working directory
  if (getcwd(path_buffer, buffer_size) != NULL) {
    return PLATFORM_SUCCESS;
  }

  return PLATFORM_ERROR;
}
