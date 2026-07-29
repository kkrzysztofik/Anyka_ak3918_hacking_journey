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
    uint32_t        first_timestamp_ms;   /* First SDK timestamp seen */
    int             timestamp_initialized; /* 0 = not set, 1 = initialized */
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
#define PUSH_NO_DATA_EXIT_THRESHOLD 1000

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

#endif /* VENDOR_DAEMON_GLOBALS_H */
