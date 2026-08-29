#include <string.h>
#include <stdint.h>

#include "sound.h"

static uint32_t rd_u32(const uint8_t *b, uint32_t off)
{
    return (uint32_t)b[off]
         | ((uint32_t)b[off+1] << 8)
         | ((uint32_t)b[off+2] << 16)
         | ((uint32_t)b[off+3] << 24);
}

int sound_parse_play_req(const uint8_t *req, uint32_t req_len, struct sound_req *out)
{
    if (req == NULL || out == NULL || req_len < 16)
        return -1;

    out->sample_rate = rd_u32(req, 0);
    out->channel_num = rd_u32(req, 4);

    int vol = (int)rd_u32(req, 8);
    if (vol < 0)                 vol = 0;
    if (vol > SOUND_VOLUME_MAX)  vol = SOUND_VOLUME_MAX;
    out->volume = vol;

    uint32_t path_len = rd_u32(req, 12);
    /* Reject a length that overruns the buffer or the path field. Checked
     * before any copy, so a lying path_len can never overread. */
    if (path_len == 0 || path_len > SOUND_PATH_MAX || 16 + path_len > req_len)
        return -1;
    /* The sender must NUL-terminate; we never synthesise one. */
    if (req[16 + path_len - 1] != '\0')
        return -1;

    memcpy(out->path, req + 16, path_len);

    /* Confine playback to the SD-card tree. Absolute paths only; no ".." escapes. */
    if (out->path[0] != '/')
        return -1;
    if (strstr(out->path, "/../") != NULL)
        return -1;
    {
        size_t n = strlen(out->path);
        if (n >= 3 && strcmp(out->path + n - 3, "/..") == 0)
            return -1;
    }
    if (strncmp(out->path, SOUND_PATH_PREFIX, sizeof(SOUND_PATH_PREFIX) - 1) != 0)
        return -1;
    return 0;
}

int sound_dup_mono_to_stereo(const unsigned char *src, int len, unsigned char *dest)
{
    int samples = len / 2;
    int j;

    for (j = 0; j < samples; ++j) {
        dest[j * 4]     = src[j * 2];
        dest[j * 4 + 1] = src[j * 2 + 1];
        dest[j * 4 + 2] = src[j * 2];
        dest[j * 4 + 3] = src[j * 2 + 1];
    }
    return samples * 4;
}
