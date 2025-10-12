/**
 * @file service_dispatcher.h
 * @brief ONVIF Service Dispatcher - Standardized Callback Interface
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SERVICE_DISPATCHER_H
#define ONVIF_SERVICE_DISPATCHER_H

#include <stddef.h>

#include "networking/http/http_parser.h"

/* Forward declaration for gSOAP context */
struct soap;

/* ============================================================================
 * Standardized Service Callback Interface
 * ============================================================================ */

/**
 * @brief Standard ONVIF service operation handler function type
 *
 * This is the canonical signature that all ONVIF service handlers must follow.
 * Based on the device service pattern that has proven effective for memory
 * optimization and maintainability.
 *
 * @param operation_name ONVIF operation name (e.g., "GetDeviceInformation")
 * @param request HTTP request containing SOAP envelope
 * @param response HTTP response to populate with SOAP envelope
 * @return ONVIF_SUCCESS on success, error code on failure
 */
typedef int (*onvif_service_operation_handler_t)(const char* operation_name,
                                                 const http_request_t* request,
                                                 http_response_t* response);

/**
 * @brief Get service capability structure callback
 * @param ctx gSOAP context for allocating structures
 * @param capabilities_ptr Output pointer to capability structure (type depends on service)
 * @return ONVIF_SUCCESS on success, error code otherwise
 *
 * Service-specific capability types:
 * - Device: tt__DeviceCapabilities*
 * - Media: tt__MediaCapabilities*
 * - PTZ: tt__PTZCapabilities*
 * - Imaging: tt__ImagingCapabilities*
 */
typedef int (*onvif_service_get_capabilities_t)(struct soap* ctx, void** capabilities_ptr);

/**
 * @brief Service registration information
 *
 * Contains metadata and handler for a complete ONVIF service.
 * All services must provide this information to participate in
 * the standardized dispatch system.
 */
typedef struct {
  /** Service name (e.g., "device", "media", "ptz") */
  const char* service_name;

  /** Service namespace URI */
  const char* namespace_uri;

  /** Primary operation handler for this service */
  onvif_service_operation_handler_t operation_handler;

  /** Service initialization function (optional) */
  int (*init_handler)(void);

  /** Service cleanup function (optional) */
  void (*cleanup_handler)(void);

  /** Service capabilities check (optional) */
  int (*capabilities_handler)(const char* capability_name);

  /** Get service capability structure (optional) */
  onvif_service_get_capabilities_t get_capabilities;

  /** Reserved for future extensions */
  void* reserved[3];
} onvif_service_registration_t;

/**
 * @brief Service callback registration helper
 *
 * Standardized way to register a service with the dispatcher.
 * This helper ensures all services follow the same registration pattern
 * and provides future-proof interface evolution.
 *
 * @param registration Service registration information
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_service_dispatcher_register_service(const onvif_service_registration_t* registration);

/**
 * @brief Service callback unregistration helper
 *
 * Remove a service from the dispatcher. Used during cleanup or
 * service reconfiguration.
 *
 * @param service_name Name of service to unregister
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_service_dispatcher_unregister_service(const char* service_name);

/**
 * @brief Dispatch request to appropriate service
 *
 * Main dispatch function that routes HTTP requests to the correct
 * service based on SOAP action or URL path analysis.
 *
 * @param service_name Target service name
 * @param operation_name ONVIF operation name
 * @param request HTTP request
 * @param response HTTP response
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                      const http_request_t* request, http_response_t* response);

/**
 * @brief Check if service is registered
 *
 * @param service_name Service name to check
 * @return 1 if registered, 0 if not registered
 */
int onvif_service_dispatcher_is_registered(const char* service_name);

/**
 * @brief Get list of registered services
 *
 * @param services Array to populate with service names
 * @param max_services Maximum number of services to return
 * @return Number of services returned, negative on error
 */
int onvif_service_dispatcher_get_services(const char** services, size_t max_services);

/**
 * @brief Get service capability structure
 *
 * Queries a registered service for its ONVIF capability structure.
 * The capability structure is allocated using the provided gSOAP context
 * and is managed by that context's memory.
 *
 * @param service_name Service to query (e.g., "device", "media", "ptz", "imaging")
 * @param ctx gSOAP context for memory allocation
 * @param capabilities_ptr Output pointer to service capability structure
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR_NOT_FOUND if service not registered,
 *         ONVIF_ERROR_NOT_SUPPORTED if service doesn't provide capabilities,
 *         other error code on failure
 */
int onvif_service_dispatcher_get_capabilities(const char* service_name, struct soap* ctx,
                                              void** capabilities_ptr);

/**
 * @brief Initialize service dispatcher
 *
 * Must be called before any service registration or dispatch operations.
 *
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_service_dispatcher_init(void);

/**
 * @brief Cleanup service dispatcher
 *
 * Unregisters all services and cleans up dispatcher resources.
 * Should be called during system shutdown.
 */
void onvif_service_dispatcher_cleanup(void);

/**
 * @brief Flag to indicate cleanup is in progress
 *
 * This flag is used by service modules to prevent recursive cleanup calls
 * during service dispatcher shutdown.
 */
extern int g_cleanup_in_progress;

/* ============================================================================
 * Convenience Macros for Service Registration
 * ============================================================================ */

/**
 * @brief Create service registration structure
 *
 * Convenience macro to create a properly initialized service registration.
 * Reduces boilerplate and ensures consistent initialization.
 *
 * @param name Service name
 * @param ns Namespace URI
 * @param handler Operation handler function
 * @param init_fn Initialization function (NULL if not needed)
 * @param cleanup_fn Cleanup function (NULL if not needed)
 */
#define ONVIF_SERVICE_REGISTRATION(name, ns, handler, init_fn, cleanup_fn)                         \
  {                                                                                                \
    .service_name = (name), .namespace_uri = (ns), .operation_handler = (handler),                 \
    .init_handler = (init_fn), .cleanup_handler = (cleanup_fn), .capabilities_handler = NULL,      \
    .get_capabilities = NULL,                                                                      \
    .reserved = {NULL, NULL, NULL}                                                                 \
  }

/**
 * @brief Register service with minimal parameters
 *
 * Convenience macro for simple service registration with just
 * name, namespace, and handler.
 */
#define ONVIF_REGISTER_SERVICE(name, ns, handler)                                                  \
  onvif_service_dispatcher_register_service(                                                       \
    &(onvif_service_registration_t)ONVIF_SERVICE_REGISTRATION(name, ns, handler, NULL, NULL))

#endif /* ONVIF_SERVICE_DISPATCHER_H */