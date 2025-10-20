/**
 * @file gsoap_mock.h
 * @brief CMocka-based gSOAP mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef GSOAP_MOCK_H
#define GSOAP_MOCK_H

#include <stdbool.h>

#include "cmocka_wrapper.h"
#include "protocol/gsoap/onvif_gsoap_response.h"

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

/* Forward declaration for context type */
typedef struct onvif_gsoap_context_s onvif_gsoap_context_t;

/**
 * @brief Wrapped onvif_gsoap_init function
 * @param ctx gSOAP context
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_init, value)
 */
int __wrap_onvif_gsoap_init(onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_cleanup function
 * @param ctx gSOAP context
 */
void __wrap_onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_reset function
 * @param ctx gSOAP context
 */
void __wrap_onvif_gsoap_reset(onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_has_error function
 * @param ctx gSOAP context
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_has_error, value)
 */
int __wrap_onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_get_error function
 * @param ctx gSOAP context
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_error, value)
 */
const char* __wrap_onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_get_response_data function
 * @param ctx gSOAP context
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_response_data, ptr)
 */
const char* __wrap_onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_get_response_length function
 * @param ctx gSOAP context
 * @return Mock return value configured via will_return(__wrap_onvif_gsoap_get_response_length,
 * value)
 */
size_t __wrap_onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx);

/**
 * @brief Wrapped onvif_gsoap_generate_response_with_callback function
 * @param ctx gSOAP context
 * @param callback Response generation callback (expected via expect_any)
 * @param user_data User data passed to callback (expected via expect_any)
 * @return Mock return value configured via will_return()
 */
int __wrap_onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx, onvif_response_callback_t callback, void* user_data);

/**
 * @brief Wrapped onvif_gsoap_generate_fault_response function
 * @param ctx gSOAP context
 * @param fault_code Fault code string (expected via expect_string)
 * @param fault_string Fault string (expected via expect_string)
 * @param fault_actor Fault actor string (expected via expect_string)
 * @param fault_detail Fault detail (expected via expect_any)
 * @param output_buffer Output buffer for fault response
 * @param buffer_size Size of output buffer
 * @return Mock return value configured via will_return()
 */
int __wrap_onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, const char* fault_code, const char* fault_string, const char* fault_actor,
                                               const char* fault_detail, char* output_buffer, size_t buffer_size);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void gsoap_mock_use_real_function(bool use_real);

#ifdef __cplusplus
}
#endif

#endif // GSOAP_MOCK_H
