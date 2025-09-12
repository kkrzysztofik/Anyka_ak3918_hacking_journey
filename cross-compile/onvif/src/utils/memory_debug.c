/**
 * @file memory_debug.c
 * @brief Memory debugging utilities implementation.
 */

#include "memory_debug.h"
#include "platform.h"
#include <stdio.h>
#include <stdlib.h>

static int memory_debug_initialized = 0;

int memory_debug_init(void) {
    if (memory_debug_initialized) {
        return 0;
    }
    
#ifdef DMALLOC
    // Initialize dmalloc
    if (dmalloc_init() != 0) {
        platform_log_error("Failed to initialize dmalloc\n");
        return -1;
    }
    
    // Set dmalloc options for leak detection
    char dmalloc_config[256];
    snprintf(dmalloc_config, sizeof(dmalloc_config), 
             "debug=0x%x,inter=%d,log=%s", 
             DMALLOC_DEBUG_FLAGS, DMALLOC_LOG_INTERVAL, ONVIF_DMALLOC_LOG_FILE);
    dmalloc_debug_setup(dmalloc_config);
    
    platform_log_info("Memory debugging initialized with dmalloc\n");
#else
    platform_log_info("Memory debugging not available (dmalloc not compiled)\n");
#endif
    
    memory_debug_initialized = 1;
    return 0;
}

void memory_debug_cleanup(void) {
    if (!memory_debug_initialized) {
        return;
    }
    
#ifdef DMALLOC
    // Check for leaks before cleanup
    memory_debug_check_leaks();
    
    // Cleanup dmalloc
    dmalloc_shutdown();
    
    platform_log_info("Memory debugging cleaned up\n");
#endif
    
    memory_debug_initialized = 0;
}

void memory_debug_log_stats(void) {
#ifdef DMALLOC
    if (memory_debug_initialized) {
        platform_log_info("Memory statistics:\n");
        dmalloc_log_stats();
    }
#endif
}

int memory_debug_check_leaks(void) {
#ifdef DMALLOC
    if (!memory_debug_initialized) {
        return 0;
    }
    
    // Check for memory leaks
    if (dmalloc_log_unfreed() != 0) {
        platform_log_error("Memory leaks detected!\n");
        return -1;
    }
    
    platform_log_info("No memory leaks detected\n");
    return 0;
#else
    return 0;
#endif
}

void* memory_debug_malloc(size_t size, const char *file, int line) {
#ifdef DMALLOC
    if (!memory_debug_initialized) {
        return malloc(size);
    }
    
    void *ptr = dmalloc_malloc(file, line, size, DMALLOC_FUNC_MALLOC, 0);
    if (ptr) {
        platform_log_debug("Allocated %zu bytes at %p from %s:%d\n", size, ptr, file, line);
    } else {
        platform_log_error("Failed to allocate %zu bytes from %s:%d\n", size, file, line);
    }
    return ptr;
#else
    return malloc(size);
#endif
}

void* memory_debug_realloc(void *ptr, size_t size, const char *file, int line) {
#ifdef DMALLOC
    if (!memory_debug_initialized) {
        return realloc(ptr, size);
    }
    
    void *new_ptr = dmalloc_realloc(file, line, ptr, size, DMALLOC_FUNC_REALLOC, 0);
    if (new_ptr) {
        platform_log_debug("Reallocated %zu bytes at %p from %s:%d\n", size, new_ptr, file, line);
    } else {
        platform_log_error("Failed to reallocate %zu bytes from %s:%d\n", size, file, line);
    }
    return new_ptr;
#else
    return realloc(ptr, size);
#endif
}

void memory_debug_free(void *ptr, const char *file, int line) {
#ifdef DMALLOC
    if (!memory_debug_initialized) {
        free(ptr);
        return;
    }
    
    if (ptr) {
        platform_log_debug("Freed memory at %p from %s:%d\n", ptr, file, line);
        dmalloc_free(file, line, ptr, DMALLOC_FUNC_FREE);
    }
#else
    free(ptr);
#endif
}
