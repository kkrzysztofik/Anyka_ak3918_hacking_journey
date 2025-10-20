/**
 * @file mock_service_dispatcher.c
 * @brief Pure CMocka service dispatcher mock with helper functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "mock_service_dispatcher.h"

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "services/common/service_dispatcher.h"

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void service_dispatcher_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * Static State Variables for Call Tracking
 * ============================================================================ */

static int g_register_call_count = 0;
static int g_unregister_call_count = 0;
static int g_dispatch_call_count = 0;

static onvif_service_registration_t g_last_registration;
static char g_last_unregister_service[256] = {0};
static char g_last_dispatch_service[256] = {0};
static char g_last_dispatch_operation[256] = {0};

/* ============================================================================
 * CMocka Wrapped Service Dispatcher Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_register_service
 */
int __wrap_onvif_service_dispatcher_register_service(
  const onvif_service_registration_t* registration) {
  if (g_use_real_functions) {
    return __real_onvif_service_dispatcher_register_service(registration);
  }

  check_expected_ptr(registration);

  // Track call and store registration data
  g_register_call_count++;
  if (registration) {
    g_last_registration = *registration;
  }

  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_unregister_service
 */
int __wrap_onvif_service_dispatcher_unregister_service(const char* service_name) {
  if (g_use_real_functions) {
    return __real_onvif_service_dispatcher_unregister_service(service_name);
  }

  check_expected_ptr(service_name);

  // Track call and store service name
  g_unregister_call_count++;
  if (service_name) {
    strncpy(g_last_unregister_service, service_name, sizeof(g_last_unregister_service) - 1);
    g_last_unregister_service[sizeof(g_last_unregister_service) - 1] = '\0';
  }

  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_init
 */
int __wrap_onvif_service_dispatcher_init(void) {
  if (g_use_real_functions) {
    return __real_onvif_service_dispatcher_init();
  }

  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_cleanup
 */
void __wrap_onvif_service_dispatcher_cleanup(void) {
  if (g_use_real_functions) {
    __real_onvif_service_dispatcher_cleanup();
    return;
  }

  // No mock return needed for void function
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_dispatch
 */
int __wrap_onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                             void* request, void* response) {
  if (g_use_real_functions) {
    return __real_onvif_service_dispatcher_dispatch(service_name, operation_name, request,
                                                    response);
  }

  check_expected_ptr(service_name);
  check_expected_ptr(operation_name);
  check_expected_ptr(request);
  check_expected_ptr(response);

  // Track dispatch call
  g_dispatch_call_count++;
  if (service_name) {
    strncpy(g_last_dispatch_service, service_name, sizeof(g_last_dispatch_service) - 1);
    g_last_dispatch_service[sizeof(g_last_dispatch_service) - 1] = '\0';
  }
  if (operation_name) {
    strncpy(g_last_dispatch_operation, operation_name, sizeof(g_last_dispatch_operation) - 1);
    g_last_dispatch_operation[sizeof(g_last_dispatch_operation) - 1] = '\0';
  }

  return (int)mock();
}

/* ============================================================================
 * Helper Functions for Easy Test Setup (wraps will_return)
 * ============================================================================ */

/**
 * @brief Initialize/reset mock state
 */
void mock_service_dispatcher_init(void) {
  g_register_call_count = 0;
  g_unregister_call_count = 0;
  g_dispatch_call_count = 0;
  memset(&g_last_registration, 0, sizeof(g_last_registration));
  memset(g_last_unregister_service, 0, sizeof(g_last_unregister_service));
  memset(g_last_dispatch_service, 0, sizeof(g_last_dispatch_service));
  memset(g_last_dispatch_operation, 0, sizeof(g_last_dispatch_operation));
}

/**
 * @brief Cleanup mock state (no-op with CMocka)
 */
void mock_service_dispatcher_cleanup(void) {
  // No-op - CMocka handles cleanup automatically
}

/**
 * @brief Set result for next service registration call
 */
void mock_service_dispatcher_set_register_result(int result) {
  will_return(__wrap_onvif_service_dispatcher_register_service, result);
}

/**
 * @brief Set result for next service unregistration call
 */
void mock_service_dispatcher_set_unregister_result(int result) {
  will_return(__wrap_onvif_service_dispatcher_unregister_service, result);
}

/**
 * @brief Set result for next service dispatch call
 */
void mock_service_dispatcher_set_dispatch_result(int result) {
  will_return(__wrap_onvif_service_dispatcher_dispatch, result);
}

/**
 * @brief Get number of times register service was called
 */
int mock_service_dispatcher_get_register_call_count(void) {
  return g_register_call_count;
}

/**
 * @brief Get number of times unregister service was called
 */
int mock_service_dispatcher_get_unregister_call_count(void) {
  return g_unregister_call_count;
}

/**
 * @brief Get number of times dispatch was called
 */
int mock_service_dispatcher_get_dispatch_call_count(void) {
  return g_dispatch_call_count;
}

/**
 * @brief Get last service registration data
 */
const onvif_service_registration_t* mock_service_dispatcher_get_last_registration(void) {
  return &g_last_registration;
}

/**
 * @brief Get last dispatched service name
 */
const char* mock_service_dispatcher_get_last_dispatch_service(void) {
  return g_last_dispatch_service;
}

/**
 * @brief Get last dispatched operation name
 */
const char* mock_service_dispatcher_get_last_dispatch_operation(void) {
  return g_last_dispatch_operation;
}

