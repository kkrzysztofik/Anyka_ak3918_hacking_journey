/**
 * @file onvif_request.h
 * @brief Transport-agnostic ONVIF request/response structures.
 * 
 * This module defines common request and response structures that are
 * independent of the underlying transport protocol (HTTP, SOAP, etc.).
 * This allows services to be decoupled from transport-specific details.
 */

#ifndef ONVIF_REQUEST_H
#define ONVIF_REQUEST_H

#include <stdint.h>
#include <stddef.h>
#include "onvif_types.h"

/**
 * @brief Transport-agnostic ONVIF request structure
 */
typedef struct {
    onvif_action_type_t action;    /**< ONVIF action being requested */
    char *body;                    /**< Request body (SOAP XML, etc.) */
    size_t body_length;            /**< Length of request body */
    char *headers;                 /**< Optional headers (transport-specific) */
    size_t headers_length;         /**< Length of headers */
    void *transport_data;          /**< Opaque transport-specific data */
} onvif_request_t;

/**
 * @brief Transport-agnostic ONVIF response structure
 */
typedef struct {
    int status_code;               /**< Response status code */
    char *body;                    /**< Response body (SOAP XML, etc.) */
    size_t body_length;            /**< Length of response body */
    char *content_type;            /**< Response content type */
    void *transport_data;          /**< Opaque transport-specific data */
} onvif_response_t;

/**
 * @brief Service request handler function type
 */
typedef int (*onvif_service_handler_t)(onvif_action_type_t action, 
                                       const onvif_request_t *request, 
                                       onvif_response_t *response);

#endif /* ONVIF_REQUEST_H */
