#ifndef VENDOR_DAEMON_GLOBALS_H
#define VENDOR_DAEMON_GLOBALS_H

#include <stdint.h>
#include <stdio.h>
#include <pthread.h>

/* ---- Push-mode frame delivery state ------------------------------------- */

struct push_stream_state {
    pthread_t       thread;
    volatile int    active;
    void           *stream_handle;
    uint32_t        stream_id;
};

#define PUSH_STREAM_SLOT_COUNT  2
#define PUSH_POLL_SLEEP_MS      5  /* Sleep on no-data (slightly less than ref's 10ms) */

/*
 * Maximum consecutive no-data iterations before the push thread self-exits.
 * At PUSH_POLL_SLEEP_MS=5, 1000 iterations ~ 5 seconds.  Prevents zombie
 * threads when the SDK pipeline is broken, and guarantees that
 * stop_push_slot()'s pthread_join() completes in bounded time.
 */
#define PUSH_NO_DATA_EXIT_THRESHOLD 1000

/* ---- Shutdown flag ------------------------------------------------------- */
extern volatile int g_shutdown;

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

/* ---- Wall-clock timing for inter-frame delta tracking ------------------ */
extern uint64_t g_last_wall_clock_us;

/* ---- Push stream state array ------------------------------------------- */
extern struct push_stream_state g_push_streams[PUSH_STREAM_SLOT_COUNT];

#endif /* VENDOR_DAEMON_GLOBALS_H */
