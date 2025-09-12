/**
 * @file response_helpers.c
 * @brief Common response handling utilities for ONVIF services
 */

#include "response_helpers.h"
#include "unified_soap_generator.h"
#include "utils/error_handling.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>

int onvif_response_init(onvif_response_t *response, size_t buffer_size) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }
  
  response->status_code = 200;
  response->content_type = "application/soap+xml";
  response->body = malloc(buffer_size);
  if (!response->body) {
    return ONVIF_ERROR;
  }
  response->body_length = 0;
  response->body[0] = '\0';
  
  return ONVIF_SUCCESS;
}

void onvif_response_cleanup(onvif_response_t *response) {
  if (response && response->body) {
    free(response->body);
    response->body = NULL;
    response->body_length = 0;
  }
}

int onvif_response_set_body(onvif_response_t *response, const char *body_content) {
  if (!response || !response->body || !body_content) {
    return ONVIF_ERROR_INVALID;
  }
  
  strncpy(response->body, body_content, ONVIF_RESPONSE_BUFFER_SIZE - 1);
  response->body[ONVIF_RESPONSE_BUFFER_SIZE - 1] = '\0';
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}

int onvif_response_set_body_printf(onvif_response_t *response, const char *format, ...) {
  if (!response || !response->body || !format) {
    return ONVIF_ERROR_INVALID;
  }
  
  va_list args;
  va_start(args, format);
  int result = vsnprintf(response->body, ONVIF_RESPONSE_BUFFER_SIZE, format, args);
  va_end(args);
  
  if (result < 0) {
    return ONVIF_ERROR;
  }
  
  response->body_length = (result < ONVIF_RESPONSE_BUFFER_SIZE) ? result : ONVIF_RESPONSE_BUFFER_SIZE - 1;
  response->body[response->body_length] = '\0';
  
  return ONVIF_SUCCESS;
}

int onvif_response_soap_fault(onvif_response_t *response, const char *fault_code, const char *fault_string) {
  if (!response || !response->body) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = soap_generate_fault(response->body, ONVIF_RESPONSE_BUFFER_SIZE, fault_code, fault_string);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}

int onvif_response_device_success(onvif_response_t *response, const char *action, const char *body_content) {
  if (!response || !response->body || !action || !body_content) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = soap_generate_success(response->body, ONVIF_RESPONSE_BUFFER_SIZE, ONVIF_SERVICE_DEVICE, action, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}

int onvif_response_media_success(onvif_response_t *response, const char *action, const char *body_content) {
  if (!response || !response->body || !action || !body_content) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = soap_generate_success(response->body, ONVIF_RESPONSE_BUFFER_SIZE, ONVIF_SERVICE_MEDIA, action, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}

int onvif_response_ptz_success(onvif_response_t *response, const char *action, const char *body_content) {
  if (!response || !response->body || !action || !body_content) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = soap_generate_success(response->body, ONVIF_RESPONSE_BUFFER_SIZE, ONVIF_SERVICE_PTZ, action, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}

int onvif_response_imaging_success(onvif_response_t *response, const char *action, const char *body_content) {
  if (!response || !response->body || !action || !body_content) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = soap_generate_success(response->body, ONVIF_RESPONSE_BUFFER_SIZE, ONVIF_SERVICE_IMAGING, action, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  response->body_length = strlen(response->body);
  
  return ONVIF_SUCCESS;
}
