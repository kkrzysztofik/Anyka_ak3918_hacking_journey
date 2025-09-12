/**
 * @file xml_utils.c
 * @brief XML parsing and generation utilities implementation.
 */

#include "xml_utils.h"
#include "platform.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char* xml_extract_value(const char *xml, const char *start_tag, const char *end_tag) {
    if (!xml || !start_tag || !end_tag) {
        return NULL;
    }
    
    char *start = strstr(xml, start_tag);
    if (!start) {
        return NULL;
    }
    
    start += strlen(start_tag);
    char *end = strstr(start, end_tag);
    if (!end) {
        return NULL;
    }
    
    size_t len = end - start;
    char *value = malloc(len + 1);
    if (!value) {
        platform_log_error("Failed to allocate memory for XML value extraction\n");
        return NULL;
    }
    
    strncpy(value, start, len);
    value[len] = '\0';
    return value;
}

void xml_soap_fault_response(char *response, size_t response_size, 
                            const char *fault_code, const char *fault_string) {
    if (!response || response_size == 0) {
        return;
    }
    
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
        "</soap:Envelope>", 
        fault_code ? fault_code : "soap:Receiver", 
        fault_string ? fault_string : "Internal error");
}

void xml_soap_success_response(char *response, size_t response_size, 
                              const char *action, const char *body_content) {
    xml_soap_success_response_ns(response, response_size, action, "tds", body_content);
}

void xml_soap_success_response_ns(char *response, size_t response_size, 
                                 const char *action, const char *namespace, 
                                 const char *body_content) {
    if (!response || response_size == 0 || !action) {
        return;
    }
    
    const char *ns = namespace ? namespace : "tds";
    const char *body = body_content ? body_content : "";
    
    snprintf(response, response_size,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
        "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\">\n"
        "  <soap:Body>\n"
        "    <%s:%sResponse xmlns:%s=\"http://www.onvif.org/ver10/%s/wsdl\">\n"
        "      %s\n"
        "    </%s:%sResponse>\n"
        "  </soap:Body>\n"
        "</soap:Envelope>", 
        ns, action, ns, ns, body, ns, action);
}

int xml_is_xml_content(const char *str) {
    if (!str) {
        return 0;
    }
    
    // Check for common XML indicators
    return (strstr(str, "<?xml") != NULL) || 
           (strstr(str, "<soap:") != NULL) ||
           (strstr(str, "<tds:") != NULL) ||
           (strstr(str, "<trt:") != NULL) ||
           (strstr(str, "<tptz:") != NULL) ||
           (strstr(str, "<timg:") != NULL);
}

int xml_escape_string(const char *input, char *output, size_t output_size) {
    if (!input || !output || output_size == 0) {
        return -1;
    }
    
    size_t input_len = strlen(input);
    size_t output_pos = 0;
    
    for (size_t i = 0; i < input_len && output_pos < output_size - 1; i++) {
        switch (input[i]) {
            case '<':
                if (output_pos + 4 < output_size) {
                    strcpy(output + output_pos, "&lt;");
                    output_pos += 4;
                } else {
                    return -1; // Buffer too small
                }
                break;
            case '>':
                if (output_pos + 4 < output_size) {
                    strcpy(output + output_pos, "&gt;");
                    output_pos += 4;
                } else {
                    return -1; // Buffer too small
                }
                break;
            case '&':
                if (output_pos + 5 < output_size) {
                    strcpy(output + output_pos, "&amp;");
                    output_pos += 5;
                } else {
                    return -1; // Buffer too small
                }
                break;
            case '"':
                if (output_pos + 6 < output_size) {
                    strcpy(output + output_pos, "&quot;");
                    output_pos += 6;
                } else {
                    return -1; // Buffer too small
                }
                break;
            case '\'':
                if (output_pos + 6 < output_size) {
                    strcpy(output + output_pos, "&apos;");
                    output_pos += 6;
                } else {
                    return -1; // Buffer too small
                }
                break;
            default:
                output[output_pos++] = input[i];
                break;
        }
    }
    
    output[output_pos] = '\0';
    return 0;
}
