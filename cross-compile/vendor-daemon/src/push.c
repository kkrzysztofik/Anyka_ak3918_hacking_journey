/* pthread_timedjoin_np is a GNU extension; must precede every header. */
#define _GNU_SOURCE

#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <time.h>
#include <pthread.h>
#include <unistd.h>

#include "push.h"
#include "globals.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#include "vd_ring_buffer.h"

/* ---- Timestamp forward-clamp bounds -------------------------------------
 *
 * A step larger than TS_MAX_FORWARD_MS between published timestamps is treated
 * as a capture stall rather than elapsed media time -- the ISP day/night switch
 * blocks the encoder for hundreds of ms, and passing that gap through makes
 * live players resync or drop the session.  Such a step is replaced by the last
 * plausible frame interval; only intervals inside [TS_SANE_MIN_MS,
 * TS_SANE_MAX_MS] are learned as plausible.
 *
 * Live VLC rejects deltas beyond ~9s (bound 9000000 µs). A 250ms cap was too
 * tight: ISP day/night stalls ~0.5–2s carry real wall-clock vs.ts gaps that
 * must pass through, or the published clock falls behind the live edge for
 * good. Cap under the VLC bound so real stalls pass through; still clamp
 * pathological multi-second leaps.
 *
 * Do NOT re-add wall-clock "stall catchup" here: vs.ts is already wall-based,
 * and a publish-side gap only means frames backlogged in the SDK queue. Jumping
 * PTS toward wall while the backlog drains ratchets ts_corr_ms forward until
 * VLC's 9s bound trips ("Timestamp conversion failed" / early picture skipped).
 *
 * # ponytail: 5s forward cap (was 250ms); lower only if conversion fails return.
 */
#define TS_MAX_FORWARD_MS 5000
#define TS_SANE_MIN_MS    16
#define TS_SANE_MAX_MS    1000

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

/* Audio SDK chain, owned start-to-finish by the audio push thread.  All three
 * are opened in handle_audio_start_push() and torn down in reverse in
 * handle_audio_stop_push().  File-static rather than in push_stream_state
 * because that struct is shared with the video path and none of these fields
 * mean anything there. */
static void *g_ai_handle      = NULL;
static void *g_aenc_handle    = NULL;
static void *g_astream_handle = NULL;

/**
 * push_slot_index - Map a stream_id to a g_push_streams array index.
 *
 * @param stream_id  Stream identifier (VD_STREAM_MAIN, VD_STREAM_SUB or VD_STREAM_AUDIO).
 * @return           Array index (0, 1 or 2) on success, -1 for unknown stream_id.
 */
static int push_slot_index(uint32_t stream_id)
{
    switch (stream_id) {
    case VD_STREAM_MAIN:
        return 0;
    case VD_STREAM_SUB:
        return 1;
    case VD_STREAM_AUDIO:
        return 2;
    default:
        return -1;
    }
}

/**
 * push_stream_id_to_ring_stream - Map a push stream_id to the ring buffer stream_id constant.
 *
 * @param stream_id  Push stream identifier (VD_STREAM_MAIN, VD_STREAM_SUB or
 *                   VD_STREAM_AUDIO).
 * @return           Corresponding ring buffer VD_STREAM_* constant.
 */
static uint32_t push_stream_id_to_ring_stream(uint32_t stream_id)
{
    switch (stream_id) {
    case VD_STREAM_SUB:
        return VD_STREAM_SUB;
    case VD_STREAM_AUDIO:
        return VD_STREAM_AUDIO;
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
            if (no_data_count % PUSH_NO_DATA_WARN_INTERVAL == 0) {
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
                /* Crash-only: the pipeline is dead and nothing in this process
                 * can rebuild it. Exiting hands recovery to anyka-init, whose
                 * backoff/crash-loop/storm-guard policy already exists. The
                 * kernel closing /dev/ion, /dev/video0 and /dev/uio0 cleans the
                 * SDK state better than vd_obj_close_all() does. */
                _exit(1);
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

        /* Timestamp normalization: subtract first timestamp to produce 0-based values,
         * then forward-clamp oversized jumps (see TS_MAX_FORWARD_MS). */
        if (!state->timestamp_initialized) {
            state->first_timestamp_ms = raw_timestamp_ms;
            state->last_raw_timestamp_ms = raw_timestamp_ms;
            state->raw_timestamp_epoch_ms = 0;
            state->timestamp_initialized = 1;
            timestamp_ms = 0;
            log_info("event=timestamp_anchor stream=%u first_ts_ms=%u diag_monotonic_ms=%llu",
                     state->stream_id,
                     raw_timestamp_ms,
                     (unsigned long long)diag_monotonic_ms());
        } else {
            /* The 32-bit SDK clock wraps ~every 49.7 days.  Extending against the
             * fixed first timestamp goes stale once the clock laps the anchor, so
             * track a 64-bit epoch from consecutive-sample rollovers instead:
             * a backward jump of more than half the 32-bit space means the SDK
             * clock wrapped, and each wrap adds one 2^32 epoch. */
            if (raw_timestamp_ms < state->last_raw_timestamp_ms &&
                state->last_raw_timestamp_ms - raw_timestamp_ms > UINT32_MAX / 2) {
                state->raw_timestamp_epoch_ms += UINT64_C(1) << 32;
                log_warn("event=timestamp_wrap stream=%u raw_ts=%u epoch=%llu diag_monotonic_ms=%llu",
                         state->stream_id,
                         raw_timestamp_ms,
                         (unsigned long long)state->raw_timestamp_epoch_ms,
                         (unsigned long long)diag_monotonic_ms());
            }
            uint64_t raw64 = state->raw_timestamp_epoch_ms + raw_timestamp_ms;
            state->last_raw_timestamp_ms = raw_timestamp_ms;
            uint64_t first64 = (uint64_t)state->first_timestamp_ms;
            uint64_t delta = raw64 - first64;
            int64_t base_ms = (delta > UINT32_MAX) ? (int64_t)UINT32_MAX : (int64_t)delta;

            /*
             * Apply the running correction before judging the step.  The
             * correction is signed, so the candidate can land below the
             * normalized value or below zero; saturate it into the publishable
             * range rather than let it wrap.
             */
            int64_t cand = base_ms + state->ts_corr_ms;
            if (cand < 0) {
                cand = 0;
            } else if (cand > (int64_t)UINT32_MAX) {
                cand = (int64_t)UINT32_MAX;
            }

            int64_t last_out = state->last_out_ts_ms;
            int64_t forward = cand - last_out;
            uint64_t now_mono = diag_monotonic_ms();

            if (forward > TS_MAX_FORWARD_MS) {
                /* Pathological leap (above VLC's ~9s comfort with margin). */
                int64_t step = state->last_sane_interval_ms;
                if (step > 250) {
                    step = 250;
                } else if (step < TS_SANE_MIN_MS) {
                    step = TS_SANE_MIN_MS;
                }
                int64_t clamped = last_out + step;
                if (clamped > (int64_t)UINT32_MAX) {
                    clamped = (int64_t)UINT32_MAX;
                }
                state->ts_corr_ms += clamped - cand;
                log_warn("event=timestamp_forward_clamp stream=%u raw_ts=%u cand=%lld out=%lld step=%lld corr_ms=%lld diag_monotonic_ms=%llu",
                         state->stream_id,
                         raw_timestamp_ms,
                         (long long)cand,
                         (long long)clamped,
                         (long long)step,
                         (long long)state->ts_corr_ms,
                         (unsigned long long)now_mono);
                cand = clamped;
            } else if (forward <= 0) {
                /* Never publish a regression or duplicate PTS — VLC/FFmpeg treat
                 * equal DTS as non-monotonic and eventually trip conversion fail. */
                int64_t step = 1; /* minimal advance; keep media clock moving */
                int64_t clamped = last_out + step;
                if (clamped > (int64_t)UINT32_MAX)
                    clamped = (int64_t)UINT32_MAX;
                state->ts_corr_ms += clamped - cand;
                log_warn("event=timestamp_backward_clamp stream=%u raw_ts=%u cand=%lld out=%lld step=%lld corr_ms=%lld",
                         state->stream_id,
                         raw_timestamp_ms,
                         (long long)cand,
                         (long long)clamped,
                         (long long)step,
                         (long long)state->ts_corr_ms);
                cand = clamped;
            } else if (forward >= TS_SANE_MIN_MS && forward <= TS_SANE_MAX_MS) {
                state->last_sane_interval_ms = forward;
            }
            timestamp_ms = (uint32_t)cand;
        }

        state->last_out_ts_ms = (int64_t)timestamp_ms;

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

            /* Only the main stream drives liveness: if it stalls we are
             * broken regardless of what the sub stream is doing. */
            if (state->stream_id == 0) {
                FILE *hb = fopen(PUSH_HEARTBEAT_PATH, "w");
                if (hb) {
                    fprintf(hb, "%llu\n", (unsigned long long)frames_pushed);
                    fclose(hb);
                }
            }
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

/**
 * audio_push_thread - Dedicated pthread for encoded-audio delivery.
 *
 * Polls ak_aenc_get_stream(), which returns a LIST of encoded frames rather
 * than the single frame ak_venc_get_stream() yields, writes each to the ring
 * with stream_id=VD_STREAM_AUDIO, and releases every entry.
 *
 * @param arg   Pointer to the struct push_stream_state for the audio slot.
 * @return      Always NULL.
 */
static void *audio_push_thread(void *arg)
{
    struct push_stream_state *state = (struct push_stream_state *)arg;
    uint64_t frames_pushed = 0;
    uint64_t polls = 0;
    uint64_t get_stream_errs = 0;
    uint64_t ring_drops = 0;
    uint32_t seq_no = 0;

    log_info("event=audio_push_lifecycle state=start thread_id=%lu diag_monotonic_ms=%llu",
             (unsigned long)pthread_self(),
             (unsigned long long)diag_monotonic_ms());

    while (state->active && !g_shutdown) {
        struct list_head stream_head;
        INIT_LIST_HEAD(&stream_head);

        /*
         * Pace every iteration like aenc_demo does. ak_aenc_get_stream() may
         * return 0 ("success") with an EMPTY list when no frame is ready yet;
         * sleeping only on a non-zero return would then busy-spin and starve
         * the SDK's read_pcm_thread on this single core.
         */
        if (ak_aenc_get_stream(g_astream_handle, &stream_head) != 0) {
            get_stream_errs++;
            struct timespec ts = { .tv_sec = 0, .tv_nsec = PUSH_POLL_SLEEP_MS * 1000000L };
            nanosleep(&ts, NULL);
            polls++;
            continue;
        }

        struct aenc_entry *entry;
        struct aenc_entry *tmp;
        list_for_each_entry_safe(entry, tmp, &stream_head, list) {
            uint32_t frame_len = entry->stream.len;

            /*
             * Anchor audio to the MAIN video stream's timestamp origin rather
             * than to its own first frame.  Audio is fanned into both the main
             * and sub queues by the Rust bridge, and video normalizes against
             * its own first timestamp -- so an independent audio anchor puts
             * the two clocks an arbitrary offset apart and lip-sync drifts by
             * however long audio started after video.
             *
             * If video has not anchored yet there is nothing to sync against;
             * drop the frame rather than invent an origin we would have to
             * correct later.
             */
            if (!g_push_streams[0].timestamp_initialized) {
                ak_aenc_release_stream(entry);
                continue;
            }
            uint32_t first_ms = g_push_streams[0].first_timestamp_ms;
            uint32_t raw_ms   = (uint32_t)entry->stream.ts;
            uint32_t timestamp_ms = (raw_ms >= first_ms) ? (raw_ms - first_ms) : 0;

            int ring_slot;
            pthread_mutex_lock(&g_ring_write_lock);
            /*
             * ponytail: audio is written as VD_FRAME_TYPE_P so the ring's
             * eviction logic sheds it before video keyframes under pressure.
             * Video is the primary product on a security camera. Upgrade path:
             * give the ring a per-stream priority field if audio dropouts turn
             * out to matter more than an extra video P-frame.
             */
            ring_slot = vd_ring_write(g_ring_buffer, entry->stream.data, frame_len,
                                      timestamp_ms, seq_no,
                                      VD_FRAME_TYPE_P, VD_STREAM_AUDIO);
            if (ring_slot >= 0) {
                fill_slot_timing(g_ring_buffer, ring_slot);
            } else {
                ring_drops++;
            }
            pthread_mutex_unlock(&g_ring_write_lock);

            if (ring_slot >= 0) {
                struct vd_frame_notify notif;
                notif.slot_index = (uint32_t)ring_slot;
                notif.frame_len  = frame_len;
                notif.flags      = VD_NOTIFY_LAST_FRAGMENT;
                notif.stream_id  = VD_STREAM_AUDIO;
                notif.seq_no     = seq_no;
                if (send_frame_notification(VD_STREAM_AUDIO, &notif) != 0) {
                    log_warn("[audio] notification write failed, client may have disconnected");
                }
                frames_pushed++;
            }
            seq_no++;
            ak_aenc_release_stream(entry);
        }
        polls++;

        if (frames_pushed > 0 && (frames_pushed % 300) == 0) {
            log_info("[audio] frames=%llu polls=%llu errs=%llu drops=%llu diag_monotonic_ms=%llu",
                     (unsigned long long)frames_pushed,
                     (unsigned long long)polls,
                     (unsigned long long)get_stream_errs,
                     (unsigned long long)ring_drops,
                     (unsigned long long)diag_monotonic_ms());
        }

        struct timespec ts = { .tv_sec = 0, .tv_nsec = PUSH_POLL_SLEEP_MS * 1000000L };
        nanosleep(&ts, NULL);
    }

    log_info("event=audio_push_lifecycle state=exit frames_pushed=%llu polls=%llu errs=%llu drops=%llu diag_monotonic_ms=%llu",
             (unsigned long long)frames_pushed,
             (unsigned long long)polls,
             (unsigned long long)get_stream_errs,
             (unsigned long long)ring_drops,
             (unsigned long long)diag_monotonic_ms());
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
 * A slot whose join failed keeps join_pending set for the life of the process.
 * It is not retried: the thread was detached, so it can no longer be joined,
 * and a worker parked in the SDK will not become joinable by asking again --
 * retrying would only spend another PUSH_JOIN_TIMEOUT_SEC per attempt. The flag
 * exists to stop the slot being reused, which is the actual hazard.
 *
 * @param idx  Index into g_push_streams (0 for main stream, 1 for sub stream).
 * @return     0 if the thread was joined, -1 if it did not join (this call or an
 *             earlier one); in that case the worker may still be running.
 */
int stop_push_slot(int idx)
{
    struct push_stream_state *state;
    if (idx < 0 || idx >= PUSH_STREAM_SLOT_COUNT) {
        return 0;
    }
    state = &g_push_streams[idx];
    if (state->join_pending) {
        /* An earlier stop gave up on this worker.  It may still be running, so this is not the
         * "nothing to do" case: reporting success here would let the caller tear down memory the
         * thread can still touch. */
        log_error("event=push_thread_lifecycle state=stop_still_wedged stream=%u slot=%d diag_monotonic_ms=%llu",
                  state->stream_id,
                  idx,
                  (unsigned long long)diag_monotonic_ms());
        return -1;
    }
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
        /* Either way the thread is unaccounted for, so the slot stays reserved.  But the two
         * cases are not the same fault and must not share a log line: ETIMEDOUT means the worker
         * is still running inside the SDK, while EINVAL/ESRCH mean the join itself was invalid,
         * which is a bug here rather than a wedged vendor call. */
        state->join_pending = 1;

        if (rc == ETIMEDOUT) {
            log_error("event=push_thread_lifecycle state=join_timeout stream=%u slot=%d timeout_sec=%d diag_monotonic_ms=%llu",
                      state->stream_id,
                      idx,
                      PUSH_JOIN_TIMEOUT_SEC,
                      (unsigned long long)diag_monotonic_ms());
            /* Detach so the thread's resources are reclaimed if it ever does return.
             * Do not clear stream_handle here: the detached worker may still call
             * ak_venc_get_stream/ak_venc_release_stream with it. */
            int drc = pthread_detach(state->thread);
            if (drc != 0) {
                /* Neither joined nor detached: the thread is unreachable and its stack will not
                 * be reclaimed.  Say so rather than let the timeout line imply a clean handoff. */
                log_error("event=push_thread_lifecycle state=detach_failed stream=%u slot=%d rc=%d diag_monotonic_ms=%llu",
                          state->stream_id,
                          idx,
                          drc,
                          (unsigned long long)diag_monotonic_ms());
            }
        } else {
            log_error("event=push_thread_lifecycle state=join_error stream=%u slot=%d rc=%d diag_monotonic_ms=%llu",
                      state->stream_id,
                      idx,
                      rc,
                      (unsigned long long)diag_monotonic_ms());
        }
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
    if (state->join_pending) {
        /* A previous worker on this slot never returned.  Starting another would hand it the same
         * state struct, overwrite the thread handle the old one is still associated with, and
         * reset the ring the old one may still be writing. */
        log_error("[push] stream=%u slot=%d has an unjoined worker, refusing start_push", stream_id, idx);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (state->active) {
        log_warn("[push] stream=%u already active, ignoring start_push", stream_id);
        log_info("event=push_cmd cmd=19 status=ok stream=%u active_before=%d reason=already_active diag_monotonic_ms=%llu",
                 stream_id,
                 state->active,
                 (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_OK, NULL, 0);
    }

    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_STREAM,
                       &state->stream_handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    state->stream_id = stream_id;
    state->active = 1;
    /* Reset timestamp normalization state on push start */
    state->timestamp_initialized = 0;
    state->first_timestamp_ms = 0;
    state->last_raw_timestamp_ms = 0;
    state->raw_timestamp_epoch_ms = 0;
    state->last_out_ts_ms = 0;
    state->last_sane_interval_ms = 66;
    state->ts_corr_ms = 0;

    /* Reset ring buffer if this is the first push activation (no other slot
     * was active).  Clears stale sequences/flags from a previous session so the
     * consumer doesn't see immediate overflow.  With three slots a two-slot
     * 0/1 flip no longer answers "am I the first?", and getting it wrong resets
     * the ring underneath a stream that is already publishing. */
    int any_other_active = 0;
    for (int i = 0; i < PUSH_STREAM_SLOT_COUNT; i++) {
        if (i != idx && g_push_streams[i].active) {
            any_other_active = 1;
            break;
        }
    }
    if (!any_other_active && g_ring_buffer) {
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
        if (stop_push_slot(idx) != 0) {
            log_error("[push] failed to stop push-based frame delivery (stream=%u)", stream_id);
            log_error("event=push_cmd cmd=20 status=error scope=single stream=%u reason=stop_failed diag_monotonic_ms=%llu",
                      stream_id,
                      (unsigned long long)diag_monotonic_ms());
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
        log_info("[push] push-based frame delivery stopped (stream=%u)", stream_id);
        log_info("event=push_cmd cmd=20 status=ok scope=single stream=%u diag_monotonic_ms=%llu",
                 stream_id,
                 (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_OK, NULL, 0);
    }

    int failed = 0;
    for (int i = 0; i < PUSH_STREAM_SLOT_COUNT; i++) {
        failed |= (stop_push_slot(i) != 0);
    }
    if (failed) {
        log_error("[push] failed to stop push-based frame delivery (all streams)");
        log_error("event=push_cmd cmd=20 status=error scope=all reason=stop_failed diag_monotonic_ms=%llu",
                  (unsigned long long)diag_monotonic_ms());
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    log_info("[push] push-based frame delivery stopped (all streams)");
    log_info("event=push_cmd cmd=20 status=ok scope=all diag_monotonic_ms=%llu",
             (unsigned long long)diag_monotonic_ms());
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_audio_start_push - IPC handler for CMD_AUDIO_START_PUSH.
 *
 * Opens the full audio SDK chain and spawns the audio push thread.
 *
 * Wire format: [u32 sample_rate][u32 channel_num][u32 frame_interval_ms] = 12 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_audio_start_push(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 12) {
        log_warn("[audio] start_push: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    uint32_t sample_rate  = req_read_u32(req, 0);
    uint32_t channel_num  = req_read_u32(req, 4);
    uint32_t interval_ms  = req_read_u32(req, 8);

    int idx = push_slot_index(VD_STREAM_AUDIO);
    struct push_stream_state *state = &g_push_streams[idx];

    if (state->active) {
        log_warn("[audio] already active, ignoring start_push");
        return send_response(fd, STATUS_OK, NULL, 0);
    }

    struct pcm_param ai_param;
    memset(&ai_param, 0, sizeof(ai_param));
    ai_param.sample_bits = 16;          /* SDK supports 16 only */
    ai_param.channel_num = channel_num;
    ai_param.sample_rate = sample_rate;

    g_ai_handle = ak_ai_open(&ai_param);
    if (g_ai_handle == NULL) {
        log_error("[audio] ak_ai_open failed rate=%u ch=%u", sample_rate, channel_num);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* Filters off: AEC/NR/AGC are 8K-only and tuned for two-way voice, not for
     * a monitoring mic.  Resample off because we open the ADC at the rate we
     * actually want. */
    ak_ai_set_aec(g_ai_handle, AUDIO_FUNC_DISABLE);
    ak_ai_set_nr_agc(g_ai_handle, AUDIO_FUNC_DISABLE);
    ak_ai_set_resample(g_ai_handle, AUDIO_FUNC_DISABLE);
    ak_ai_set_source(g_ai_handle, AI_SOURCE_MIC);
    ak_ai_clear_frame_buffer(g_ai_handle);

    /* Must precede start_capture.  ak_ai.h documents the range as [10,125] ms
     * but AAC at 8 kHz needs 128; aenc_demo sets it anyway and works, so log
     * the return code rather than treating it as fatal. */
    int iv_ret = ak_ai_set_frame_interval(g_ai_handle, (int)interval_ms);
    if (iv_ret != 0) {
        log_warn("[audio] set_frame_interval(%u) returned %d; continuing", interval_ms, iv_ret);
    }
    ak_ai_start_capture(g_ai_handle);

    struct audio_param aenc_param;
    memset(&aenc_param, 0, sizeof(aenc_param));
    aenc_param.type        = AK_AUDIO_TYPE_AAC;
    aenc_param.sample_bits = 16;
    aenc_param.channel_num = channel_num;
    aenc_param.sample_rate = sample_rate;

    g_aenc_handle = ak_aenc_open(&aenc_param);
    if (g_aenc_handle == NULL) {
        log_error("[audio] ak_aenc_open failed");
        goto fail_ai;
    }

    /* CUT, not SAVE: RTP AAC-hbr and FLV both want raw AAC frames.  SAVE
     * prepends an ADTS header, which is right for files and wrong here -- it
     * yields a negotiated track that decodes to garbage. */
    struct aenc_attr attr;
    attr.aac_head = AENC_AAC_CUT_FRAME_HEAD;
    ak_aenc_set_attr(g_aenc_handle, &attr);

    g_astream_handle = ak_aenc_request_stream(g_ai_handle, g_aenc_handle);
    if (g_astream_handle == NULL) {
        log_error("[audio] ak_aenc_request_stream failed");
        goto fail_aenc;
    }

    state->stream_id = VD_STREAM_AUDIO;
    state->active = 1;

    if (pthread_create(&state->thread, NULL, audio_push_thread, state) != 0) {
        log_error("[audio] pthread_create failed: %s", strerror(errno));
        state->active = 0;
        goto fail_stream;
    }

    log_info("[audio] push started rate=%u ch=%u interval=%ums",
             sample_rate, channel_num, interval_ms);
    return send_response(fd, STATUS_OK, NULL, 0);

fail_stream:
    ak_aenc_cancel_stream(g_astream_handle);
    g_astream_handle = NULL;
fail_aenc:
    ak_aenc_close(g_aenc_handle);
    g_aenc_handle = NULL;
fail_ai:
    ak_ai_stop_capture(g_ai_handle);
    ak_ai_close(g_ai_handle);
    g_ai_handle = NULL;
    return send_response(fd, STATUS_ERROR, NULL, 0);
}

/**
 * handle_audio_stop_push - IPC handler for CMD_AUDIO_STOP_PUSH.
 *
 * Stops the audio push thread and tears the SDK chain down in reverse order.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (unused for this command).
 * @param req_len Length of @p req in bytes (unused).
 * @return        0 on success, -1 on I/O error.
 */
int handle_audio_stop_push(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;

    int idx = push_slot_index(VD_STREAM_AUDIO);
    if (stop_push_slot(idx) != 0) {
        log_error("[audio] failed to stop audio push slot");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* Reverse of the open order in handle_audio_start_push(). */
    if (g_astream_handle) {
        ak_aenc_cancel_stream(g_astream_handle);
        g_astream_handle = NULL;
    }
    if (g_aenc_handle) {
        ak_aenc_close(g_aenc_handle);
        g_aenc_handle = NULL;
    }
    if (g_ai_handle) {
        ak_ai_stop_capture(g_ai_handle);
        ak_ai_close(g_ai_handle);
        g_ai_handle = NULL;
    }

    log_info("[audio] push stopped");
    return send_response(fd, STATUS_OK, NULL, 0);
}
