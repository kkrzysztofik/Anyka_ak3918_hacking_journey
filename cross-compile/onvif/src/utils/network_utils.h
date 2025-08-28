#ifndef NETWORK_UTILS_H
#define NETWORK_UTILS_H
#include <stddef.h>

int get_local_ip_address(char *ip_str, size_t ip_str_size);
int get_device_hostname(char *hostname, size_t hostname_size);
int build_device_url(const char *protocol, int port, const char *path, char *url, size_t url_size);

#endif
