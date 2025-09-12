/**
 * @file xml_utils.h
 * @brief XML parsing and generation utilities for ONVIF services.
 */

#ifndef ONVIF_XML_UTILS_H
#define ONVIF_XML_UTILS_H

#include <stddef.h>

/**
 * @brief Extract a value from XML between start and end tags
 * @param xml The XML string to parse
 * @param start_tag The opening tag (e.g., "<tds:Manufacturer>")
 * @param end_tag The closing tag (e.g., "</tds:Manufacturer>")
 * @return Allocated string containing the value, or NULL on failure
 * @note Caller must free the returned string
 */
char* xml_extract_value(const char *xml, const char *start_tag, const char *end_tag);

/**
 * @brief Generate SOAP fault response
 * @param response Buffer to store the response
 * @param response_size Size of the response buffer
 * @param fault_code SOAP fault code
 * @param fault_string SOAP fault string
 */
void xml_soap_fault_response(char *response, size_t response_size, 
                            const char *fault_code, const char *fault_string);

/**
 * @brief Generate SOAP success response
 * @param response Buffer to store the response
 * @param response_size Size of the response buffer
 * @param action The action name for the response
 * @param body_content The XML body content
 */
void xml_soap_success_response(char *response, size_t response_size, 
                              const char *action, const char *body_content);

/**
 * @brief Generate SOAP success response with custom namespace
 * @param response Buffer to store the response
 * @param response_size Size of the response buffer
 * @param action The action name for the response
 * @param namespace The namespace for the response (e.g., "tds", "trt", "tptz", "timg")
 * @param body_content The XML body content
 */
void xml_soap_success_response_ns(char *response, size_t response_size, 
                                 const char *action, const char *namespace, 
                                 const char *body_content);

/**
 * @brief Check if a string contains XML content
 * @param str The string to check
 * @return 1 if contains XML, 0 otherwise
 */
int xml_is_xml_content(const char *str);

/**
 * @brief Escape XML special characters
 * @param input The input string
 * @param output Buffer to store escaped string
 * @param output_size Size of the output buffer
 * @return 0 on success, -1 on error
 */
int xml_escape_string(const char *input, char *output, size_t output_size);

#endif /* ONVIF_XML_UTILS_H */
