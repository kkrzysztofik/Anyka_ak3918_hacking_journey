/**
 * @file generic_mock_framework.c
 * @brief Implementation of generic mock framework
 * @author kkrzysztofik
 * @date 2025
 */

#include "generic_mock_framework.h"

#include <pthread.h>
#include <stddef.h>
#include <string.h>

/* ============================================================================
 * Core Mock Functions
 * ============================================================================ */

int generic_mock_init(generic_mock_t* mock) {
  if (!mock) {
    return -1;
  }

  // Initialize mutex if not already done
  if (!mock->mutex_initialized) {
    if (pthread_mutex_init(&mock->mutex, NULL) != 0) {
      return -1;
    }
    mock->mutex_initialized = 1;
  }

  pthread_mutex_lock(&mock->mutex);

  // Set name if not already set
  if (mock->name[0] == '\0') {
    strncpy(mock->name, "generic_mock", sizeof(mock->name) - 1);
    mock->name[sizeof(mock->name) - 1] = '\0';
  }

  // Reset state
  mock->initialized = 1;
  mock->init_call_count++;
  mock->cleanup_call_count = 0;
  mock->error_simulation_enabled = 0;
  mock->error_code = 0;

  // Reset all operations
  if (mock->operations) {
    for (int i = 0; i < mock->operation_count; i++) {
      mock->operations[i].result_code = 0;
      mock->operations[i].call_count = 0;
      mock->operations[i].last_params = NULL;
      mock->operations[i].params_size = 0;
      mock->operations[i].enabled = 1;
    }
  }

  pthread_mutex_unlock(&mock->mutex);

  return 0;
}

void generic_mock_cleanup(generic_mock_t* mock) {
  if (!mock) {
    return;
  }

  if (mock->mutex_initialized) {
    pthread_mutex_lock(&mock->mutex);
  }

  mock->cleanup_call_count++;
  mock->initialized = 0;

  // Note: We don't free last_params as they're typically pointers to test data
  // that the test owns

  if (mock->mutex_initialized) {
    pthread_mutex_unlock(&mock->mutex);
    pthread_mutex_destroy(&mock->mutex);
    mock->mutex_initialized = 0;
  }
}

void generic_mock_reset(generic_mock_t* mock) {
  if (!mock || !mock->initialized) {
    return;
  }

  pthread_mutex_lock(&mock->mutex);

  // Reset call counts but keep initialized state
  int saved_init_count = mock->init_call_count;
  mock->init_call_count = saved_init_count;
  mock->cleanup_call_count = 0;
  mock->error_simulation_enabled = 0;
  mock->error_code = 0;

  // Reset all operation states
  if (mock->operations) {
    for (int i = 0; i < mock->operation_count; i++) {
      mock->operations[i].result_code = 0;
      mock->operations[i].call_count = 0;
      mock->operations[i].last_params = NULL;
      mock->operations[i].params_size = 0;
      mock->operations[i].enabled = 1;
    }
  }

  pthread_mutex_unlock(&mock->mutex);
}

/* ============================================================================
 * Operation Management
 * ============================================================================ */

int generic_mock_set_operation_result(generic_mock_t* mock, int operation_index, int result_code) {
  if (!mock || !mock->initialized || operation_index < 0 ||
      operation_index >= mock->operation_count) {
    return -1;
  }

  pthread_mutex_lock(&mock->mutex);
  mock->operations[operation_index].result_code = result_code;
  pthread_mutex_unlock(&mock->mutex);

  return 0;
}

int generic_mock_get_operation_call_count(const generic_mock_t* mock, int operation_index) {
  if (!mock || !mock->initialized || operation_index < 0 ||
      operation_index >= mock->operation_count) {
    return -1;
  }

  // Cast away const for mutex operations (state is logically const)
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  pthread_mutex_lock(&mutable_mock->mutex);
  int count = mock->operations[operation_index].call_count;
  pthread_mutex_unlock(&mutable_mock->mutex);

  return count;
}

int generic_mock_execute_operation(generic_mock_t* mock, int operation_index, const void* params) {
  if (!mock || !mock->initialized || operation_index < 0 ||
      operation_index >= mock->operation_count) {
    return -1;
  }

  pthread_mutex_lock(&mock->mutex);

  // Increment call count
  mock->operations[operation_index].call_count++;

  // Capture parameters if provided
  if (params) {
    mock->operations[operation_index].last_params = (void*)params;
  }

  // Return error code if error simulation is enabled
  int result = 0;
  if (mock->error_simulation_enabled) {
    result = mock->error_code;
  } else if (!mock->operations[operation_index].enabled) {
    result = -1; // Operation disabled
  } else {
    result = mock->operations[operation_index].result_code;
  }

  pthread_mutex_unlock(&mock->mutex);

  return result;
}

int generic_mock_set_operation_enabled(generic_mock_t* mock, int operation_index, int enabled) {
  if (!mock || !mock->initialized || operation_index < 0 ||
      operation_index >= mock->operation_count) {
    return -1;
  }

  pthread_mutex_lock(&mock->mutex);
  mock->operations[operation_index].enabled = enabled ? 1 : 0;
  pthread_mutex_unlock(&mock->mutex);

  return 0;
}

void* generic_mock_get_last_params(const generic_mock_t* mock, int operation_index) {
  if (!mock || !mock->initialized || operation_index < 0 ||
      operation_index >= mock->operation_count) {
    return NULL;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  pthread_mutex_lock(&mutable_mock->mutex);
  void* params = mock->operations[operation_index].last_params;
  pthread_mutex_unlock(&mutable_mock->mutex);

  return params;
}

/* ============================================================================
 * Error Simulation
 * ============================================================================ */

void generic_mock_enable_error_simulation(generic_mock_t* mock, int error_code) {
  if (!mock || !mock->initialized) {
    return;
  }

  pthread_mutex_lock(&mock->mutex);
  mock->error_simulation_enabled = 1;
  mock->error_code = error_code;
  pthread_mutex_unlock(&mock->mutex);
}

void generic_mock_disable_error_simulation(generic_mock_t* mock) {
  if (!mock || !mock->initialized) {
    return;
  }

  pthread_mutex_lock(&mock->mutex);
  mock->error_simulation_enabled = 0;
  mock->error_code = 0;
  pthread_mutex_unlock(&mock->mutex);
}

int generic_mock_is_error_simulation_enabled(const generic_mock_t* mock) {
  if (!mock || !mock->initialized) {
    return -1;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  pthread_mutex_lock(&mutable_mock->mutex);
  int enabled = mock->error_simulation_enabled;
  pthread_mutex_unlock(&mutable_mock->mutex);

  return enabled;
}

/* ============================================================================
 * State Query Functions
 * ============================================================================ */

int generic_mock_get_init_call_count(const generic_mock_t* mock) {
  if (!mock) {
    return -1;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  if (mock->mutex_initialized) {
    pthread_mutex_lock(&mutable_mock->mutex);
  }
  int count = mock->init_call_count;
  if (mock->mutex_initialized) {
    pthread_mutex_unlock(&mutable_mock->mutex);
  }

  return count;
}

int generic_mock_get_cleanup_call_count(const generic_mock_t* mock) {
  if (!mock) {
    return -1;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  if (mock->mutex_initialized) {
    pthread_mutex_lock(&mutable_mock->mutex);
  }
  int count = mock->cleanup_call_count;
  if (mock->mutex_initialized) {
    pthread_mutex_unlock(&mutable_mock->mutex);
  }

  return count;
}

int generic_mock_is_initialized(const generic_mock_t* mock) {
  if (!mock) {
    return -1;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  if (mock->mutex_initialized) {
    pthread_mutex_lock(&mutable_mock->mutex);
  }
  int initialized = mock->initialized;
  if (mock->mutex_initialized) {
    pthread_mutex_unlock(&mutable_mock->mutex);
  }

  return initialized;
}

int generic_mock_get_total_call_count(const generic_mock_t* mock) {
  if (!mock || !mock->initialized) {
    return -1;
  }

  // Cast away const for mutex operations
  generic_mock_t* mutable_mock = (generic_mock_t*)mock;
  pthread_mutex_lock(&mutable_mock->mutex);

  int total = 0;
  if (mock->operations) {
    for (int i = 0; i < mock->operation_count; i++) {
      total += mock->operations[i].call_count;
    }
  }

  pthread_mutex_unlock(&mutable_mock->mutex);

  return total;
}
