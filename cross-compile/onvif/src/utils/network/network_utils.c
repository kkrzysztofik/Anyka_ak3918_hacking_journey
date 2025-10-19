/**
 * @file network_utils.c
 * @brief Implementation of small helpers for IP / hostname / URL composition
 * @author kkrzysztofik
 * @date 2025
 */

#include "network_utils.h"

#include <arpa/inet.h>
#include <ifaddrs.h>
#include <netdb.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "utils/error/error_handling.h"

// Add missing function declarations
extern int gethostname(char* name, size_t len);

/* ============================================================================
 * Constants - Network Utilities
 * ============================================================================ */

/* IP address and network interface constants */
#define NETWORK_IP_BUFFER_SIZE      64  /* Buffer size for IP address strings */
#define NETWORK_INTERFACE_WLAN0_LEN 5   /* Length of "wlan0" interface name */

/* ============================================================================
 * IP Address Cache - Thread-Safe with One-Time Initialization
 * ============================================================================ */

static char g_cached_ip[NETWORK_IP_BUFFER_SIZE] = {0};           // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
static int g_ip_cache_initialized = 0;      // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
static pthread_mutex_t g_ip_cache_mutex = PTHREAD_MUTEX_INITIALIZER; // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)

/**
 * @brief Internal function to fetch IP address from system (not cached)
 * @param ip_str Output buffer for dotted quad
 * @param ip_str_size Size of buffer
 * @return 0 on success, -1 on error
 */
static int fetch_local_ip_address(char* ip_str, size_t ip_str_size) {
  struct ifaddrs* ifaddrs_ptr = NULL;
  struct ifaddrs* ifa = NULL;
  void* addr_ptr = NULL;

  if (!ip_str || ip_str_size == 0) {
    return -1;
  }

  // Default fallback
  strncpy(ip_str, "192.168.1.100", ip_str_size - 1);
  ip_str[ip_str_size - 1] = '\0';

  if (getifaddrs(&ifaddrs_ptr) == -1) {
    return -1;
  }

  // First, try to find wlan0
  for (ifa = ifaddrs_ptr; ifa != NULL; ifa = ifa->ifa_next) {
    if (!ifa->ifa_addr) {
      continue;
    }
    if (ifa->ifa_addr->sa_family != AF_INET) {
      continue;
    }
    if (strncmp(ifa->ifa_name, "wlan0", NETWORK_INTERFACE_WLAN0_LEN) != 0) {
      continue;
    }
    addr_ptr = &((struct sockaddr_in*)ifa->ifa_addr)->sin_addr;
    inet_ntop(AF_INET, addr_ptr, ip_str, ip_str_size);
    break;
  }

  if (!ifa) {
    // Fallback: first non-loopback IPv4
    for (ifa = ifaddrs_ptr; ifa != NULL; ifa = ifa->ifa_next) {
      if (!ifa->ifa_addr) {
        continue;
      }
      if (ifa->ifa_addr->sa_family != AF_INET) {
        continue;
      }
      if (strcmp(ifa->ifa_name, "lo") == 0) {
        continue;
      }
      addr_ptr = &((struct sockaddr_in*)ifa->ifa_addr)->sin_addr;
      inet_ntop(AF_INET, addr_ptr, ip_str, ip_str_size);
      break;
    }
  }

  if (ifaddrs_ptr) {
    freeifaddrs(ifaddrs_ptr);
  }
  return ONVIF_SUCCESS;
}

/**
 * @brief Get primary (wlan0) IPv4 address with caching for performance
 * @param ip_str Output buffer for dotted quad
 * @param ip_str_size Size of buffer
 * @return 0 on success, -1 on error (buffer may contain fallback)
 * @note Thread-safe. First call initializes cache, subsequent calls use cached value
 */
int get_local_ip_address(char* ip_str, size_t ip_str_size) {
  if (!ip_str || ip_str_size == 0) {
    return -1;
  }

  // Fast path: cache already initialized (no lock needed for read)
  if (__atomic_load_n(&g_ip_cache_initialized, __ATOMIC_ACQUIRE)) {
    strncpy(ip_str, g_cached_ip, ip_str_size - 1);
    ip_str[ip_str_size - 1] = '\0';
    return ONVIF_SUCCESS;
  }

  // Slow path: initialize cache (thread-safe)
  pthread_mutex_lock(&g_ip_cache_mutex);

  // Double-check after acquiring lock (another thread may have initialized)
  if (!g_ip_cache_initialized) {
    fetch_local_ip_address(g_cached_ip, sizeof(g_cached_ip));
    __atomic_store_n(&g_ip_cache_initialized, 1, __ATOMIC_RELEASE);
  }

  pthread_mutex_unlock(&g_ip_cache_mutex);

  // Copy from cache
  strncpy(ip_str, g_cached_ip, ip_str_size - 1);
  ip_str[ip_str_size - 1] = '\0';
  return ONVIF_SUCCESS;
}

/**
 * @brief Retrieve system hostname (fallback provided on failure).
 */
int get_device_hostname(char* hostname, size_t hostname_size) {
  if (!hostname || hostname_size == 0) {
    return -1;
  }

  if (gethostname(hostname, hostname_size) == 0) {
    hostname[hostname_size - 1] = '\0';
    return ONVIF_SUCCESS;
  }

  // fallback hostname
  strncpy(hostname, "anyka-camera", hostname_size - 1);
  hostname[hostname_size - 1] = '\0';
  return ONVIF_SUCCESS;
}

/**
 * @brief Construct a device URL from components.
 */
int build_device_url(const char* protocol, int port, const char* path, char* url, size_t url_size) {
  char ip_str[NETWORK_IP_BUFFER_SIZE];

  if (!protocol || !path || !url || url_size == 0) {
    return -1;
  }

  if (get_local_ip_address(ip_str, sizeof(ip_str)) != 0) {
    strncpy(ip_str, "192.168.1.100", sizeof(ip_str) - 1);
    ip_str[sizeof(ip_str) - 1] = '\0';
  }

  if (port > 0) {
    snprintf(url, url_size, "%s://%s:%d%s", protocol, ip_str, port, path);
  } else {
    snprintf(url, url_size, "%s://%s%s", protocol, ip_str, path);
  }

  return ONVIF_SUCCESS;
}
