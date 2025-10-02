/**
 * @file soap_error_tests.h
 * @brief SOAP error handling integration tests for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SOAP_ERROR_TESTS_H
#define SOAP_ERROR_TESTS_H

/**
 * @brief Setup function for SOAP error tests
 * @param state Test state pointer
 * @return 0 on success, non-zero on failure
 */
int soap_error_tests_setup(void** state);

/**
 * @brief Teardown function for SOAP error tests
 * @param state Test state pointer
 * @return 0 on success, non-zero on failure
 */
int soap_error_tests_teardown(void** state);

/**
 * @brief Test SOAP error handling for invalid XML
 *
 * Tests that the server properly handles malformed XML syntax by generating
 * an appropriate SOAP fault response with fault code and descriptive message.
 *
 * @param state Test state pointer (unused)
 */
void test_integration_soap_error_invalid_xml(void** state);

/**
 * @brief Test SOAP error handling for missing required parameter
 *
 * Tests that the server properly handles requests with missing required
 * parameters (e.g., GetStreamUri without ProfileToken) by generating
 * a SOAP fault with appropriate client error code.
 *
 * @param state Test state pointer (unused)
 */
void test_integration_soap_error_missing_param(void** state);

/**
 * @brief Test SOAP error handling for wrong operation name
 *
 * Tests that the server properly handles requests for non-existent
 * operations by generating a SOAP fault indicating unknown operation.
 *
 * @param state Test state pointer (unused)
 */
void test_integration_soap_error_wrong_operation(void** state);

/**
 * @brief Test SOAP error handling for malformed envelope
 *
 * Tests that the server properly handles empty or malformed SOAP
 * envelopes by generating appropriate fault responses.
 *
 * @param state Test state pointer (unused)
 */
void test_integration_soap_error_malformed_envelope(void** state);

#endif /* SOAP_ERROR_TESTS_H */
