/* Host-compiled unit test for sound play-request parsing. */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "sound.h"

static const char *const k_allowed_path = "/mnt/anyka_hack/onvif/sounds/a.raw";

static uint32_t put_u32(uint8_t *b, uint32_t off, uint32_t v)
{
    b[off] = v & 0xff; b[off+1] = (v >> 8) & 0xff;
    b[off+2] = (v >> 16) & 0xff; b[off+3] = (v >> 24) & 0xff;
    return off + 4;
}

static uint32_t fill_play_req(uint8_t *buf, const char *path, int volume)
{
    uint32_t path_len = (uint32_t)strlen(path) + 1; /* include NUL */
    uint32_t o = 0;
    o = put_u32(buf, o, 8000);      /* rate */
    o = put_u32(buf, o, 1);         /* channels */
    o = put_u32(buf, o, (uint32_t)volume);
    o = put_u32(buf, o, path_len);
    memcpy(buf + o, path, path_len);
    return o + path_len;
}

int main(void)
{
    uint8_t buf[256];
    struct sound_req r;
    uint32_t len;

    /* A well-formed request under the allowed prefix round-trips. */
    len = fill_play_req(buf, k_allowed_path, 3);
    assert(sound_parse_play_req(buf, len, &r) == 0);
    assert(r.sample_rate == 8000);
    assert(r.channel_num == 1);
    assert(r.volume == 3);
    assert(strcmp(r.path, k_allowed_path) == 0);

    /* Truncated header is rejected. */
    assert(sound_parse_play_req(buf, 8, &r) != 0);

    /* path_len lying beyond the buffer is rejected (no overread). */
    put_u32(buf, 12, 999);
    assert(sound_parse_play_req(buf, len, &r) != 0);
    len = fill_play_req(buf, k_allowed_path, 3);

    /* Volume is clamped to the DAC's 0-6 range, not passed through. */
    len = fill_play_req(buf, k_allowed_path, 99);
    assert(sound_parse_play_req(buf, len, &r) == 0);
    assert(r.volume == 6);
    len = fill_play_req(buf, k_allowed_path, -5);
    assert(sound_parse_play_req(buf, len, &r) == 0);
    assert(r.volume == 0);

    /* A path with no NUL terminator is rejected. */
    len = fill_play_req(buf, k_allowed_path, 3);
    buf[len - 1] = 'X';
    assert(sound_parse_play_req(buf, len, &r) != 0);

    /* Paths outside the SD-card prefix are rejected. */
    len = fill_play_req(buf, "/tmp/evil.raw", 3);
    assert(sound_parse_play_req(buf, len, &r) != 0);

    /* Path traversal under the prefix is rejected. */
    len = fill_play_req(buf, "/mnt/anyka_hack/../etc/passwd", 3);
    assert(sound_parse_play_req(buf, len, &r) != 0);

    printf("test_sound_parse: OK\n");
    return 0;
}
