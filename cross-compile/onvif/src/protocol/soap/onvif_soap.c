/**
 * @file onvif_soap.c
 * @brief Unified SOAP response generator implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This module consolidates all SOAP response functionality to eliminate
 * duplication and provide a single, consistent API for SOAP operations.
 */

#include "onvif_soap.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "services/common/onvif_request.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

/**
 * @brief Service namespace configuration
 */
typedef struct {
  const char *prefix;
  const char *uri;
} service_namespace_t;

static const service_namespace_t service_namespaces[] = {
    [ONVIF_SERVICE_DEVICE] = {"tds", "http://www.onvif.org/ver10/device/wsdl"},
    [ONVIF_SERVICE_MEDIA] = {"trt", "http://www.onvif.org/ver10/media/wsdl"},
    [ONVIF_SERVICE_PTZ] = {"tptz", "http://www.onvif.org/ver20/ptz/wsdl"},
    [ONVIF_SERVICE_IMAGING] = {"timg",
                               "http://www.onvif.org/ver20/imaging/wsdl"}};

/* ============================================================================
 * SOAP Response Generation
 * ============================================================================
 */

const char *soap_get_namespace_prefix(onvif_service_type_t service_type) {
  if (service_type >= 0 && service_type < sizeof(service_namespaces) /
                                              sizeof(service_namespaces[0])) {
    return service_namespaces[service_type].prefix;
  }
  return "tds";  // Default to device service
}

const char *soap_get_namespace_uri(onvif_service_type_t service_type) {
  if (service_type >= 0 && service_type < sizeof(service_namespaces) /
                                              sizeof(service_namespaces[0])) {
    return service_namespaces[service_type].uri;
  }
  return service_namespaces[ONVIF_SERVICE_DEVICE]
      .uri;  // Default to device service
}

int soap_generate_fault(char *response, size_t response_size,
                        const char *fault_code, const char *fault_string) {
  if (!response || !fault_code || !fault_string) {
    return ONVIF_ERROR_INVALID;
  }

  int result = snprintf(
      response, response_size,
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
      "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
      "  <soap:Body>\n"
      "    <soap:Fault>\n"
      "      <soap:Code>\n"
      "        <soap:Value>%s</soap:Value>\n"
      "      </soap:Code>\n"
      "      <soap:Reason>\n"
      "        <soap:Text>%s</soap:Text>\n"
      "      </soap:Reason>\n"
      "    </soap:Fault>\n"
      "  </soap:Body>\n"
      "</soap:Envelope>",
      fault_code, fault_string);

  if (result < 0 || (size_t)result >= response_size) {
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int soap_generate_success(char *response,
                          onvif_service_type_t service_type,  // NOLINT
                          size_t response_size, const char *action_name,
                          const char *body_content) {
  if (!response || !action_name || !body_content) {
    return ONVIF_ERROR_INVALID;
  }

  const char *prefix = soap_get_namespace_prefix(service_type);
  const char *uri = soap_get_namespace_uri(service_type);

  int result = snprintf(
      response, response_size,
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
      "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
      "  <soap:Body>\n"
      "    <%s:%sResponse xmlns:%s=\"%s\">\n"
      "      %s\n"
      "    </%s:%sResponse>\n"
      "  </soap:Body>\n"
      "</soap:Envelope>",
      prefix, action_name, prefix, uri, body_content, prefix, action_name);

  if (result < 0 || (size_t)result >= response_size) {
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int soap_generate_response(char *response, size_t response_size,
                           const soap_response_config_t *config) {
  if (!response || !config) {
    return ONVIF_ERROR_INVALID;
  }

  if (config->status_code >= 400) {
    // Generate fault response for error status codes
    return soap_generate_fault(response, response_size, SOAP_FAULT_RECEIVER,
                               "Service Error");
  }

  return soap_generate_success(response, config->service_type, response_size,
                               config->action_name, config->body_content);
}

int onvif_generate_complete_response(onvif_response_t *response,
                                     onvif_service_type_t service_type,
                                     const char *action_name,
                                     const char *body_content) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response structure
  response->status_code = 200;
  response->content_type = "application/soap+xml";

  // Allocate response body if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return ONVIF_ERROR;
    }
  }

  // Generate SOAP response
  int result = soap_generate_success(response->body, service_type,
                                     ONVIF_RESPONSE_BUFFER_SIZE, action_name,
                                     body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  response->body_length = strlen(response->body);
  return ONVIF_SUCCESS;
}

int onvif_generate_fault_response(onvif_response_t *response,
                                  const char *fault_code,
                                  const char *fault_string) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize response structure
  response->status_code = 500;
  response->content_type = "application/soap+xml";

  // Allocate response body if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return ONVIF_ERROR;
    }
  }

  // Generate SOAP fault response
  int result = soap_generate_fault(response->body, ONVIF_RESPONSE_BUFFER_SIZE,
                                   fault_code, fault_string);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  response->body_length = strlen(response->body);
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Response Buffer Management
 * ============================================================================
 */

int response_buffer_init(response_buffer_t *buffer,
                         onvif_response_t *response) {
  if (!buffer) {
    return ONVIF_ERROR_INVALID;
  }

  buffer->response = response;
  buffer->owned = (response == NULL);

  if (buffer->owned) {
    buffer->response = ONVIF_MALLOC(sizeof(onvif_response_t));
    if (!buffer->response) {
      return ONVIF_ERROR;
    }

    memset(buffer->response, 0, sizeof(onvif_response_t));
  }

  return ONVIF_SUCCESS;
}

int response_buffer_create(response_buffer_t *buffer) {
  return response_buffer_init(buffer, NULL);
}

void response_buffer_cleanup(response_buffer_t *buffer) {
  if (buffer) {
    if (buffer->owned && buffer->response) {
      onvif_response_cleanup(buffer->response);
      ONVIF_FREE(buffer->response);
    }

    buffer->response = NULL;
    buffer->owned = 0;
  }
}

onvif_response_t *response_buffer_get(response_buffer_t *buffer) {
  return buffer ? buffer->response : NULL;
}

int response_buffer_set_body(response_buffer_t *buffer, const char *body,
                             size_t length) {
  if (!buffer || !buffer->response || !body) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate or reallocate body buffer
  if (buffer->response->body) {
    ONVIF_FREE(buffer->response->body);
  }

  buffer->response->body = ONVIF_MALLOC(length + 1);
  if (!buffer->response->body) {
    return ONVIF_ERROR;
  }

  memcpy(buffer->response->body, body, length);
  buffer->response->body[length] = '\0';
  buffer->response->body_length = length;

  return ONVIF_SUCCESS;
}

int response_buffer_set_body_printf(response_buffer_t *buffer,
                                    const char *format, ...) {
  if (!buffer || !buffer->response || !format) {
    return ONVIF_ERROR_INVALID;
  }

  va_list args;
  va_start(args, format);

  // Calculate required size
  va_list args_copy;
  va_copy(args_copy, args);
  int size = vsnprintf(NULL, 0, format, args_copy);
  va_end(args_copy);

  if (size < 0) {
    va_end(args);
    return ONVIF_ERROR;
  }

  // Allocate buffer
  char *temp_buffer = ONVIF_MALLOC(size + 1);
  if (!temp_buffer) {
    va_end(args);
    return ONVIF_ERROR;
  }

  // Format the string
  (void)vsnprintf(temp_buffer, size + 1, format, args);
  va_end(args);

  // Set the body
  int result = response_buffer_set_body(buffer, temp_buffer, size);
  ONVIF_FREE(temp_buffer);

  return result;
}

/* ============================================================================
 * Response Helpers
 * ============================================================================
 */

int onvif_response_init(onvif_response_t *response, size_t buffer_size) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  memset(response, 0, sizeof(onvif_response_t));

  if (buffer_size > 0) {
    response->body = ONVIF_MALLOC(buffer_size);
    if (!response->body) {
      return ONVIF_ERROR;
    }
  }

  response->status_code = 200;
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

void onvif_response_cleanup(onvif_response_t *response) {
  if (response) {
    if (response->body) {
      ONVIF_FREE(response->body);
      response->body = NULL;
    }

    response->body_length = 0;
    response->status_code = 0;
    response->content_type = NULL;
  }
}

int onvif_response_set_body(onvif_response_t *response,
                            const char *body_content) {
  if (!response || !body_content) {
    return ONVIF_ERROR_INVALID;
  }

  size_t content_length = strlen(body_content);

  // Allocate or reallocate body buffer
  if (response->body) {
    ONVIF_FREE(response->body);
  }

  response->body = ONVIF_MALLOC(content_length + 1);
  if (!response->body) {
    return ONVIF_ERROR;
  }

  strcpy(response->body, body_content);
  response->body_length = content_length;

  return ONVIF_SUCCESS;
}

int onvif_response_set_body_printf(onvif_response_t *response,
                                   const char *format, ...) {
  if (!response || !format) {
    return ONVIF_ERROR_INVALID;
  }

  va_list args;
  va_start(args, format);

  // Calculate required size
  va_list args_copy;
  va_copy(args_copy, args);
  int size = vsnprintf(NULL, 0, format, args_copy);
  va_end(args_copy);

  if (size < 0) {
    va_end(args);
    return ONVIF_ERROR;
  }

  // Allocate buffer
  char *temp_buffer = ONVIF_MALLOC(size + 1);
  if (!temp_buffer) {
    va_end(args);
    return ONVIF_ERROR;
  }

  // Format the string
  (void)vsnprintf(temp_buffer, size + 1, format, args);
  va_end(args);

  // Set the body
  int result = onvif_response_set_body(response, temp_buffer);
  ONVIF_FREE(temp_buffer);

  return result;
}

int onvif_response_soap_fault(onvif_response_t *response,
                              const char *fault_code,
                              const char *fault_string) {
  if (!response || !fault_code || !fault_string) {
    return ONVIF_ERROR_INVALID;
  }

  response->status_code = 500;
  response->content_type = "application/soap+xml";

  // Allocate response body if not already allocated
  if (!response->body) {
    response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!response->body) {
      return ONVIF_ERROR;
    }
  }

  // Generate SOAP fault response
  int result = soap_generate_fault(response->body, ONVIF_RESPONSE_BUFFER_SIZE,
                                   fault_code, fault_string);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  response->body_length = strlen(response->body);
  return ONVIF_SUCCESS;
}

int onvif_response_device_success(onvif_response_t *response,
                                  const char *action,
                                  const char *body_content) {
  return onvif_generate_complete_response(response, ONVIF_SERVICE_DEVICE,
                                          action, body_content);
}

int onvif_response_media_success(onvif_response_t *response, const char *action,
                                 const char *body_content) {
  return onvif_generate_complete_response(response, ONVIF_SERVICE_MEDIA, action,
                                          body_content);
}

int onvif_response_ptz_success(onvif_response_t *response, const char *action,
                               const char *body_content) {
  return onvif_generate_complete_response(response, ONVIF_SERVICE_PTZ, action,
                                          body_content);
}

int onvif_response_imaging_success(onvif_response_t *response,
                                   const char *action,
                                   const char *body_content) {
  return onvif_generate_complete_response(response, ONVIF_SERVICE_IMAGING,
                                          action, body_content);
}
