/**
 * @file gsoap_mock_cmocka.h
 * @brief CMocka-based gSOAP mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef GSOAP_MOCK_CMOCKA_H
#define GSOAP_MOCK_CMOCKA_H

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

// Forward declare gSOAP functions without pulling in full headers
// to avoid conflicts with CMocka macros

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * CMocka Wrapped gSOAP Functions
 * ============================================================================
 * All gSOAP functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief Wrapped onvif_gsoap_init function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_init, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);
 * int result = onvif_gsoap_init();
 * @endcode
 */
int __wrap_onvif_gsoap_init(void);

/**
 * @brief Wrapped onvif_gsoap_cleanup function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_cleanup, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_cleanup, ONVIF_SUCCESS);
 * onvif_gsoap_cleanup();
 * @endcode
 */
void __wrap_onvif_gsoap_cleanup(void);

/**
 * @brief Wrapped onvif_gsoap_reset function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_reset, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_reset, ONVIF_SUCCESS);
 * onvif_gsoap_reset();
 * @endcode
 */
void __wrap_onvif_gsoap_reset(void);

/**
 * @brief Wrapped onvif_gsoap_has_error function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_has_error, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_has_error, 0);  // No error
 * int has_error = onvif_gsoap_has_error();
 * @endcode
 */
int __wrap_onvif_gsoap_has_error(void);

/**
 * @brief Wrapped onvif_gsoap_get_error function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_error, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_get_error, ONVIF_SUCCESS);
 * int error = onvif_gsoap_get_error();
 * @endcode
 */
int __wrap_onvif_gsoap_get_error(void);

/**
 * @brief Wrapped onvif_gsoap_get_response_data function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_response_data, ptr)
 *
 * Example usage in tests:
 * @code
 * const char* mock_response = "<xml>response</xml>";
 * will_return(__wrap_onvif_gsoap_get_response_data, mock_response);
 * const char* data = onvif_gsoap_get_response_data();
 * @endcode
 */
const char* __wrap_onvif_gsoap_get_response_data(void);

/**
 * @brief Wrapped onvif_gsoap_get_response_length function
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_response_length, value)
 *
 * Example usage in tests:
 * @code
 * will_return(__wrap_onvif_gsoap_get_response_length, 100);
 * size_t len = onvif_gsoap_get_response_length();
 * @endcode
 */
size_t __wrap_onvif_gsoap_get_response_length(void);

/**
 * @brief Wrapped onvif_gsoap_generate_response_with_callback function
 * @param service_name Service name (expected via expect_string)
 * @param operation_name Operation name (expected via expect_string)
 * @param callback Response generation callback (expected via expect_any)
 * @param user_data User data passed to callback (expected via expect_any)
 * @return Mock return value configured via will_return()
 *
 * Example usage in tests:
 * @code
 * expect_string(__wrap_onvif_gsoap_generate_response_with_callback, service_name, "Device");
 * expect_string(__wrap_onvif_gsoap_generate_response_with_callback, operation_name, "GetDeviceInformation");
 * expect_any(__wrap_onvif_gsoap_generate_response_with_callback, callback);
 * expect_any(__wrap_onvif_gsoap_generate_response_with_callback, user_data);
 * will_return(__wrap_onvif_gsoap_generate_response_with_callback, ONVIF_SUCCESS);
 * @endcode
 */
int __wrap_onvif_gsoap_generate_response_with_callback(const char* service_name,
                                                        const char* operation_name,
                                                        void* callback,
                                                        void* user_data);

/**
 * @brief Wrapped onvif_gsoap_generate_fault_response function
 * @param fault_code Fault code string (expected via expect_string)
 * @param fault_string Fault string (expected via expect_string)
 * @param fault_detail Fault detail (expected via expect_any)
 * @return Mock return value configured via will_return()
 *
 * Example usage in tests:
 * @code
 * expect_string(__wrap_onvif_gsoap_generate_fault_response, fault_code, "Sender");
 * expect_string(__wrap_onvif_gsoap_generate_fault_response, fault_string, "Invalid request");
 * expect_any(__wrap_onvif_gsoap_generate_fault_response, fault_detail);
 * will_return(__wrap_onvif_gsoap_generate_fault_response, ONVIF_SUCCESS);
 * @endcode
 */
int __wrap_onvif_gsoap_generate_fault_response(const char* fault_code,
                                                const char* fault_string,
                                                const char* fault_detail);

#ifdef __cplusplus
}
#endif

#endif // GSOAP_MOCK_CMOCKA_H
