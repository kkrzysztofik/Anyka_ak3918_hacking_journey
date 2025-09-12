/**
 * @file memory_debug.h
 * @brief Memory debugging utilities using dmalloc.
 * 
 * This module provides systematic memory debugging integration
 * with dmalloc for leak detection and memory management validation.
 */

#ifndef MEMORY_DEBUG_H
#define MEMORY_DEBUG_H

#include <stdlib.h>
#include <stdio.h>

#ifdef DMALLOC
#include <dmalloc.h>
#endif

/**
 * @brief Initialize memory debugging
 * @return 0 on success, -1 on error
 */
int memory_debug_init(void);

/**
 * @brief Cleanup memory debugging
 */
void memory_debug_cleanup(void);

/**
 * @brief Log memory statistics
 */
void memory_debug_log_stats(void);

/**
 * @brief Check for memory leaks
 * @return 0 if no leaks, -1 if leaks detected
 */
int memory_debug_check_leaks(void);

/**
 * @brief Memory allocation wrapper with debugging
 * @param size Size to allocate
 * @param file Source file name
 * @param line Line number
 * @return Allocated memory or NULL
 */
void* memory_debug_malloc(size_t size, const char *file, int line);

/**
 * @brief Memory reallocation wrapper with debugging
 * @param ptr Pointer to reallocate
 * @param size New size
 * @param file Source file name
 * @param line Line number
 * @return Reallocated memory or NULL
 */
void* memory_debug_realloc(void *ptr, size_t size, const char *file, int line);

/**
 * @brief Memory free wrapper with debugging
 * @param ptr Pointer to free
 * @param file Source file name
 * @param line Line number
 */
void memory_debug_free(void *ptr, const char *file, int line);

/**
 * @brief Memory debugging macros
 */
#ifdef DMALLOC
#define MEMORY_DEBUG_MALLOC(size) memory_debug_malloc(size, __FILE__, __LINE__)
#define MEMORY_DEBUG_REALLOC(ptr, size) memory_debug_realloc(ptr, size, __FILE__, __LINE__)
#define MEMORY_DEBUG_FREE(ptr) memory_debug_free(ptr, __FILE__, __LINE__)
#else
#define MEMORY_DEBUG_MALLOC(size) malloc(size)
#define MEMORY_DEBUG_REALLOC(ptr, size) realloc(ptr, size)
#define MEMORY_DEBUG_FREE(ptr) free(ptr)
#endif

/**
 * @brief Memory debugging enabled check
 */
#ifdef DMALLOC
#define MEMORY_DEBUG_ENABLED 1
#else
#define MEMORY_DEBUG_ENABLED 0
#endif

#endif /* MEMORY_DEBUG_H */
