/**
 * @file mock_service_dispatcher.h
 * @brief Pure CMocka service dispatcher mock with helper functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef MOCK_SERVICE_DISPATCHER_H
#define MOCK_SERVICE_DISPATCHER_H

#include <stdbool.h>
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

/**
 * @brief CMocka wrapped version of onvif_service_dispatcher_dispatch
 */
int __wrap_onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                             void* request, void* response);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void service_dispatcher_mock_use_real_function(bool use_real);

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

/**
 * @brief Initialize mock service dispatcher state
 * @note Resets all call counters and tracked data
 */
void mock_service_dispatcher_init(void);

/**
 * @brief Cleanup mock service dispatcher state
 * @note This is a no-op with CMocka - state is managed automatically
 */
void mock_service_dispatcher_cleanup(void);

/* ============================================================================
 * Mock Result Configuration (wraps will_return)
 * ============================================================================ */

/**
 * @brief Set result for next service registration call
 * @param result Result code to return from __wrap_onvif_service_dispatcher_register_service
 */
void mock_service_dispatcher_set_register_result(int result);

/**
 * @brief Set result for next service unregistration call
 * @param result Result code to return from __wrap_onvif_service_dispatcher_unregister_service
 */
void mock_service_dispatcher_set_unregister_result(int result);

/**
 * @brief Set result for next service dispatch call
 * @param result Result code to return from __wrap_onvif_service_dispatcher_dispatch
 */
void mock_service_dispatcher_set_dispatch_result(int result);

/* ============================================================================
 * Mock Call Tracking
 * ============================================================================ */

/**
 * @brief Get number of times register service was called
 * @return Call count
 */
int mock_service_dispatcher_get_register_call_count(void);

/**
 * @brief Get number of times unregister service was called
 * @return Call count
 */
int mock_service_dispatcher_get_unregister_call_count(void);

/**
 * @brief Get number of times dispatch was called
 * @return Call count
 */
int mock_service_dispatcher_get_dispatch_call_count(void);

/* ============================================================================
 * Mock Data Retrieval
 * ============================================================================ */

/**
 * @brief Get last service registration data
 * @return Pointer to last registration or NULL
 */
const onvif_service_registration_t* mock_service_dispatcher_get_last_registration(void);

/**
 * @brief Get last dispatched service name
 * @return Service name string or NULL
 */
const char* mock_service_dispatcher_get_last_dispatch_service(void);

/**
 * @brief Get last dispatched operation name
 * @return Operation name string or NULL
 */
const char* mock_service_dispatcher_get_last_dispatch_operation(void);


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

#endif /* MOCK_SERVICE_DISPATCHER_H */
