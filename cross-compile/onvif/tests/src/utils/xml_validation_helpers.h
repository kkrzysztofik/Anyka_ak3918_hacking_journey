/**
 * @file xml_validation_helpers.h
 * @brief XML parsing and validation helpers for gSOAP response testing using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef XML_VALIDATION_HELPERS_H
#define XML_VALIDATION_HELPERS_H

#include <stddef.h>

#include "protocol/gsoap/onvif_gsoap_core.h"

/**
 * @brief Validate SOAP fault response by parsing with gSOAP
 * @param xml XML document to validate
 * @param fault_code Expected fault code (e.g., "soap:Client")
 * @param fault_string Expected fault string
 * @param fault_detail Expected fault detail (can be NULL to skip)
 * @return ONVIF_SUCCESS if valid SOAP fault with expected values, error code otherwise
 * @note Uses gSOAP deserialization to parse and validate
 */
int validate_soap_fault_xml(const char* xml, const char* fault_code, const char* fault_string,
                            const char* fault_detail);

/**
 * @brief Validate SOAP envelope structure using gSOAP
 * @param xml XML document to validate
 * @return ONVIF_SUCCESS if valid SOAP envelope, error code otherwise
 */
int validate_soap_envelope(const char* xml);

/**
 * @brief Check if XML is valid and well-formed
 * @param xml XML document to validate
 * @return ONVIF_SUCCESS if well-formed, error code otherwise
 */
int is_well_formed_xml(const char* xml);

#endif // XML_VALIDATION_HELPERS_H
