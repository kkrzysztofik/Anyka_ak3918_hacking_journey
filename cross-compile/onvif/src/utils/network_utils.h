/**
 * @file network_utils.h
 * @brief Minimal helpers for discovering local IP/hostname and building URLs.
 */
#ifndef NETWORK_UTILS_H
#define NETWORK_UTILS_H
#include <stddef.h>

/** Obtain primary (wlan0) IPv4 address into buffer. */
int get_local_ip_address(char *ip_str, size_t ip_str_size);
/** Retrieve device hostname (fallback value if system call fails). */
int get_device_hostname(char *hostname, size_t hostname_size);
/** Compose device URL: protocol://ip[:port]/path */
int build_device_url(const char *protocol, int port, const char *path, char *url, size_t url_size);

#endif
