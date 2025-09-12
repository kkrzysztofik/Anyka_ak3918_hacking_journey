/**
 * @file unified_soap_generator.c
 * @brief Unified SOAP response generator implementation
 */

#include "unified_soap_generator.h"
#include "utils/error_handling.h"
#include "utils/safe_string.h"
#include "utils/memory_manager.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/**
 * @brief Service namespace configuration
 */
typedef struct {
  const char *prefix;
  const char *uri;
} service_namespace_t;

static const service_namespace_t service_namespaces[] = {
  [ONVIF_SERVICE_DEVICE]   = {"tds", "http://www.onvif.org/ver10/device/wsdl"},
  [ONVIF_SERVICE_MEDIA]    = {"trt", "http://www.onvif.org/ver10/media/wsdl"},
  [ONVIF_SERVICE_PTZ]      = {"tptz", "http://www.onvif.org/ver20/ptz/wsdl"},
  [ONVIF_SERVICE_IMAGING]  = {"timg", "http://www.onvif.org/ver20/imaging/wsdl"}
};

const char *soap_get_namespace_prefix(onvif_service_type_t service_type) {
  if (service_type >= 0 && service_type < sizeof(service_namespaces) / sizeof(service_namespaces[0])) {
    return service_namespaces[service_type].prefix;
  }
  return "tds"; // Default to device service
}

const char *soap_get_namespace_uri(onvif_service_type_t service_type) {
  if (service_type >= 0 && service_type < sizeof(service_namespaces) / sizeof(service_namespaces[0])) {
    return service_namespaces[service_type].uri;
  }
  return service_namespaces[ONVIF_SERVICE_DEVICE].uri; // Default to device service
}

int soap_generate_fault(char *response, size_t response_size, const char *fault_code, const char *fault_string) {
  if (!response || !fault_code || !fault_string) {
    return ONVIF_ERROR_INVALID;
  }

  int result = snprintf(response, response_size,
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
    "</soap:Envelope>", fault_code, fault_string);

  if (result < 0 || (size_t)result >= response_size) {
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int soap_generate_success(char *response, size_t response_size, onvif_service_type_t service_type,
                         const char *action_name, const char *body_content) {
  if (!response || !action_name || !body_content) {
    return ONVIF_ERROR_INVALID;
  }

  const char *prefix = soap_get_namespace_prefix(service_type);
  const char *uri = soap_get_namespace_uri(service_type);

  int result = snprintf(response, response_size,
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

int soap_generate_response(char *response, size_t response_size, const soap_response_config_t *config) {
  if (!response || !config) {
    return ONVIF_ERROR_INVALID;
  }

  if (config->status_code >= 400) {
    // Generate fault response for error status codes
    return soap_generate_fault(response, response_size, SOAP_FAULT_RECEIVER, "Service Error");
  }

  return soap_generate_success(response, response_size, config->service_type, 
                              config->action_name, config->body_content);
}

int onvif_generate_complete_response(onvif_response_t *response, onvif_service_type_t service_type,
                                    const char *action_name, const char *body_content) {
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
  int result = soap_generate_success(response->body, ONVIF_RESPONSE_BUFFER_SIZE, 
                                   service_type, action_name, body_content);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  response->body_length = strlen(response->body);
  return ONVIF_SUCCESS;
}

int onvif_generate_fault_response(onvif_response_t *response, const char *fault_code, const char *fault_string) {
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
  int result = soap_generate_fault(response->body, ONVIF_RESPONSE_BUFFER_SIZE, fault_code, fault_string);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  response->body_length = strlen(response->body);
  return ONVIF_SUCCESS;
}
