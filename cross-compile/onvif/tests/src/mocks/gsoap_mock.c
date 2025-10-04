/**
 * @file gsoap_mock.c
 * @brief CMocka-based gSOAP mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include "gsoap_mock.h"

#include <stdbool.h>

#include "cmocka_wrapper.h"


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

int __wrap_onvif_gsoap_init(void) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_init();
  }

  function_called();
  return (int)mock();
}

void __wrap_onvif_gsoap_cleanup(void) {
  if (g_use_real_functions) {
    __real_onvif_gsoap_cleanup();
    return;
  }

  function_called();
  // void return - no mock() call needed
}

void __wrap_onvif_gsoap_reset(void) {
  if (g_use_real_functions) {
    __real_onvif_gsoap_reset();
    return;
  }

  function_called();
  // void return - no mock() call needed
}

int __wrap_onvif_gsoap_has_error(void) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_has_error();
  }

  function_called();
  return (int)mock();
}

int __wrap_onvif_gsoap_get_error(void) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_error();
  }

  function_called();
  return (int)mock();
}

const char* __wrap_onvif_gsoap_get_response_data(void) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_response_data();
  }

  function_called();
  return (const char*)mock();
}

size_t __wrap_onvif_gsoap_get_response_length(void) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_get_response_length();
  }

  function_called();
  return (size_t)mock();
}

/* ============================================================================
 * gSOAP Response Generation Functions
 * ============================================================================ */

int __wrap_onvif_gsoap_generate_response_with_callback(const char* service_name,
                                                       const char* operation_name, void* callback,
                                                       void* user_data) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_generate_response_with_callback(service_name, operation_name,
                                                              callback, user_data);
  }

  check_expected_ptr(service_name);
  check_expected_ptr(operation_name);
  check_expected_ptr(callback);
  check_expected_ptr(user_data);
  function_called();
  return (int)mock();
}

int __wrap_onvif_gsoap_generate_fault_response(const char* fault_code, const char* fault_string,
                                               const char* fault_detail) {
  if (g_use_real_functions) {
    return __real_onvif_gsoap_generate_fault_response(fault_code, fault_string, fault_detail);
  }

  check_expected_ptr(fault_code);
  check_expected_ptr(fault_string);
  check_expected_ptr(fault_detail);
  function_called();
  return (int)mock();
}
