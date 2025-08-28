/**
 * @file http_server.c
 * @brief Minimal HTTP server handling ONVIF SOAP POST requests.
 *
 * Implementation notes:
 *  - Single listening socket, one thread reading/servicing sequential clients.
 *  - For simplicity each accept is processed immediately; no persistent keep-alive.
 *  - Request parsing is rudimentary: reads into fixed buffer, searches for action tokens.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <pthread.h>
#include <errno.h>
#include "http_server.h"
#include "../utils/constants.h"
#include "../services/device/onvif_device.h"
#include "../services/media/onvif_media.h"
#include "../services/ptz/onvif_ptz.h"
#include "../services/imaging/onvif_imaging.h"

#define MAX_REQUEST_SIZE 8192
#define MAX_RESPONSE_SIZE 16384

static int server_running = 0;
static int server_socket = -1;
static pthread_t server_thread;

/**
 * @brief Background thread accepting and handling HTTP clients.
 */
static void *server_thread_func(void *arg) {
    (void)arg; // Suppress unused parameter warning
    
    while (server_running) { 
        int client = accept(server_socket, NULL, NULL); 
        if (client < 0) continue; 
        
        char request[MAX_REQUEST_SIZE]; 
        ssize_t n = recv(client, request, sizeof(request)-1, 0); 
        if (n <= 0) { 
            close(client); 
            continue; 
        } 
        
        request[n] = '\0'; 
        char method[16], path[256], version[16]; 
        if (sscanf(request, "%15s %255s %15s", method, path, version) != 3) { 
            close(client); 
            continue; 
        } 
        
        if (strcmp(method, "POST") == 0) {
            char response[MAX_RESPONSE_SIZE];
            int handled = 0;

            /* Device service: GetDeviceInformation */
            if (!handled && strstr(path, "/device_service") && strstr(request, "GetDeviceInformation")) {
                struct device_info info;
                onvif_device_get_device_information(&info);
                snprintf(response, sizeof(response),
                    ONVIF_SOAP_DEVICE_GET_DEVICE_INFORMATION_RESPONSE,
                    info.manufacturer, info.model, info.firmware_version,
                    info.serial_number, info.hardware_id);
                handled = 1;
            }

            /* Media service: GetProfiles */
            if (!handled && strstr(path, "/media_service") && strstr(request, "GetProfiles")) {
                struct media_profile *profiles = NULL; int count = 0;
                if (onvif_media_get_profiles(&profiles, &count) == 0) {
                    /* Build dynamic profiles list */
                    char profiles_xml[8192]; profiles_xml[0] = '\0';
                    for (int i = 0; i < count; i++) {
                        char entry[1024];
                        snprintf(entry, sizeof(entry), ONVIF_SOAP_MEDIA_GET_PROFILES_PROFILE_ENTRY,
                            profiles[i].token, profiles[i].name,
                            1, /* use count placeholder */
                            profiles[i].video_source.source_token,
                            profiles[i].video_source.bounds.width,
                            profiles[i].video_source.bounds.height,
                            profiles[i].video_encoder.token,
                            profiles[i].video_encoder.resolution.width,
                            profiles[i].video_encoder.resolution.height,
                            (int)profiles[i].video_encoder.quality,
                            profiles[i].video_encoder.framerate_limit,
                            profiles[i].video_encoder.encoding_interval,
                            profiles[i].video_encoder.bitrate_limit,
                            profiles[i].video_encoder.gov_length,
                            profiles[i].audio_source.source_token,
                            profiles[i].audio_encoder.bitrate,
                            profiles[i].audio_encoder.sample_rate);
                        strncat(profiles_xml, entry, sizeof(profiles_xml) - strlen(profiles_xml) - 1);
                    }
                    snprintf(response, sizeof(response), "%s%s%s", ONVIF_SOAP_MEDIA_GET_PROFILES_HEADER, profiles_xml, ONVIF_SOAP_MEDIA_GET_PROFILES_FOOTER);
                    handled = 1;
                }
            }

            /* Media service: GetStreamUri */
            if (!handled && strstr(path, "/media_service") && strstr(request, "GetStreamUri")) {
                /* Extract profile token crudely */
                const char *tok = strstr(request, "ProfileToken>");
                char profile_token[64] = "MainProfile"; /* default */
                if (tok) {
                    tok += strlen("ProfileToken>");
                    const char *end = strchr(tok, '<');
                    if (end && (end - tok) < (int)sizeof(profile_token)) {
                        strncpy(profile_token, tok, end - tok); profile_token[end - tok] = '\0';
                    }
                }
                struct stream_uri uri;
                if (onvif_media_get_stream_uri(profile_token, "RTSP", &uri) == 0) {
                    snprintf(response, sizeof(response), ONVIF_SOAP_MEDIA_GET_STREAM_URI_RESPONSE, uri.uri, uri.timeout);
                    handled = 1;
                }
            }

            /* Media service: GetSnapshotUri */
            if (!handled && strstr(path, "/media_service") && strstr(request, "GetSnapshotUri")) {
                const char *tok = strstr(request, "ProfileToken>");
                char profile_token[64] = "MainProfile";
                if (tok) {
                    tok += strlen("ProfileToken>");
                    const char *end = strchr(tok, '<');
                    if (end && (end - tok) < (int)sizeof(profile_token)) {
                        strncpy(profile_token, tok, end - tok); profile_token[end - tok] = '\0';
                    }
                }
                struct stream_uri uri;
                if (onvif_media_get_snapshot_uri(profile_token, &uri) == 0) {
                    snprintf(response, sizeof(response), ONVIF_SOAP_MEDIA_GET_SNAPSHOT_URI_RESPONSE, uri.uri, uri.timeout);
                    handled = 1;
                }
            }

            /* PTZ service path assumed /ptz_service */
            if (!handled && strstr(path, "/ptz_service")) {
                /* Extract profile token once if present */
                const char *ptok = strstr(request, "ProfileToken>");
                char profile_token[64] = "MainProfile";
                if (ptok) {
                    ptok += strlen("ProfileToken>");
                    const char *end = strchr(ptok, '<');
                    if (end && (end - ptok) < (int)sizeof(profile_token)) { strncpy(profile_token, ptok, end - ptok); profile_token[end - ptok] = '\0'; }
                }

                /* GetStatus */
                if (!handled && strstr(request, "GetStatus")) {
                    struct ptz_status status; if (onvif_ptz_get_status(profile_token, &status) == 0) {
                        const char *move_state = (status.move_status.pan_tilt == PTZ_MOVE_MOVING) ? "MOVING" : "IDLE";
                        snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_GET_STATUS_RESPONSE, status.position.pan_tilt.x, status.position.pan_tilt.y, status.position.zoom, move_state, status.utc_time);
                        handled = 1; }
                }
                /* AbsoluteMove */
                if (!handled && strstr(request, "AbsoluteMove")) {
                    /* crude parse PanTilt x y */
                    struct ptz_vector pos = {0};
                    const char *pt = strstr(request, "PanTilt x=\"");
                    if (pt) { pt += 12; pos.pan_tilt.x = strtof(pt, NULL); const char *y = strstr(pt, "y=\""); if (y) { y += 4; pos.pan_tilt.y = strtof(y, NULL);} }
                    onvif_ptz_absolute_move(profile_token, &pos, NULL);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_ABSOLUTE_MOVE_OK);
                    handled = 1;
                }
                /* RelativeMove */
                if (!handled && strstr(request, "RelativeMove")) {
                    struct ptz_vector trans = {0};
                    const char *pt = strstr(request, "PanTilt x=\"");
                    if (pt) { pt += 12; trans.pan_tilt.x = strtof(pt, NULL); const char *y = strstr(pt, "y=\""); if (y) { y += 4; trans.pan_tilt.y = strtof(y, NULL);} }
                    onvif_ptz_relative_move(profile_token, &trans, NULL);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_RELATIVE_MOVE_OK);
                    handled = 1;
                }
                /* ContinuousMove */
                if (!handled && strstr(request, "ContinuousMove")) {
                    struct ptz_speed vel = {0};
                    const char *pt = strstr(request, "PanTilt x=\"");
                    if (pt) { pt += 12; vel.pan_tilt.x = strtof(pt, NULL); const char *y = strstr(pt, "y=\""); if (y) { y += 4; vel.pan_tilt.y = strtof(y, NULL);} }
                    onvif_ptz_continuous_move(profile_token, &vel, 10000);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_CONTINUOUS_MOVE_OK);
                    handled = 1;
                }
                /* Stop */
                if (!handled && strstr(request, "Stop")) {
                    onvif_ptz_stop(profile_token, 1, 0);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_STOP_OK);
                    handled = 1;
                }
                /* GotoHomePosition */
                if (!handled && strstr(request, "GotoHomePosition")) {
                    onvif_ptz_goto_home_position(profile_token, NULL);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_GOTO_HOME_OK);
                    handled = 1;
                }
                /* SetHomePosition */
                if (!handled && strstr(request, "SetHomePosition")) {
                    onvif_ptz_set_home_position(profile_token);
                    snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_SET_HOME_OK);
                    handled = 1;
                }
                /* GetPresets */
                if (!handled && strstr(request, "GetPresets")) {
                    struct ptz_preset *plist; int pcount; if (onvif_ptz_get_presets(profile_token, &plist, &pcount) == 0) {
                        char buf[4096]; buf[0] = '\0';
                        for (int i=0;i<pcount;i++){ char e[256]; snprintf(e, sizeof(e), ONVIF_SOAP_PTZ_GET_PRESETS_ENTRY, plist[i].name, plist[i].token); strncat(buf, e, sizeof(buf)-strlen(buf)-1);} 
                        snprintf(response, sizeof(response), "%s%s%s", ONVIF_SOAP_PTZ_GET_PRESETS_HEADER, buf, ONVIF_SOAP_PTZ_GET_PRESETS_FOOTER);
                        handled = 1; }
                }
                /* SetPreset */
                if (!handled && strstr(request, "SetPreset")) {
                    /* parse Name>value< */
                    char name[64] = "Preset"; const char *nm = strstr(request, "Name>"); if (nm){ nm+=5; const char *end=strchr(nm,'<'); if(end && (end-nm)<(int)sizeof(name)){ strncpy(name,nm,end-nm); name[end-nm]='\0'; }}
                    char token[64]; if (onvif_ptz_set_preset(profile_token, name, token, sizeof(token))==0){ snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_SET_PRESET_RESPONSE, token); handled=1; }
                }
                /* GotoPreset */
                if (!handled && strstr(request, "GotoPreset")) {
                    char token[64] = ""; const char *tk = strstr(request, "PresetToken>"); if (tk){ tk+=12; const char *end=strchr(tk,'<'); if(end && (end-tk)<(int)sizeof(token)){ strncpy(token,tk,end-tk); token[end-tk]='\0'; }}
                    if (token[0] && onvif_ptz_goto_preset(profile_token, token, NULL) == 0) { snprintf(response, sizeof(response), ONVIF_SOAP_PTZ_GOTO_PRESET_OK); handled = 1; }
                }
            }

            if (handled) {
                int len = (int)strlen(response);
                char header[256];
                snprintf(header, sizeof(header),
                    "HTTP/1.1 200 OK\r\n"
                    "Content-Type: application/soap+xml; charset=utf-8\r\n"
                    "Content-Length: %d\r\n\r\n", len);
                send(client, header, strlen(header), 0);
                send(client, response, len, 0);
            } else {
                char notfound[] = "HTTP/1.1 404 Not Found\r\nContent-Length: 13\r\n\r\n404 Not Found";
                send(client, notfound, sizeof(notfound) - 1, 0);
            }
        }
        close(client); 
    } 
    return NULL;
}

/**
 * @brief Start HTTP server on given port.
 * @param port TCP port number.
 * @return 0 on success else -1.
 */
int http_server_start(int port) {
    if (server_running) return -1;
    
    struct sockaddr_in server_addr; 
    server_socket = socket(AF_INET, SOCK_STREAM, 0); 
    if (server_socket < 0) return -1; 
    
    int opt = 1; 
    setsockopt(server_socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)); 
    
    server_addr.sin_family = AF_INET; 
    server_addr.sin_addr.s_addr = INADDR_ANY; 
    server_addr.sin_port = htons(port); 
    
    if (bind(server_socket, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) { 
        close(server_socket); 
        return -1; 
    } 
    
    if (listen(server_socket, 5) < 0) { 
        close(server_socket); 
        return -1; 
    } 
    
    server_running = 1; 
    pthread_create(&server_thread, NULL, server_thread_func, NULL); 
    
    printf("HTTP server started on port %d\n", port);
    return 0;
}

/**
 * @brief Stop HTTP server and join worker thread.
 * @return 0 always (idempotent).
 */
int http_server_stop(void) {
    if (!server_running) return 0; 
    server_running = 0; 
    if (server_socket >= 0) close(server_socket); 
    pthread_join(server_thread, NULL); 
    return 0; 
}
