/**
 * @file mock_service_dispatcher.h
 * @brief Mock implementation for service dispatcher testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MOCK_SERVICE_DISPATCHER_H
#define MOCK_SERVICE_DISPATCHER_H

#include <stddef.h>

#include "services/common/service_dispatcher.h"

/* ============================================================================
 * Mock Service Dispatcher State
 * ============================================================================ */

// Mock state variables
extern int g_mock_register_result;       // NOLINT
extern int g_mock_unregister_result;     // NOLINT
extern int g_mock_dispatch_result;       // NOLINT
extern int g_mock_register_call_count;   // NOLINT
extern int g_mock_unregister_call_count; // NOLINT
extern int g_mock_dispatch_call_count;   // NOLINT
extern int g_mock_is_registered_result;  // NOLINT
extern int g_mock_get_services_count;    // NOLINT
extern int g_mock_init_result;           // NOLINT
extern int g_mock_init_call_count;       // NOLINT
extern int g_mock_cleanup_call_count;    // NOLINT

// Mock service registration data
extern onvif_service_registration_t g_mock_last_registration; // NOLINT
extern char g_mock_last_unregister_service[64];               // NOLINT
extern char g_mock_last_dispatch_service[64];                 // NOLINT
extern char g_mock_last_dispatch_operation[64];               // NOLINT

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

/**
 * @brief Initialize mock service dispatcher state
 */
void mock_service_dispatcher_init(void);

/**
 * @brief Cleanup mock service dispatcher state
 */
void mock_service_dispatcher_cleanup(void);

/**
 * @brief Set mock registration result
 * @param result Result to return from registration
 */
void mock_service_dispatcher_set_register_result(int result);

/**
 * @brief Set mock unregistration result
 * @param result Result to return from unregistration
 */
void mock_service_dispatcher_set_unregister_result(int result);

/**
 * @brief Set mock dispatch result
 * @param result Result to return from dispatch
 */
void mock_service_dispatcher_set_dispatch_result(int result);

/**
 * @brief Set mock is_registered result
 * @param result Result to return from is_registered
 */
void mock_service_dispatcher_set_is_registered_result(int result);

/**
 * @brief Set mock get_services count
 * @param count Number of services to return
 */
void mock_service_dispatcher_set_get_services_count(int count);

/**
 * @brief Set mock init result
 * @param result Result to return from init
 */
void mock_service_dispatcher_set_init_result(int result);

/* ============================================================================
 * Mock Query Functions
 * ============================================================================ */

/**
 * @brief Get registration call count
 * @return Number of times register was called
 */
int mock_service_dispatcher_get_register_call_count(void);

/**
 * @brief Get unregistration call count
 * @return Number of times unregister was called
 */
int mock_service_dispatcher_get_unregister_call_count(void);

/**
 * @brief Get dispatch call count
 * @return Number of times dispatch was called
 */
int mock_service_dispatcher_get_dispatch_call_count(void);

/**
 * @brief Get init call count
 * @return Number of times init was called
 */
int mock_service_dispatcher_get_init_call_count(void);

/**
 * @brief Get cleanup call count
 * @return Number of times cleanup was called
 */
int mock_service_dispatcher_get_cleanup_call_count(void);

/**
 * @brief Get last registration data
 * @return Pointer to last registration structure
 */
const onvif_service_registration_t* mock_service_dispatcher_get_last_registration(void);

/**
 * @brief Get last unregister service name
 * @return Last service name passed to unregister
 */
const char* mock_service_dispatcher_get_last_unregister_service(void);

/**
 * @brief Get last dispatch service name
 * @return Last service name passed to dispatch
 */
const char* mock_service_dispatcher_get_last_dispatch_service(void);

/**
 * @brief Get last dispatch operation name
 * @return Last operation name passed to dispatch
 */
const char* mock_service_dispatcher_get_last_dispatch_operation(void);

/* ============================================================================
 * Mock Implementation Functions
 * ============================================================================ */

/**
 * @brief Mock implementation of onvif_service_dispatcher_register_service
 * @param registration Service registration information
 * @return Mock result
 */
int mock_onvif_service_dispatcher_register_service(
  const onvif_service_registration_t* registration);

/**
 * @brief Mock implementation of onvif_service_dispatcher_unregister_service
 * @param service_name Name of service to unregister
 * @return Mock result
 */
int mock_onvif_service_dispatcher_unregister_service(const char* service_name);

/**
 * @brief Mock implementation of onvif_service_dispatcher_dispatch
 * @param service_name Target service name
 * @param operation_name ONVIF operation name
 * @param request HTTP request
 * @param response HTTP response
 * @return Mock result
 */
int mock_onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                           const http_request_t* request,
                                           http_response_t* response);

/**
 * @brief Mock implementation of onvif_service_dispatcher_is_registered
 * @param service_name Service name to check
 * @return Mock result
 */
int mock_onvif_service_dispatcher_is_registered(const char* service_name);

/**
 * @brief Mock implementation of onvif_service_dispatcher_get_services
 * @param services Array to populate with service names
 * @param max_services Maximum number of services to return
 * @return Mock result
 */
int mock_onvif_service_dispatcher_get_services(const char** services, size_t max_services);

/**
 * @brief Mock implementation of onvif_service_dispatcher_init
 * @return Mock result
 */
int mock_onvif_service_dispatcher_init(void);

/**
 * @brief Mock implementation of onvif_service_dispatcher_cleanup
 */
void mock_onvif_service_dispatcher_cleanup(void);

#endif /* MOCK_SERVICE_DISPATCHER_H */
