/**
 * @file http_onvif_adapter.h
 * @brief HTTP to ONVIF request/response adapter.
 * 
 * This module provides conversion functions between HTTP-specific structures
 * and transport-agnostic ONVIF request/response structures.
 */

#ifndef HTTP_ONVIF_ADAPTER_H
#define HTTP_ONVIF_ADAPTER_H

#include "http_parser.h"
#include "common/onvif_request.h"

/**
 * @brief Convert HTTP request to ONVIF request
 * @param http_req HTTP request structure
 * @param onvif_req ONVIF request structure (output)
 * @return 0 on success, negative error code on failure
 */
int http_to_onvif_request(const http_request_t *http_req, onvif_request_t *onvif_req);

/**
 * @brief Convert ONVIF response to HTTP response
 * @param onvif_resp ONVIF response structure
 * @param http_resp HTTP response structure (output)
 * @return 0 on success, negative error code on failure
 */
int onvif_to_http_response(const onvif_response_t *onvif_resp, http_response_t *http_resp);

/**
 * @brief Cleanup ONVIF request (free allocated memory)
 * @param onvif_req ONVIF request structure
 */
void onvif_request_cleanup(onvif_request_t *onvif_req);

/**
 * @brief Cleanup ONVIF response (free allocated memory)
 * @param onvif_resp ONVIF response structure
 */
void onvif_response_cleanup(onvif_response_t *onvif_resp);

#endif /* HTTP_ONVIF_ADAPTER_H */
