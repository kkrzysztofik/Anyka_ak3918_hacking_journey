/**
 * @file memory_manager.c
 * @brief Unified memory management system implementation.
 */

#include "memory_manager.h"
#include "platform/platform.h"
#include <stdlib.h>
#include <string.h>

static memory_tracker_t g_memory_tracker = {0};
static bool g_memory_manager_initialized = false;

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

static int memory_tracker_add(void *ptr, size_t size, const char *file, int line, const char *function) {
  if (!g_memory_manager_initialized) {
    return 0; // Not tracking
  }
  
  if (g_memory_tracker.count >= g_memory_tracker.capacity) {
    // Expand tracker
    size_t new_capacity = g_memory_tracker.capacity * 2;
    memory_allocation_t *new_allocations = realloc(g_memory_tracker.allocations, 
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

void* onvif_malloc(size_t size, const char *file, int line, const char *function) {
  void *ptr = malloc(size);
  if (!ptr) {
    platform_log_error("malloc failed for %zu bytes at %s:%d in %s()\n", 
                       size, file, line, function);
    return NULL;
  }
  
  memory_tracker_add(ptr, size, file, line, function);
  return ptr;
}

void* onvif_calloc(size_t count, size_t size, const char *file, int line, const char *function) {
  void *ptr = calloc(count, size);
  if (!ptr) {
    platform_log_error("calloc failed for %zu * %zu bytes at %s:%d in %s()\n", 
                       count, size, file, line, function);
    return NULL;
  }
  
  memory_tracker_add(ptr, count * size, file, line, function);
  return ptr;
}

void* onvif_realloc(void *ptr, size_t size, const char *file, int line, const char *function) {
  if (!ptr) {
    // realloc(NULL, size) is equivalent to malloc(size)
    return onvif_malloc(size, file, line, function);
  }
  
  // Remove old allocation from tracker
  memory_tracker_remove(ptr);
  
  void *new_ptr = realloc(ptr, size);
  if (!new_ptr) {
    platform_log_error("realloc failed for %zu bytes at %s:%d in %s()\n", 
                       size, file, line, function);
    return NULL;
  }
  
  memory_tracker_add(new_ptr, size, file, line, function);
  return new_ptr;
}

void onvif_free(void *ptr, const char *file, int line, const char *function) {
  if (!ptr) {
    return;
  }
  
  memory_tracker_remove(ptr);
  free(ptr);
}

void onvif_auto_ptr_cleanup(onvif_auto_ptr_t *auto_ptr) {
  if (!auto_ptr || !auto_ptr->ptr) {
    return;
  }
  
  onvif_free(auto_ptr->ptr, auto_ptr->file, auto_ptr->line, auto_ptr->function);
  auto_ptr->ptr = NULL;
}

int memory_pool_init(memory_pool_t *pool, size_t block_size, size_t block_count) {
  if (!pool || block_size == 0 || block_count == 0) {
    return -1;
  }
  
  pool->block_size = block_size;
  pool->block_count = block_count;
  pool->free_blocks = block_count;
  
  // Allocate pool memory
  pool->pool = malloc(block_size * block_count);
  if (!pool->pool) {
    platform_log_error("Failed to allocate memory pool\n");
    return -1;
  }
  
  // Initialize free list
  pool->free_list = malloc(block_count * sizeof(void*));
  if (!pool->free_list) {
    platform_log_error("Failed to allocate free list\n");
    free(pool->pool);
    return -1;
  }
  
  // Initialize free list with all blocks
  for (size_t i = 0; i < block_count; i++) {
    pool->free_list[i] = (char*)pool->pool + i * block_size;
  }
  
  platform_log_info("Memory pool initialized: %zu blocks of %zu bytes\n", 
                    block_count, block_size);
  return 0;
}

void memory_pool_cleanup(memory_pool_t *pool) {
  if (!pool) {
    return;
  }
  
  if (pool->pool) {
    free(pool->pool);
    pool->pool = NULL;
  }
  
  if (pool->free_list) {
    free(pool->free_list);
    pool->free_list = NULL;
  }
  
  pool->block_size = 0;
  pool->block_count = 0;
  pool->free_blocks = 0;
}

void* memory_pool_alloc(memory_pool_t *pool) {
  if (!pool || pool->free_blocks == 0) {
    return NULL;
  }
  
  void *ptr = pool->free_list[--pool->free_blocks];
  return ptr;
}

void memory_pool_free(memory_pool_t *pool, void *ptr) {
  if (!pool || !ptr || pool->free_blocks >= pool->block_count) {
    return;
  }
  
  // Validate pointer is within pool
  if ((char*)ptr < (char*)pool->pool || (char*)ptr >= (char*)pool->pool + pool->block_size * pool->block_count) {
    platform_log_error("Invalid pointer for memory pool free\n");
    return;
  }
  
  pool->free_list[pool->free_blocks++] = ptr;
}
