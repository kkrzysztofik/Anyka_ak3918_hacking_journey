#ifndef VENDOR_DAEMON_PUSH_H
#define VENDOR_DAEMON_PUSH_H

#include <stdint.h>

int handle_venc_start_push(int fd, const uint8_t *req, uint32_t req_len);
int handle_venc_stop_push(int fd, const uint8_t *req, uint32_t req_len);
int handle_audio_start_push(int fd, const uint8_t *req, uint32_t req_len);
int handle_audio_stop_push(int fd, const uint8_t *req, uint32_t req_len);
/* Returns 0 if the push thread was joined, -1 if it timed out and was detached
 * (in which case it may still be running and touching the ring buffer). */
int stop_push_slot(int idx);
/* Stop the audio push slot and tear its SDK chain down; 0 on clean join,
 * -1 if the worker is wedged (daemon restart is the recovery path). */
int push_stop_audio(void);

#endif /* VENDOR_DAEMON_PUSH_H */
