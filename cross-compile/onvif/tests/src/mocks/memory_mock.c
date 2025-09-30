/**
 * @file memory_mock.c
 * @brief Mock implementation for memory management operations
 * @author kkrzysztofik
 * @date 2025
 */

#include "memory_mock.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ============================================================================
 * Constants
 * ============================================================================ */

/**
 * @brief Dummy pointer value used for mock allocation results
 *
 * This constant represents a valid non-NULL pointer address used by the mock
 * to simulate successful memory allocations without actual memory allocation.
 */
#define MEMORY_MOCK_DUMMY_PTR ((void*)0x12345678)

/* ============================================================================
 * Mock State Variables
 * ============================================================================ */

static struct {
  // Allocation results
  void* malloc_result;
  void* calloc_result;
  void* realloc_result;

  // Call counts
  size_t malloc_calls;
  size_t calloc_calls;
  size_t realloc_calls;
  size_t free_calls;

  // Memory tracking
  size_t total_allocated;
  size_t total_freed;
  size_t current_usage;

  // Error simulation
  int allocation_failure_enabled;
  size_t fail_after_count;
  size_t allocation_count;
  // NOLINTBEGIN(cppcoreguidelines-avoid-non-const-global-variables)
  // Mock state must be mutable to track test execution state
} g_memory_mock_state = {0};
// NOLINTEND(cppcoreguidelines-avoid-non-const-global-variables)

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

void memory_mock_init(void) {
  memset(&g_memory_mock_state, 0, sizeof(g_memory_mock_state));
  g_memory_mock_state.malloc_result = MEMORY_MOCK_DUMMY_PTR;
  g_memory_mock_state.calloc_result = MEMORY_MOCK_DUMMY_PTR;
  g_memory_mock_state.realloc_result = MEMORY_MOCK_DUMMY_PTR;
}

void memory_mock_cleanup(void) {
  // Reset all state
  memory_mock_reset();
}

void memory_mock_reset(void) {
  memset(&g_memory_mock_state, 0, sizeof(g_memory_mock_state));
  g_memory_mock_state.malloc_result = MEMORY_MOCK_DUMMY_PTR;
  g_memory_mock_state.calloc_result = MEMORY_MOCK_DUMMY_PTR;
  g_memory_mock_state.realloc_result = MEMORY_MOCK_DUMMY_PTR;
}

/* ============================================================================
 * Memory Allocation Mock Functions
 * ============================================================================ */

void memory_mock_set_malloc_result(void* result) {
  g_memory_mock_state.malloc_result = result;
}

void memory_mock_set_calloc_result(void* result) {
  g_memory_mock_state.calloc_result = result;
}

void memory_mock_set_realloc_result(void* result) {
  g_memory_mock_state.realloc_result = result;
}

size_t memory_mock_get_malloc_call_count(void) {
  return g_memory_mock_state.malloc_calls;
}

size_t memory_mock_get_calloc_call_count(void) {
  return g_memory_mock_state.calloc_calls;
}

size_t memory_mock_get_realloc_call_count(void) {
  return g_memory_mock_state.realloc_calls;
}

size_t memory_mock_get_free_call_count(void) {
  return g_memory_mock_state.free_calls;
}

/* ============================================================================
 * Memory Tracking Mock Functions
 * ============================================================================ */

size_t memory_mock_get_total_allocated(void) {
  return g_memory_mock_state.total_allocated;
}

size_t memory_mock_get_total_freed(void) {
  return g_memory_mock_state.total_freed;
}

size_t memory_mock_get_current_usage(void) {
  return g_memory_mock_state.current_usage;
}

int memory_mock_has_leaks(void) {
  return (g_memory_mock_state.current_usage > 0) ? 1 : 0;
}

/* ============================================================================
 * Memory Error Simulation
 * ============================================================================ */

void memory_mock_enable_allocation_failure(size_t fail_after) {
  g_memory_mock_state.allocation_failure_enabled = 1;
  g_memory_mock_state.fail_after_count = fail_after;
  g_memory_mock_state.allocation_count = 0;
}

void memory_mock_disable_allocation_failure(void) {
  g_memory_mock_state.allocation_failure_enabled = 0;
}

int memory_mock_is_allocation_failure_enabled(void) {
  return g_memory_mock_state.allocation_failure_enabled;
}

/* ============================================================================
 * Mock Implementation Helpers
 * ============================================================================ */

static void* mock_allocate(size_t size) {
  g_memory_mock_state.allocation_count++;

  // Check if we should fail this allocation
  if (g_memory_mock_state.allocation_failure_enabled &&
      g_memory_mock_state.allocation_count > g_memory_mock_state.fail_after_count) {
    return NULL;
  }

  // Update tracking
  g_memory_mock_state.total_allocated += size;
  g_memory_mock_state.current_usage += size;

  return g_memory_mock_state.malloc_result;
}

static void mock_free(void* ptr) {
  if (ptr) {
    g_memory_mock_state.free_calls++;
    // In a real mock, we'd track the actual size freed
    // For simplicity, we'll just decrement by a fixed amount
    if (g_memory_mock_state.current_usage > 0) {
      g_memory_mock_state.current_usage--;
      g_memory_mock_state.total_freed++;
    }
  }
}

/* ============================================================================
 * Mock Function Implementations
 * ============================================================================ */

// These would be used to override the real memory functions in tests
// For now, they're just placeholders for the mock interface

void* mock_malloc(size_t size) {
  g_memory_mock_state.malloc_calls++;
  return mock_allocate(size);
}

void* mock_calloc(size_t num, size_t size) {
  g_memory_mock_state.calloc_calls++;
  return mock_allocate(num * size);
}

void* mock_realloc(void* ptr, size_t size) {
  g_memory_mock_state.realloc_calls++;

  // Free old pointer if reallocating
  if (ptr) {
    mock_free(ptr);
  }

  return mock_allocate(size);
}

void mock_free_func(void* ptr) {
  mock_free(ptr);
}
