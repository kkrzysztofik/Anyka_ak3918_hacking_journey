/**
 * @file test_gsoap_utils.c
 * @brief Shared utilities for gSOAP protocol tests
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_gsoap_utils.h"

#include <stdio.h>
#include <string.h>

#include "mocks/gsoap_mock.h"
#include "utils/error/error_handling.h"

/**
 * @brief Helper function to set up context for parsing tests
 * @param ctx gSOAP context to initialize
 * @param soap_request SOAP request envelope string
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int setup_parsing_test(onvif_gsoap_context_t* ctx, const char* soap_request) {
  if (!ctx || !soap_request) {
    printf("DEBUG: setup_parsing_test: Invalid parameters - ctx=%p, soap_request=%p\n", ctx,
           soap_request);
    return ONVIF_ERROR_INVALID;
  }

  printf("DEBUG: setup_parsing_test: Starting setup with SOAP request length: %zu\n",
         strlen(soap_request));

  // Initialize context
  printf("DEBUG: setup_parsing_test: Calling onvif_gsoap_init\n");
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    printf("DEBUG: setup_parsing_test: onvif_gsoap_init failed with result: %d\n", result);
    return result;
  }
  printf("DEBUG: setup_parsing_test: onvif_gsoap_init succeeded\n");

  // Initialize request parsing
  printf("DEBUG: setup_parsing_test: Calling onvif_gsoap_init_request_parsing\n");
  result = onvif_gsoap_init_request_parsing(ctx, soap_request, strlen(soap_request));
  if (result != ONVIF_SUCCESS) {
    printf("DEBUG: setup_parsing_test: onvif_gsoap_init_request_parsing failed with result: %d\n",
           result);
    onvif_gsoap_cleanup(ctx);
    return result;
  }
  printf("DEBUG: setup_parsing_test: onvif_gsoap_init_request_parsing succeeded\n");

  printf("DEBUG: setup_parsing_test: Setup completed successfully\n");
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
