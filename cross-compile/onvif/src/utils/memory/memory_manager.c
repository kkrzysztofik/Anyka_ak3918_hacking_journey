/**
 * @file memory_manager.c
 * @brief Simplified memory management system implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "memory_manager.h"

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platform/platform.h"

static memory_tracker_t g_memory_tracker = {0};    // NOLINT
static bool g_memory_manager_initialized = false;  // NOLINT

int memory_manager_init(void) {
  if (g_memory_manager_initialized) {
    return 0;
  }

  g_memory_tracker.capacity = 1024;
  g_memory_tracker.allocations =
      malloc(g_memory_tracker.capacity * sizeof(memory_allocation_t));
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
                         g_memory_tracker.allocations[i].size,
                         g_memory_tracker.allocations[i].file,
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

  platform_log_info("Memory stats: %zu active allocations, %zu bytes total\n",
                    active_allocations, total_allocated);
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
                         g_memory_tracker.allocations[i].size,
                         g_memory_tracker.allocations[i].file,
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

static int memory_tracker_add(void *ptr, size_t size, const char *file,
                              int line, const char *function) {
  if (!g_memory_manager_initialized) {
    return 0;  // Not tracking
  }

  NULL_CHECK_RETURN(ptr, -1);
  NULL_CHECK_RETURN(file, -1);
  NULL_CHECK_RETURN(function, -1);

  if (g_memory_tracker.count >= g_memory_tracker.capacity) {
    // Expand tracker
    size_t new_capacity = g_memory_tracker.capacity * 2;
    memory_allocation_t *new_allocations =
        realloc(g_memory_tracker.allocations,
                new_capacity * sizeof(memory_allocation_t));
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

static int memory_tracker_remove(void *ptr) {
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

void *onvif_malloc(size_t size, const char *file, int line,
                   const char *function) {
  NULL_CHECK_RETURN(file, NULL);
  NULL_CHECK_RETURN(function, NULL);

  if (size == 0 || size > SIZE_MAX / 2) {
    platform_log_error("Invalid size %zu at %s:%d\n", size, __FILE__, __LINE__);
    return NULL;
  }

  void *ptr = malloc(size);
  if (!ptr) {
    platform_log_error("malloc failed for %zu bytes at %s:%d in %s()\n", size,
                       file, line, function);
    return NULL;
  }

  memory_tracker_add(ptr, size, file, line, function);
  return ptr;
}

void *onvif_calloc(size_t count, size_t size, const char *file, int line,
                   const char *function) {
  void *ptr = calloc(count, size);
  if (!ptr) {
    platform_log_error("calloc failed for %zu * %zu bytes at %s:%d in %s()\n",
                       count, size, file, line, function);
    return NULL;
  }

  memory_tracker_add(ptr, count * size, file, line, function);
  return ptr;
}

void *onvif_realloc(void *ptr, size_t size, const char *file, int line,
                    const char *function) {
  if (!ptr) {
    // realloc(NULL, size) is equivalent to malloc(size)
    return onvif_malloc(size, file, line, function);
  }

  // Remove old allocation from tracker
  memory_tracker_remove(ptr);

  void *new_ptr = realloc(ptr, size);
  if (!new_ptr) {
    platform_log_error("realloc failed for %zu bytes at %s:%d in %s()\n", size,
                       file, line, function);
    return NULL;
  }

  memory_tracker_add(new_ptr, size, file, line, function);
  return new_ptr;
}

void onvif_free(void *ptr, const char *file, int line, const char *function) {
  if (!ptr) {
    return;
  }

  // Check if pointer is already freed (double-free protection)
  if (memory_tracker_remove(ptr) != 0) {
    platform_log_error("Double-free detected at %s:%d in %s()\n", file, line,
                       function);
    return;  // Don't free again
  }

  free(ptr);
}

/* Safe string functions */
char *memory_safe_strdup(const char *str) {
  if (!str) {
    return NULL;
  }

  size_t len = strlen(str) + 1;
  char *result = ONVIF_MALLOC(len);
  if (result) {
    strcpy(result, str);
  }
  return result;
}

int memory_safe_strcpy(char *dest, size_t dest_size, const char *src) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }

  size_t src_len = strlen(src);
  if (src_len >= dest_size) {
    platform_log_error("String too long for destination buffer: %zu >= %zu\n",
                       src_len, dest_size);
    return -1;
  }

  strcpy(dest, src);
  return 0;
}

int memory_safe_strncpy(char *dest, size_t dest_size, const char *src,
                        size_t n) {
  if (!dest || !src || dest_size == 0) {
    return -1;
  }

  size_t copy_len = n < dest_size - 1 ? n : dest_size - 1;
  strncpy(dest, src, copy_len);
  dest[copy_len] = '\0';
  return 0;
}

int memory_safe_snprintf(char *dest, size_t dest_size, const char *format,
                         ...) {
  if (!dest || !format || dest_size == 0) {
    return -1;
  }

  va_list args;
  va_start(args, format);
  int result = vsnprintf(dest, dest_size, format, args);
  va_end(args);

  if (result < 0) {
    platform_log_error("vsnprintf failed at %s:%d\n", __FILE__, __LINE__);
    return -1;
  }

  if ((size_t)result >= dest_size) {
    platform_log_error("Buffer overflow in snprintf: %d >= %zu\n", result,
                       dest_size);
    return -1;
  }

  return result;
}

/* Safe memory operations */
void *memory_safe_memcpy(void *dest, size_t dest_size, const void *src,
                         size_t src_size) {
  if (!dest || !src || dest_size == 0 || src_size == 0) {
    return NULL;
  }

  if (src_size > dest_size) {
    platform_log_error("Source size exceeds destination size: %zu > %zu\n",
                       src_size, dest_size);
    return NULL;
  }

  return memcpy(dest, src, src_size);
}

void *memory_safe_memset(void *dest, size_t dest_size, size_t n, int value) {
  if (!dest || dest_size == 0) {
    return NULL;
  }

  if (n > dest_size) {
    platform_log_error("Memset size exceeds destination size: %zu > %zu\n", n,
                       dest_size);
    return NULL;
  }

  return memset(dest, value, n);
}
