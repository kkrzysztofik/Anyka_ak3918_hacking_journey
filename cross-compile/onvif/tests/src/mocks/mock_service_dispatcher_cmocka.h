/**
 * @file mock_service_dispatcher_cmocka.h
 * @brief CMocka-based mock header for service dispatcher testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MOCK_SERVICE_DISPATCHER_CMOCKA_H
#define MOCK_SERVICE_DISPATCHER_CMOCKA_H

#include <stddef.h>

#include "services/common/service_dispatcher.h"

/* ============================================================================
 * CMocka Wrapped Function Declarations
 * ============================================================================ */

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_register_service
 */
int __wrap_onvif_service_dispatcher_register_service(
  const onvif_service_registration_t* registration);

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_unregister_service
 */
int __wrap_onvif_service_dispatcher_unregister_service(const char* service_name);

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_init
 */
int __wrap_onvif_service_dispatcher_init(void);

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_cleanup
 */
void __wrap_onvif_service_dispatcher_cleanup(void);

/* ============================================================================
 * Helper Functions for Test Setup
 * ============================================================================ */

/**
 * @brief Set the last registration data for verification
 */
void mock_service_dispatcher_set_last_registration(
  const onvif_service_registration_t* registration);

/**
 * @brief Set the last unregister service name for verification
 */
void mock_service_dispatcher_set_last_unregister_service(const char* service_name);

/* ============================================================================
 * CMocka Helper Macros
 * ============================================================================ */

/**
 * @brief Expect service dispatcher registration call
 */
#define EXPECT_SERVICE_DISPATCHER_REGISTER()                                                       \
  expect_function_call(__wrap_onvif_service_dispatcher_register_service)

/**
 * @brief Expect service dispatcher unregister call
 */
#define EXPECT_SERVICE_DISPATCHER_UNREGISTER()                                                     \
  expect_function_call(__wrap_onvif_service_dispatcher_unregister_service)

/**
 * @brief Expect service dispatcher init call
 */
#define EXPECT_SERVICE_DISPATCHER_INIT() expect_function_call(__wrap_onvif_service_dispatcher_init)

/**
 * @brief Expect service dispatcher cleanup call
 */
#define EXPECT_SERVICE_DISPATCHER_CLEANUP()                                                        \
  expect_function_call(__wrap_onvif_service_dispatcher_cleanup)

/**
 * @brief Set service dispatcher register result
 */
#define SET_SERVICE_DISPATCHER_REGISTER_RESULT(result)                                             \
  will_return(__wrap_onvif_service_dispatcher_register_service, result)

/**
 * @brief Set service dispatcher unregister result
 */
#define SET_SERVICE_DISPATCHER_UNREGISTER_RESULT(result)                                           \
  will_return(__wrap_onvif_service_dispatcher_unregister_service, result)

/**
 * @brief Set service dispatcher init result
 */
#define SET_SERVICE_DISPATCHER_INIT_RESULT(result)                                                 \
  will_return(__wrap_onvif_service_dispatcher_init, result)

#endif /* MOCK_SERVICE_DISPATCHER_CMOCKA_H */
