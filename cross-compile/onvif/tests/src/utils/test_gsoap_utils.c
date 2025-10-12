/**
 * @file test_gsoap_utils.c
 * @brief Shared utilities for gSOAP protocol tests
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_gsoap_utils.h"

#include <stdio.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "core/config/config.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "utils/error/error_handling.h"

/**
 * @brief Setup mock expectation for config_runtime_get_int http_verbose call
 * @note This helper encapsulates the mock setup pattern used before every
 *       onvif_gsoap_init() call. Each init() invocation requires a fresh
 *       mock expectation. The mock checks parameters, calls function_called(),
 *       then returns the mocked value.
 */
void setup_http_verbose_mock(void) {
  expect_function_call(__wrap_config_runtime_get_int);
  expect_value(__wrap_config_runtime_get_int, section, CONFIG_SECTION_LOGGING);
  expect_string(__wrap_config_runtime_get_int, key, "http_verbose");
  expect_any(__wrap_config_runtime_get_int, out_value);
  will_return(__wrap_config_runtime_get_int, ONVIF_SUCCESS);
}

/**
 * @brief Helper function to set up context for parsing tests
 * @param ctx gSOAP context to initialize
 * @param soap_request SOAP request envelope string
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int setup_parsing_test(onvif_gsoap_context_t* ctx, const char* soap_request) {
  if (!ctx || !soap_request) {
    return ONVIF_ERROR_INVALID;
  }

  // Mock config_runtime_get_int call for http_verbose check
  setup_http_verbose_mock();

  // Initialize context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Initialize request parsing
  result = onvif_gsoap_init_request_parsing(ctx, soap_request, strlen(soap_request));
  if (result != ONVIF_SUCCESS) {
    onvif_gsoap_cleanup(ctx);
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Suite setup - enable real gSOAP functions for protocol parsing tests
 * @param state Test state (unused)
 * @return 0 on success
 * @note These tests validate SOAP parsing, so they need real gSOAP functionality.
 *       The __wrap_ functions stay compiled but route to __real_ implementations.
 */
int gsoap_core_suite_setup(void** state) {
  (void)state;
  gsoap_mock_use_real_function(true);
  return 0;
}

/**
 * @brief Suite teardown - restore mock behavior for other test suites
 * @param state Test state (unused)
 * @return 0 on success
 */
int gsoap_core_suite_teardown(void** state) {
  (void)state;
  gsoap_mock_use_real_function(false);
  return 0;
}
