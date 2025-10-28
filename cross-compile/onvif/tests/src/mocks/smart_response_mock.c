/**
 * @file smart_response_mock.c
 * @brief Implementation of CMocka-based smart response builder mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "smart_response_mock.h"

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Real Function Declarations (for passthrough mode)
 * ============================================================================
 */

extern int __real_smart_response_build_with_dynamic_buffer(http_response_t* response, const char* soap_content);
extern int __real_smart_response_build_with_buffer_pool(http_response_t* response, const char* soap_content, buffer_pool_t* buffer_pool);
extern int __real_smart_response_build(http_response_t* response, const char* soap_content, size_t estimated_size, buffer_pool_t* buffer_pool);
extern size_t __real_smart_response_estimate_size(const char* soap_content);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void smart_response_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * CMocka Wrapped Smart Response Functions
 * ============================================================================ */

int __wrap_smart_response_build_with_dynamic_buffer(http_response_t* response, const char* soap_content) {
  if (g_use_real_functions) {
    return __real_smart_response_build_with_dynamic_buffer(response, soap_content);
  }

  function_called();
  check_expected_ptr(response);
  check_expected_ptr(soap_content);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS && response != NULL && soap_content != NULL) {
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-const-cast)
    response->body = (char*)soap_content;
    response->body_length = (int)strlen(soap_content);
    response->status_code = HTTP_STATUS_OK;
  }
  return result;
}

int __wrap_smart_response_build_with_buffer_pool(http_response_t* response, const char* soap_content, buffer_pool_t* buffer_pool) {
  if (g_use_real_functions) {
    return __real_smart_response_build_with_buffer_pool(response, soap_content, buffer_pool);
  }

  function_called();
  check_expected_ptr(response);
  check_expected_ptr(soap_content);
  check_expected_ptr(buffer_pool);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS && response != NULL && soap_content != NULL) {
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-const-cast)
    response->body = (char*)soap_content;
    response->body_length = (int)strlen(soap_content);
    response->status_code = HTTP_STATUS_OK;
  }
  return result;
}

int __wrap_smart_response_build(http_response_t* response, const char* soap_content, size_t estimated_size, buffer_pool_t* buffer_pool) {
  if (g_use_real_functions) {
    return __real_smart_response_build(response, soap_content, estimated_size, buffer_pool);
  }

  function_called();
  check_expected_ptr(response);
  check_expected_ptr(soap_content);
  check_expected(estimated_size);
  check_expected_ptr(buffer_pool);

  int result = (int)mock();
  if (result == ONVIF_SUCCESS && response != NULL && soap_content != NULL) {
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-const-cast)
    response->body = (char*)soap_content;
    response->body_length = (int)strlen(soap_content);
    response->status_code = HTTP_STATUS_OK;
  }
  return result;
}

size_t __wrap_smart_response_estimate_size(const char* soap_content) {
  if (g_use_real_functions) {
    return __real_smart_response_estimate_size(soap_content);
  }

  function_called();
  check_expected_ptr(soap_content);
  return (size_t)mock();
}
