/**
 * @file test_gsoap_utils.h
 * @brief Shared utilities for gSOAP protocol tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_GSOAP_UTILS_H
#define TEST_GSOAP_UTILS_H

#include <stddef.h>

#include "protocol/gsoap/onvif_gsoap_core.h"

/**
 * @brief Helper function to set up context for parsing tests
 * @param ctx gSOAP context to initialize
 * @param soap_request SOAP request envelope string
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int setup_parsing_test(onvif_gsoap_context_t* ctx, const char* soap_request);

/**
 * @brief Suite setup - enable real gSOAP functions for protocol parsing tests
 * @param state Test state (unused)
 * @return 0 on success
 * @note These tests validate SOAP parsing, so they need real gSOAP functionality.
 *       The __wrap_ functions stay compiled but route to __real_ implementations.
 */
int gsoap_core_suite_setup(void** state);

/**
 * @brief Suite teardown - restore mock behavior for other test suites
 * @param state Test state (unused)
 * @return 0 on success
 */
int gsoap_core_suite_teardown(void** state);

#endif // TEST_GSOAP_UTILS_H
