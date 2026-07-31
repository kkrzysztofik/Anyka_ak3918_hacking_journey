#include "globals.h"
#include "log.h"

/* SDK close entry points used by the session-object sweep. */
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/* Ring header, for the daemon epoch baked into handle tokens. */
#include "vd_ring_buffer.h"

#include <pthread.h>
#include <stdlib.h>
#include <time.h>

volatile sig_atomic_t g_shutdown = 0;

int g_control_fd = -1;

void *g_ring_buffer = NULL;

int g_ctrl_server_fd      = -1;
int g_frame_main_server_fd = -1;
int g_frame_sub_server_fd  = -1;
int g_frame_main_client_fd = -1;
int g_frame_sub_client_fd  = -1;

pthread_mutex_t g_frame_main_client_lock = PTHREAD_MUTEX_INITIALIZER;
pthread_mutex_t g_frame_sub_client_lock  = PTHREAD_MUTEX_INITIALIZER;

int g_saved_stdout = -1;
int g_saved_stderr = -1;
FILE *g_log_fp = NULL;

/* Zero-initialized at start; main() resets with memset() before use. */
struct push_stream_state g_push_streams[PUSH_STREAM_SLOT_COUNT];

/* Built empty at daemon start, so it can never hold an object from a previous
 * generation. */
struct vd_obj_slot g_obj_slots[VD_OBJ_SLOTS];

/* Low 16 bits of this daemon generation's epoch, or 0 if the ring is absent.
 * Truncation is forced by the 32-bit token layout; see globals.h. It weakens
 * the daemon-side collision margin to 2^-16, which is acceptable because this
 * check is defence in depth -- the client's epoch gate still compares the full
 * 32-bit value before anything is sent. */
static uint16_t vd_daemon_epoch16(void)
{
    if (g_ring_buffer == NULL)
        return 0;
    return (uint16_t)(vd_ring_get_header(g_ring_buffer)->epoch & VD_TOK_EPOCH_MASK);
}

uint64_t vd_obj_token(int slot)
{
    uint32_t token;

    if (slot < 0 || slot >= VD_OBJ_SLOTS || !g_obj_slots[slot].live)
        return 0;

    token  = (uint32_t)(g_obj_slots[slot].kind & VD_TOK_KIND_MASK);
    token |= ((uint32_t)slot & VD_TOK_INDEX_MASK) << VD_TOK_INDEX_SHIFT;
    token |= ((uint32_t)g_obj_slots[slot].slot_gen & VD_TOK_GEN_MASK) << VD_TOK_GEN_SHIFT;
    token |= ((uint32_t)vd_daemon_epoch16()) << VD_TOK_EPOCH_SHIFT;
    return (uint64_t)token;
}

int vd_obj_resolve(uint64_t token, uint8_t expect_kind, void **out_ptr)
{
    uint32_t t = (uint32_t)token;
    uint8_t  kind;
    uint32_t index;
    uint16_t gen;
    uint16_t epoch;

    if (g_ring_buffer == NULL) {
        log_warn("[obj] resolve with no ring; rejecting token 0x%08x", t);
        return -1;
    }

    kind  = (uint8_t)(t & VD_TOK_KIND_MASK);
    index = (t >> VD_TOK_INDEX_SHIFT) & VD_TOK_INDEX_MASK;
    gen   = (uint16_t)((t >> VD_TOK_GEN_SHIFT) & VD_TOK_GEN_MASK);
    epoch = (uint16_t)((t >> VD_TOK_EPOCH_SHIFT) & VD_TOK_EPOCH_MASK);

    if (kind != expect_kind) {
        log_warn("[obj] token 0x%08x kind=%u but command expects %u",
                 t, (unsigned)kind, (unsigned)expect_kind);
        return -1;
    }
    if (epoch != vd_daemon_epoch16()) {
        log_warn("[obj] token 0x%08x from epoch %u, current %u (stale)",
                 t, (unsigned)epoch, (unsigned)vd_daemon_epoch16());
        return -1;
    }
    if (index >= VD_OBJ_SLOTS || !g_obj_slots[index].live) {
        log_warn("[obj] token 0x%08x names slot %u which is not live", t, index);
        return -1;
    }
    if ((g_obj_slots[index].slot_gen & VD_TOK_GEN_MASK) != gen) {
        log_warn("[obj] token 0x%08x slot_gen %u but slot %u is at %u (reused)",
                 t, (unsigned)gen, index,
                 (unsigned)(g_obj_slots[index].slot_gen & VD_TOK_GEN_MASK));
        return -1;
    }

    *out_ptr = g_obj_slots[index].ptr;
    return 0;
}

int vd_obj_register(uint8_t kind, void *ptr)
{
    int i;

    if (ptr == NULL || kind == VD_OBJ_KIND_NONE)
        return -1;

    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live)
            continue;
        /* Bump on allocation, not on release: a token from the slot's previous
         * occupant must stop matching the moment the slot is reused, even
         * within the same daemon generation. */
        g_obj_slots[i].slot_gen++;
        g_obj_slots[i].ptr  = ptr;
        g_obj_slots[i].kind = kind;
        g_obj_slots[i].live = 1;
        return i;
    }

    log_error("[obj] registry full (%d slots); leaking kind=%u ptr=%p",
              VD_OBJ_SLOTS, (unsigned)kind, ptr);
    return -1;
}

void vd_obj_unregister(uint8_t kind, void *ptr)
{
    int i;

    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live &&
            g_obj_slots[i].kind == kind &&
            g_obj_slots[i].ptr == ptr) {
            g_obj_slots[i].live = 0;
            g_obj_slots[i].ptr  = NULL;
            return;
        }
    }
}

/* Close order: streams depend on encoders, encoders on inputs. Tearing down in
 * the other direction hands the SDK a closed input under a running encoder. */
static const uint8_t g_obj_close_order[] = {
    VD_OBJ_KIND_STREAM,
    VD_OBJ_KIND_VENC,
    VD_OBJ_KIND_AENC,
    VD_OBJ_KIND_VI,
    VD_OBJ_KIND_AI,
};

/* Timeout for the SDK stream cancel, seconds. Shared by session cleanup and
 * the VENC cancel-stream handler. */
#define VD_CANCEL_STREAM_TIMEOUT_SEC 3

struct vd_cancel_arg {
    void        *handle;
    int          result;
    volatile int done;   /* set by the worker when ak_venc_cancel_stream returns */
};

static void *vd_cancel_worker(void *arg)
{
    struct vd_cancel_arg *ca = (struct vd_cancel_arg *)arg;
    ca->result = ak_venc_cancel_stream(ca->handle);
    __atomic_store_n(&ca->done, 1, __ATOMIC_RELEASE);
    return NULL;
}

/*
 * vd_cancel_stream_bounded - Cancel an SDK stream, giving up after a timeout.
 *
 * ak_venc_cancel_stream stops libmpi_venc's internal capture_thread, which must
 * happen before the VI it feeds on is closed (otherwise that thread calls
 * ak_vi_release_frame on freed state and the daemon segfaults). The call can
 * block indefinitely on a wedged encoder, so it runs on a detached thread and we
 * wait only VD_CANCEL_STREAM_TIMEOUT_SEC. On timeout we leak the small arg (the
 * detached thread may still touch it) and continue teardown: a leaked capture
 * thread is less bad than hanging the accept loop.
 */
int vd_cancel_stream_bounded(void *handle, int *out_result)
{
    struct vd_cancel_arg *ca = (struct vd_cancel_arg *)malloc(sizeof(*ca));
    if (ca == NULL) {
        log_error("[obj] cancel_stream: malloc failed; skipping cancel ptr=%p", handle);
        return -1;
    }
    ca->handle = handle;
    ca->result = -1;
    ca->done = 0;

    pthread_t tid;
    if (pthread_create(&tid, NULL, vd_cancel_worker, ca) != 0) {
        log_error("[obj] cancel_stream: pthread_create failed ptr=%p", handle);
        free(ca);
        return -1;
    }
    pthread_detach(tid);

    struct timespec deadline;
    clock_gettime(CLOCK_MONOTONIC, &deadline);
    deadline.tv_sec += VD_CANCEL_STREAM_TIMEOUT_SEC;

    while (!__atomic_load_n(&ca->done, __ATOMIC_ACQUIRE)) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > deadline.tv_sec ||
            (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)) {
            log_error("[obj] cancel_stream timed out after %ds ptr=%p (leaking arg)",
                      VD_CANCEL_STREAM_TIMEOUT_SEC, handle);
            return -2; /* intentional leak of ca; worker may still write it */
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000000L };
        nanosleep(&ts, NULL);
    }

    if (out_result != NULL)
        *out_result = ca->result;
    log_info("[obj] cancelled stream ptr=%p ret=%d", handle, ca->result);
    free(ca);
    return 0;
}

static void vd_obj_close_one(uint8_t kind, void *ptr)
{
    int ret = 0;

    switch (kind) {
    case VD_OBJ_KIND_STREAM:
        /* Cancel the SDK stream so libmpi_venc's capture_thread stops before the
         * VI it reads is closed. stop_push_slot() only stops the daemon's own
         * push reader, NOT this thread. */
        (void)vd_cancel_stream_bounded(ptr, NULL);
        return;
    case VD_OBJ_KIND_VENC:
        ret = ak_venc_close(ptr);
        break;
    case VD_OBJ_KIND_AENC:
        ret = ak_aenc_close(ptr);
        break;
    case VD_OBJ_KIND_VI:
        /* capture_off before close: the safe teardown onvif-rust does on clean
         * shutdown, moved here so a SIGKILLed client gets it too. */
        (void)ak_vi_capture_off(ptr);
        ret = ak_vi_close(ptr);
        break;
    case VD_OBJ_KIND_AI:
        ret = ak_ai_close(ptr);
        break;
    default:
        return;
    }

    if (ret != 0)
        log_warn("[obj] close kind=%u ptr=%p returned %d (continuing)",
                 (unsigned)kind, ptr, ret);
    else
        log_info("[obj] closed leaked kind=%u ptr=%p", (unsigned)kind, ptr);
}

/* Live STREAM that never entered (or fell out of) the token table. Reclaimed
 * by vd_obj_close_all so a failed cancel spawn cannot leave an untracked
 * capture_thread. */
static void *g_stream_orphan = NULL;

void vd_stream_orphan_set(void *handle)
{
    if (handle == NULL)
        return;
    if (g_stream_orphan != NULL && g_stream_orphan != handle) {
        log_error("[obj] displacing orphan stream %p with %p", g_stream_orphan, handle);
        (void)vd_cancel_stream_bounded(g_stream_orphan, NULL);
    }
    g_stream_orphan = handle;
}

void vd_stream_orphan_clear(void *handle)
{
    if (g_stream_orphan == handle)
        g_stream_orphan = NULL;
}

void vd_obj_close_all(void)
{
    size_t k;
    int i;

    if (g_stream_orphan != NULL) {
        void *orphan = g_stream_orphan;
        g_stream_orphan = NULL;
        log_warn("[obj] reclaiming orphan stream %p", orphan);
        (void)vd_cancel_stream_bounded(orphan, NULL);
    }

    for (k = 0; k < sizeof(g_obj_close_order) / sizeof(g_obj_close_order[0]); k++) {
        uint8_t kind = g_obj_close_order[k];
        for (i = 0; i < VD_OBJ_SLOTS; i++) {
            if (!g_obj_slots[i].live || g_obj_slots[i].kind != kind)
                continue;
            vd_obj_close_one(kind, g_obj_slots[i].ptr);
            g_obj_slots[i].live = 0;
            g_obj_slots[i].ptr  = NULL;
        }
    }

    /* Anything left is a kind with no close path (streams); drop it so the
     * next session starts from an empty table regardless. */
    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        g_obj_slots[i].live = 0;
        g_obj_slots[i].ptr  = NULL;
    }
}
