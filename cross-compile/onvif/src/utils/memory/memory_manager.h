/**
 * @file memory_manager.h
 * @brief Simplified memory management system with basic leak detection
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_MEMORY_MANAGER_H
#define ONVIF_MEMORY_MANAGER_H

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "utils/error/error_handling.h"
#include "platform/platform.h"

/* Memory allocation tracking */
typedef struct {
  void *ptr;
  size_t size;
  const char *file;
  int line;
  const char *function;
} memory_allocation_t;

typedef struct {
  memory_allocation_t *allocations;
  size_t count;
  size_t capacity;
} memory_tracker_t;

/* Core memory management functions */
int memory_manager_init(void);
void memory_manager_cleanup(void);
void memory_manager_log_stats(void);
int memory_manager_check_leaks(void);

/* Basic allocation functions */
void* onvif_malloc(size_t size, const char *file, int line, const char *function);
void* onvif_calloc(size_t count, size_t size, const char *file, int line, const char *function);
void* onvif_realloc(void *ptr, size_t size, const char *file, int line, const char *function);
void onvif_free(void *ptr, const char *file, int line, const char *function);

/* Essential macros */
#define ONVIF_MALLOC(size) onvif_malloc((size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_CALLOC(count, size) onvif_calloc((count), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_REALLOC(ptr, size) onvif_realloc((ptr), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_FREE(ptr) onvif_free((ptr), __FILE__, __LINE__, __FUNCTION__)

/* Safe memory macros */
#define MEMORY_SAFE_FREE(ptr) \
    do { \
        if (ptr) { \
            ONVIF_FREE(ptr); \
            (ptr) = NULL; \
        } \
    } while(0)

/* Null pointer protection */
#define NULL_CHECK_RETURN(ptr, ret_val) \
    do { \
        if ((ptr) == NULL) { \
            platform_log_error("Null pointer detected at %s:%d in %s()\n", \
                               __FILE__, __LINE__, __FUNCTION__); \
            return ret_val; \
        } \
    } while(0)

/* Memory validation */
#define MEMORY_VALIDATE_SIZE(size) \
    ({ \
        int _valid = 1; \
        if ((size) == 0 || (size) > SIZE_MAX / 2) { \
            platform_log_error("Invalid size %zu at %s:%d\n", (size), __FILE__, __LINE__); \
            _valid = 0; \
        } \
        _valid; \
    })

/* Safe string functions */
char *memory_safe_strdup(const char *str);
int memory_safe_strcpy(char *dest, size_t dest_size, const char *src);
int memory_safe_strncpy(char *dest, size_t dest_size, const char *src, size_t n);
int memory_safe_snprintf(char *dest, size_t dest_size, const char *format, ...);

/* Safe memory operations */
void *memory_safe_memcpy(void *dest, size_t dest_size, const void *src, size_t src_size);
void *memory_safe_memset(void *dest, size_t dest_size, int c, size_t n);

#endif /* ONVIF_MEMORY_MANAGER_H */
