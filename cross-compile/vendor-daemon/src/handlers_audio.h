#ifndef VENDOR_DAEMON_HANDLERS_AUDIO_H
#define VENDOR_DAEMON_HANDLERS_AUDIO_H

#include <stdint.h>

int handle_ai_open(int fd, const uint8_t *req, uint32_t req_len);
int handle_ai_close(int fd, const uint8_t *req, uint32_t req_len);
int handle_ai_set_adc_volume(int fd, const uint8_t *req, uint32_t req_len);
int handle_ai_set_aslc_volume(int fd, const uint8_t *req, uint32_t req_len);
int handle_aenc_open(int fd, const uint8_t *req, uint32_t req_len);
int handle_aenc_close(int fd, const uint8_t *req, uint32_t req_len);
int handle_aenc_set_attr(int fd, const uint8_t *req, uint32_t req_len);
int handle_audio_play(int fd, const uint8_t *req, uint32_t req_len);

#endif /* VENDOR_DAEMON_HANDLERS_AUDIO_H */
