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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>


// Add missing function declarations
extern int gethostname(char *name, size_t len);

/**
 * @brief Get primary (wlan0) IPv4 address.
 * @param ip_str Output buffer for dotted quad.
 * @param ip_str_size Size of buffer.
 * @return 0 on success, -1 on error (buffer may contain fallback).
 */
int get_local_ip_address(char *ip_str, size_t ip_str_size) {
  struct ifaddrs *ifaddrs_ptr = NULL;
  struct ifaddrs *ifa = NULL;
  void *addr_ptr = NULL;

  if (!ip_str || ip_str_size == 0) return -1;

  strncpy(ip_str, "192.168.1.100", ip_str_size - 1);
  ip_str[ip_str_size - 1] = '\0';

  if (getifaddrs(&ifaddrs_ptr) == -1) return -1;

  for (ifa = ifaddrs_ptr; ifa != NULL; ifa = ifa->ifa_next) {
    if (!ifa->ifa_addr) continue;
    if (ifa->ifa_addr->sa_family != AF_INET) continue;
    if (strncmp(ifa->ifa_name, "wlan0", 5) != 0)
      continue; /* only consider wlan0 */
    addr_ptr = &((struct sockaddr_in *)ifa->ifa_addr)->sin_addr;
    inet_ntop(AF_INET, addr_ptr, ip_str, ip_str_size);
    break; /* stop after wlan0 found */
  }
  if (!ifa) {
    /* Fallback: first non-loopback IPv4 */
    for (ifa = ifaddrs_ptr; ifa != NULL; ifa = ifa->ifa_next) {
      if (!ifa->ifa_addr) continue;
      if (ifa->ifa_addr->sa_family != AF_INET) continue;
      if (strcmp(ifa->ifa_name, "lo") == 0) continue;
      addr_ptr = &((struct sockaddr_in *)ifa->ifa_addr)->sin_addr;
      inet_ntop(AF_INET, addr_ptr, ip_str, ip_str_size);
      break;
    }
  }

  if (ifaddrs_ptr) freeifaddrs(ifaddrs_ptr);
  return 0;
}

/**
 * @brief Retrieve system hostname (fallback provided on failure).
 */
int get_device_hostname(char *hostname, size_t hostname_size) {
  if (!hostname || hostname_size == 0) return -1;

  if (gethostname(hostname, hostname_size) == 0) {
    hostname[hostname_size - 1] = '\0';
    return 0;
  }

  // fallback hostname
  strncpy(hostname, "anyka-camera", hostname_size - 1);
  hostname[hostname_size - 1] = '\0';
  return 0;
}

/**
 * @brief Construct a device URL from components.
 */
int build_device_url(const char *protocol, int port, const char *path,
                     char *url, size_t url_size) {
  char ip_str[64];

  if (!protocol || !path || !url || url_size == 0) return -1;

  if (get_local_ip_address(ip_str, sizeof(ip_str)) != 0) {
    strncpy(ip_str, "192.168.1.100", sizeof(ip_str) - 1);
    ip_str[sizeof(ip_str) - 1] = '\0';
  }

  if (port > 0) {
    snprintf(url, url_size, "%s://%s:%d%s", protocol, ip_str, port, path);
  } else {
    snprintf(url, url_size, "%s://%s%s", protocol, ip_str, path);
  }

  return 0;
}
