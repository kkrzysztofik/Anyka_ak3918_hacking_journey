/* ws_discovery.c - Minimal WS-Discovery responder for ONVIF */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>
#include <errno.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <net/if.h>
#include <sys/socket.h>
#include <pthread.h>
#include <unistd.h>

#include "ws_discovery.h"
#include "network_utils.h"
#include "constants.h"
#include "platform.h"

#define WS_DISCOVERY_PORT 3702
#define WS_DISCOVERY_ADDR "239.255.255.250"
#define MAX_UDP_SIZE 4096

/* Some stripped uClibc headers may omit ip_mreq; provide minimal fallback */
#ifndef IP_ADD_MEMBERSHIP
#define IP_ADD_MEMBERSHIP 35
#endif
#ifndef HAVE_IP_MREQ
#ifndef _LINUX_IN_H
struct ip_mreq {
    struct in_addr imr_multiaddr; /* IP multicast address of group */
    struct in_addr imr_interface; /* local IP address of interface */
};
#endif
#endif


static int discovery_running = 0;
static int discovery_socket = -1;
static pthread_t discovery_thread;
static int g_http_port = 8080;
static char g_endpoint_uuid[80] = {0}; /* urn:uuid:... */
static pthread_mutex_t g_endpoint_uuid_mutex = PTHREAD_MUTEX_INITIALIZER;

/* Announcement interval (seconds) for periodic Hello re-broadcast */
#define HELLO_INTERVAL 300

/* Derive a pseudo-MAC from hostname (avoids platform ifreq dependency) */
static void derive_pseudo_mac(unsigned char mac[6]) {
    char host[128];
    if (get_device_hostname(host, sizeof(host)) != 0) {
        strcpy(host, "anyka");
    }
    unsigned hash = 5381; for (char *p = host; *p; ++p) hash = ((hash << 5) + hash) + (unsigned char)(*p);
    /* Construct locally administered unicast MAC (x2 bit set, x1 bit cleared) */
    mac[0] = 0x02; /* locally administered */
    mac[1] = (hash >> 24) & 0xFF;
    mac[2] = (hash >> 16) & 0xFF;
    mac[3] = (hash >> 8) & 0xFF;
    mac[4] = hash & 0xFF;
    mac[5] = (hash >> 5) & 0xFF;
}

static void build_endpoint_uuid(void) {
        unsigned char mac[6];
        derive_pseudo_mac(mac);
        /* Simple deterministic UUID style using MAC expanded */
        snprintf(g_endpoint_uuid, sizeof(g_endpoint_uuid),
                         "urn:uuid:%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
                         mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                         mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[0], mac[1], mac[2], mac[3]);
}

static void gen_msg_uuid(char *out, size_t len){
        snprintf(out, len, "%08x-%04x-%04x-%04x-%04x%08x",
                         rand(), rand() & 0xFFFF, (rand() & 0x0FFF) | 0x4000,
                         (rand() & 0x3FFF) | 0x8000, rand() & 0xFFFF, rand());
}

static void get_ip(char *ipbuf, size_t len) {
        if (get_local_ip_address(ipbuf, len) != 0) {
                strncpy(ipbuf, "192.168.1.100", len-1); ipbuf[len-1]='\0';
        }
}

static void send_multicast(const char *payload) {
        int sock = socket(AF_INET, SOCK_DGRAM, 0);
        if (sock < 0) return;
        struct sockaddr_in addr; memset(&addr,0,sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = htons(WS_DISCOVERY_PORT);
        addr.sin_addr.s_addr = inet_addr(WS_DISCOVERY_ADDR);
        sendto(sock, payload, strlen(payload), 0, (struct sockaddr*)&addr, sizeof(addr));
        close(sock);
}

static void send_hello(void) {
        char ip[64]; char msg_id[64]; get_ip(ip,sizeof(ip)); gen_msg_uuid(msg_id,sizeof(msg_id));
        char xml[1024];
    snprintf(xml, sizeof(xml), WSD_HELLO_TEMPLATE, msg_id, g_endpoint_uuid, ip, g_http_port);
        send_multicast(xml);
}

static void send_bye(void) {
        char ip[64]; char msg_id[64]; get_ip(ip,sizeof(ip)); gen_msg_uuid(msg_id,sizeof(msg_id));
        char xml[768];
    snprintf(xml, sizeof(xml), WSD_BYE_TEMPLATE, msg_id, g_endpoint_uuid);
        send_multicast(xml);
}

static void *discovery_loop(void *arg){
    (void)arg;
    struct sockaddr_in local_addr; 
    memset(&local_addr, 0, sizeof(local_addr));
    local_addr.sin_family = AF_INET;
    local_addr.sin_addr.s_addr = htonl(INADDR_ANY);
    local_addr.sin_port = htons(WS_DISCOVERY_PORT);

    discovery_socket = socket(AF_INET, SOCK_DGRAM, 0);
    if (discovery_socket < 0) {
        perror("wsd socket");
        discovery_running = 0;
        return NULL;
    }
    int reuse = 1;
    setsockopt(discovery_socket, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));
    if (bind(discovery_socket, (struct sockaddr*)&local_addr, sizeof(local_addr)) < 0) {
        perror("wsd bind");
        close(discovery_socket); discovery_socket=-1; discovery_running=0; return NULL;
    }
    struct ip_mreq mreq; 
    mreq.imr_multiaddr.s_addr = inet_addr(WS_DISCOVERY_ADDR);
    mreq.imr_interface.s_addr = htonl(INADDR_ANY);
    if (setsockopt(discovery_socket, IPPROTO_IP, IP_ADD_MEMBERSHIP, &mreq, sizeof(mreq))<0) {
        perror("wsd mcast join");
    }

        char buf[MAX_UDP_SIZE];
        srand((unsigned)time(NULL));
        pthread_mutex_lock(&g_endpoint_uuid_mutex);
        if (!g_endpoint_uuid[0]) build_endpoint_uuid();
        pthread_mutex_unlock(&g_endpoint_uuid_mutex);
        time_t last_hello = 0;
        send_hello(); /* initial announcement */
    while (discovery_running) {
        struct sockaddr_in src; socklen_t slen = sizeof(src);
        ssize_t n = recvfrom(discovery_socket, buf, sizeof(buf)-1, 0, (struct sockaddr*)&src, &slen);
        if (n <= 0) {
            if (!discovery_running) break; 
            continue;
        }
        buf[n] = '\0';
                if (strstr(buf, "Probe")) {
                        char msg_id[64]; gen_msg_uuid(msg_id,sizeof(msg_id));
                        char ip[64]; get_ip(ip,sizeof(ip));
                        char response[1024];
                        snprintf(response, sizeof(response), WSD_PROBE_MATCH_TEMPLATE, msg_id, g_endpoint_uuid, ip, g_http_port);
                        sendto(discovery_socket, response, strlen(response), 0, (struct sockaddr*)&src, slen);
                }
                time_t now = time(NULL);
                if (now - last_hello >= HELLO_INTERVAL) { send_hello(); last_hello = now; }
    }
    if (discovery_socket>=0) { close(discovery_socket); discovery_socket=-1; }
    return NULL;
}

int ws_discovery_start(int http_port){
    if (discovery_running) return 0;
    g_http_port = http_port;
    discovery_running = 1;
    if (pthread_create(&discovery_thread, NULL, discovery_loop, NULL) != 0) {
        discovery_running = 0; return -1;
    }
    return 0;
}

int ws_discovery_stop(void){
    if (!discovery_running) return 0;
    discovery_running = 0;
    /* Closing socket will break recvfrom */
    if (discovery_socket>=0) close(discovery_socket);
    /* Timed join fallback */
    for (int i=0;i<50;i++) { /* wait up to ~5s */
        if (pthread_join(discovery_thread, NULL) == 0) break;
    platform_sleep_us(100000); /* 100ms */
    }
    send_bye();
    return 0;
}
