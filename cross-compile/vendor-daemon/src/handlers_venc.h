#ifndef VENDOR_DAEMON_HANDLERS_VENC_H
#define VENDOR_DAEMON_HANDLERS_VENC_H

#include <stdint.h>

int handle_venc_set_cfg_path(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_open(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_close(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_set_rc(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_set_iframe(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_request_stream(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_cancel_stream(int fd, const uint8_t *req, uint32_t req_len);

#endif /* VENDOR_DAEMON_HANDLERS_VENC_H */
