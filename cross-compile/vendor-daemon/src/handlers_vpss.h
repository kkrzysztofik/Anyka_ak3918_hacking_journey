#ifndef VENDOR_DAEMON_HANDLERS_VPSS_H
#define VENDOR_DAEMON_HANDLERS_VPSS_H

#include <stdint.h>

int handle_vpss_init(int fd, const uint8_t *req, uint32_t req_len);
int handle_vpss_destroy(int fd, const uint8_t *req, uint32_t req_len);

#endif /* VENDOR_DAEMON_HANDLERS_VPSS_H */
