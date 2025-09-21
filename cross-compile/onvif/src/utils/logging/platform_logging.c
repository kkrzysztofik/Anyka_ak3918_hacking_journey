/**
 * @file platform_logging.c
 * @brief Enhanced platform logging utilities with timestamps and log levels
 * @author kkrzysztofik
 * @date 2025
 */

#include "platform_logging.h"

#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>

/* Log level strings */
static const char* LOG_LEVEL_STRINGS[] = {"ERROR", "WARN ", "NOTICE", "INFO ",
                                          "DEBUG"};

/* Log level colors for terminal output */
static const char* LOG_LEVEL_COLORS[] = {
    "\033[1;31m", /* ERROR - Red */
    "\033[1;33m", /* WARN  - Yellow */
    "\033[1;36m", /* NOTICE - Cyan */
    "\033[1;32m", /* INFO  - Green */
    "\033[1;37m"  /* DEBUG - White */
};

static const char* COLOR_RESET = "\033[0m";

/* Global logging configuration */
static platform_logging_config_t g_log_config = {.enabled = true,  // NOLINT
                                                 .use_colors = true,
                                                 .use_timestamps = true,
                                                 .min_level = PLATFORM_LOG_INFO,
                                                 .tag = "ONVIF"};

/**
 * @brief Get current timestamp as formatted string
 * @param buffer Buffer to store timestamp string (must be at least 32 bytes)
 * @param size Size of the buffer
 * @return 0 on success, -1 on error
 */
static int get_timestamp(char* buffer, size_t size) {
  if (!buffer || size < 32) {
    return -1;
  }

  struct timeval tv;
  struct tm* tm_info;

  if (gettimeofday(&tv, NULL) != 0) {
    return -1;
  }

  tm_info = localtime(&tv.tv_sec);
  if (!tm_info) {
    return -1;
  }

  /* Format: YYYY-MM-DD HH:MM:SS.mmm */
  int written = strftime(buffer, size, "%Y-%m-%d %H:%M:%S", tm_info);
  if (written == 0) {
    return -1;
  }

  /* Add milliseconds */
  snprintf(buffer + written, size - written, ".%03ld", tv.tv_usec / 1000);

  return 0;
}

/**
 * @brief Check if a log level should be printed
 * @param level Log level to check
 * @return true if should be printed, false otherwise
 */
static bool should_log(platform_log_level_t level) {
  return g_log_config.enabled && level <= g_log_config.min_level;
}

/**
 * @brief Print log message with enhanced formatting
 * @param level Log level
 * @param file Source file name
 * @param function Function name
 * @param line Line number
 * @param format Printf-style format string
 * @param args Variable arguments
 * @return Number of characters printed
 */
int platform_log_printf(platform_log_level_t level, const char* file,
                        const char* function, int line, const char* format,
                        va_list args) {
  if (!should_log(level)) {
    return 0;
  }

  char timestamp[32];
  int chars_written = 0;

  /* Get timestamp if enabled */
  if (g_log_config.use_timestamps) {
    if (get_timestamp(timestamp, sizeof(timestamp)) != 0) {
      strcpy(timestamp, "0000-00-00 00:00:00.000");
    }
  } else {
    strcpy(timestamp, "");
  }

  /* Start with timestamp */
  if (g_log_config.use_timestamps) {
    chars_written += printf("[%s] ", timestamp);
  }

  /* Add log level with color if enabled */
  if (g_log_config.use_colors) {
    chars_written += printf("%s[%s]%s ", LOG_LEVEL_COLORS[level],
                            LOG_LEVEL_STRINGS[level], COLOR_RESET);
  } else {
    chars_written += printf("[%s] ", LOG_LEVEL_STRINGS[level]);
  }

  /* Add tag */
  chars_written += printf("[%s] ", g_log_config.tag);

  /* Add source location for debug and error levels */
  if (level <= PLATFORM_LOG_DEBUG) {
    const char* filename = strrchr(file, '/');
    filename = filename ? filename + 1 : file;
    chars_written += printf("<%s:%s:%d> ", filename, function, line);
  }

  /* Print the actual message */
  chars_written += vprintf(format, args);

  /* Ensure newline */
  if (format[strlen(format) - 1] != '\n') {
    chars_written += printf("\n");
  }

  fflush(stdout);
  return chars_written;
}

/**
 * @brief Set logging configuration
 * @param config New logging configuration
 */
void platform_logging_set_config(const platform_logging_config_t* config) {
  if (config) {
    g_log_config = *config;
  }
}

/**
 * @brief Get current logging configuration
 * @param config Pointer to store current configuration
 */
void platform_logging_get_config(platform_logging_config_t* config) {
  if (config) {
    *config = g_log_config;
  }
}

/**
 * @brief Set minimum log level
 * @param level Minimum log level to print
 */
void platform_logging_set_level(platform_log_level_t level) {
  g_log_config.min_level = level;
}

/**
 * @brief Enable or disable logging
 * @param enabled true to enable logging, false to disable
 */
void platform_logging_set_enabled(bool enabled) {
  g_log_config.enabled = enabled;
}

/**
 * @brief Set logging tag
 * @param tag New tag string (max 31 characters)
 */
void platform_logging_set_tag(const char* tag) {
  if (tag) {
    strncpy(g_log_config.tag, tag, sizeof(g_log_config.tag) - 1);
    g_log_config.tag[sizeof(g_log_config.tag) - 1] = '\0';
  }
}
