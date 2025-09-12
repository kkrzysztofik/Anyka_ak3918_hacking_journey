/**
 * @file memory_manager.h
 * @brief Unified memory management system with leak detection and cleanup tracking.
 */

#ifndef ONVIF_MEMORY_MANAGER_H
#define ONVIF_MEMORY_MANAGER_H

#include "error_handling.h"
#include <stddef.h>

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

/* Memory management functions */
int memory_manager_init(void);
void memory_manager_cleanup(void);
void memory_manager_log_stats(void);
int memory_manager_check_leaks(void);

/* Unified allocation functions */
void* onvif_malloc(size_t size, const char *file, int line, const char *function);
void* onvif_calloc(size_t count, size_t size, const char *file, int line, const char *function);
void* onvif_realloc(void *ptr, size_t size, const char *file, int line, const char *function);
void onvif_free(void *ptr, const char *file, int line, const char *function);

/* Memory tracking macros */
#define ONVIF_MALLOC(size) onvif_malloc((size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_CALLOC(count, size) onvif_calloc((count), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_REALLOC(ptr, size) onvif_realloc((ptr), (size), __FILE__, __LINE__, __FUNCTION__)
#define ONVIF_FREE(ptr) onvif_free((ptr), __FILE__, __LINE__, __FUNCTION__)

/* RAII-style memory management */
typedef struct {
  void *ptr;
  const char *file;
  int line;
  const char *function;
} onvif_auto_ptr_t;

#define ONVIF_AUTO_PTR_INIT(ptr) { .ptr = (ptr), .file = __FILE__, .line = __LINE__, .function = __FUNCTION__ }

void onvif_auto_ptr_cleanup(onvif_auto_ptr_t *auto_ptr);

/* Smart pointer macros */
#define ONVIF_AUTO_PTR(type, name, size) \
  type *name = (type*)ONVIF_MALLOC(size); \
  onvif_auto_ptr_t name##_auto = ONVIF_AUTO_PTR_INIT(name); \
  if (name) { \
    onvif_auto_ptr_cleanup(&name##_auto); \
  }

/* Memory pool for fixed-size allocations */
typedef struct {
  void *pool;
  size_t block_size;
  size_t block_count;
  size_t free_blocks;
  void **free_list;
} memory_pool_t;

int memory_pool_init(memory_pool_t *pool, size_t block_size, size_t block_count);
void memory_pool_cleanup(memory_pool_t *pool);
void* memory_pool_alloc(memory_pool_t *pool);
void memory_pool_free(memory_pool_t *pool, void *ptr);

#endif /* ONVIF_MEMORY_MANAGER_H */
