#ifndef VENDOR_DAEMON_GLOBALS_H
#define VENDOR_DAEMON_GLOBALS_H

#include <stdint.h>
#include <stdio.h>
#include <pthread.h>
#include <signal.h>

/* ---- Push-mode frame delivery state ------------------------------------- */

struct push_stream_state {
    pthread_t       thread;
    volatile int    active;
    void           *stream_handle;
    uint32_t        stream_id;
    /* Timestamp normalization state */
    uint32_t        first_timestamp_ms;   /* First SDK timestamp seen (anchor) */
    int             timestamp_initialized; /* 0 = not set, 1 = initialized */
    /*
     * Rollover tracking.  The 32-bit SDK clock wraps ~every 49.7 days; the
     * epoch is incremented by 2^32 each time a consecutive-sample backward
     * jump of more than half the 32-bit space is observed, so `raw64 =
     * raw_timestamp_epoch_ms + raw_timestamp_ms` stays monotonic across any
     * number of wraps (unlike extending against the fixed first timestamp,
     * which goes stale once the clock laps the anchor).
     */
    uint32_t        last_raw_timestamp_ms;  /* Previous SDK timestamp seen */
    uint64_t        raw_timestamp_epoch_ms; /* 2^32 per detected rollover */
    /*
     * Timestamp continuity state.  Pathological vs.ts leaps (>5s) are clamped;
     * regressions are held monotonic; ISP stalls where wall time advances but
     * vs.ts does not get a capped catch-up so live players stay near the edge.
     * ts_corr_ms carries the offset so later frames stay continuous.
     *
     * All int64_t: the arithmetic mixes a running signed correction with 32-bit
     * timestamps, and a wider signed type makes the over/underflow checks
     * ordinary comparisons instead of unsigned-wrap reasoning.
     */
    int64_t         last_out_ts_ms;         /* Last timestamp published to the ring */
    int64_t         last_sane_interval_ms;  /* Init 66; updated when delta in 16..1000 */
    int64_t         ts_corr_ms;             /* Added into normalized out after clamps */
    /*
     * Set when stop_push_slot() gave up waiting for this worker.
     *
     * Distinct from `active`, which is only the stop *request*: a wedged thread has already seen
     * active=0 and still not returned, so the slot's `thread`, `stream_handle` and `stream_id`
     * may still be read by it.  Reusing the slot in that window would hand a second thread the
     * same state and reset the ring underneath the first.  Main thread only; the worker never
     * reads it.
     */
    int             join_pending;
};

#define PUSH_STREAM_SLOT_COUNT  2
#define PUSH_POLL_SLEEP_MS      5  /* Sleep on no-data (slightly less than ref's 10ms) */

/*
 * Maximum consecutive no-data iterations before the push thread self-exits.
 * At PUSH_POLL_SLEEP_MS=5, 1000 iterations ~ 5 seconds.  Prevents zombie
 * threads when the SDK pipeline is broken.
 *
 * NOTE: this bounds the loop only when ak_venc_get_stream() *returns*.  If it
 * blocks inside the SDK the counter never advances, so it cannot on its own
 * bound stop_push_slot() -- see PUSH_JOIN_TIMEOUT_SEC.
 */
/* 6000 * PUSH_POLL_SLEEP_MS = 30s. Fatal, so the margin over a legitimate
 * ISP day/night stall (0.5-2s per the note in push.c) has to be large.
 * Calibration knob: lower it only with dusk evidence. */
#define PUSH_NO_DATA_EXIT_THRESHOLD 6000

/* Liveness beacon read by anyka-init's monitor. tmpfs, so no SD writes. */
#define PUSH_HEARTBEAT_PATH "/tmp/vd_heartbeat"

/*
 * How many consecutive no-data polls before the run is worth a log line.
 *
 * An empty poll is the *normal* case: frames arrive every ~66 ms at 15 fps and
 * PUSH_POLL_SLEEP_MS is 5, so every inter-frame gap produces ~13 of them.  The
 * counter resets on each frame, so logging the first one of a run (`== 1`) fired
 * ~15x/s per stream and wrote ~4.5 KB/s to vfat on the SD card, permanently --
 * 88 MB observed on-device, contending for the one core that also encodes and
 * sends video.  At 5 ms per poll this threshold means "silent below 1 s, then a
 * line per second", which is the only regime that indicates a real stall.
 */
#define PUSH_NO_DATA_WARN_INTERVAL 200

/*
 * How long stop_push_slot() waits for a push thread before giving up on it.
 * A thread parked inside a blocking SDK call cannot be interrupted, and an
 * unbounded pthread_join() there turns a wedged encoder into a daemon that
 * ignores SIGTERM entirely.
 */
#define PUSH_JOIN_TIMEOUT_SEC 3

/* ---- Shutdown flag ------------------------------------------------------- */
/*
 * sig_atomic_t, not int: C11 7.14.1.1p5 only guarantees a signal handler may
 * assign to a volatile sig_atomic_t (or a lock-free atomic).  Anything else is
 * undefined, however well a 32-bit aligned store happens to behave on ARM.
 */
extern volatile sig_atomic_t g_shutdown;

/* ---- Control client guard ------------------------------------------------ */
extern int g_control_fd;

/* ---- Shared memory ring buffer (Approach A) ----------------------------- */
extern void *g_ring_buffer;

/* ---- Socket file descriptors -------------------------------------------- */
extern int g_ctrl_server_fd;
extern int g_frame_main_server_fd;
extern int g_frame_sub_server_fd;
extern int g_frame_main_client_fd;   /* At most 1 main frame client */
extern int g_frame_sub_client_fd;    /* At most 1 sub frame client */

/* Per-channel locks: keep main/sub notification channels independent. */
extern pthread_mutex_t g_frame_main_client_lock;
extern pthread_mutex_t g_frame_sub_client_lock;

/* ---- Logging ------------------------------------------------------------ */
extern int g_saved_stdout;
extern int g_saved_stderr;
extern FILE *g_log_fp;

/* ---- Push stream state array ------------------------------------------- */
extern struct push_stream_state g_push_streams[PUSH_STREAM_SLOT_COUNT];

/* ---- Session object registry -------------------------------------------
 *
 * Every SDK object opened on behalf of the control client is recorded here so
 * it can be closed when that client goes away.
 *
 * This exists because cleanup cannot live on the client side: four of six
 * client-side handle Drops are deliberate no-ops in IPC mode, so onvif-rust
 * never sends CLOSE, and no Drop runs at all under SIGKILL -- which is exactly
 * the case that matters. Resetting here covers every way the client can vanish.
 *
 * The table is also the backing store for handle tokens: a token names a slot
 * rather than carrying a raw SDK pointer across the process boundary.
 */

/* Object kinds. These are the `kind` nibble of a handle token, so they must fit
 * in 4 bits and their values must never change. */
#define VD_OBJ_KIND_NONE    0
#define VD_OBJ_KIND_VI      1
#define VD_OBJ_KIND_VENC    2
#define VD_OBJ_KIND_AI      3
#define VD_OBJ_KIND_AENC    4
#define VD_OBJ_KIND_STREAM  5
#define VD_OBJ_KIND_COUNT   6

/* Live objects tracked per generation. The token reserves 6 bits for the slot
 * index, but this box opens a handful: 2 VI + 2 VENC + 2 stream + audio. 64 is
 * far more than can ever be live and keeps the table in one cache-friendly
 * block. */
#define VD_OBJ_SLOTS 64

struct vd_obj_slot {
    void    *ptr;       /* SDK object; meaningless unless live */
    uint16_t slot_gen;  /* bumped on every allocation of this slot */
    uint8_t  kind;      /* VD_OBJ_KIND_* */
    uint8_t  live;      /* 1 while the object is open */
};

extern struct vd_obj_slot g_obj_slots[VD_OBJ_SLOTS];

/**
 * vd_obj_register - Record a freshly opened SDK object.
 *
 * @param kind  VD_OBJ_KIND_* describing the object.
 * @param ptr   SDK pointer to remember.
 * @return      Slot index on success, -1 if the table is full.
 */
int vd_obj_register(uint8_t kind, void *ptr);

/**
 * vd_obj_unregister - Forget an object the client closed explicitly.
 *
 * Idempotent: unknown pointers are ignored, so a double CLOSE is harmless.
 *
 * @param kind  VD_OBJ_KIND_* the caller expects.
 * @param ptr   SDK pointer to release.
 */
void vd_obj_unregister(uint8_t kind, void *ptr);

/**
 * vd_cancel_stream_bounded - Cancel an SDK stream, giving up after a timeout.
 *
 * Runs ak_venc_cancel_stream on a detached worker and waits a bounded time.
 * On timeout the small worker arg is intentionally leaked (the worker may still
 * write it).
 *
 * @param handle      Stream handle from ak_venc_request_stream.
 * @param out_result  Optional; set to the SDK return code when cancel completes.
 * @return            0 if cancel completed, -1 on malloc/pthread failure,
 *                    -2 on timeout.
 */
int vd_cancel_stream_bounded(void *handle, int *out_result);

/**
 * vd_stream_orphan_set - Remember a live STREAM that has no object-table slot.
 *
 * Used when register fails or cancel spawn fails after unregister, so the
 * capture_thread can still be reclaimed by displacing a newer orphan.
 * Displacing a previous orphan best-effort cancels it first.
 */
void vd_stream_orphan_set(void *handle);

/** Clear the orphan slot if it still names @p handle (e.g. after cancel OK). */
void vd_stream_orphan_clear(void *handle);

/* ---- Handle tokens ------------------------------------------------------
 *
 * Handles are opaque tokens naming a table slot, never raw SDK pointers. A
 * pointer marshalled across the process boundary is meaningless to the client
 * and, after a restart, the allocator can hand the same address back -- so a
 * stale handle from the previous generation would look perfectly valid.
 *
 * Layout is 32 bits, NOT 64. The client is 32-bit ARM and carries handles as
 * `void *` (`hal/common/video.rs`), so anything above bit 31 is truncated on
 * the way back and cannot be validated:
 *
 *   bits 31..16  epoch      (16)  low half of the daemon generation
 *   bits 15..10  slot_gen   (6)   bumped on every reuse of the slot
 *   bits  9..4   slot_index (6)   VD_OBJ_SLOTS == 64
 *   bits  3..0   kind       (4)   VD_OBJ_KIND_*
 *
 * `kind` is never 0 for a live object, so a valid token is never 0 and cannot
 * be confused with a NULL handle.
 *
 * slot_gen is what stops an epoch-local ABA: close a VI on slot 3 and open
 * another within the same generation, and a stale token for the old occupant
 * still matches epoch and index. The per-slot generation makes it stop
 * matching.
 */
#define VD_TOK_KIND_MASK    0x0Fu
#define VD_TOK_INDEX_SHIFT  4
#define VD_TOK_INDEX_MASK   0x3Fu
#define VD_TOK_GEN_SHIFT    10
#define VD_TOK_GEN_MASK     0x3Fu
#define VD_TOK_EPOCH_SHIFT  16
#define VD_TOK_EPOCH_MASK   0xFFFFu

/* Compile-time layout guards: table sizes must fit the token bit fields. */
typedef char vd_tok_slots_fit[
    (VD_OBJ_SLOTS <= (VD_TOK_INDEX_MASK + 1)) ? 1 : -1];
typedef char vd_tok_kinds_fit[
    (VD_OBJ_KIND_COUNT <= (VD_TOK_KIND_MASK + 1)) ? 1 : -1];

/**
 * vd_obj_token - Build the client-facing token naming @p slot.
 *
 * @param slot  Slot index returned by vd_obj_register().
 * @return      Token, or 0 if @p slot is out of range or free.
 */
uint64_t vd_obj_token(int slot);

/**
 * vd_obj_resolve - Validate a token and yield the SDK pointer it names.
 *
 * Validates in order: kind matches what the command expects (a VENC token
 * handed to a VI command is rejected, not dereferenced), epoch matches this
 * daemon generation, index is in bounds, the slot is live, and slot_gen
 * matches the slot's current value.
 *
 * @param token        Token received from the client.
 * @param expect_kind  VD_OBJ_KIND_* the calling command requires.
 * @param out_ptr      Receives the SDK pointer on success.
 * @return             0 on success, -1 if the token is stale or malformed.
 */
int vd_obj_resolve(uint64_t token, uint8_t expect_kind, void **out_ptr);

#endif /* VENDOR_DAEMON_GLOBALS_H */
