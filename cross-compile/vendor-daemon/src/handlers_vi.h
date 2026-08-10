#ifndef VENDOR_DAEMON_HANDLERS_VI_H
#define VENDOR_DAEMON_HANDLERS_VI_H

#include <stdint.h>

int handle_vi_match_sensor(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_open(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_close(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_get_sensor_resolution(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_set_channel_attr(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_capture_on(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_capture_off(int fd, const uint8_t *req, uint32_t req_len);
int handle_vi_set_flip_mirror(int fd, const uint8_t *req, uint32_t req_len);

#endif /* VENDOR_DAEMON_HANDLERS_VI_H */
