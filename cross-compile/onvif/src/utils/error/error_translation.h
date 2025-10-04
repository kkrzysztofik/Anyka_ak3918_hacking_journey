/**
 * @file error_translation.h
 * @brief Error code translation utilities
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ERROR_TRANSLATION_H
#define ERROR_TRANSLATION_H

#include "stdsoap2.h"
#include "utils/error/error_handling.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Convert ONVIF error code to string
 * @param error_code ONVIF error code
 * @return Human-readable error string
 */
static inline const char* onvif_error_to_string(int error_code) {
  switch (error_code) {
  case ONVIF_SUCCESS:
    return "ONVIF_SUCCESS";
  case ONVIF_ERROR_INVALID:
    return "ONVIF_ERROR_INVALID";
  case ONVIF_ERROR_MEMORY:
    return "ONVIF_ERROR_MEMORY";
  case ONVIF_ERROR_PARSE_FAILED:
    return "ONVIF_ERROR_PARSE_FAILED";
  case ONVIF_ERROR_NOT_IMPLEMENTED:
    return "ONVIF_ERROR_NOT_IMPLEMENTED";
  case ONVIF_ERROR_NOT_FOUND:
    return "ONVIF_ERROR_NOT_FOUND";
  case ONVIF_ERROR_ALREADY_EXISTS:
    return "ONVIF_ERROR_ALREADY_EXISTS";
  case ONVIF_ERROR_TIMEOUT:
    return "ONVIF_ERROR_TIMEOUT";
  case ONVIF_ERROR_NETWORK:
    return "ONVIF_ERROR_NETWORK";
  case ONVIF_ERROR_AUTHENTICATION_FAILED:
    return "ONVIF_ERROR_AUTHENTICATION_FAILED";
  case ONVIF_ERROR_AUTHORIZATION_FAILED:
    return "ONVIF_ERROR_AUTHORIZATION_FAILED";
  case ONVIF_ERROR_INVALID_PARAMETER:
    return "ONVIF_ERROR_INVALID_PARAMETER";
  case ONVIF_ERROR_NULL:
    return "ONVIF_ERROR_NULL";
  default:
    return "UNKNOWN_ERROR";
  }
}

/**
 * @brief Convert gSOAP error code to string
 * @param soap_error gSOAP error code
 * @return Human-readable error string
 */
static inline const char* soap_error_to_string(int soap_error) {
  switch (soap_error) {
  case SOAP_OK:
    return "SOAP_OK";
  case SOAP_CLI_FAULT:
    return "SOAP_CLI_FAULT";
  case SOAP_SVR_FAULT:
    return "SOAP_SVR_FAULT";
  case SOAP_TAG_MISMATCH:
    return "SOAP_TAG_MISMATCH";
  case SOAP_TYPE:
    return "SOAP_TYPE";
  case SOAP_SYNTAX_ERROR:
    return "SOAP_SYNTAX_ERROR";
  case SOAP_NO_TAG:
    return "SOAP_NO_TAG";
  case SOAP_IOB:
    return "SOAP_IOB";
  case SOAP_MUSTUNDERSTAND:
    return "SOAP_MUSTUNDERSTAND";
  case SOAP_NAMESPACE:
    return "SOAP_NAMESPACE";
  case SOAP_USER_ERROR:
    return "SOAP_USER_ERROR";
  case SOAP_FATAL_ERROR:
    return "SOAP_FATAL_ERROR";
  case SOAP_FAULT:
    return "SOAP_FAULT";
  case SOAP_VERSIONMISMATCH:
    return "SOAP_VERSIONMISMATCH";
  case SOAP_DATAENCODINGUNKNOWN:
    return "SOAP_DATAENCODINGUNKNOWN";
  default:
    return "UNKNOWN_SOAP_ERROR";
  }
}

#ifdef __cplusplus
}
#endif

#endif /* ERROR_TRANSLATION_H */
