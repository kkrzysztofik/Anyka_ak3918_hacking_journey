/* Host-compiled unit test for sound play-request parsing. */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "sound.h"

static uint32_t put_u32(uint8_t *b, uint32_t off, uint32_t v)
{
    b[off] = v & 0xff; b[off+1] = (v >> 8) & 0xff;
    b[off+2] = (v >> 16) & 0xff; b[off+3] = (v >> 24) & 0xff;
    return off + 4;
}

int main(void)
{
    uint8_t buf[128];
    struct sound_req r;

    /* A well-formed request round-trips. */
    uint32_t o = 0;
    o = put_u32(buf, o, 8000);   /* rate */
    o = put_u32(buf, o, 1);      /* channels */
    o = put_u32(buf, o, 3);      /* volume */
    o = put_u32(buf, o, 9);      /* path_len */
    memcpy(buf + o, "/x/a.raw", 9); o += 9;
    assert(sound_parse_play_req(buf, o, &r) == 0);
    assert(r.sample_rate == 8000);
    assert(r.channel_num == 1);
    assert(r.volume == 3);
    assert(strcmp(r.path, "/x/a.raw") == 0);

    /* Truncated header is rejected. */
    assert(sound_parse_play_req(buf, 8, &r) != 0);

    /* path_len lying beyond the buffer is rejected (no overread). */
    o = put_u32(buf, 12, 999);
    assert(sound_parse_play_req(buf, 25, &r) != 0);
    put_u32(buf, 12, 9);

    /* Volume is clamped to the DAC's 0-6 range, not passed through. */
    put_u32(buf, 8, 99);
    assert(sound_parse_play_req(buf, 25, &r) == 0);
    assert(r.volume == 6);
    put_u32(buf, 8, (uint32_t)-5);
    assert(sound_parse_play_req(buf, 25, &r) == 0);
    assert(r.volume == 0);
    put_u32(buf, 8, 3);

    /* A path with no NUL terminator is rejected. */
    memcpy(buf + 16, "AAAAAAAAA", 9);
    assert(sound_parse_play_req(buf, 25, &r) != 0);

    printf("test_sound_parse: OK\n");
    return 0;
}
