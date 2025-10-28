/**
 * @file onvif_service_test_helpers.h
 * @brief Shared helpers for service initialization logic in unit tests.
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SERVICES_COMMON_ONVIF_SERVICE_TEST_HELPERS_H
#define SERVICES_COMMON_ONVIF_SERVICE_TEST_HELPERS_H

#ifdef UNIT_TESTING

#include <stdio.h>

#include "platform/platform.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"

/**
 * @brief Register a service with the dispatcher and handle failure consistently.
 *
 * @param registration Service registration descriptor.
 * @param initialized_flag Pointer to the service initialization flag (may be NULL).
 * @param cleanup_fn Cleanup function to invoke on registration failure (may be NULL).
 * @param service_name Logical service name for diagnostics.
 * @return Dispatcher result code.
 */
static inline int onvif_service_unit_register(const onvif_service_registration_t* registration, int* initialized_flag, void (*cleanup_fn)(void),
                                              const char* service_name) {
  int result = onvif_service_dispatcher_register_service(registration);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("Failed to register %s service with dispatcher: %d\n", service_name, result);
    if (result != ONVIF_ERROR_ALREADY_EXISTS) {
      if (cleanup_fn != NULL) {
        cleanup_fn();
      }
      if (initialized_flag != NULL) {
        *initialized_flag = 0;
      }
    }
  } else {
    platform_log_info("%s service initialized and registered with dispatcher\n", service_name);
  }
  return result;
}

#endif // UNIT_TESTING

#endif // SERVICES_COMMON_ONVIF_SERVICE_TEST_HELPERS_H
