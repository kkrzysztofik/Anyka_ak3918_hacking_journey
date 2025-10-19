/**
 * @file transport_types.h
 * @brief Transport protocol type definitions and utilities
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TRANSPORT_TYPES_H
#define TRANSPORT_TYPES_H

#include <stdbool.h>
#include <stddef.h>

/**
 * @brief ONVIF transport protocol enumeration
 *
 * Maps to gSOAP's tt__TransportProtocol enum values:
 * - ONVIF_TRANSPORT_UDP = tt__TransportProtocol__UDP (0)
 * - ONVIF_TRANSPORT_TCP = tt__TransportProtocol__TCP (1)
 * - ONVIF_TRANSPORT_RTSP = tt__TransportProtocol__RTSP (2)
 * - ONVIF_TRANSPORT_HTTP = tt__TransportProtocol__HTTP (3)
 */
typedef enum {
  ONVIF_TRANSPORT_UDP = 0,     /**< UDP transport protocol */
  ONVIF_TRANSPORT_TCP = 1,     /**< TCP transport protocol */
  ONVIF_TRANSPORT_RTSP = 2,    /**< RTSP transport protocol */
  ONVIF_TRANSPORT_HTTP = 3,    /**< HTTP transport protocol */
  ONVIF_TRANSPORT_UNKNOWN = -1 /**< Unknown/invalid transport protocol */
} onvif_transport_protocol_t;

/**
 * @brief Network protocol type enumeration
 *
 * Maps to gSOAP's tt__NetworkProtocolType enum values:
 * - ONVIF_NETWORK_HTTP = tt__NetworkProtocolType__HTTP (0)
 * - ONVIF_NETWORK_RTSP = tt__NetworkProtocolType__RTSP (2)
 */
typedef enum {
  ONVIF_NETWORK_HTTP = 0,    /**< HTTP network protocol */
  ONVIF_NETWORK_RTSP = 2,    /**< RTSP network protocol */
  ONVIF_NETWORK_UNKNOWN = -1 /**< Unknown/invalid network protocol */
} onvif_network_protocol_t;

/**
 * @brief Transport protocol configuration structure
 */
typedef struct {
  onvif_transport_protocol_t protocol; /**< Transport protocol type */
  int port;                            /**< Port number */
  bool enabled;                        /**< Whether protocol is enabled */
#define ONVIF_URI_PREFIX_MAX_LEN 16
  char uri_prefix[ONVIF_URI_PREFIX_MAX_LEN]; /**< URI prefix (e.g., "rtsp://", "http://") */
} onvif_transport_config_t;

/**
 * @brief Network protocol configuration structure
 */
typedef struct {
  onvif_network_protocol_t protocol; /**< Network protocol type */
  int port;                          /**< Port number */
  bool enabled;                      /**< Whether protocol is enabled */
  bool tls_enabled;                  /**< Whether TLS/SSL is enabled */
} onvif_network_config_t;

/* ==================== Transport Protocol Utilities ==================== */

/**
 * @brief Convert transport protocol enum to string
 * @param protocol Transport protocol enum value
 * @return String representation of the protocol, or "UNKNOWN" if invalid
 */
const char* onvif_transport_protocol_to_string(onvif_transport_protocol_t protocol);

/**
 * @brief Convert string to transport protocol enum
 * @param str String representation of the protocol
 * @return Transport protocol enum value, or ONVIF_TRANSPORT_UNKNOWN if invalid
 */
onvif_transport_protocol_t onvif_string_to_transport_protocol(const char* str);

/**
 * @brief Check if transport protocol is valid
 * @param protocol Transport protocol enum value
 * @return true if valid, false otherwise
 */
bool onvif_transport_protocol_is_valid(onvif_transport_protocol_t protocol);

/**
 * @brief Get default port for transport protocol
 * @param protocol Transport protocol enum value
 * @return Default port number, or -1 if unknown
 */
int onvif_transport_protocol_get_default_port(onvif_transport_protocol_t protocol);

/**
 * @brief Get URI prefix for transport protocol
 * @param protocol Transport protocol enum value
 * @return URI prefix string, or NULL if unknown
 */
const char* onvif_transport_protocol_get_uri_prefix(onvif_transport_protocol_t protocol);

/* ==================== Network Protocol Utilities ==================== */

/**
 * @brief Convert network protocol enum to string
 * @param protocol Network protocol enum value
 * @return String representation of the protocol, or "UNKNOWN" if invalid
 */
const char* onvif_network_protocol_to_string(onvif_network_protocol_t protocol);

/**
 * @brief Convert string to network protocol enum
 * @param str String representation of the protocol
 * @return Network protocol enum value, or ONVIF_NETWORK_UNKNOWN if invalid
 */
onvif_network_protocol_t onvif_string_to_network_protocol(const char* str);

/**
 * @brief Check if network protocol is valid
 * @param protocol Network protocol enum value
 * @return true if valid, false otherwise
 */
bool onvif_network_protocol_is_valid(onvif_network_protocol_t protocol);

/**
 * @brief Get default port for network protocol
 * @param protocol Network protocol enum value
 * @return Default port number, or -1 if unknown
 */
int onvif_network_protocol_get_default_port(onvif_network_protocol_t protocol);

/* ==================== gSOAP Integration Utilities ==================== */

/**
 * @brief Convert ONVIF transport protocol to gSOAP tt__TransportProtocol
 * @param protocol ONVIF transport protocol enum value
 * @return gSOAP tt__TransportProtocol enum value
 */
int onvif_transport_to_gsoap_enum(onvif_transport_protocol_t protocol);

/**
 * @brief Convert gSOAP tt__TransportProtocol to ONVIF transport protocol
 * @param gsoap_protocol gSOAP tt__TransportProtocol enum value
 * @return ONVIF transport protocol enum value
 */
onvif_transport_protocol_t onvif_gsoap_to_transport_enum(int gsoap_protocol);

/**
 * @brief Convert ONVIF network protocol to gSOAP tt__NetworkProtocolType
 * @param protocol ONVIF network protocol enum value
 * @return gSOAP tt__NetworkProtocolType enum value
 */
int onvif_network_to_gsoap_enum(onvif_network_protocol_t protocol);

/**
 * @brief Convert gSOAP tt__NetworkProtocolType to ONVIF network protocol
 * @param gsoap_protocol gSOAP tt__NetworkProtocolType enum value
 * @return ONVIF network protocol enum value
 */
onvif_network_protocol_t onvif_gsoap_to_network_enum(int gsoap_protocol);

#endif /* TRANSPORT_TYPES_H */
