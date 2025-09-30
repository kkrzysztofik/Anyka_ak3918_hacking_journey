/**
 * @file service_dispatcher.c
 * @brief ONVIF Service Dispatcher Implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "services/common/service_dispatcher.h"

#include <bits/pthreadtypes.h>
#include <pthread.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"

/* Missing error constants */
#define ONVIF_ERROR_NOT_INITIALIZED -40
#define ONVIF_ERROR_RESOURCE_LIMIT  -41

/* ============================================================================
 * Private Types and Constants
 * ============================================================================ */

/** Maximum number of services that can be registered */
#define MAX_REGISTERED_SERVICES 16

/** Service registry entry */
typedef struct {
  onvif_service_registration_t registration;
  int active; /**< 1 if service is active, 0 if slot is free */
} service_registry_entry_t;

/* ============================================================================
 * Global State
 * ============================================================================ */

/** Global service registry */
static service_registry_entry_t g_service_registry[MAX_REGISTERED_SERVICES]; // NOLINT

/** Registry initialization flag */
static int g_dispatcher_initialized = 0; // NOLINT

/** Flag to prevent recursive cleanup calls */
int g_cleanup_in_progress = 0; // NOLINT

/** Mutex for thread-safe access to dispatcher state */
static pthread_mutex_t g_dispatcher_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT

/* ============================================================================
 * Private Functions
 * ============================================================================ */

/**
 * @brief Find service by name in registry
 * @param service_name Service name to find
 * @return Pointer to registry entry, NULL if not found
 */
static service_registry_entry_t* find_service_entry(const char* service_name) {
  if (!service_name) {
    return NULL;
  }

  for (size_t i = 0; i < MAX_REGISTERED_SERVICES; i++) {
    if (g_service_registry[i].active && g_service_registry[i].registration.service_name &&
        strcmp(g_service_registry[i].registration.service_name, service_name) == 0) {
      return &g_service_registry[i];
    }
  }

  return NULL;
}

/**
 * @brief Find free slot in registry
 * @return Pointer to free entry, NULL if registry is full
 */
static service_registry_entry_t* find_free_slot(void) {
  for (size_t i = 0; i < MAX_REGISTERED_SERVICES; i++) {
    if (!g_service_registry[i].active) {
      return &g_service_registry[i];
    }
  }
  return NULL;
}

/**
 * @brief Validate service registration parameters
 * @param registration Registration to validate
 * @return ONVIF_SUCCESS if valid, error code otherwise
 */
static int validate_registration(const onvif_service_registration_t* registration) {
  if (!registration) {
    platform_log_error("Service registration is NULL");
    return ONVIF_ERROR_INVALID;
  }

  if (!registration->service_name || strlen(registration->service_name) == 0) {
    platform_log_error("Service name is NULL or empty");
    return ONVIF_ERROR_INVALID;
  }

  if (!registration->namespace_uri || strlen(registration->namespace_uri) == 0) {
    platform_log_error("Service namespace URI is NULL or empty");
    return ONVIF_ERROR_INVALID;
  }

  if (!registration->operation_handler) {
    platform_log_error("Service operation handler is NULL");
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Public Interface Implementation
 * ============================================================================ */

int onvif_service_dispatcher_init(void) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (g_dispatcher_initialized) {
    platform_log_debug("Service dispatcher already initialized");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_SUCCESS;
  }

  // Clear the registry
  memset(g_service_registry, 0, sizeof(g_service_registry));

  g_dispatcher_initialized = 1;
  platform_log_info("Service dispatcher initialized successfully");

  pthread_mutex_unlock(&g_dispatcher_mutex);
  return ONVIF_SUCCESS;
}

void onvif_service_dispatcher_cleanup(void) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized) {
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return;
  }

  // Set flag to prevent recursive cleanup
  g_cleanup_in_progress = 1;

  // Call cleanup handlers for all registered services
  for (size_t i = 0; i < MAX_REGISTERED_SERVICES; i++) {
    if (g_service_registry[i].active) {
      if (g_service_registry[i].registration.cleanup_handler) {
        platform_log_debug("Calling cleanup handler for service: %s",
                           g_service_registry[i].registration.service_name);
        g_service_registry[i].registration.cleanup_handler();
      }
      g_service_registry[i].active = 0;
    }
  }

  g_cleanup_in_progress = 0;
  g_dispatcher_initialized = 0;

  pthread_mutex_unlock(&g_dispatcher_mutex);

  // Destroy mutex after cleanup is complete
  pthread_mutex_destroy(&g_dispatcher_mutex);

  platform_log_info("Service dispatcher cleanup completed");
}

int onvif_service_dispatcher_register_service(const onvif_service_registration_t* registration) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized) {
    platform_log_error("Service dispatcher not initialized");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  int result = validate_registration(registration);
  if (result != ONVIF_SUCCESS) {
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return result;
  }

  // Check if service is already registered
  if (find_service_entry(registration->service_name)) {
    platform_log_error("Service already registered: %s", registration->service_name);
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_ALREADY_EXISTS;
  }

  // Find free slot
  service_registry_entry_t* entry = find_free_slot();
  if (!entry) {
    platform_log_error("Service registry is full, cannot register: %s", registration->service_name);
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_RESOURCE_LIMIT;
  }

  // Copy registration data
  entry->registration = *registration;
  entry->active = 1;

  // Call initialization handler if provided
  if (registration->init_handler) {
    platform_log_debug("Calling init handler for service: %s", registration->service_name);
    result = registration->init_handler();
    if (result != ONVIF_SUCCESS) {
      platform_log_error("Service initialization failed: %s", registration->service_name);
      entry->active = 0;
      pthread_mutex_unlock(&g_dispatcher_mutex);
      return result;
    }
  }

  platform_log_info("Service registered successfully: %s (namespace: %s)",
                    registration->service_name, registration->namespace_uri);

  pthread_mutex_unlock(&g_dispatcher_mutex);
  return ONVIF_SUCCESS;
}

int onvif_service_dispatcher_unregister_service(const char* service_name) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized) {
    platform_log_error("Service dispatcher not initialized");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  if (!service_name) {
    platform_log_error("Service name is NULL");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_INVALID;
  }

  service_registry_entry_t* entry = find_service_entry(service_name);
  if (!entry) {
    platform_log_error("Service not found: %s", service_name);
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Call cleanup handler only if not already in cleanup (prevent recursion)
  // This prevents infinite loops when cleanup handlers call unregister
  if (entry->registration.cleanup_handler && !g_cleanup_in_progress) {
    platform_log_debug("Calling cleanup handler for service: %s", service_name);
    entry->registration.cleanup_handler();
  } else if (g_cleanup_in_progress) {
    platform_log_debug("Skipping cleanup handler for service: %s (already in global cleanup)",
                       service_name);
  }

  entry->active = 0;
  platform_log_info("Service unregistered: %s", service_name);

  pthread_mutex_unlock(&g_dispatcher_mutex);
  return ONVIF_SUCCESS;
}

int onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                      const http_request_t* request, http_response_t* response) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized) {
    platform_log_error("Service dispatcher not initialized");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  if (!service_name || !operation_name || !request || !response) {
    platform_log_error("Invalid parameters for dispatch");
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_INVALID;
  }

  service_registry_entry_t* entry = find_service_entry(service_name);
  if (!entry) {
    platform_log_error("Service not found: %s", service_name);
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  platform_log_debug("Dispatching %s:%s to service handler", service_name, operation_name);

  // Store handler reference before unlocking mutex
  onvif_service_operation_handler_t handler = entry->registration.operation_handler;

  pthread_mutex_unlock(&g_dispatcher_mutex);

  // Call the service operation handler without holding the mutex
  // This allows the handler to potentially call other dispatcher functions
  int result = handler(operation_name, request, response);

  if (result == ONVIF_SUCCESS) {
    platform_log_debug("Service operation completed successfully: %s:%s", service_name,
                       operation_name);
  } else {
    platform_log_error("Service operation failed: %s:%s (result: %d)", service_name, operation_name,
                       result);
  }

  return result;
}

int onvif_service_dispatcher_is_registered(const char* service_name) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized || !service_name) {
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return 0;
  }

  int result = (find_service_entry(service_name) != NULL) ? 1 : 0;
  pthread_mutex_unlock(&g_dispatcher_mutex);
  return result;
}

int onvif_service_dispatcher_get_services(const char** services, size_t max_services) {
  pthread_mutex_lock(&g_dispatcher_mutex);

  if (!g_dispatcher_initialized) {
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  if (!services || max_services == 0) {
    pthread_mutex_unlock(&g_dispatcher_mutex);
    return ONVIF_ERROR_INVALID;
  }

  size_t count = 0;
  for (size_t i = 0; i < MAX_REGISTERED_SERVICES && count < max_services; i++) {
    if (g_service_registry[i].active) {
      services[count] = g_service_registry[i].registration.service_name;
      count++;
    }
  }

  pthread_mutex_unlock(&g_dispatcher_mutex);
  return (int)count;
}
// Test comment
