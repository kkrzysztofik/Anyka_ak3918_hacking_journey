/* http_server.c - Simple HTTP server for ONVIF SOAP requests */

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
#include "../services/device/onvif_device.h"
#include "../services/media/onvif_media.h"
#include "../services/ptz/onvif_ptz.h"
#include "../services/imaging/onvif_imaging.h"

#define MAX_REQUEST_SIZE 8192
#define MAX_RESPONSE_SIZE 16384

static int server_running = 0;
static int server_socket = -1;
static pthread_t server_thread;

static const char *soap_envelope_start = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\" xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\" xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\n<soap:Body>\n";
static const char *soap_envelope_end = "</soap:Body>\n</soap:Envelope>\n";

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
            if (strstr(path, "/device_service") && strstr(request, "GetDeviceInformation")) { 
                struct device_info info; 
                onvif_device_get_device_information(&info); 
                snprintf(response, sizeof(response), 
                    "%s<tds:GetDeviceInformationResponse>"
                    "  <tds:Manufacturer>%s</tds:Manufacturer>"
                    "  <tds:Model>%s</tds:Model>"
                    "  <tds:FirmwareVersion>%s</tds:FirmwareVersion>"
                    "  <tds:SerialNumber>%s</tds:SerialNumber>"
                    "  <tds:HardwareId>%s</tds:HardwareId>"
                    "</tds:GetDeviceInformationResponse>%s", 
                    soap_envelope_start, info.manufacturer, info.model, 
                    info.firmware_version, info.serial_number, 
                    info.hardware_id, soap_envelope_end); 
                
                int len = strlen(response); 
                char header[256]; 
                snprintf(header, sizeof(header), 
                    "HTTP/1.1 200 OK\r\n"
                    "Content-Type: application/soap+xml; charset=utf-8\r\n"
                    "Content-Length: %d\r\n\r\n", len); 
                send(client, header, strlen(header), 0); 
                send(client, response, len, 0); 
            } else { 
                char notfound[] = "HTTP/1.1 404 Not Found\r\nContent-Length: 13\r\n\r\n404 Not Found"; 
                send(client, notfound, sizeof(notfound)-1, 0); 
            } 
        } 
        close(client); 
    } 
    return NULL;
}

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

int http_server_stop(void) { 
    if (!server_running) return 0; 
    server_running = 0; 
    if (server_socket >= 0) close(server_socket); 
    pthread_join(server_thread, NULL); 
    return 0; 
}
