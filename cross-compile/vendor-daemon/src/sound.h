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

/* Duplicate s16le mono samples into interleaved stereo.
 *
 * The DA accepts stereo only: handing it a mono buffer makes each channel take
 * every other sample, which halves the effective rate and doubles the pitch.
 * See ak_ao_demo.c:66 (copy_for_dual_channel).
 *
 * `dest` must have room for `len * 2` bytes. A trailing odd byte is dropped.
 * Returns the number of bytes written to `dest`. */
int sound_dup_mono_to_stereo(const unsigned char *src, int len, unsigned char *dest);

#endif /* VENDOR_DAEMON_SOUND_H */
