/**
 * @file memory_manager.h
 * @brief Comprehensive memory management system with dynamic buffers and leak
 * detection
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides a unified memory management system that includes:
 * - Memory allocation tracking and leak detection
 * - Dynamic buffer management for XML responses
 * - Buffer safety utilities and validation
 * - Performance monitoring and statistics
 */

#ifndef ONVIF_MEMORY_MANAGER_H
#define ONVIF_MEMORY_MANAGER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ============================================================================
 * Constants and Types
 * ============================================================================
 */

// Dynamic buffer configuration constants
#define DYNAMIC_BUFFER_INITIAL_SIZE  1024               // Initial buffer size
#define DYNAMIC_BUFFER_GROWTH_FACTOR 2                  // Growth factor for reallocation
#define DYNAMIC_BUFFER_MAX_SIZE      (16 * 1024 * 1024) // 16MB maximum
#define DYNAMIC_BUFFER_ALIGNMENT     8                  // Memory alignment

// Buffer safety configuration constants
#define BUFFER_SAFETY_MAX_STRING_LEN 4096        // Maximum safe string length
#define BUFFER_SAFETY_MAX_PATH_LEN   1024        // Maximum safe path length
#define BUFFER_SAFETY_MAX_XML_LEN    (64 * 1024) // Maximum safe XML length

// Buffer safety validation flags
typedef enum {
  BUFFER_SAFETY_VALIDATE_NULL_TERMINATED = 0x01,  // Ensure null termination
  BUFFER_SAFETY_VALIDATE_UTF8 = 0x02,             // Validate UTF-8 encoding
  BUFFER_SAFETY_VALIDATE_XML_SAFE = 0x04,         // Validate XML safety
  BUFFER_SAFETY_VALIDATE_NO_CONTROL_CHARS = 0x08, // No control characters
  BUFFER_SAFETY_VALIDATE_PRINTABLE_ONLY = 0x10    // Printable characters only
} buffer_safety_flags_t;

// Buffer state flags
typedef enum {
  DYNAMIC_BUFFER_STATE_INITIALIZED = 0x01,
  DYNAMIC_BUFFER_STATE_READONLY = 0x02,
  DYNAMIC_BUFFER_STATE_ERROR = 0x04
} dynamic_buffer_state_t;

// Memory allocation tracking
typedef struct {
  void* ptr;
  size_t size;
  const char* file;
  int line;
  const char* function;
} memory_allocation_t;

typedef struct {
  memory_allocation_t* allocations;
  size_t count;
  size_t capacity;
} memory_tracker_t;

// Buffer statistics for monitoring
typedef struct {
  size_t total_allocations;
  size_t total_reallocations;
  size_t peak_size;
  size_t current_size;
  uint64_t total_bytes_written;
} dynamic_buffer_stats_t;

// Main dynamic buffer structure
typedef struct {
  char* data;                   // Buffer data pointer
  size_t capacity;              // Total allocated capacity
  size_t length;                // Current used length
  size_t position;              // Current write position
  dynamic_buffer_state_t state; // Buffer state flags
  dynamic_buffer_stats_t stats; // Usage statistics
  void* user_data;              // User-defined data pointer
} dynamic_buffer_t;

// Buffer safety statistics
typedef struct {
  size_t total_validations;          // Total validation operations
  size_t failed_validations;         // Failed validation operations
  size_t buffer_overflows_prevented; // Buffer overflows prevented
  size_t memory_leaks_detected;      // Memory leaks detected
  uint64_t total_validation_time_us; // Total validation time
} buffer_safety_stats_t;

/* Core memory management functions */
int memory_manager_init(void);
void memory_manager_cleanup(void);
void memory_manager_log_stats(void);
int memory_manager_check_leaks(void);
size_t memory_manager_get_allocated_size(void);

/* Basic allocation functions */
void* onvif_malloc(size_t size, const char* file, int line, const char* function);
void* onvif_calloc(size_t count, size_t size, const char* file, int line, const char* function);
void* onvif_realloc(void* ptr, size_t size, const char* file, int line, const char* function);
void onvif_free(void* ptr, const char* file, int line, const char* function);

/* Essential macros */
#define ONVIF_MALLOC(size)        onvif_malloc((size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_CALLOC(count, size) onvif_calloc((count), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_REALLOC(ptr, size)  onvif_realloc((ptr), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_FREE(ptr)           onvif_free((ptr), __FILE__, __LINE__, __FUNCTION__)

/* Safe memory macros */
#define MEMORY_SAFE_FREE(ptr)                                                                                                                        \
  do {                                                                                                                                               \
    if (ptr) {                                                                                                                                       \
      ONVIF_FREE(ptr);                                                                                                                               \
      (ptr) = NULL;                                                                                                                                  \
    }                                                                                                                                                \
  } while (0)

/* Null pointer protection */
#define NULL_CHECK_RETURN(ptr, ret_val)                                                                                                              \
  do {                                                                                                                                               \
    if ((ptr) == NULL) {                                                                                                                             \
      platform_log_error("Null pointer detected at %s:%d in %s()\n", __FILE__, __LINE__, __FUNCTION__);                                              \
      return ret_val;                                                                                                                                \
    }                                                                                                                                                \
  } while (0)

/* Memory validation */
#define MEMORY_VALIDATE_SIZE(size)                                                                                                                   \
  ({                                                                                                                                                 \
    int _valid = 1;                                                                                                                                  \
    if ((size) == 0 || (size) > SIZE_MAX / 2) {                                                                                                      \
      platform_log_error("Invalid size %zu at %s:%d\n", (size), __FILE__, __LINE__);                                                                 \
      _valid = 0;                                                                                                                                    \
    }                                                                                                                                                \
    _valid;                                                                                                                                          \
  })

/* Safe string functions */
char* memory_safe_strdup(const char* str);
int memory_safe_strcpy(char* dest, size_t dest_size, const char* src);
int memory_safe_strncpy(char* dest, size_t dest_size, const char* src, size_t n);
int memory_safe_snprintf(char* dest, size_t dest_size, const char* format, ...);

/* Safe memory operations */
void* memory_safe_memcpy(void* dest, size_t dest_size, const void* src, size_t src_size);
void* memory_safe_memset(void* dest, size_t dest_size, size_t n, int value);

/* ============================================================================
 * Dynamic Buffer Management Functions
 * ============================================================================
 */

/**
 * @brief Initialize a dynamic buffer with default settings
 * @param buffer Pointer to buffer structure to initialize
 * @param initial_size Initial buffer size (0 for default)
 * @return 0 on success, negative error code on failure
 */
int dynamic_buffer_init(dynamic_buffer_t* buffer, size_t initial_size);

/**
 * @brief Initialize a dynamic buffer with custom settings
 * @param buffer Pointer to buffer structure to initialize
 * @param initial_size Initial buffer size
 * @param growth_factor Growth factor for reallocation
 * @param max_size Maximum buffer size
 * @return 0 on success, negative error code on failure
 */
int dynamic_buffer_init_custom(dynamic_buffer_t* buffer, size_t initial_size, size_t growth_factor, size_t max_size);

/**
 * @brief Clean up and free dynamic buffer resources
 * @param buffer Pointer to buffer structure to clean up
 */
void dynamic_buffer_cleanup(dynamic_buffer_t* buffer);

/**
 * @brief Reset buffer to initial state (keep allocated memory)
 * @param buffer Pointer to buffer structure to reset
 */
void dynamic_buffer_reset(dynamic_buffer_t* buffer);

/**
 * @brief Ensure buffer has at least the specified capacity
 * @param buffer Pointer to buffer structure
 * @param required_capacity Minimum required capacity
 * @return 0 on success, negative error code on failure
 */
int dynamic_buffer_ensure_capacity(dynamic_buffer_t* buffer, size_t required_capacity);

/**
 * @brief Append data to the buffer
 * @param buffer Pointer to buffer structure
 * @param data Data to append
 * @param length Length of data to append
 * @return 0 on success, negative error code on failure
 */
int dynamic_buffer_append(dynamic_buffer_t* buffer, const void* data, size_t length);

/**
 * @brief Append formatted string to the buffer
 * @param buffer Pointer to buffer structure
 * @param format printf-style format string
 * @param ... Format arguments
 * @return Number of characters written, or negative error code on failure
 */
int dynamic_buffer_appendf(dynamic_buffer_t* buffer, const char* format, ...);

/**
 * @brief Append string to the buffer
 * @param buffer Pointer to buffer structure
 * @param str String to append
 * @return 0 on success, negative error code on failure
 */
int dynamic_buffer_append_string(dynamic_buffer_t* buffer, const char* str);

/**
 * @brief Get current buffer data pointer
 * @param buffer Pointer to buffer structure
 * @return Pointer to buffer data, or NULL if not initialized
 */
const char* dynamic_buffer_data(const dynamic_buffer_t* buffer);

/**
 * @brief Get current buffer length
 * @param buffer Pointer to buffer structure
 * @return Current buffer length
 */
size_t dynamic_buffer_length(const dynamic_buffer_t* buffer);

/**
 * @brief Get current buffer capacity
 * @param buffer Pointer to buffer structure
 * @return Current buffer capacity
 */
size_t dynamic_buffer_capacity(const dynamic_buffer_t* buffer);

/**
 * @brief Get available space in buffer
 * @param buffer Pointer to buffer structure
 * @return Available space for writing
 */
size_t dynamic_buffer_available(const dynamic_buffer_t* buffer);

/**
 * @brief Check if buffer is in error state
 * @param buffer Pointer to buffer structure
 * @return true if buffer is in error state, false otherwise
 */
bool dynamic_buffer_has_error(const dynamic_buffer_t* buffer);

/**
 * @brief Get buffer statistics
 * @param buffer Pointer to buffer structure
 * @return Pointer to buffer statistics structure
 */
const dynamic_buffer_stats_t* dynamic_buffer_get_stats(const dynamic_buffer_t* buffer);

/* ============================================================================
 * Buffer Safety Functions
 * ============================================================================
 */

/**
 * @brief Safely copy string with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string
 * @param max_src_len Maximum source length to copy
 * @return Number of characters copied (excluding null terminator), or -1 on
 * error
 */
int buffer_safe_strcpy(char* dest, size_t dest_size, const char* src, size_t max_src_len);

/**
 * @brief Safely concatenate string with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string to append
 * @param max_src_len Maximum source length to append
 * @return Number of characters appended, or -1 on error
 */
int buffer_safe_strcat(char* dest, size_t dest_size, const char* src, size_t max_src_len);

/**
 * @brief Safely format string with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param format printf-style format string
 * @param ... Format arguments
 * @return Number of characters written (excluding null terminator), or -1 on
 * error
 */
int buffer_safe_snprintf(char* dest, size_t dest_size, const char* format, ...);

/**
 * @brief Validate string for safety
 * @param str String to validate
 * @param max_len Maximum allowed length
 * @param flags Validation flags
 * @return 0 if valid, negative error code if invalid
 */
int buffer_validate_string(const char* str, size_t max_len, buffer_safety_flags_t flags);

/**
 * @brief Safely append XML element
 * @param buffer Destination buffer
 * @param buffer_size Size of destination buffer
 * @param element_name Element name
 * @param content Element content (NULL for empty element)
 * @param attributes NULL-terminated array of attribute name-value pairs
 * @return Number of characters appended, or -1 on error
 */
int buffer_safe_append_xml_element(char* buffer, size_t buffer_size, const char* element_name, const char* content, const char** attributes);

/**
 * @brief Safely escape XML content
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string to escape
 * @param src_len Length of source string
 * @return Number of characters written, or -1 on error
 */
int buffer_safe_escape_xml(char* dest, size_t dest_size, const char* src, size_t src_len);

/**
 * @brief Get buffer safety statistics
 * @return Pointer to statistics structure
 */
const buffer_safety_stats_t* buffer_safety_get_stats(void);

/**
 * @brief Reset buffer safety statistics
 */
void buffer_safety_reset_stats(void);

#endif /* ONVIF_MEMORY_MANAGER_H */
