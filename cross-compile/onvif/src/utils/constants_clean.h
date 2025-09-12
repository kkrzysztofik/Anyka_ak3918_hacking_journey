/**
 * @file constants_clean.h
 * @brief Essential constants without circular dependencies.
 */

#ifndef ONVIF_CONSTANTS_CLEAN_H
#define ONVIF_CONSTANTS_CLEAN_H

/* Network Ports */
#define ONVIF_HTTP_PORT_DEFAULT         8080
#define ONVIF_HTTPS_PORT_DEFAULT        443
#define ONVIF_RTSP_PORT_DEFAULT         554
#define ONVIF_SNAPSHOT_PORT_DEFAULT     3000
#define ONVIF_HTTP_STANDARD_PORT        80
#define ONVIF_WS_DISCOVERY_PORT         3702

/* Buffer Sizes */
#define ONVIF_RESPONSE_BUFFER_SIZE      4096
#define ONVIF_XML_BUFFER_SIZE           1024
#define ONVIF_CONFIG_BUFFER_SIZE        4096

/* Network Addresses */
#define ONVIF_IP_PLACEHOLDER            "[IP]"
#define ONVIF_WS_DISCOVERY_MULTICAST    "239.255.255.250"

/* String Lengths */
#define ONVIF_MAX_SERVICE_NAME_LEN      64
#define ONVIF_MAX_XADDR_LEN             256
#define ONVIF_MAX_TOKEN_LEN             64
#define ONVIF_MAX_PROFILE_NAME_LEN      32

/* File Paths */
#define ONVIF_CONFIG_FILE_DEFAULT       "/etc/jffs2/anyka_cfg.ini"
#define ONVIF_CONFIG_FILE               ONVIF_CONFIG_FILE_DEFAULT
#define ONVIF_DMALLOC_LOG_FILE          "dmalloc.log"

/* IR LED Constants */
#define IRLED_LEVEL_DEFAULT             80

/* Service Endpoints */
#define ONVIF_DEVICE_SERVICE_PATH      "/onvif/device_service"
#define ONVIF_MEDIA_SERVICE_PATH       "/onvif/media_service"
#define ONVIF_PTZ_SERVICE_PATH         "/onvif/ptz_service"
#define ONVIF_IMAGING_SERVICE_PATH     "/onvif/imaging_service"

/* Stream Paths */
#define RTSP_MAIN_STREAM_PATH          "/vs0"
#define RTSP_SUB_STREAM_PATH           "/vs1"
#define SNAPSHOT_PATH                  "/snapshot.bmp"

/* HTTP Status Codes */
#define HTTP_STATUS_OK                  200
#define HTTP_STATUS_BAD_REQUEST         400
#define HTTP_STATUS_UNAUTHORIZED        401
#define HTTP_STATUS_NOT_FOUND           404
#define HTTP_STATUS_INTERNAL_ERROR      500

/* SOAP Fault Codes */
#define SOAP_FAULT_RECEIVER             "soap:Receiver"
#define SOAP_FAULT_SENDER               "soap:Sender"

/* ONVIF Service Namespaces */
#define ONVIF_DEVICE_NS                "http://www.onvif.org/ver10/device/wsdl"
#define ONVIF_MEDIA_NS                 "http://www.onvif.org/ver10/media/wsdl"
#define ONVIF_PTZ_NS                   "http://www.onvif.org/ver20/ptz/wsdl"
#define ONVIF_IMAGING_NS               "http://www.onvif.org/ver20/imaging/wsdl"

/* SOAP Response Templates */
#define ONVIF_SOAP_IMAGING_SET_SETTINGS_OK     "<SetImagingSettings>OK</SetImagingSettings>"
#define ONVIF_SOAP_IMAGING_SET_SETTINGS_FAIL   "<SetImagingSettings>FAIL</SetImagingSettings>"
#define ONVIF_SOAP_IMAGING_GET_OPTIONS_RESPONSE "<GetOptions>Options</GetOptions>"
#define ONVIF_SOAP_IMAGING_GET_SETTINGS_RESPONSE "<GetImagingSettings>Settings</GetImagingSettings>"

/* WS-Discovery Constants */
#define WSD_HELLO_INTERVAL_SECONDS      300
#define WSD_HELLO_TEMPLATE              "<Hello>%s-%s-%s-%d</Hello>"
#define WSD_BYE_TEMPLATE                "<Bye>%s-%s</Bye>"  
#define WSD_PROBE_MATCH_TEMPLATE        "<ProbeMatch>%s-%s-%s-%d</ProbeMatch>"

#endif /* ONVIF_CONSTANTS_CLEAN_H */
