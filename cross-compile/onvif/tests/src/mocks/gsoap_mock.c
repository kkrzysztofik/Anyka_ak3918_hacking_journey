/**
 * @file gsoap_mock.c
 * @brief CMocka-based gSOAP mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include "gsoap_mock.h"

#include <stdbool.h>
#include <stddef.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * External Real Function Declarations
 * ============================================================================ */

/* Forward declaration for context type */
typedef struct onvif_gsoap_context_s onvif_gsoap_context_t;

/* These are the real implementations that the linker will resolve via --wrap */
extern int __real_onvif_gsoap_init(onvif_gsoap_context_t* ctx);
extern void __real_onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx);
extern void __real_onvif_gsoap_reset(onvif_gsoap_context_t* ctx);
extern int __real_onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx);
extern const char* __real_onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx);
extern const char* __real_onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx);
extern size_t __real_onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx);
extern int __real_onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                               const char* service_name,
                                                               const char* operation_name,
                                                               void* callback, void* user_data);
extern int __real_onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx,
                                                       const char* fault_code,
                                                       const char* fault_string,
                                                       const char* fault_detail);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void gsoap_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * gSOAP Core Functions
 * ============================================================================ */

int __wrap_onvif_gsoap_init(onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_init(ctx);
  }

  function_called();
  return (int)mock();
}

void __wrap_onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    __real_onvif_gsoap_cleanup(ctx);
    return;
  }

  function_called();
  // void return - no mock() call needed
}

void __wrap_onvif_gsoap_reset(onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    __real_onvif_gsoap_reset(ctx);
    return;
  }

  function_called();
  // void return - no mock() call needed
}

int __wrap_onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_has_error(ctx);
  }

  function_called();
  return (int)mock();
}

const char* __wrap_onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_error(ctx);
  }

  function_called();
  return (const char*)mock();
}

const char* __wrap_onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_response_data(ctx);
  }

  function_called();
  return (const char*)mock();
}

size_t __wrap_onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_response_length(ctx);
  }

  function_called();
  return (size_t)mock();
}

/* ============================================================================
 * gSOAP Response Generation Functions
 * ============================================================================ */

int __wrap_onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                       const char* service_name,
                                                       const char* operation_name, void* callback,
                                                       void* user_data) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_generate_response_with_callback(ctx, service_name, operation_name,
                                                              callback, user_data);
  }

  check_expected_ptr(service_name);
  check_expected_ptr(operation_name);
  check_expected_ptr(callback);
  check_expected_ptr(user_data);
  function_called();
  return (int)mock();
}

int __wrap_onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, const char* fault_code,
                                               const char* fault_string,
                                               const char* fault_detail) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_generate_fault_response(ctx, fault_code, fault_string, fault_detail);
  }

  check_expected_ptr(fault_code);
  check_expected_ptr(fault_string);
  check_expected_ptr(fault_detail);
  function_called();
  return (int)mock();
}
