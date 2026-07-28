/* pthread_timedjoin_np is a GNU extension; must precede every header. */
#define _GNU_SOURCE

#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <time.h>
#include <pthread.h>

#include "push.h"
#include "globals.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_venc.h"
#include "vd_ring_buffer.h"

/* ---- Internal helpers (file-static) ------------------------------------- */

/*
 * The main and sub push threads share the single global ring buffer, but
 * vd_ring_write() is a single-producer primitive: it loads write_seq, derives a
 * slot from it, copies the payload and only then increments write_seq. Two
 * unsynchronised producers can therefore select the same slot, clobber each
 * other's payload and advance write_seq twice for one usable frame — the
 * consumer rejects the duplicate as a stale notification and never advances
 * read_seq for it, so the ring permanently loses capacity.
 */
static pthread_mutex_t g_ring_write_lock = PTHREAD_MUTEX_INITIALIZER;

/**
 * push_slot_index - Map a stream_id to a g_push_streams array index.
 *
 * @param stream_id  Stream identifier (VD_STREAM_MAIN or VD_STREAM_SUB).
 * @return           Array index (0 or 1) on success, -1 for unknown stream_id.
 */
static int push_slot_index(uint32_t stream_id)
{
    switch (stream_id) {
    case VD_STREAM_MAIN:
        return 0;
    case VD_STREAM_SUB:
        return 1;
    default:
        return -1;
    }
}

/**
 * push_stream_id_to_ring_stream - Map a push stream_id to the ring buffer stream_id constant.
 *
 * @param stream_id  Push stream identifier (VD_STREAM_MAIN or VD_STREAM_SUB).
 * @return           Corresponding ring buffer VD_STREAM_* constant.
 */
static uint32_t push_stream_id_to_ring_stream(uint32_t stream_id)
{
    switch (stream_id) {
    case VD_STREAM_SUB:
        return VD_STREAM_SUB;
    case VD_STREAM_MAIN:
    default:
        return VD_STREAM_MAIN;
    }
}

/**
 * convert_frame_type - Convert an SDK video_frame_type to a ring buffer VD_FRAME_TYPE_* constant.
 *
 * @param sdk_type  SDK frame type enum value.
 * @return          Corresponding VD_FRAME_TYPE_* constant (uint32_t).
 */
static uint32_t convert_frame_type(enum video_frame_type sdk_type)
{
    switch (sdk_type) {
    case FRAME_TYPE_I:  return VD_FRAME_TYPE_I;
    case FRAME_TYPE_B:  return VD_FRAME_TYPE_B;
    case FRAME_TYPE_PI: return VD_FRAME_TYPE_PI;
    case FRAME_TYPE_P:
    default:            return VD_FRAME_TYPE_P;
    }
}

/**
 * fill_slot_timing - Populate wall-clock timing field in a slot header.
 *
 * Fills wall_clock_us in the slot header after a successful vd_ring_write() call.
 *
 * @param ring_base  Base pointer of the shared memory ring buffer.
 * @param slot_idx   Index of the slot to update.
 */
static void fill_slot_timing(void *ring_base, int slot_idx)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    uint64_t wall_us = (uint64_t)now.tv_sec * 1000000ULL + (uint64_t)now.tv_nsec / 1000ULL;

    struct vd_slot_header *slot = vd_ring_get_slot_hdr(ring_base, (uint32_t)slot_idx);
    slot->wall_clock_us = wall_us;
}

/**
 * slot_state_name - Return a human-readable string for a VD_SLOT_* state constant.
 *
 * @param state  VD_SLOT_* constant.
 * @return       Constant string literal for the state, or "UNKNOWN".
 */
static const char *slot_state_name(uint32_t state)
{
    switch (state) {
    case VD_SLOT_EMPTY:
        return "EMPTY";
    case VD_SLOT_WRITING:
        return "WRITING";
    case VD_SLOT_READY:
        return "READY";
    case VD_SLOT_READING:
        return "READING";
    default:
        return "UNKNOWN";
    }
}

/**
 * push_frame_thread - Dedicated pthread entry point for push-based frame delivery.
 *
 * Polls ak_venc_get_stream() in a tight loop, writes frames to the ring
 * buffer, and pushes unsolicited 20-byte notifications to the frame client.
 * The Rust side just reads notifications — zero polling, zero wasted IPC.
 *
 * @param arg   Pointer to the struct push_stream_state for this stream slot.
 * @return      Always NULL.
 */
static void *push_frame_thread(void *arg)
{
    struct push_stream_state *state = (struct push_stream_state *)arg;
    struct video_stream vs;
    uint64_t frames_pushed = 0;
    uint64_t no_data_count = 0;
    uint32_t ring_stream_id = push_stream_id_to_ring_stream(state->stream_id);

    log_info("event=push_thread_lifecycle state=start stream=%u handle=%p thread_id=%lu diag_monotonic_ms=%llu",
             state->stream_id,
             state->stream_handle,
             (unsigned long)pthread_self(),
             (unsigned long long)diag_monotonic_ms());

    while (state->active && !g_shutdown) {
        memset(&vs, 0, sizeof(vs));
        int ret = ak_venc_get_stream(state->stream_handle, &vs);

        if (ret != 0) {
            /* No data — brief sleep to avoid busy-spin */
            no_data_count++;
            if (no_data_count == 1 || no_data_count % 1000 == 0) {
                log_warn("event=push_get_stream_error stream=%u handle=%p sdk_ret=%d no_data_count=%llu diag_monotonic_ms=%llu",
                         state->stream_id,
                         state->stream_handle,
                         ret,
                         (unsigned long long)no_data_count,
                         (unsigned long long)diag_monotonic_ms());
            }
            if (no_data_count >= PUSH_NO_DATA_EXIT_THRESHOLD) {
                log_error("event=push_thread_lifecycle state=exit_no_data stream=%u handle=%p no_data_count=%llu diag_monotonic_ms=%llu",
                          state->stream_id,
                          state->stream_handle,
                          (unsigned long long)no_data_count,
                          (unsigned long long)diag_monotonic_ms());
                state->active = 0;
                break;
            }
            struct timespec ts = { .tv_sec = 0, .tv_nsec = PUSH_POLL_SLEEP_MS * 1000000L };
            nanosleep(&ts, NULL);
            continue;
        }

        /* Success — reset consecutive no-data counter */
        no_data_count = 0;

        uint32_t frame_len = vs.len;
        uint32_t raw_timestamp_ms = (uint32_t)vs.ts;
        uint32_t timestamp_ms;
        uint32_t seq_no = (uint32_t)vs.seq_no;
        uint32_t ring_frame_type = convert_frame_type(vs.frame_type);

        /* Timestamp normalization: subtract first timestamp to produce 0-based values */
        if (!state->timestamp_initialized) {
            state->first_timestamp_ms = raw_timestamp_ms;
            state->timestamp_initialized = 1;
            timestamp_ms = 0;
            log_info("event=timestamp_anchor stream=%u first_ts_ms=%u diag_monotonic_ms=%llu",
                     state->stream_id,
                     raw_timestamp_ms,
                     (unsigned long long)diag_monotonic_ms());
        } else {
            /* 32-bit SDK timestamps wrap ~every 49.7 days; extend with u64 before subtract */
            uint64_t raw64 = (uint64_t)raw_timestamp_ms;
            uint64_t first64 = (uint64_t)state->first_timestamp_ms;
            if (raw64 < first64) {
                log_warn("event=timestamp_wrap stream=%u raw_ts=%u first_ts=%u diag_monotonic_ms=%llu",
                         state->stream_id,
                         raw_timestamp_ms,
                         state->first_timestamp_ms,
                         (unsigned long long)diag_monotonic_ms());
                /* 32-bit SDK clock may wrap multiple times over very long runs; extend until aligned. */
                while (raw64 < first64) {
                    raw64 += (uint64_t)1U << 32;
                }
            }
            uint64_t delta = raw64 - first64;
            timestamp_ms = (delta > UINT32_MAX) ? UINT32_MAX : (uint32_t)delta;
        }

        log_debug("event=timestamp_normalize stream=%u raw_ts=%u normalized_ts=%u seq_no=%u diag_monotonic_ms=%llu",
                  state->stream_id,
                  raw_timestamp_ms,
                  timestamp_ms,
                  seq_no,
                  (unsigned long long)diag_monotonic_ms());

        /* Try ring buffer write */
        int ring_slot = -1;
        if (g_ring_buffer != NULL && frame_len <= VD_SHM_SLOT_DATA_SIZE) {
            pthread_mutex_lock(&g_ring_write_lock);
            ring_slot = vd_ring_write(g_ring_buffer, vs.data, frame_len,
                                       timestamp_ms, seq_no,
                                       ring_frame_type, ring_stream_id);
            log_debug("event=ring_write_attempt stream=%u seq_no=%u frame_type=%u frame_len=%u slot=%d diag_monotonic_ms=%llu",
                      state->stream_id,
                      seq_no,
                      ring_frame_type,
                      frame_len,
                      ring_slot,
                      (unsigned long long)diag_monotonic_ms());

            /* I-frame priority eviction on overflow */
            if (ring_slot == -1 && ring_frame_type == VD_FRAME_TYPE_I) {
                struct vd_ring_header *hdr = vd_ring_get_header(g_ring_buffer);
                __atomic_add_fetch(&hdr->overflow_count, 1, __ATOMIC_RELAXED);

                int evicted = vd_ring_evict_oldest_pframe(g_ring_buffer);
                if (evicted >= 0) {
                    __atomic_add_fetch(&hdr->eviction_count, 1, __ATOMIC_RELAXED);
                    log_info("event=ring_evict_pframe stream=%u evicted_slot=%d seq_no=%u overflow_count=%u eviction_count=%u diag_monotonic_ms=%llu",
                             state->stream_id,
                             evicted,
                             seq_no,
                             hdr->overflow_count,
                             hdr->eviction_count,
                             (unsigned long long)diag_monotonic_ms());
                    ring_slot = vd_ring_write(g_ring_buffer, vs.data, frame_len,
                                               timestamp_ms, seq_no,
                                               ring_frame_type, ring_stream_id);
                }
            } else if (ring_slot == -1) {
                /* P/Pi-frame overflow */
                struct vd_ring_header *hdr = vd_ring_get_header(g_ring_buffer);
                __atomic_add_fetch(&hdr->overflow_count, 1, __ATOMIC_RELAXED);
                __atomic_add_fetch(&hdr->dropped_count, 1, __ATOMIC_RELAXED);
                log_warn("event=ring_overflow_drop stream=%u seq_no=%u frame_type=%u frame_len=%u overflow_count=%u dropped_count=%u diag_monotonic_ms=%llu",
                         state->stream_id,
                         seq_no,
                         ring_frame_type,
                         frame_len,
                         hdr->overflow_count,
                         hdr->dropped_count,
                         (unsigned long long)diag_monotonic_ms());
            }

            if (ring_slot >= 0) {
                /* Populate wall-clock timing in the slot */
                fill_slot_timing(g_ring_buffer, ring_slot);
            }
            pthread_mutex_unlock(&g_ring_write_lock);
        }

        if (ring_slot >= 0) {
            {
                struct vd_ring_header *hdr = vd_ring_get_header(g_ring_buffer);
                struct vd_slot_header *slot_hdr = vd_ring_get_slot_hdr(g_ring_buffer, (uint32_t)ring_slot);
                log_debug("event=ring_write stream=%u slot=%d slot_state=%u slot_state_name=%s seq_no=%u slot_seq_no=%u slot_stream=%u slot_ts_ms=%lu write_seq=%u read_seq=%u diag_monotonic_ms=%llu",
                          state->stream_id,
                          ring_slot,
                          slot_hdr->state,
                          slot_state_name(slot_hdr->state),
                          seq_no,
                          slot_hdr->seq_no,
                          slot_hdr->stream_id,
                          (unsigned long)slot_hdr->timestamp_ms,
                          hdr->write_seq,
                          hdr->read_seq,
                          (unsigned long long)diag_monotonic_ms());
            }

            /* Push notification to frame client (if connected) */
            struct vd_frame_notify notif;
            notif.slot_index = (uint32_t)ring_slot;
            notif.frame_len = frame_len;
            notif.flags = VD_NOTIFY_LAST_FRAGMENT;
            notif.stream_id = ring_stream_id;
            notif.seq_no = seq_no;
            if (send_frame_notification(state->stream_id, &notif) != 0) {
                log_warn("[push] notification write failed, client may have disconnected");
            }
            frames_pushed++;
        } else if (ring_frame_type != VD_FRAME_TYPE_I) {
            /* P/Pi-frame dropped during overflow — send drop notification if client connected */
            struct vd_frame_notify notif;
            notif.slot_index = 0;
            notif.frame_len = 0;
            notif.flags = VD_NOTIFY_FRAME_DROPPED;
            notif.stream_id = ring_stream_id;
            notif.seq_no = seq_no;
            (void)send_frame_notification(state->stream_id, &notif);
            log_warn("event=push_drop_notify stream=%u seq_no=%u frame_type=%u diag_monotonic_ms=%llu",
                     state->stream_id,
                     seq_no,
                     ring_frame_type,
                     (unsigned long long)diag_monotonic_ms());
        }
        /* else: I-frame couldn't fit even after eviction — dropped (rare) */

        /* Release SDK frame immediately */
        ak_venc_release_stream(state->stream_handle, &vs);

        if (frames_pushed > 0 && (frames_pushed % 300) == 0) {
            log_info("[push] stream=%u frames=%llu no_data=%llu",
                     state->stream_id,
                     (unsigned long long)frames_pushed,
                     (unsigned long long)no_data_count);
        }
    }

    {
        const char *exit_reason = g_shutdown ? "shutdown" : "stop_requested";
        log_info("event=push_thread_lifecycle state=exit stream=%u reason=%s frames=%llu no_data=%llu thread_id=%lu diag_monotonic_ms=%llu",
                 state->stream_id,
                 exit_reason,
                 (unsigned long long)frames_pushed,
                 (unsigned long long)no_data_count,
                 (unsigned long)pthread_self(),
                 (unsigned long long)diag_monotonic_ms());
    }
    return NULL;
}

/* ---- Public interface ---------------------------------------------------- */

/**
 * stop_push_slot - Signal the push thread at slot idx to stop and join it.
 *
 * Sets the active flag to 0 and waits up to PUSH_JOIN_TIMEOUT_SEC for the
 * thread to exit.  Does nothing if the slot is out of range or already
 * inactive.
 *
 * The wait is bounded because the thread's loop condition is only re-tested
 * between ak_venc_get_stream() calls: if the SDK parks inside that call, the
 * thread never observes the flag and an unbounded join would hang shutdown
 * forever.  On timeout the thread is detached and the caller told, so it can
 * decide whether teardown of memory the thread may still touch is safe.
 *
 * @param idx  Index into g_push_streams (0 for main stream, 1 for sub stream).
 * @return     0 if the thread was joined, -1 if it timed out and was detached.
 */
int stop_push_slot(int idx)
{
    struct push_stream_state *state;
    if (idx < 0 || idx >= PUSH_STREAM_SLOT_COUNT) {
        return 0;
    }
    state = &g_push_streams[idx];
    if (!state->active) {
        log_info("event=push_thread_lifecycle state=stop_skip slot=%d active=0 diag_monotonic_ms=%llu",
                 idx,
                 (unsigned long long)diag_monotonic_ms());
        return 0;
    }
    log_info("event=push_thread_lifecycle state=stop_request stream=%u slot=%d active_before=%d diag_monotonic_ms=%llu",
             state->stream_id,
             idx,
             state->active,
             (unsigned long long)diag_monotonic_ms());
    state->active = 0;

    struct timespec deadline;
    /* pthread_timedjoin_np deadlines are absolute and on CLOCK_REALTIME. */
    clock_gettime(CLOCK_REALTIME, &deadline);
    deadline.tv_sec += PUSH_JOIN_TIMEOUT_SEC;

    int rc = pthread_timedjoin_np(state->thread, NULL, &deadline);
    if (rc != 0) {
        log_error("event=push_thread_lifecycle state=join_timeout stream=%u slot=%d timeout_sec=%d rc=%d diag_monotonic_ms=%llu",
                  state->stream_id,
                  idx,
                  PUSH_JOIN_TIMEOUT_SEC,
                  rc,
                  (unsigned long long)diag_monotonic_ms());
        /* Detach so the thread's resources are reclaimed if it ever does return. */
        pthread_detach(state->thread);
        state->stream_handle = NULL;
        return -1;
    }

    log_info("event=push_thread_lifecycle state=joined stream=%u slot=%d diag_monotonic_ms=%llu",
             state->stream_id,
             idx,
             (unsigned long long)diag_monotonic_ms());
    state->stream_handle = NULL;
    return 0;
}

/**
 * handle_venc_start_push - IPC handler for CMD_VENC_START_PUSH.
 *
 * Starts push-based frame delivery for the specified stream by spawning a
 * push_frame_thread.  Idempotent if the stream is already active.
 *
 * Wire format: [u64 stream_handle][u32 stream_id] = 12 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_start_push(int fd, const uint8_t *req, uint32_t req_len)
{
    uint32_t stream_id;
    int idx;
    struct push_stream_state *state;

    log_info("event=push_cmd cmd=19 fd=%d req_len=%u diag_monotonic_ms=%llu",
             fd,
             req_len,
             (unsigned long long)diag_monotonic_ms());

    /* Wire format: [u64 stream_handle][u32 stream_id] */
    if (req_len < (sizeof(uint64_t) + sizeof(uint32_t))) {
        log_warn("event=push_cmd cmd=19 status=error reason=req_too_short req_len=%u diag_monotonic_ms=%llu",
                 req_len,
                 (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    stream_id = req_read_u32(req, sizeof(uint64_t));

    idx = push_slot_index(stream_id);
    if (idx < 0) {
        log_error("[push] unsupported stream_id=%u", stream_id);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    state = &g_push_streams[idx];
    if (state->active) {
        log_warn("[push] stream=%u already active, ignoring start_push", stream_id);
        log_info("event=push_cmd cmd=19 status=ok stream=%u active_before=%d reason=already_active diag_monotonic_ms=%llu",
                 stream_id,
                 state->active,
                 (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_OK, NULL, 0);
    }

    state->stream_handle = req_read_handle(req, 0);
    state->stream_id = stream_id;
    state->active = 1;
    /* Reset timestamp normalization state on push start */
    state->timestamp_initialized = 0;
    state->first_timestamp_ms = 0;

    /* Reset ring buffer if this is the first push activation (both slots
     * were inactive).  Clears stale sequences/flags from a previous session
     * so the consumer doesn't see immediate overflow. */
    int other_idx = (idx == 0) ? 1 : 0;
    if (!g_push_streams[other_idx].active && g_ring_buffer) {
        vd_ring_reset(g_ring_buffer);
        log_info("event=ring_reset reason=push_start stream=%u diag_monotonic_ms=%llu",
                 stream_id, (unsigned long long)diag_monotonic_ms());
    }

    if (pthread_create(&state->thread, NULL, push_frame_thread, state) != 0) {
        log_error("[push] failed to create push thread for stream=%u: %s",
                  stream_id, strerror(errno));
        state->active = 0;
        state->stream_handle = NULL;
        log_error("event=push_cmd cmd=19 status=error stream=%u reason=thread_create_failed errno=%d diag_monotonic_ms=%llu",
                  stream_id,
                  errno,
                  (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    log_info("[push] push-based frame delivery started (stream=%u handle=%p)",
             stream_id, state->stream_handle);
    log_info("event=push_cmd cmd=19 status=ok stream=%u handle=%p active_after=%d diag_monotonic_ms=%llu",
             stream_id,
             state->stream_handle,
             state->active,
             (unsigned long long)diag_monotonic_ms());
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_venc_stop_push - IPC handler for CMD_VENC_STOP_PUSH.
 *
 * Stops push-based frame delivery.  If req_len >= 4, stops only the stream
 * identified by the u32 stream_id in the payload; otherwise stops all streams.
 *
 * Wire format: [u32 stream_id] = 4 bytes, or empty payload to stop all streams.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_stop_push(int fd, const uint8_t *req, uint32_t req_len)
{
    log_info("event=push_cmd cmd=20 fd=%d req_len=%u diag_monotonic_ms=%llu",
             fd,
             req_len,
             (unsigned long long)diag_monotonic_ms());
    if (req_len >= sizeof(uint32_t)) {
        uint32_t stream_id = req_read_u32(req, 0);
        int idx;
        idx = push_slot_index(stream_id);
        if (idx < 0) {
            log_error("[push] unsupported stream_id=%u for stop_push", stream_id);
            log_error("event=push_cmd cmd=20 status=error stream=%u reason=unsupported_stream diag_monotonic_ms=%llu",
                      stream_id,
                      (unsigned long long)diag_monotonic_ms());
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
        stop_push_slot(idx);
        log_info("[push] push-based frame delivery stopped (stream=%u)", stream_id);
        log_info("event=push_cmd cmd=20 status=ok scope=single stream=%u diag_monotonic_ms=%llu",
                 stream_id,
                 (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_OK, NULL, 0);
    }

    stop_push_slot(0);
    stop_push_slot(1);
    log_info("[push] push-based frame delivery stopped (all streams)");
    log_info("event=push_cmd cmd=20 status=ok scope=all diag_monotonic_ms=%llu",
             (unsigned long long)diag_monotonic_ms());
    return send_response(fd, STATUS_OK, NULL, 0);
}
