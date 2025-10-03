/**
 * @file mock_service_dispatcher_cmocka.c
 * @brief CMocka-based mock implementation for service dispatcher testing
 * @author kkrzysztofik
 * @date 2025
 */

#include "mock_service_dispatcher_cmocka.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Global Variables for Tracking Mock State
 * ============================================================================ */

static onvif_service_registration_t g_mock_last_registration;
static char g_mock_last_unregister_service[256];

/* ============================================================================
 * CMocka Wrapped Service Dispatcher Functions
 * ============================================================================ */

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_register_service
 */
int __wrap_onvif_service_dispatcher_register_service(
  const onvif_service_registration_t* registration) {
  check_expected_ptr(registration);

  // Store the registration data for verification
  if (registration) {
    mock_service_dispatcher_set_last_registration(registration);
  }

  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_unregister_service
 */
int __wrap_onvif_service_dispatcher_unregister_service(const char* service_name) {
  check_expected_ptr(service_name);

  // Store the service name for verification
  if (service_name) {
    mock_service_dispatcher_set_last_unregister_service(service_name);
  }

  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_init
 */
int __wrap_onvif_service_dispatcher_init(void) {
  return (int)mock();
}

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_cleanup
 */
void __wrap_onvif_service_dispatcher_cleanup(void) {
  // No mock return needed for void function
}

/* ============================================================================
 * Helper Functions for Test Setup
 * ============================================================================ */

/**
 * @brief Set the last registration data for verification
 */
void mock_service_dispatcher_set_last_registration(
  const onvif_service_registration_t* registration) {
  if (registration) {
    g_mock_last_registration = *registration;
  }
}

/**
 * @brief Set the last unregister service name for verification
 */
void mock_service_dispatcher_set_last_unregister_service(const char* service_name) {
  if (service_name) {
    strncpy(g_mock_last_unregister_service, service_name,
            sizeof(g_mock_last_unregister_service) - 1);
    g_mock_last_unregister_service[sizeof(g_mock_last_unregister_service) - 1] = '\0';
  }
}
