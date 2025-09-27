/**
 * @file memory_manager.c
 * @brief Comprehensive memory management system with dynamic buffers and leak
 * detection
 * @author kkrzysztofik
 * @date 2025
 *
 * This module implements a unified memory management system that includes:
 * - Memory allocation tracking and leak detection
 * - Dynamic buffer management for XML responses
 * - Buffer safety utilities and validation
 * - Performance monitoring and statistics
 */

#include "memory_manager.h"

#include <errno.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platform/platform.h"

/* ============================================================================
 * Global Variables
 * ============================================================================
 */

static memory_tracker_t g_memory_tracker = {0};           // NOLINT
static bool g_memory_manager_initialized = false;         // NOLINT
static buffer_safety_stats_t g_buffer_safety_stats = {0}; // NOLINT

// Global error message buffer for dynamic buffers
static char g_dynamic_buffer_error_msg[256] = {0};

int memory_manager_init(void) {
  if (g_memory_manager_initialized) {
    return 0;
  }

  g_memory_tracker.capacity = 1024;
  g_memory_tracker.allocations = malloc(g_memory_tracker.capacity * sizeof(memory_allocation_t));
  if (!g_memory_tracker.allocations) {
    platform_log_error("Failed to initialize memory tracker\n");
    return -1;
  }

  g_memory_tracker.count = 0;
  g_memory_manager_initialized = true;

  platform_log_info("Memory manager initialized\n");
  return 0;
}

void memory_manager_cleanup(void) {
  if (!g_memory_manager_initialized) {
    return;
  }

  // Check for leaks
  memory_manager_check_leaks();

  // Free all tracked allocations
  for (size_t i = 0; i < g_memory_tracker.count; i++) {
    if (g_memory_tracker.allocations[i].ptr) {
      platform_log_error("Leaked %zu bytes allocated at %s:%d in %s()\n",
                         g_memory_tracker.allocations[i].size, g_memory_tracker.allocations[i].file,
                         g_memory_tracker.allocations[i].line,
                         g_memory_tracker.allocations[i].function);
      free(g_memory_tracker.allocations[i].ptr);
    }
  }

  free(g_memory_tracker.allocations);
  g_memory_tracker.allocations = NULL;
  g_memory_tracker.count = 0;
  g_memory_tracker.capacity = 0;
  g_memory_manager_initialized = false;

  platform_log_info("Memory manager cleaned up\n");
}

void memory_manager_log_stats(void) {
  if (!g_memory_manager_initialized) {
    return;
  }

  size_t total_allocated = 0;
  size_t active_allocations = 0;

  for (size_t i = 0; i < g_memory_tracker.count; i++) {
    if (g_memory_tracker.allocations[i].ptr) {
      total_allocated += g_memory_tracker.allocations[i].size;
      active_allocations++;
    }
  }

  platform_log_info("Memory stats: %zu active allocations, %zu bytes total\n", active_allocations,
                    total_allocated);
}

int memory_manager_check_leaks(void) {
  if (!g_memory_manager_initialized) {
    return 0;
  }

  int leak_count = 0;

  for (size_t i = 0; i < g_memory_tracker.count; i++) {
    if (g_memory_tracker.allocations[i].ptr) {
      leak_count++;
      platform_log_error("Memory leak: %zu bytes allocated at %s:%d in %s()\n",
                         g_memory_tracker.allocations[i].size, g_memory_tracker.allocations[i].file,
                         g_memory_tracker.allocations[i].line,
                         g_memory_tracker.allocations[i].function);
    }
  }

  if (leak_count > 0) {
    platform_log_error("Found %d memory leaks\n", leak_count);
    return -1;
  }

  platform_log_info("No memory leaks detected\n");
  return 0;
}

static int memory_tracker_add(void* ptr, size_t size, const char* file, int line,
                              const char* function) {
  if (!g_memory_manager_initialized) {
    return 0; // Not tracking
  }

  NULL_CHECK_RETURN(ptr, -1);
  NULL_CHECK_RETURN(file, -1);
  NULL_CHECK_RETURN(function, -1);

  if (g_memory_tracker.count >= g_memory_tracker.capacity) {
    // Expand tracker
    size_t new_capacity = g_memory_tracker.capacity * 2;
    memory_allocation_t* new_allocations =
      realloc(g_memory_tracker.allocations, new_capacity * sizeof(memory_allocation_t));
    if (!new_allocations) {
      platform_log_error("Failed to expand memory tracker\n");
      return -1;
    }
    g_memory_tracker.allocations = new_allocations;
    g_memory_tracker.capacity = new_capacity;
  }

  g_memory_tracker.allocations[g_memory_tracker.count].ptr = ptr;
  g_memory_tracker.allocations[g_memory_tracker.count].size = size;
  g_memory_tracker.allocations[g_memory_tracker.count].file = file;
  g_memory_tracker.allocations[g_memory_tracker.count].line = line;
  g_memory_tracker.allocations[g_memory_tracker.count].function = function;
  g_memory_tracker.count++;

  return 0;
}

static int memory_tracker_remove(void* ptr) {
  if (!g_memory_manager_initialized || !ptr) {
    return 0;
  }

  for (size_t i = 0; i < g_memory_tracker.count; i++) {
    if (g_memory_tracker.allocations[i].ptr == ptr) {
      g_memory_tracker.allocations[i].ptr = NULL;
      return 0;
    }
  }

  platform_log_warning("Attempted to free untracked memory at %p\n", ptr);
  return -1;
}

void* onvif_malloc(size_t size, const char* file, int line, const char* function) {
  NULL_CHECK_RETURN(file, NULL);
  NULL_CHECK_RETURN(function, NULL);

  if (size == 0 || size > SIZE_MAX / 2) {
    platform_log_error("Invalid size %zu at %s:%d\n", size, __FILE__, __LINE__);
    return NULL;
  }

  void* ptr = malloc(size);
  if (!ptr) {
    platform_log_error("malloc failed for %zu bytes at %s:%d in %s()\n", size, file, line,
                       function);
    return NULL;
  }

  memory_tracker_add(ptr, size, file, line, function);
  return ptr;
}

void* onvif_calloc(size_t count, size_t size, const char* file, int line, const char* function) {
  void* ptr = calloc(count, size);
  if (!ptr) {
    platform_log_error("calloc failed for %zu * %zu bytes at %s:%d in %s()\n", count, size, file,
                       line, function);
    return NULL;
  }

  memory_tracker_add(ptr, count * size, file, line, function);
  return ptr;
}

void* onvif_realloc(void* ptr, size_t size, const char* file, int line, const char* function) {
  if (!ptr) {
    // realloc(NULL, size) is equivalent to malloc(size)
    return onvif_malloc(size, file, line, function);
  }

  // Remove old allocation from tracker
  memory_tracker_remove(ptr);

  void* new_ptr = realloc(ptr, size);
  if (!new_ptr) {
    platform_log_error("realloc failed for %zu bytes at %s:%d in %s()\n", size, file, line,
                       function);
    return NULL;
  }

  memory_tracker_add(new_ptr, size, file, line, function);
  return new_ptr;
}

void onvif_free(void* ptr, const char* file, int line, const char* function) {
  if (!ptr) {
    return;
  }

  // Check if pointer is already freed (double-free protection)
  if (memory_tracker_remove(ptr) != 0) {
    platform_log_error("Double-free detected at %s:%d in %s()\n", file, line, function);
    return; // Don't free again
  }

  free(ptr);
}

/* Safe string functions */
char* memory_safe_strdup(const char* str) {
  if (!str) {
    return NULL;
  }

  size_t len = strlen(str) + 1;
  char* result = ONVIF_MALLOC(len);
  if (result) {
    strcpy(result, str);
  }
  return result;
}

int memory_safe_strcpy(char* dest, size_t dest_size, const char* src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }

  size_t src_len = strlen(src);
  if (src_len >= dest_size) {
    platform_log_error("String too long for destination buffer: %zu >= %zu\n", src_len, dest_size);
    return -1;
  }

  strcpy(dest, src);
  return 0;
}

int memory_safe_strncpy(char* dest, size_t dest_size, const char* src, size_t n) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }

  size_t copy_len = n < dest_size - 1 ? n : dest_size - 1;
  strncpy(dest, src, copy_len);
  dest[copy_len] = '\0';
  return 0;
}

int memory_safe_snprintf(char* dest, size_t dest_size, const char* format, ...) {
  if (!dest || !format || dest_size == 0) {
    return -1;
  }

  va_list args;
  va_start(args, format);

  // For simple format strings, use vsnprintf directly
  if (strcmp(format, "<%s") == 0 || strcmp(format, " %s=\"%s\"") == 0 ||
      strcmp(format, "/>") == 0 || strcmp(format, "</%s>") == 0 ||
      strcmp(format, ">%s</%s>") == 0 || strcmp(format, "<%s>") == 0) {
    int result = vsnprintf(dest, dest_size, format, args);
    va_end(args);

    if (result < 0) {
      platform_log_error("vsnprintf failed at %s:%d\n", __FILE__, __LINE__);
      return -1;
    }

    if ((size_t)result >= dest_size) {
      platform_log_error("Buffer overflow in snprintf: %d >= %zu\n", result, dest_size);
      return -1;
    }

    return result;
  }

  // For complex format strings, implement manual string building to avoid hangs
  platform_log_debug("memory_safe_snprintf: manual building for format: %s\n", format);

  char* pos = dest;
  size_t remaining = dest_size - 1; // Leave space for null terminator
  const char* fmt = format;

  while (*fmt && remaining > 0) {
    if (*fmt == '%' && *(fmt + 1) == 's') {
      // Handle %s format specifier
      const char* str = va_arg(args, const char*);
      if (!str)
        str = "(null)";

      while (*str && remaining > 0) {
        *pos++ = *str++;
        remaining--;
      }
      fmt += 2; // Skip %s
    } else {
      // Copy literal character
      *pos++ = *fmt++;
      remaining--;
    }
  }

  va_end(args);

  // Null terminate
  *pos = '\0';

  int result = pos - dest;
  platform_log_debug("memory_safe_snprintf: manual build returned %d\n", result);

  if (result < 0) {
    platform_log_error("Manual string building failed at %s:%d\n", __FILE__, __LINE__);
    return -1;
  }

  if ((size_t)result >= dest_size) {
    platform_log_error("Buffer overflow in manual build: %d >= %zu\n", result, dest_size);
    return -1;
  }

  return result;
}

/* Safe memory operations */
void* memory_safe_memcpy(void* dest, size_t dest_size, const void* src, size_t src_size) {
  if (!dest || !src || dest_size == 0 || src_size == 0) {
    return NULL;
  }

  if (src_size > dest_size) {
    platform_log_error("Source size exceeds destination size: %zu > %zu\n", src_size, dest_size);
    return NULL;
  }

  return memcpy(dest, src, src_size);
}

void* memory_safe_memset(void* dest, size_t dest_size, size_t n, int value) {
  if (!dest || dest_size == 0) {
    return NULL;
  }

  if (n > dest_size) {
    platform_log_error("Memset size exceeds destination size: %zu > %zu\n", n, dest_size);
    return NULL;
  }

  return memset(dest, value, n);
}

/* ============================================================================
 * Dynamic Buffer Management Implementation
 * ============================================================================
 */

/**
 * @brief Set error message for the buffer
 * @param buffer Pointer to buffer structure
 * @param format Error message format string
 * @param ... Format arguments
 */
static void set_buffer_error(dynamic_buffer_t* buffer, const char* format, ...) {
  va_list args;
  va_start(args, format);
  vsnprintf(g_dynamic_buffer_error_msg, sizeof(g_dynamic_buffer_error_msg), format, args);
  va_end(args);

  buffer->state |= DYNAMIC_BUFFER_STATE_ERROR;
  platform_log_error("Dynamic Buffer Error: %s", g_dynamic_buffer_error_msg);
  g_buffer_safety_stats.failed_validations++;
}

/**
 * @brief Calculate next buffer size based on growth strategy
 * @param current_size Current buffer size
 * @param required_size Required buffer size
 * @param growth_factor Growth factor
 * @param max_size Maximum allowed size
 * @return Next buffer size
 */
static size_t calculate_next_size(size_t current_size, size_t required_size, size_t growth_factor,
                                  size_t max_size) {
  size_t next_size = current_size;

  // Double the size until we have enough space
  while (next_size < required_size) {
    next_size *= growth_factor;

    // Check for overflow
    if (next_size < current_size) {
      return max_size;
    }

    // Respect maximum size limit
    if (next_size > max_size) {
      return max_size;
    }
  }

  return next_size;
}

/**
 * @brief Align size to specified alignment boundary
 * @param size Size to align
 * @param alignment Alignment boundary
 * @return Aligned size
 */
static size_t align_size(size_t size, size_t alignment) {
  return (size + alignment - 1) & ~(alignment - 1);
}

int dynamic_buffer_init(dynamic_buffer_t* buffer, size_t initial_size) {
  if (!buffer) {
    platform_log_error("Dynamic Buffer: NULL buffer pointer");
    return -EINVAL;
  }

  // Initialize buffer structure
  memset(buffer, 0, sizeof(dynamic_buffer_t));

  // Use default initial size if not specified
  if (initial_size == 0) {
    initial_size = DYNAMIC_BUFFER_INITIAL_SIZE;
  }

  // Align initial size
  initial_size = align_size(initial_size, DYNAMIC_BUFFER_ALIGNMENT);

  // Allocate initial buffer using tracked memory
  buffer->data = ONVIF_MALLOC(initial_size);
  if (!buffer->data) {
    set_buffer_error(buffer, "Failed to allocate initial buffer of %zu bytes", initial_size);
    return -ENOMEM;
  }

  // Initialize buffer state
  buffer->capacity = initial_size;
  buffer->length = 0;
  buffer->position = 0;
  buffer->state = DYNAMIC_BUFFER_STATE_INITIALIZED;
  buffer->data[0] = '\0'; // Null-terminate

  // Initialize statistics
  buffer->stats.total_allocations = 1;
  buffer->stats.peak_size = initial_size;
  buffer->stats.current_size = initial_size;

  platform_log_debug("Dynamic Buffer: Initialized with %zu bytes", initial_size);
  return 0;
}

int dynamic_buffer_init_custom(dynamic_buffer_t* buffer, size_t initial_size, size_t growth_factor,
                               size_t max_size) {
  if (!buffer) {
    platform_log_error("Dynamic Buffer: NULL buffer pointer");
    return -EINVAL;
  }

  // Validate parameters
  if (growth_factor < 2) {
    platform_log_error("Dynamic Buffer: Invalid growth factor %zu", growth_factor);
    return -EINVAL;
  }

  if (max_size < initial_size) {
    platform_log_error("Dynamic Buffer: Max size %zu < initial size %zu", max_size, initial_size);
    return -EINVAL;
  }

  // Initialize with default settings first
  int result = dynamic_buffer_init(buffer, initial_size);
  if (result != 0) {
    return result;
  }

  platform_log_debug("Dynamic Buffer: Custom initialization - growth: %zu, max: %zu", growth_factor,
                     max_size);
  return 0;
}

void dynamic_buffer_cleanup(dynamic_buffer_t* buffer) {
  if (!buffer) {
    return;
  }

  if (buffer->data) {
    ONVIF_FREE(buffer->data);
    buffer->data = NULL;
  }

  // Clear all state
  memset(buffer, 0, sizeof(dynamic_buffer_t));

  platform_log_debug("Dynamic Buffer: Cleaned up");
}

void dynamic_buffer_reset(dynamic_buffer_t* buffer) {
  if (!buffer || !(buffer->state & DYNAMIC_BUFFER_STATE_INITIALIZED)) {
    return;
  }

  buffer->length = 0;
  buffer->position = 0;
  buffer->state &= ~DYNAMIC_BUFFER_STATE_ERROR; // Clear error state

  if (buffer->data) {
    buffer->data[0] = '\0'; // Null-terminate
  }

  platform_log_debug("Dynamic Buffer: Reset to initial state");
}

int dynamic_buffer_ensure_capacity(dynamic_buffer_t* buffer, size_t required_capacity) {
  if (!buffer || !(buffer->state & DYNAMIC_BUFFER_STATE_INITIALIZED)) {
    return -EINVAL;
  }

  if (buffer->state & DYNAMIC_BUFFER_STATE_READONLY) {
    set_buffer_error(buffer, "Cannot modify read-only buffer");
    return -EPERM;
  }

  // Check if we already have enough capacity
  if (buffer->capacity >= required_capacity) {
    return 0;
  }

  // Calculate new size
  size_t new_size = calculate_next_size(buffer->capacity, required_capacity,
                                        DYNAMIC_BUFFER_GROWTH_FACTOR, DYNAMIC_BUFFER_MAX_SIZE);

  if (new_size < required_capacity) {
    set_buffer_error(buffer, "Required capacity %zu exceeds maximum size %zu", required_capacity,
                     DYNAMIC_BUFFER_MAX_SIZE);
    return -ENOSPC;
  }

  // Reallocate buffer using tracked memory
  char* new_data = ONVIF_REALLOC(buffer->data, new_size);
  if (!new_data) {
    set_buffer_error(buffer, "Failed to reallocate buffer to %zu bytes", new_size);
    return -ENOMEM;
  }

  // Update buffer state
  buffer->data = new_data;
  buffer->capacity = new_size;
  buffer->stats.total_reallocations++;
  buffer->stats.peak_size = new_size;
  buffer->stats.current_size = new_size;

  platform_log_debug("Dynamic Buffer: Expanded to %zu bytes", new_size);
  return 0;
}

int dynamic_buffer_append(dynamic_buffer_t* buffer, const void* data, size_t length) {
  if (!buffer || !data) {
    return -EINVAL;
  }

  if (length == 0) {
    return 0; // Nothing to append
  }

  // Ensure we have enough capacity
  int result = dynamic_buffer_ensure_capacity(buffer, buffer->length + length + 1);
  if (result != 0) {
    return result;
  }

  // Append data
  memcpy(buffer->data + buffer->length, data, length);
  buffer->length += length;
  buffer->data[buffer->length] = '\0'; // Null-terminate
  buffer->stats.total_bytes_written += length;

  return 0;
}

int dynamic_buffer_appendf(dynamic_buffer_t* buffer, const char* format, ...) {
  if (!buffer || !format) {
    return -EINVAL;
  }

  // Format the string into a temporary buffer
  char temp_buffer[1024];
  va_list args;
  va_start(args, format);
  int result = vsnprintf(temp_buffer, sizeof(temp_buffer), format, args);
  va_end(args);

  if (result < 0) {
    set_buffer_error(buffer, "Format string error");
    return -EINVAL;
  }

  // Handle case where formatted string is larger than temp buffer
  if (result >= (int)sizeof(temp_buffer)) {
    // Allocate larger temporary buffer
    char* large_buffer = ONVIF_MALLOC(result + 1);
    if (!large_buffer) {
      set_buffer_error(buffer, "Failed to allocate temporary buffer for formatting");
      return -ENOMEM;
    }

    va_start(args, format);
    vsnprintf(large_buffer, result + 1, format, args);
    va_end(args);

    int append_result = dynamic_buffer_append(buffer, large_buffer, result);
    ONVIF_FREE(large_buffer);
    return append_result;
  }

  return dynamic_buffer_append(buffer, temp_buffer, result);
}

int dynamic_buffer_append_string(dynamic_buffer_t* buffer, const char* str) {
  if (!str) {
    return 0; // NULL string is treated as empty
  }

  return dynamic_buffer_append(buffer, str, strlen(str));
}

const char* dynamic_buffer_data(const dynamic_buffer_t* buffer) {
  if (!buffer || !(buffer->state & DYNAMIC_BUFFER_STATE_INITIALIZED)) {
    return NULL;
  }

  return buffer->data;
}

size_t dynamic_buffer_length(const dynamic_buffer_t* buffer) {
  if (!buffer) {
    return 0;
  }

  return buffer->length;
}

size_t dynamic_buffer_capacity(const dynamic_buffer_t* buffer) {
  if (!buffer) {
    return 0;
  }

  return buffer->capacity;
}

size_t dynamic_buffer_available(const dynamic_buffer_t* buffer) {
  if (!buffer) {
    return 0;
  }

  return buffer->capacity - buffer->length;
}

bool dynamic_buffer_has_error(const dynamic_buffer_t* buffer) {
  if (!buffer) {
    return true;
  }

  return (buffer->state & DYNAMIC_BUFFER_STATE_ERROR) != 0;
}

const dynamic_buffer_stats_t* dynamic_buffer_get_stats(const dynamic_buffer_t* buffer) {
  if (!buffer) {
    return NULL;
  }

  return &buffer->stats;
}

/* ============================================================================
 * Enhanced Buffer Safety Functions
 * ============================================================================
 */

int buffer_safe_strcpy(char* dest, size_t dest_size, const char* src, size_t max_src_len) {
  if (!dest || !src || dest_size == 0) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  size_t src_len = strlen(src);
  if (max_src_len > 0 && src_len > max_src_len) {
    src_len = max_src_len;
  }

  if (src_len >= dest_size) {
    platform_log_error("String too long for destination buffer: %zu >= %zu", src_len, dest_size);
    g_buffer_safety_stats.buffer_overflows_prevented++;
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  strncpy(dest, src, src_len);
  dest[src_len] = '\0';
  g_buffer_safety_stats.total_validations++;
  return (int)src_len;
}

int buffer_safe_strcat(char* dest, size_t dest_size, const char* src, size_t max_src_len) {
  if (!dest || !src || dest_size == 0) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  size_t dest_len = strlen(dest);
  size_t src_len = strlen(src);
  if (max_src_len > 0 && src_len > max_src_len) {
    src_len = max_src_len;
  }

  if (dest_len + src_len >= dest_size) {
    platform_log_error("String concatenation would overflow buffer: %zu + %zu >= %zu", dest_len,
                       src_len, dest_size);
    g_buffer_safety_stats.buffer_overflows_prevented++;
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  strncat(dest, src, src_len);
  g_buffer_safety_stats.total_validations++;
  return (int)src_len;
}

int buffer_safe_snprintf(char* dest, size_t dest_size, const char* format, ...) {
  if (!dest || !format || dest_size == 0) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  va_list args;
  va_start(args, format);
  int result = vsnprintf(dest, dest_size, format, args);
  va_end(args);

  if (result < 0) {
    platform_log_error("vsnprintf failed");
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  if ((size_t)result >= dest_size) {
    platform_log_error("Buffer overflow in snprintf: %d >= %zu", result, dest_size);
    g_buffer_safety_stats.buffer_overflows_prevented++;
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  g_buffer_safety_stats.total_validations++;
  return result;
}

int buffer_validate_string(const char* str, size_t max_len, buffer_safety_flags_t flags) {
  if (!str) {
    g_buffer_safety_stats.failed_validations++;
    return -EINVAL;
  }

  size_t len = strlen(str);
  if (max_len > 0 && len > max_len) {
    platform_log_error("String too long: %zu > %zu", len, max_len);
    g_buffer_safety_stats.failed_validations++;
    return -EINVAL;
  }

  // Validate null termination
  if (flags & BUFFER_SAFETY_VALIDATE_NULL_TERMINATED) {
    if (str[len] != '\0') {
      platform_log_error("String not null-terminated");
      g_buffer_safety_stats.failed_validations++;
      return -EINVAL;
    }
  }

  // Validate printable characters only
  if (flags & BUFFER_SAFETY_VALIDATE_PRINTABLE_ONLY) {
    for (size_t i = 0; i < len; i++) {
      if (str[i] < 32 || str[i] > 126) {
        platform_log_error("Non-printable character at position %zu: %d", i, (int)str[i]);
        g_buffer_safety_stats.failed_validations++;
        return -EINVAL;
      }
    }
  }

  g_buffer_safety_stats.total_validations++;
  return 0;
}

int buffer_safe_append_xml_element(char* buffer, size_t buffer_size, const char* element_name,
                                   const char* content, const char** attributes) {
  if (!buffer || !element_name || buffer_size == 0) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  size_t current_len = strlen(buffer);
  size_t remaining = buffer_size - current_len - 1; // Leave space for null terminator

  // Build element string
  char element[1024];
  int result = snprintf(element, sizeof(element), "<%s", element_name);
  if (result < 0 || (size_t)result >= sizeof(element)) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  // Add attributes if provided
  if (attributes) {
    for (int i = 0; attributes[i] && attributes[i + 1]; i += 2) {
      result += snprintf(element + result, sizeof(element) - result, " %s=\"%s\"", attributes[i],
                         attributes[i + 1]);
      if (result < 0 || (size_t)result >= sizeof(element)) {
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
    }
  }

  // Add content or make self-closing
  if (content) {
    result +=
      snprintf(element + result, sizeof(element) - result, ">%s</%s>", content, element_name);
  } else {
    result += snprintf(element + result, sizeof(element) - result, "/>");
  }

  if (result < 0 || (size_t)result >= sizeof(element)) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  // Check if we have enough space
  if ((size_t)result >= remaining) {
    platform_log_error("XML element too large for buffer: %d >= %zu", result, remaining);
    g_buffer_safety_stats.buffer_overflows_prevented++;
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  // Append to buffer
  strcat(buffer, element);
  g_buffer_safety_stats.total_validations++;
  return result;
}

int buffer_safe_escape_xml(char* dest, size_t dest_size, const char* src, size_t src_len) {
  if (!dest || !src || dest_size == 0) {
    g_buffer_safety_stats.failed_validations++;
    return -1;
  }

  size_t dest_pos = 0;
  for (size_t i = 0; i < src_len && dest_pos < dest_size - 1; i++) {
    switch (src[i]) {
    case '<':
      if (dest_pos + 4 < dest_size) {
        strcpy(dest + dest_pos, "&lt;");
        dest_pos += 4;
      } else {
        g_buffer_safety_stats.buffer_overflows_prevented++;
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
      break;
    case '>':
      if (dest_pos + 4 < dest_size) {
        strcpy(dest + dest_pos, "&gt;");
        dest_pos += 4;
      } else {
        g_buffer_safety_stats.buffer_overflows_prevented++;
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
      break;
    case '&':
      if (dest_pos + 5 < dest_size) {
        strcpy(dest + dest_pos, "&amp;");
        dest_pos += 5;
      } else {
        g_buffer_safety_stats.buffer_overflows_prevented++;
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
      break;
    case '"':
      if (dest_pos + 6 < dest_size) {
        strcpy(dest + dest_pos, "&quot;");
        dest_pos += 6;
      } else {
        g_buffer_safety_stats.buffer_overflows_prevented++;
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
      break;
    case '\'':
      if (dest_pos + 6 < dest_size) {
        strcpy(dest + dest_pos, "&apos;");
        dest_pos += 6;
      } else {
        g_buffer_safety_stats.buffer_overflows_prevented++;
        g_buffer_safety_stats.failed_validations++;
        return -1;
      }
      break;
    default:
      dest[dest_pos++] = src[i];
      break;
    }
  }

  dest[dest_pos] = '\0';
  g_buffer_safety_stats.total_validations++;
  return (int)dest_pos;
}

const buffer_safety_stats_t* buffer_safety_get_stats(void) {
  return &g_buffer_safety_stats;
}

void buffer_safety_reset_stats(void) {
  memset(&g_buffer_safety_stats, 0, sizeof(g_buffer_safety_stats));
}
