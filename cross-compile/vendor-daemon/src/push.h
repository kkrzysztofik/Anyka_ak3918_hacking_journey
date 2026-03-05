#ifndef VENDOR_DAEMON_PUSH_H
#define VENDOR_DAEMON_PUSH_H

#include <stdint.h>

int handle_venc_start_push(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_stop_push(int fd, const uint8_t *req, uint32_t req_len);
void stop_push_slot(int idx);

#endif /* VENDOR_DAEMON_PUSH_H */
