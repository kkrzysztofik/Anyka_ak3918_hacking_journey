#ifndef VENDOR_DAEMON_SOUND_H
#define VENDOR_DAEMON_SOUND_H

#include <stdint.h>

#define SOUND_PATH_MAX      256
/* Playback paths must live under the SD-card payload tree. */
#define SOUND_PATH_PREFIX   "/mnt/anyka_hack/"
#define SOUND_VOLUME_MAX    6     /* ak_ao dac range is [0,6]; 0 is mute */

struct sound_req {
    unsigned int sample_rate;
    unsigned int channel_num;
    int          volume;
    char         path[SOUND_PATH_MAX];
};

/* Parse a CMD_AUDIO_PLAY payload. Returns 0 on success, -1 on malformed input.
 * Volume is clamped to [0, SOUND_VOLUME_MAX]. Path must be absolute, contain no
 * "..", and start with SOUND_PATH_PREFIX. */
int sound_parse_play_req(const uint8_t *req, uint32_t req_len, struct sound_req *out);

/* Start playback on the worker thread. Returns 0 if accepted, 1 if busy,
 * -1 on failure to start. */
int sound_play_async(const struct sound_req *req);

/* True if the worker is currently playing. */
int sound_is_playing(void);

#endif /* VENDOR_DAEMON_SOUND_H */
