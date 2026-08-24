/* Host-compiled unit test for push slot mapping.
 *
 * push_slot_index() and push_stream_id_to_ring_stream() are file-static, so we
 * include the translation unit and stub the handful of symbols it references.
 */
#define _GNU_SOURCE

#include <assert.h>
#include <signal.h>
#include <stdio.h>
#include <stdarg.h>
#include <stdint.h>

#include "globals.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/* --- Stubs for symbols push.c references but this test never calls -------- */
struct video_stream;
struct vd_frame_notify;

int ak_venc_get_stream(void *h, struct video_stream *vs) { (void)h; (void)vs; return -1; }
int ak_venc_release_stream(void *h, struct video_stream *vs) { (void)h; (void)vs; return 0; }
void *ak_ai_open(const struct pcm_param *p) { (void)p; return NULL; }
int ak_ai_set_aec(void *h, int e) { (void)h; (void)e; return 0; }
int ak_ai_set_nr_agc(void *h, int e) { (void)h; (void)e; return 0; }
int ak_ai_set_resample(void *h, int e) { (void)h; (void)e; return 0; }
int ak_ai_set_source(void *h, enum ai_source s) { (void)h; (void)s; return 0; }
int ak_ai_clear_frame_buffer(void *h) { (void)h; return 0; }
int ak_ai_set_frame_interval(void *h, int i) { (void)h; (void)i; return 0; }
int ak_ai_start_capture(void *h) { (void)h; return 0; }
int ak_ai_stop_capture(void *h) { (void)h; return 0; }
int ak_ai_close(void *h) { (void)h; return 0; }
void *ak_aenc_open(const struct audio_param *p) { (void)p; return NULL; }
int ak_aenc_set_attr(void *h, const struct aenc_attr *a) { (void)h; (void)a; return 0; }
void *ak_aenc_request_stream(void *ai, void *enc) { (void)ai; (void)enc; return NULL; }
int ak_aenc_get_stream(void *sh, struct list_head *lh) { (void)sh; (void)lh; return -1; }
int ak_aenc_release_stream(struct aenc_entry *e) { (void)e; return 0; }
int ak_aenc_cancel_stream(void *sh) { (void)sh; return 0; }
int ak_aenc_close(void *h) { (void)h; return 0; }
int send_response(int fd, int32_t status, const void *data, uint32_t len) {
    (void)fd; (void)status; (void)data; (void)len; return 0;
}
int send_frame_notification(uint32_t sid, const struct vd_frame_notify *n) {
    (void)sid; (void)n; return 0;
}
void vd_obj_close_all(void) {}
int vd_obj_resolve(uint64_t tok, uint8_t kind, void **out) {
    (void)tok; (void)kind; (void)out; return -1;
}
uint64_t diag_monotonic_ms(void) { return 0; }
void log_log(int level, const char *file, int line, const char *fmt, ...) {
    (void)level; (void)file; (void)line; (void)fmt;
}

/* --- Externs push.c uses that globals.c would normally provide ------------ */
struct push_stream_state g_push_streams[PUSH_STREAM_SLOT_COUNT];
void *g_ring_buffer = NULL;
volatile sig_atomic_t g_shutdown = 0;

#include "push.c"

int main(void)
{
    /* Every real stream maps to a distinct slot. */
    assert(push_slot_index(VD_STREAM_MAIN)  == 0);
    assert(push_slot_index(VD_STREAM_SUB)   == 1);
    assert(push_slot_index(VD_STREAM_AUDIO) == 2);

    /* Unknown ids are still rejected. */
    assert(push_slot_index(99) == -1);

    /* The slot table must actually have room for the audio slot. */
    assert(PUSH_STREAM_SLOT_COUNT == 3);

    /* Ring stream id round-trips. */
    assert(push_stream_id_to_ring_stream(VD_STREAM_MAIN)  == VD_STREAM_MAIN);
    assert(push_stream_id_to_ring_stream(VD_STREAM_SUB)   == VD_STREAM_SUB);
    assert(push_stream_id_to_ring_stream(VD_STREAM_AUDIO) == VD_STREAM_AUDIO);

    printf("test_push_slots: PASS\n");
    return 0;
}