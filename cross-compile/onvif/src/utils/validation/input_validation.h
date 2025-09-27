/**
 * @file input_validation.h
 * @brief Comprehensive input validation utilities for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef INPUT_VALIDATION_H
#define INPUT_VALIDATION_H

#include "networking/http/http_parser.h"

#include <stddef.h>

/**
 * @brief Validate HTTP method
 * @param method HTTP method string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_method(const char* method);

/**
 * @brief Validate HTTP path for security
 * @param path HTTP request path
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_path(const char* path);

/**
 * @brief Validate HTTP version
 * @param version HTTP version string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_version(const char* version);

/**
 * @brief Validate Content-Length header
 * @param content_length Content length value
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_content_length(size_t content_length);

/**
 * @brief Validate SOAP action name
 * @param action SOAP action string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_soap_action(const char* action);

/**
 * @brief Validate XML content for basic security
 * @param xml XML content string
 * @param max_length Maximum allowed length
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_xml_content(const char* xml, size_t max_length);

/**
 * @brief Validate HTTP request comprehensively
 * @param request HTTP request structure
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_request(const http_request_t* request);

/**
 * @brief Sanitize string input
 * @param input Input string to sanitize
 * @param output Output buffer for sanitized string
 * @param output_size Size of output buffer
 * @return ONVIF_OPERATION_SUCCESS on success, ONVIF_OPERATION_FAILED on failure
 */
int sanitize_string_input(const char* input, char* output, size_t output_size);

#endif // INPUT_VALIDATION_H
