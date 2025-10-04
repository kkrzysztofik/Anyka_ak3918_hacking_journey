/**
 * @file xml_validation_helpers.c
 * @brief Implementation of XML parsing and validation helpers using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 */

#include "xml_validation_helpers.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "utils/error/error_handling.h"

int validate_soap_fault_xml(const char* xml, const char* fault_code, const char* fault_string,
                            const char* fault_detail) {
  if (!xml || !fault_code || !fault_string) {
    return ONVIF_ERROR_INVALID;
  }

  // Create temporary gSOAP context for parsing
  onvif_gsoap_context_t ctx;
  if (onvif_gsoap_init(&ctx) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Initialize request parsing with the XML
  int result = onvif_gsoap_init_request_parsing(&ctx, xml, strlen(xml));
  if (result != ONVIF_SUCCESS) {
    onvif_gsoap_cleanup(&ctx);
    return result;
  }

  // For fault responses, we don't need to parse the envelope completely
  // Just check if the XML contains SOAP Fault elements and the expected values
  onvif_gsoap_cleanup(&ctx);

  // Check for SOAP Fault structure
  if (!strstr(xml, "SOAP-ENV:Fault") && !strstr(xml, "soap:Fault")) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate fault code - use simple string search
  if (!strstr(xml, fault_code)) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate fault string
  if (!strstr(xml, fault_string)) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate fault detail if provided
  if (fault_detail && !strstr(xml, fault_detail)) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_soap_envelope(const char* xml) {
  if (!xml) {
    return ONVIF_ERROR_INVALID;
  }

  // Create temporary gSOAP context for parsing
  onvif_gsoap_context_t ctx;
  if (onvif_gsoap_init(&ctx) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Initialize request parsing with the XML
  int result = onvif_gsoap_init_request_parsing(&ctx, xml, strlen(xml));
  if (result != ONVIF_SUCCESS) {
    onvif_gsoap_cleanup(&ctx);
    return result;
  }

  // Parse SOAP envelope - if this succeeds, envelope is valid
  result = onvif_gsoap_parse_soap_envelope(&ctx, __func__);

  onvif_gsoap_cleanup(&ctx);
  return result;
}

int is_well_formed_xml(const char* xml) {
  if (!xml || strlen(xml) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Create temporary gSOAP context for parsing
  onvif_gsoap_context_t ctx;
  if (onvif_gsoap_init(&ctx) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }

  // Try to parse - if gSOAP can parse it, it's well-formed
  int result = onvif_gsoap_init_request_parsing(&ctx, xml, strlen(xml));

  onvif_gsoap_cleanup(&ctx);
  return result;
}
