/**
 * @file gsoap_mock.c
 * @brief CMocka-based gSOAP mock implementation using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include "gsoap_mock.h"

/* ============================================================================
 * gSOAP Core Functions
 * ============================================================================ */

int __wrap_onvif_gsoap_init(void) {
  function_called();
  return (int)mock();
}

void __wrap_onvif_gsoap_cleanup(void) {
  function_called();
  // void return - no mock() call needed
}

void __wrap_onvif_gsoap_reset(void) {
  function_called();
  // void return - no mock() call needed
}

int __wrap_onvif_gsoap_has_error(void) {
  function_called();
  return (int)mock();
}

int __wrap_onvif_gsoap_get_error(void) {
  function_called();
  return (int)mock();
}

const char* __wrap_onvif_gsoap_get_response_data(void) {
  function_called();
  return (const char*)mock();
}

size_t __wrap_onvif_gsoap_get_response_length(void) {
  function_called();
  return (size_t)mock();
}

/* ============================================================================
 * gSOAP Response Generation Functions
 * ============================================================================ */

int __wrap_onvif_gsoap_generate_response_with_callback(const char* service_name,
                                                        const char* operation_name,
                                                        void* callback,
                                                        void* user_data) {
  check_expected_ptr(service_name);
  check_expected_ptr(operation_name);
  check_expected_ptr(callback);
  check_expected_ptr(user_data);
  function_called();
  return (int)mock();
}

int __wrap_onvif_gsoap_generate_fault_response(const char* fault_code,
                                                const char* fault_string,
                                                const char* fault_detail) {
  check_expected_ptr(fault_code);
  check_expected_ptr(fault_string);
  check_expected_ptr(fault_detail);
  function_called();
  return (int)mock();
}
