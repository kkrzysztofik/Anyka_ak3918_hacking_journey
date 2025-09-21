/**
 * @file platform_logging.h
 * @brief Enhanced platform logging utilities with timestamps and log levels
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PLATFORM_LOGGING_H
#define PLATFORM_LOGGING_H

#include <stdbool.h>
#include <stdarg.h>
#include <stddef.h>

/**
 * @brief Log levels in order of severity
 */
typedef enum {
  PLATFORM_LOG_ERROR = 0,    /**< Error messages */
  PLATFORM_LOG_WARNING,      /**< Warning messages */
  PLATFORM_LOG_NOTICE,       /**< Notice messages */
  PLATFORM_LOG_INFO,         /**< Informational messages */
  PLATFORM_LOG_DEBUG,        /**< Debug messages */
} platform_log_level_t;

/**
 * @brief Logging configuration structure
 */
typedef struct {
  bool enabled;                    /**< Enable/disable logging */
  bool use_colors;                 /**< Enable/disable color output */
  bool use_timestamps;             /**< Enable/disable timestamps */
  platform_log_level_t min_level; /**< Minimum log level to print */
  char tag[32];                    /**< Log tag identifier */
} platform_logging_config_t;

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
                       va_list args);

/**
 * @brief Set logging configuration
 * @param config New logging configuration
 */
void platform_logging_set_config(const platform_logging_config_t* config);

/**
 * @brief Get current logging configuration
 * @param config Pointer to store current configuration
 */
void platform_logging_get_config(platform_logging_config_t* config);

/**
 * @brief Set minimum log level
 * @param level Minimum log level to print
 */
void platform_logging_set_level(platform_log_level_t level);

/**
 * @brief Enable or disable logging
 * @param enabled true to enable logging, false to disable
 */
void platform_logging_set_enabled(bool enabled);

/**
 * @brief Set logging tag
 * @param tag New tag string (max 31 characters)
 */
void platform_logging_set_tag(const char* tag);

/* Convenience macros for logging */
#define PLATFORM_LOG_ERROR(format, ...) \
  do { \
    va_list args; \
    va_start(args, format); \
    platform_log_printf(PLATFORM_LOG_ERROR, __FILE__, __FUNCTION__, __LINE__, format, args); \
    va_end(args); \
  } while(0)

#define PLATFORM_LOG_WARNING(format, ...) \
  do { \
    va_list args; \
    va_start(args, format); \
    platform_log_printf(PLATFORM_LOG_WARNING, __FILE__, __FUNCTION__, __LINE__, format, args); \
    va_end(args); \
  } while(0)

#define PLATFORM_LOG_NOTICE(format, ...) \
  do { \
    va_list args; \
    va_start(args, format); \
    platform_log_printf(PLATFORM_LOG_NOTICE, __FILE__, __FUNCTION__, __LINE__, format, args); \
    va_end(args); \
  } while(0)

#define PLATFORM_LOG_INFO(format, ...) \
  do { \
    va_list args; \
    va_start(args, format); \
    platform_log_printf(PLATFORM_LOG_INFO, __FILE__, __FUNCTION__, __LINE__, format, args); \
    va_end(args); \
  } while(0)

#define PLATFORM_LOG_DEBUG(format, ...) \
  do { \
    va_list args; \
    va_start(args, format); \
    platform_log_printf(PLATFORM_LOG_DEBUG, __FILE__, __FUNCTION__, __LINE__, format, args); \
    va_end(args); \
  } while(0)

#endif /* PLATFORM_LOGGING_H */
