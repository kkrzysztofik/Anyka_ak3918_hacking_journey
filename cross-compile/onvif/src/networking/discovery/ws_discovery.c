/**
 * @file ws_discovery.c
 * @brief Minimal WS-Discovery responder for ONVIF
 * @author kkrzysztofik
 * @date 2025
 */

#define _GNU_SOURCE
#include "ws_discovery.h"

#include <arpa/inet.h>
#include <asm/socket.h>
#include <bits/pthreadtypes.h>
#include <errno.h>
#include <net/if.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "platform/platform.h"
#include "utils/network/network_utils.h"

/* WS-Discovery algorithm constants */
#define MAC_ADDRESS_SIZE          6           /* Standard MAC address size in bytes */
#define DJB2_HASH_INIT            5381        /* DJB2 hash algorithm initial value */
#define DJB2_HASH_SHIFT           5           /* DJB2 hash left shift amount */
#define MAC_LOCAL_ADMIN_FLAG      0x02        /* Locally administered MAC address flag */
#define LCG_MULTIPLIER            1103515245U /* Linear Congruential Generator multiplier (glibc) */
#define LCG_INCREMENT             12345U      /* Linear Congruential Generator increment (glibc) */
#define LCG_RAND_SHIFT            16          /* Bit shift to extract random value from LCG seed */
#define LCG_RAND_MASK             0xFFFF      /* Mask for 16-bit random value */

/* Common bit manipulation constants */
#define SHIFT_8_BITS              8           /* Bit shift for extracting second byte */
#define SHIFT_16_BITS             16          /* Bit shift for extracting third byte */
#define SHIFT_24_BITS             24          /* Bit shift for extracting fourth byte */
#define BYTE_MASK                 0xFF        /* Mask for extracting single byte */

/* UUID generation constants (RFC 4122) */
#define UUID_VERSION_MASK         0x0FFF      /* Mask for UUID version field (clear version bits) */
#define UUID_VERSION_4_FLAG       0x4000      /* UUID version 4 (random) flag */
#define UUID_VARIANT_MASK         0x3FFF      /* Mask for UUID variant field (clear variant bits) */
#define UUID_VARIANT_RFC_FLAG     0x8000      /* UUID variant RFC 4122 flag */

/* Some stripped uClibc headers may omit ip_mreq; provide minimal fallback */
#ifndef IP_ADD_MEMBERSHIP
#define IP_ADD_MEMBERSHIP 35
#endif

/* Only define ip_mreq if it's not already available */
#if !defined(HAVE_IP_MREQ) && !defined(_LINUX_IN_H) && !defined(__USE_MISC)
struct ip_mreq {
  struct in_addr imr_multiaddr; /* IP multicast address of group */
  struct in_addr imr_interface; /* local IP address of interface */
};
#endif

// Global state variables
static int g_discovery_running = 0;  // NOLINT
static int g_discovery_socket = -1;  // NOLINT
static pthread_t g_discovery_thread; // NOLINT

// Global configuration
static int g_http_port = HTTP_PORT_DEFAULT;             // NOLINT
static char g_endpoint_uuid[ONVIF_MAX_XADDR_LEN] = {0}; // NOLINT
                                                        /* urn:uuid:... */
static pthread_mutex_t g_endpoint_uuid_mutex =          // NOLINT
  PTHREAD_MUTEX_INITIALIZER;

/* Announcement interval (seconds) for periodic Hello re-broadcast */

/* Derive a pseudo-MAC from hostname (avoids platform ifreq dependency) */
static void derive_pseudo_mac(unsigned char mac[MAC_ADDRESS_SIZE]) {
  char host[ONVIF_MAX_SERVICE_NAME_LEN];
  if (get_device_hostname(host, sizeof(host)) != 0) {
    strcpy(host, "anyka");
  }
  unsigned hash = DJB2_HASH_INIT;
  for (char* ptr = host; *ptr; ++ptr) {
    hash = ((hash << DJB2_HASH_SHIFT) + hash) + (unsigned char)(*ptr);
  }
  /* Construct locally administered unicast MAC (x2 bit set, x1 bit cleared) */
  mac[0] = MAC_LOCAL_ADMIN_FLAG;
  mac[1] = (hash >> SHIFT_24_BITS) & BYTE_MASK;
  mac[2] = (hash >> SHIFT_16_BITS) & BYTE_MASK;
  mac[3] = (hash >> SHIFT_8_BITS) & BYTE_MASK;
  mac[4] = hash & BYTE_MASK;
  mac[5] = (hash >> DJB2_HASH_SHIFT) & BYTE_MASK;
}

static void build_endpoint_uuid(void) {
  unsigned char mac[MAC_ADDRESS_SIZE];
  derive_pseudo_mac(mac);
  /* Simple deterministic UUID style using MAC expanded */
  int result =
    snprintf(g_endpoint_uuid, sizeof(g_endpoint_uuid),
             "urn:uuid:%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%"
             "02x%02x%02x",
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[0], mac[1], mac[2], mac[3]);
  (void)result; // Suppress unused variable warning
}

static void gen_msg_uuid(char* out, size_t len) {
  // Use time-based seed for better randomness
  static unsigned int seed = 0;
  if (seed == 0) {
    seed = (unsigned int)time(NULL) ^ (unsigned int)getpid();
  }

  // Generate pseudo-random values using simple LCG
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand1 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand2 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand3 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand4 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand5 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;
  seed = seed * LCG_MULTIPLIER + LCG_INCREMENT;
  unsigned int rand6 = (seed >> LCG_RAND_SHIFT) & LCG_RAND_MASK;

  int result = snprintf(out, len, "%08x-%04x-%04x-%04x-%04x%08x", rand1, rand2, (rand3 & UUID_VERSION_MASK) | UUID_VERSION_4_FLAG, (rand4 & UUID_VARIANT_MASK) | UUID_VARIANT_RFC_FLAG, rand5, rand6);
  (void)result; // Suppress unused variable warning
}

static void get_ip(char* ip_buffer, size_t len) {
  if (get_local_ip_address(ip_buffer, len) != 0) {
    strncpy(ip_buffer, "192.168.1.100", len - 1);
    ip_buffer[len - 1] = '\0';
  }
}

static void send_multicast(const char* payload) {
  int sock = socket(AF_INET, SOCK_DGRAM, 0);
  if (sock < 0) {
    return;
  }

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_port = htons(ONVIF_WS_DISCOVERY_PORT);
  addr.sin_addr.s_addr = inet_addr(ONVIF_WS_DISCOVERY_MULTICAST);

  ssize_t result = sendto(sock, payload, strlen(payload), 0, (struct sockaddr*)&addr, sizeof(addr));
  (void)result; // Suppress unused variable warning

  close(sock);
}

static void send_hello(void) {
  char ip_address[ONVIF_IP_BUFFER_SIZE];
  char msg_id[64];
  get_ip(ip_address, sizeof(ip_address));
  gen_msg_uuid(msg_id, sizeof(msg_id));
  char xml[ONVIF_XML_BUFFER_SIZE];
  int result = snprintf(xml, sizeof(xml), WSD_HELLO_TEMPLATE, msg_id, g_endpoint_uuid, ip_address, g_http_port);
  (void)result; // Suppress unused variable warning
  send_multicast(xml);
}

static void send_bye(void) {
  char ip_address[ONVIF_IP_BUFFER_SIZE];
  char msg_id[64];
  get_ip(ip_address, sizeof(ip_address));
  gen_msg_uuid(msg_id, sizeof(msg_id));
  char xml[ONVIF_XML_BUFFER_SIZE];
  int result = snprintf(xml, sizeof(xml), WSD_BYE_TEMPLATE, msg_id, g_endpoint_uuid);
  (void)result; // Suppress unused variable warning
  send_multicast(xml);
}

static void* discovery_loop(void* arg) {
  (void)arg;
  struct sockaddr_in local_addr;
  memset(&local_addr, 0, sizeof(local_addr));
  local_addr.sin_family = AF_INET;
  local_addr.sin_addr.s_addr = htonl(INADDR_ANY);
  local_addr.sin_port = htons(ONVIF_WS_DISCOVERY_PORT);

  g_discovery_socket = socket(AF_INET, SOCK_DGRAM, 0);
  if (g_discovery_socket < 0) {
    perror("wsd socket");
    g_discovery_running = 0;
    return NULL;
  }
  int reuse = 1;
  int result = setsockopt(g_discovery_socket, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));
  (void)result; // Suppress unused variable warning
  if (bind(g_discovery_socket, (struct sockaddr*)&local_addr, sizeof(local_addr)) < 0) {
    perror("wsd bind");
    close(g_discovery_socket);
    g_discovery_socket = -1;
    g_discovery_running = 0;
    return NULL;
  }
  struct ip_mreq mreq;
  mreq.imr_multiaddr.s_addr = inet_addr(ONVIF_WS_DISCOVERY_MULTICAST);
  mreq.imr_interface.s_addr = htonl(INADDR_ANY);
  if (setsockopt(g_discovery_socket, IPPROTO_IP, IP_ADD_MEMBERSHIP, &mreq, sizeof(mreq)) < 0) {
    perror("wsd mcast join");
  }

  char buf[ONVIF_RESPONSE_BUFFER_SIZE];
  pthread_mutex_lock(&g_endpoint_uuid_mutex);
  if (!g_endpoint_uuid[0]) {
    build_endpoint_uuid();
  }
  pthread_mutex_unlock(&g_endpoint_uuid_mutex);
  time_t last_hello = 0;
  send_hello(); /* initial announcement */
  while (g_discovery_running) {
    struct sockaddr_in src;
    socklen_t slen = sizeof(src);
    ssize_t bytes_received = recvfrom(g_discovery_socket, buf, sizeof(buf) - 1, 0, (struct sockaddr*)&src, &slen);
    if (bytes_received <= 0) {
      if (!g_discovery_running) {
        break;
      }
      continue;
    }
    buf[bytes_received] = '\0';
    if (strstr(buf, "Probe")) {
      char msg_id[64];
      gen_msg_uuid(msg_id, sizeof(msg_id));
      char ip_address[64];
      get_ip(ip_address, sizeof(ip_address));
      char response[ONVIF_RESPONSE_BUFFER_SIZE];
      int snprintf_result = snprintf(response, sizeof(response), WSD_PROBE_MATCH_TEMPLATE, msg_id, g_endpoint_uuid, ip_address, g_http_port);
      (void)snprintf_result; // Suppress unused variable warning
      ssize_t sendto_result = sendto(g_discovery_socket, response, strlen(response), 0, (struct sockaddr*)&src, slen);
      (void)sendto_result; // Suppress unused variable warning
    }
    time_t now = time(NULL);
    if (now - last_hello >= WSD_HELLO_INTERVAL_SECONDS) {
      send_hello();
      last_hello = now;
    }
  }
  if (g_discovery_socket >= 0) {
    close(g_discovery_socket);
    g_discovery_socket = -1;
  }
  return NULL;
}

int ws_discovery_start(int http_port) {
  if (g_discovery_running) {
    return 0;
  }
  g_http_port = http_port;
  g_discovery_running = 1;
  if (pthread_create(&g_discovery_thread, NULL, discovery_loop, NULL) != 0) {
    g_discovery_running = 0;
    return -1;
  }
  return 0;
}

int ws_discovery_stop(void) {
  if (!g_discovery_running) {
    return 0;
  }

  platform_log_debug("Stopping WS-Discovery service...\n");
  g_discovery_running = 0;

  /* Closing socket will break recvfrom */
  if (g_discovery_socket >= 0) {
    close(g_discovery_socket);
    g_discovery_socket = -1;
  }

  /* Wait for thread with timeout */
  platform_log_debug("Waiting for discovery thread to finish...\n");
  struct timespec timeout;
  clock_gettime(CLOCK_REALTIME, &timeout);
  timeout.tv_sec += 2; // 2 second timeout

  int join_result = pthread_timedjoin_np(g_discovery_thread, NULL, &timeout);
  if (join_result == ETIMEDOUT) {
    platform_log_warning("Discovery thread did not finish within timeout, continuing...\n");
    // Try to cancel the thread as a last resort
    pthread_cancel(g_discovery_thread);
    // Wait a bit more for cancellation to take effect
    struct timespec cancel_timeout;
    clock_gettime(CLOCK_REALTIME, &cancel_timeout);
    cancel_timeout.tv_sec += 1;
    pthread_timedjoin_np(g_discovery_thread, NULL, &cancel_timeout);
  } else if (join_result != 0) {
    platform_log_warning("Failed to join discovery thread: %s\n", strerror(join_result));
  } else {
    platform_log_debug("Discovery thread finished successfully\n");
  }

  send_bye();
  platform_log_debug("WS-Discovery service stopped\n");
  return 0;
}
