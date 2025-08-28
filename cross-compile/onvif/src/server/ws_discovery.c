/* ws_discovery.c - Basic WS-Discovery implementation for ONVIF */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <pthread.h>
#include <errno.h>
#include "ws_discovery.h"

#define WS_DISCOVERY_PORT 3702
#define WS_DISCOVERY_ADDR "239.255.255.250"
#define MAX_UDP_SIZE 8192

static int discovery_running = 0;
static int discovery_socket = -1;
static pthread_t discovery_thread;

static void generate_uuid(char *uuid_str, size_t size) { 
    snprintf(uuid_str, size, "%08x-%04x-%04x-%04x-%012lx", 
        rand(), rand() & 0xFFFF, rand() & 0xFFFF, 
        rand() & 0xFFFF, (unsigned long)rand() << 32 | rand()); 
}
+
+int ws_discovery_start(void) { if (discovery_running) return -1; discovery_running = 1; pthread_create(&discovery_thread, NULL, (void*(*)(void*)) (^(void){ struct sockaddr_in server_addr, client_addr; socklen_t client_len = sizeof(client_addr); char buffer[MAX_UDP_SIZE]; discovery_socket = socket(AF_INET, SOCK_DGRAM, 0); if (discovery_socket < 0) return NULL; int opt = 1; setsockopt(discovery_socket, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)); server_addr.sin_family = AF_INET; server_addr.sin_addr.s_addr = INADDR_ANY; server_addr.sin_port = htons(WS_DISCOVERY_PORT); if (bind(discovery_socket, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) { close(discovery_socket); return NULL; } struct ip_mreq mreq; mreq.imr_multiaddr.s_addr = inet_addr(WS_DISCOVERY_ADDR); mreq.imr_interface.s_addr = INADDR_ANY; setsockopt(discovery_socket, IPPROTO_IP, IP_ADD_MEMBERSHIP, &mreq, sizeof(mreq)); while (discovery_running) { ssize_t bytes_received = recvfrom(discovery_socket, buffer, sizeof(buffer)-1, 0, (struct sockaddr*)&client_addr, &client_len); if (bytes_received > 0) { buffer[bytes_received] = '\0'; if (strstr(buffer, "http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe")) { char uuid1[64], uuid2[64]; generate_uuid(uuid1,sizeof(uuid1)); generate_uuid(uuid2,sizeof(uuid2)); char response[1024]; char ip[64] = "192.168.1.100"; snprintf(response, sizeof(response), "<?xml version=\"1.0\"?><soap:Envelope><soap:Header><wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</wsa:Action></soap:Header><soap:Body><wsd:ProbeMatches><wsd:ProbeMatch><wsa:EndpointReference><wsa:Address>urn:uuid:%s</wsa:Address></wsa:EndpointReference><wsd:Types>wsdp:Device tds:Device</wsd:Types><wsd:Scopes>onvif://www.onvif.org/name/Anyka</wsd:Scopes><wsd:XAddrs>http://%s:8080/onvif/device_service</wsd:XAddrs></wsd:ProbeMatch></wsd:ProbeMatches></soap:Body></soap:Envelope>", uuid2, ip); int sock = socket(AF_INET, SOCK_DGRAM, 0); if (sock>=0) { sendto(sock, response, strlen(response), 0, (struct sockaddr*)&client_addr, client_len); close(sock);} } } close(discovery_socket); return NULL;}), NULL); return 0; }
+
+int ws_discovery_stop(void) { if (!discovery_running) return 0; discovery_running = 0; if (discovery_socket>=0) close(discovery_socket); pthread_join(discovery_thread, NULL); return 0; }
