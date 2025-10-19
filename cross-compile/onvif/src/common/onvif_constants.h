/**
 * @file onvif_constants.h
 * @brief ONVIF constants and configuration values
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_CONSTANTS_H
#define ONVIF_CONSTANTS_H

/* Network Ports */
#define ONVIF_RTSP_PORT_DEFAULT     554
#define ONVIF_SNAPSHOT_PORT_DEFAULT 3000
#define ONVIF_HTTP_STANDARD_PORT    80
#define ONVIF_WS_DISCOVERY_PORT     3702

/* Buffer Sizes */
#define ONVIF_RESPONSE_BUFFER_SIZE 131072 /* 128KB buffer for large SOAP responses */
#define ONVIF_XML_BUFFER_SIZE                                                                      \
  8192 /* Increased from 1024 to prevent buffer overflow in capabilities                           \
          response */
#define ONVIF_CONFIG_BUFFER_SIZE 4096

/* Network Addresses */
#define ONVIF_IP_PLACEHOLDER         "[IP]"
#define ONVIF_WS_DISCOVERY_MULTICAST "239.255.255.250"

/* String Lengths */
#define ONVIF_MAX_SERVICE_NAME_LEN 64
#define ONVIF_MAX_XADDR_LEN        256
#define ONVIF_MAX_TOKEN_LEN        64
#define ONVIF_MAX_PROFILE_NAME_LEN 32

/* File Paths */
#define ONVIF_CONFIG_FILE_DEFAULT "/etc/jffs2/anyka_cfg.ini"
#define ONVIF_CONFIG_FILE         ONVIF_CONFIG_FILE_DEFAULT
#define ONVIF_DMALLOC_LOG_FILE    "dmalloc.log"

/* IR LED Constants */
#define IRLED_LEVEL_DEFAULT 80

/* Service Endpoints */
#define ONVIF_DEVICE_SERVICE_PATH  "/onvif/device_service"
#define ONVIF_MEDIA_SERVICE_PATH   "/onvif/media_service"
#define ONVIF_PTZ_SERVICE_PATH     "/onvif/ptz_service"
#define ONVIF_IMAGING_SERVICE_PATH "/onvif/imaging_service"

/* Stream Paths */
#define RTSP_MAIN_STREAM_PATH "/vs0"
#define RTSP_SUB_STREAM_PATH  "/vs1"
#define SNAPSHOT_PATH         "/snapshot.jpeg"

/* Time and Date Constants */
#define TM_YEAR_OFFSET 1900

/* ONVIF Version Constants */
#define ONVIF_VERSION_MAJOR 24 /* ONVIF specification version (December 2024) */
#define ONVIF_VERSION_MINOR 12 /* ONVIF specification version (December 2024) */

/* HTTP Status Codes */
#define HTTP_STATUS_OK             200
#define HTTP_STATUS_BAD_REQUEST    400
#define HTTP_STATUS_UNAUTHORIZED   401
#define HTTP_STATUS_NOT_FOUND      404
#define HTTP_STATUS_INTERNAL_ERROR 500

/* SOAP Fault Codes */
#define SOAP_FAULT_RECEIVER "soap:Receiver"
#define SOAP_FAULT_SENDER   "soap:Sender"

/* WS-Discovery Constants */
#define WSD_HELLO_INTERVAL_SECONDS 300
#define WSD_HELLO_TEMPLATE         "<Hello>%s-%s-%s-%d</Hello>"
#define WSD_BYE_TEMPLATE           "<Bye>%s-%s</Bye>"
#define WSD_PROBE_MATCH_TEMPLATE   "<ProbeMatch>%s-%s-%s-%d</ProbeMatch>"

/* Error return codes are defined in utils/error/error_handling.h */

#endif /* ONVIF_CONSTANTS_H */
