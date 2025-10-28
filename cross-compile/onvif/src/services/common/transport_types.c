/**
 * @file transport_types.c
 * @brief Transport protocol type definitions and utilities implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "transport_types.h"

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "utils/error/error_handling.h"

/* ==================== Transport Protocol Utilities ==================== */

const char* onvif_transport_protocol_to_string(onvif_transport_protocol_t protocol) {
  switch (protocol) {
  case ONVIF_TRANSPORT_UDP:
    return "UDP";
  case ONVIF_TRANSPORT_TCP:
    return "TCP";
  case ONVIF_TRANSPORT_RTSP:
    return "RTSP";
  case ONVIF_TRANSPORT_HTTP:
    return "HTTP";
  case ONVIF_TRANSPORT_UNKNOWN:
  default:
    return "UNKNOWN";
  }
}

onvif_transport_protocol_t onvif_string_to_transport_protocol(const char* str) {
  if (!str) {
    return ONVIF_TRANSPORT_UNKNOWN;
  }

  if (strcmp(str, "UDP") == 0) {
    return ONVIF_TRANSPORT_UDP;
  }
  if (strcmp(str, "TCP") == 0) {
    return ONVIF_TRANSPORT_TCP;
  }
  if (strcmp(str, "RTSP") == 0) {
    return ONVIF_TRANSPORT_RTSP;
  }
  if (strcmp(str, "HTTP") == 0) {
    return ONVIF_TRANSPORT_HTTP;
  }
  return ONVIF_TRANSPORT_UNKNOWN;
}

bool onvif_transport_protocol_is_valid(onvif_transport_protocol_t protocol) {
  return (protocol >= ONVIF_TRANSPORT_UDP && protocol <= ONVIF_TRANSPORT_HTTP);
}

int onvif_transport_protocol_get_default_port(onvif_transport_protocol_t protocol) {
  switch (protocol) {
  case ONVIF_TRANSPORT_UDP:
  case ONVIF_TRANSPORT_TCP:
  case ONVIF_TRANSPORT_RTSP:
    return ONVIF_RTSP_PORT_DEFAULT; // 554 for RTP/RTSP protocols
  case ONVIF_TRANSPORT_HTTP:
    return HTTP_PORT_DEFAULT; // 8080
  case ONVIF_TRANSPORT_UNKNOWN:
  default:
    return -1;
  }
}

const char* onvif_transport_protocol_get_uri_prefix(onvif_transport_protocol_t protocol) {
  switch (protocol) {
  case ONVIF_TRANSPORT_UDP:
    return "udp://";
  case ONVIF_TRANSPORT_TCP:
    return "tcp://";
  case ONVIF_TRANSPORT_RTSP:
    return "rtsp://";
  case ONVIF_TRANSPORT_HTTP:
    return "http://";
  case ONVIF_TRANSPORT_UNKNOWN:
  default:
    return NULL;
  }
}

/* ==================== Network Protocol Utilities ==================== */

const char* onvif_network_protocol_to_string(onvif_network_protocol_t protocol) {
  switch (protocol) {
  case ONVIF_NETWORK_HTTP:
    return "HTTP";
  case ONVIF_NETWORK_RTSP:
    return "RTSP";
  case ONVIF_NETWORK_UNKNOWN:
  default:
    return "UNKNOWN";
  }
}

onvif_network_protocol_t onvif_string_to_network_protocol(const char* str) {
  if (!str) {
    return ONVIF_NETWORK_UNKNOWN;
  }

  if (strcmp(str, "HTTP") == 0) {
    return ONVIF_NETWORK_HTTP;
  }
  if (strcmp(str, "RTSP") == 0) {
    return ONVIF_NETWORK_RTSP;
  }
  return ONVIF_NETWORK_UNKNOWN;
}

bool onvif_network_protocol_is_valid(onvif_network_protocol_t protocol) {
  return (protocol >= ONVIF_NETWORK_HTTP && protocol <= ONVIF_NETWORK_RTSP);
}

int onvif_network_protocol_get_default_port(onvif_network_protocol_t protocol) {
  switch (protocol) {
  case ONVIF_NETWORK_HTTP:
    return ONVIF_HTTP_STANDARD_PORT; // 80
  case ONVIF_NETWORK_RTSP:
    return ONVIF_RTSP_PORT_DEFAULT; // 554
  case ONVIF_NETWORK_UNKNOWN:
  default:
    return ONVIF_ERROR_INVALID;
  }
}

/* ==================== gSOAP Integration Utilities ==================== */

int onvif_transport_to_gsoap_enum(onvif_transport_protocol_t protocol) {
  // Direct mapping since our enum values match gSOAP's tt__TransportProtocol
  return (int)protocol;
}

onvif_transport_protocol_t onvif_gsoap_to_transport_enum(int gsoap_protocol) {
  // Direct mapping since our enum values match gSOAP's tt__TransportProtocol
  if (gsoap_protocol >= 0 && gsoap_protocol <= 3) {
    return (onvif_transport_protocol_t)gsoap_protocol;
  }
  return ONVIF_TRANSPORT_UNKNOWN;
}

int onvif_network_to_gsoap_enum(onvif_network_protocol_t protocol) {
  // Map our enum values to gSOAP's tt__NetworkProtocolType
  switch (protocol) {
  case ONVIF_NETWORK_HTTP:
    return 0; // tt__NetworkProtocolType__HTTP
  case ONVIF_NETWORK_RTSP:
    return 2; // tt__NetworkProtocolType__RTSP
  case ONVIF_NETWORK_UNKNOWN:
  default:
    return -1; // Invalid
  }
}

onvif_network_protocol_t onvif_gsoap_to_network_enum(int gsoap_protocol) {
  // Map gSOAP's tt__NetworkProtocolType to our enum values
  switch (gsoap_protocol) {
  case 0: // tt__NetworkProtocolType__HTTP
    return ONVIF_NETWORK_HTTP;
  case 2: // tt__NetworkProtocolType__RTSP
    return ONVIF_NETWORK_RTSP;
  case 1: // tt__NetworkProtocolType__HTTPS - not supported
  default:
    return ONVIF_NETWORK_UNKNOWN;
  }
}
