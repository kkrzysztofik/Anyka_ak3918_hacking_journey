/**
 * @file soap_helpers.c
 * @brief Common SOAP XML generation utilities for ONVIF services
 */

#include "soap_helpers.h"
#include <stdio.h>
#include <string.h>

void soap_fault_response(char *response, size_t response_size, const char *fault_code, const char *fault_string) {
  snprintf(response, response_size, 
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
}

void soap_success_response_with_namespace(char *response, size_t response_size, 
                                         const char *action, const char *namespace, 
                                         const char *body_content) {
  snprintf(response, response_size,
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
    "  <soap:Body>\n"
    "    <%s:%sResponse xmlns:%s=\"http://www.onvif.org/ver10/device/wsdl\">\n"
    "      %s\n"
    "    </%s:%sResponse>\n"
    "  </soap:Body>\n"
    "</soap:Envelope>", namespace, action, namespace, body_content, namespace, action);
}

void soap_device_success_response(char *response, size_t response_size, 
                                 const char *action, const char *body_content) {
  snprintf(response, response_size,
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
    "  <soap:Body>\n"
    "    <tds:%sResponse xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\n"
    "      %s\n"
    "    </tds:%sResponse>\n"
    "  </soap:Body>\n"
    "</soap:Envelope>", action, body_content, action);
}

void soap_media_success_response(char *response, size_t response_size, 
                                const char *action, const char *body_content) {
  snprintf(response, response_size,
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
    "  <soap:Body>\n"
    "    <trt:%sResponse xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\">\n"
    "      %s\n"
    "    </trt:%sResponse>\n"
    "  </soap:Body>\n"
    "</soap:Envelope>", action, body_content, action);
}

void soap_ptz_success_response(char *response, size_t response_size, 
                              const char *action, const char *body_content) {
  snprintf(response, response_size,
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
    "  <soap:Body>\n"
    "    <tptz:%sResponse xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n"
    "      %s\n"
    "    </tptz:%sResponse>\n"
    "  </soap:Body>\n"
    "</soap:Envelope>", action, body_content, action);
}

void soap_imaging_success_response(char *response, size_t response_size, 
                                  const char *action, const char *body_content) {
  snprintf(response, response_size,
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
    "  <soap:Body>\n"
    "    <timg:%sResponse xmlns:timg=\"http://www.onvif.org/ver20/imaging/wsdl\">\n"
    "      %s\n"
    "    </timg:%sResponse>\n"
    "  </soap:Body>\n"
    "</soap:Envelope>", action, body_content, action);
}
