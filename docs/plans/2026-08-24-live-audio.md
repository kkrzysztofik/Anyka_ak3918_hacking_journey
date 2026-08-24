# Live AAC Audio Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver microphone audio as an AAC track alongside video on both RTSP and HTTP-FLV.

**Architecture:** The `vendor-daemon` (C) gains a third push thread that owns the whole Anyka audio SDK chain (`ak_ai_open` → `ak_aenc_request_stream` → poll `ak_aenc_get_stream`) and writes encoded AAC frames into the *existing* shared-memory ring with `stream_id = VD_STREAM_AUDIO`. Everything downstream already exists: `bridge.rs` routes `StreamId::Audio` into both stream queues, and the SDP and FLV paths emit an audio track as soon as `bridge.audio_config` is `Some`. The single new Rust responsibility is computing the 2-byte AAC AudioSpecificConfig and publishing it.

**Tech Stack:** C (uClibc, ARMv5TE cross-compile) for the daemon; Rust (tokio, mockall) for onvif-rust; Anyka SDK `libplat_ai` / `libmpi_aenc` / `libakaudiocodec` (already linked in the Makefile — no build changes).

**Design doc:** `docs/plans/2026-08-24-live-audio-design.md`

**Branch:** `feat/live-audio` (already created, design doc committed)

---

## Before You Start

Read these first. They are not optional context — several tasks below will make no sense without them.

- `docs/plans/2026-08-24-live-audio-design.md` — the decisions and their evidence
- `cross-compile/vendor-daemon/src/push.c:160-420` — `push_frame_thread`, the video push loop you are mirroring
- `cross-compile/vendor-daemon/src/push.c:573-720` — `handle_venc_start_push` / `handle_venc_stop_push`
- `cross-compile/vendor-daemon/include/vd_ring_buffer.h:86-163` — ring constants and slot/notify structs
- `cross-compile/anyka_reference/aenc_demo/ak_aenc_demo.c:300-400` — the **authoritative** SDK call sequence

Set up the toolchain once per shell:

```bash
cd /home/kmk/dev/anyka-dev
source setenv.sh          # exports $CARGO and the cross toolchain
```

**Toolchain rules that will bite you:**
- Always use `$CARGO` (the vendored toolchain), never a system `cargo`.
- Host-side Rust commands need `--target x86_64-unknown-linux-gnu`.
- The ARM build must run from `cross-compile/onvif-rust/`, not the workspace root, or cargo silently links with the host toolchain.
- `cargo clippy` needs the toolchain bin dir first on `PATH` or it dies with E0514.

---

## Task 1: Grow the push slot table to three

Pure refactor, no audio yet. This isolates the two hardcoded-index bugs that growing the table exposes, so they don't get tangled up with new functionality.

**Files:**
- Modify: `cross-compile/vendor-daemon/src/globals.h:54`
- Modify: `cross-compile/vendor-daemon/src/push.c:64-91` (`push_slot_index`, `push_stream_id_to_ring_stream`)
- Modify: `cross-compile/vendor-daemon/src/push.c:634` (ring-reset check)
- Modify: `cross-compile/vendor-daemon/src/push.c:707-708` (stop-all loop)
- Create: `cross-compile/vendor-daemon/tests/test_push_slots.c`

**Step 1: Write the failing test**

`push_slot_index` and `push_stream_id_to_ring_stream` are `static`, so the test includes `push.c` directly and stubs the symbols it references. `vd_ring_*` are `static inline` in the header and need no stubs.

Create `cross-compile/vendor-daemon/tests/test_push_slots.c`:

```c
/* Host-compiled unit test for push slot mapping.
 *
 * push_slot_index() and push_stream_id_to_ring_stream() are file-static, so we
 * include the translation unit and stub the handful of symbols it references.
 */
#include <assert.h>
#include <stdio.h>

/* --- Stubs for symbols push.c references but this test never calls -------- */
struct video_stream;
struct vd_frame_notify;

int ak_venc_get_stream(void *h, struct video_stream *vs) { (void)h; (void)vs; return -1; }
int ak_venc_release_stream(void *h, struct video_stream *vs) { (void)h; (void)vs; return 0; }
int send_response(int fd, int status, const unsigned char *d, unsigned int l) {
    (void)fd; (void)status; (void)d; (void)l; return 0;
}
int send_frame_notification(unsigned int sid, const struct vd_frame_notify *n) {
    (void)sid; (void)n; return 0;
}
void vd_obj_close_all(void) {}
int vd_obj_resolve(unsigned long long tok, int kind, void **out) {
    (void)tok; (void)kind; (void)out; return -1;
}

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
```

**Step 2: Run the test to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon
gcc -std=gnu99 -I include -I src -I ../anyka_reference/venc_demo/include \
    -o /tmp/test_push_slots tests/test_push_slots.c && /tmp/test_push_slots
```

Expected: assertion failure on `push_slot_index(VD_STREAM_AUDIO) == 2` (it currently returns `-1`).

If instead it fails to *compile* on a missing SDK header, add that header's directory to `-I` rather than stubbing the type — the reference headers are plain struct definitions and compile fine on the host.

**Step 3: Make the minimal change**

In `src/globals.h:54`:

```c
#define PUSH_STREAM_SLOT_COUNT  3   /* main, sub, audio */
```

In `src/push.c`, `push_slot_index()` — add the audio case and update the doc comment:

```c
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
```

In `push_stream_id_to_ring_stream()`:

```c
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
```

**Step 4: Fix the two hardcoded-index bugs the growth exposes**

These are the actual reason this task exists. Neither is caught by the test above; both are caught by review and by the device test in Task 7.

In `push.c:634`, the ring-reset-on-first-activation check currently assumes exactly two slots:

```c
    /* WAS: int other_idx = (idx == 0) ? 1 : 0;
     *      if (!g_push_streams[other_idx].active && g_ring_buffer) { ... }
     *
     * With three slots, "am I the first activation?" means no OTHER slot is
     * active -- a two-slot flip no longer answers that, and getting it wrong
     * resets the ring underneath a stream that is already publishing.
     */
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
```

In `push.c:707-708`, "stop all" stops slots 0 and 1 by literal index, which would silently leave the audio thread running:

```c
    /* WAS: failed |= (stop_push_slot(0) != 0);
     *      failed |= (stop_push_slot(1) != 0);
     */
    int failed = 0;
    for (int i = 0; i < PUSH_STREAM_SLOT_COUNT; i++) {
        failed |= (stop_push_slot(i) != 0);
    }
```

**Step 5: Run the test to verify it passes**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon
gcc -std=gnu99 -I include -I src -I ../anyka_reference/venc_demo/include \
    -o /tmp/test_push_slots tests/test_push_slots.c && /tmp/test_push_slots
```

Expected: `test_push_slots: PASS`

**Step 6: Verify the daemon still cross-compiles**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon && make clean && make
```

Expected: builds clean. This is the real regression check for Task 1 — the slot-count change touches a global array.

**Step 7: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/globals.h \
            cross-compile/vendor-daemon/src/push.c \
            cross-compile/vendor-daemon/tests/test_push_slots.c
rtk git commit -m "refactor(vendor-daemon): make push slot table audio-ready

Grows PUSH_STREAM_SLOT_COUNT to 3 and maps VD_STREAM_AUDIO to slot 2. Replaces
two hardcoded two-slot assumptions that the growth invalidates: the
first-activation ring reset used a 0/1 flip, and stop-all iterated slots 0 and 1
by literal index, which would have left an audio thread running.

No audio is produced yet; this only makes room for it."
```

---

## Task 2: Audio push thread in the daemon

The substance of the C work. Mirrors `push_frame_thread` but drives the audio SDK chain, which is shaped differently (see design doc).

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h` (after line 76)
- Modify: `cross-compile/vendor-daemon/src/push.c` (new thread + handlers)
- Modify: `cross-compile/vendor-daemon/src/push.h`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c`

**Step 1: Add the protocol commands**

In `protocol.h`, after `CMD_AENC_SET_ATTR = 56`:

```c
    /* Audio push-mode streaming.  Unlike VENC push, these carry no handles:
     * ak_aenc_request_stream() binds input to encoder and its stream handle must
     * live exactly as long as the push thread, so the daemon owns the whole
     * chain rather than marshalling three handles across IPC. */
    CMD_AUDIO_START_PUSH          = 57,
    CMD_AUDIO_STOP_PUSH           = 58,
```

**Step 2: Declare the handlers**

In `push.h`:

```c
int handle_audio_start_push(int fd, const uint8_t *req, uint32_t req_len);
int handle_audio_stop_push(int fd, const uint8_t *req, uint32_t req_len);
```

**Step 3: Add SDK headers and audio state to push.c**

At the top of `push.c`, alongside `#include "ak_venc.h"`:

```c
#include "ak_ai.h"
#include "ak_aenc.h"
```

Add file-static state for the SDK handles. They are file-static rather than in
`push_stream_state` because that struct is shared with the video path and none of
these fields mean anything there:

```c
/* Audio SDK chain, owned start-to-finish by the audio push thread.  All three
 * are opened in handle_audio_start_push() and torn down in stop_audio_chain(). */
static void *g_ai_handle     = NULL;
static void *g_aenc_handle   = NULL;
static void *g_astream_handle = NULL;
```

**Step 4: Write the audio push thread**

Add to `push.c`, after `push_frame_thread`:

```c
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
    uint32_t seq_no = 0;

    log_info("event=audio_push_lifecycle state=start thread_id=%lu diag_monotonic_ms=%llu",
             (unsigned long)pthread_self(),
             (unsigned long long)diag_monotonic_ms());

    while (state->active && !g_shutdown) {
        struct list_head stream_head;
        INIT_LIST_HEAD(&stream_head);

        if (ak_aenc_get_stream(g_astream_handle, &stream_head) != 0) {
            struct timespec ts = { .tv_sec = 0, .tv_nsec = PUSH_POLL_SLEEP_MS * 1000000L };
            nanosleep(&ts, NULL);
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
    }

    log_info("event=audio_push_lifecycle state=exit frames_pushed=%llu diag_monotonic_ms=%llu",
             (unsigned long long)frames_pushed,
             (unsigned long long)diag_monotonic_ms());
    return NULL;
}
```

**Note on `list_for_each_entry_safe`:** confirm the exact macro name available in the SDK's list header before writing this — check `ak_global.h` / the list header included by `ak_aenc.h`. If only `list_for_each_safe` exists, use it with `list_entry()` to recover the `aenc_entry`. Do not guess; a wrong macro here compiles into a loop that walks freed memory.

**Step 5: Write the start handler**

```c
/**
 * handle_audio_start_push - IPC handler for CMD_AUDIO_START_PUSH.
 *
 * Opens the full audio SDK chain and spawns the audio push thread.
 *
 * Wire format: [u32 sample_rate][u32 channel_num][u32 frame_interval_ms] = 12 bytes.
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
```

**Step 6: Write the stop handler**

```c
/**
 * handle_audio_stop_push - IPC handler for CMD_AUDIO_STOP_PUSH.
 *
 * Stops the audio push thread and tears the SDK chain down in reverse order.
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
```

**Step 7: Register in the dispatcher**

In `dispatcher.c`, alongside the `CMD_VENC_START_PUSH` / `CMD_VENC_STOP_PUSH` cases, following whatever pattern that file already uses (table entry or switch case — match it, don't invent a new one):

```c
    case CMD_AUDIO_START_PUSH: return handle_audio_start_push(fd, req, req_len);
    case CMD_AUDIO_STOP_PUSH:  return handle_audio_stop_push(fd, req, req_len);
```

**Step 8: Build**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon && make clean && make
```

Expected: clean build. No Makefile change is needed — `-lplat_ai`, `-lmpi_aenc`, `-lakaudiocodec` and `-lakaudiofilter` are already in LDFLAGS.

Re-run the Task 1 test to confirm the new includes didn't break the host build:

```bash
gcc -std=gnu99 -I include -I src -I ../anyka_reference/venc_demo/include \
    -o /tmp/test_push_slots tests/test_push_slots.c && /tmp/test_push_slots
```

You will likely need to add stubs for the new `ak_ai_*` / `ak_aenc_*` symbols. Add them to the stub block; do not `#ifdef` them out of `push.c`.

**Step 9: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/
rtk git commit -m "feat(vendor-daemon): audio push thread producing AAC frames

Adds CMD_AUDIO_START_PUSH/STOP_PUSH and a third push thread that owns the whole
audio SDK chain: ak_ai_open, filters off, mic source, frame interval,
start_capture, ak_aenc_open as AAC, CUT_FRAME_HEAD for raw frames, then
ak_aenc_request_stream to bind input to encoder. The thread polls
ak_aenc_get_stream, which returns a list rather than a single frame, and writes
each entry to the existing ring as VD_STREAM_AUDIO.

Audio is anchored to the main video stream's timestamp origin so the two clocks
share a zero; an independent anchor would offset lip-sync by however long audio
started after video. Frames are written as P so the ring sheds audio before
video keyframes under pressure."
```

---

## Task 3: AudioSpecificConfig computation

The one piece of real bit-manipulation in this feature, so it gets a real test. Pure function, no hardware, no IPC.

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/helpers.rs`
- Test: same file, `#[cfg(test)] mod tests`

**Step 1: Write the failing test**

Add to the existing `mod tests` in `helpers.rs`:

```rust
#[test]
fn test_aac_audio_specific_config_8khz_mono() {
    // AAC-LC (objectType 2), 8000 Hz (freq index 11), 1 channel.
    // Bit layout: 00010 1011 0001 000 -> 0x15 0x88
    assert_eq!(aac_audio_specific_config(8000, 1), vec![0x15, 0x88]);
}

#[test]
fn test_aac_audio_specific_config_16khz_mono() {
    // AAC-LC, 16000 Hz (freq index 8), 1 channel -> 0x14 0x08
    assert_eq!(aac_audio_specific_config(16000, 1), vec![0x14, 0x08]);
}

#[test]
fn test_aac_audio_specific_config_roundtrips_through_sdp_helpers() {
    // The ASC must survive the helpers that actually consume it, or the SDP
    // advertises a config no client can parse.
    let asc = aac_audio_specific_config(8000, 1);
    assert_eq!(audio_channels_from_config(&asc), 1);
    assert_eq!(audio_config_hex(&asc), "1588");
}

#[test]
fn test_aac_audio_specific_config_unsupported_rate_falls_back_to_8khz() {
    // An unmapped rate must not silently emit a wrong freq index; falling back
    // to 8 kHz keeps the stream decodable and the mismatch audible rather than
    // producing an ASC that decoders reject outright.
    assert_eq!(aac_audio_specific_config(12345, 1), vec![0x15, 0x88]);
}
```

**Step 2: Run to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib aac_audio_specific_config
```

Expected: FAIL, `cannot find function 'aac_audio_specific_config'`.

**Step 3: Implement**

Add to `helpers.rs`, near `audio_channels_from_config`:

```rust
/// MPEG-4 sampling-frequency index table (ISO/IEC 14496-3 Table 1.18).
const AAC_SAMPLE_RATES: [u32; 13] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
];

/// Build the 2-byte AAC-LC AudioSpecificConfig for a sample rate and channel count.
///
/// These two bytes are consumed in three places — the SDP `config=` fmtp, the
/// FLV AudioSpecificConfig sequence header, and RTP AAC-hbr sizing — so they are
/// computed once here rather than at each call site.
///
/// Layout: 5 bits object type (2 = AAC-LC), 4 bits sampling frequency index,
/// 4 bits channel configuration, 3 bits zero padding.
pub fn aac_audio_specific_config(sample_rate: u32, channels: u32) -> Vec<u8> {
    const AAC_LC_OBJECT_TYPE: u8 = 2;
    const FALLBACK_INDEX: u8 = 11; // 8000 Hz

    let freq_index = AAC_SAMPLE_RATES
        .iter()
        .position(|&r| r == sample_rate)
        .map(|i| i as u8)
        .unwrap_or_else(|| {
            tracing::warn!(
                sample_rate,
                "Unsupported AAC sample rate; falling back to 8000 Hz in AudioSpecificConfig"
            );
            FALLBACK_INDEX
        });

    let channel_config = (channels & 0x0F) as u8;

    vec![
        (AAC_LC_OBJECT_TYPE << 3) | (freq_index >> 1),
        ((freq_index & 0x01) << 7) | (channel_config << 3),
    ]
}
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib aac_audio_specific_config
```

Expected: 4 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/streaming/helpers.rs
rtk git commit -m "feat(streaming): compute AAC AudioSpecificConfig

Two bytes consumed by the SDP config= fmtp, the FLV sequence header and RTP
AAC-hbr sizing, so they are built once rather than at each call site. Unmapped
sample rates warn and fall back to 8 kHz instead of emitting a freq index no
decoder accepts."
```

---

## Task 4: Bridge setter for `audio_config`

Five lines. This is the switch that turns the audio track on across every protocol at once.

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/bridge.rs` (near line 406)

**Step 1: Write the failing test**

In `bridge.rs`'s test module:

```rust
#[test]
fn test_set_audio_config_makes_sdp_advertise_audio_track() {
    // audio_config is the single gate on the audio m-line: service.rs passes it
    // straight to generate_av_sdp, which omits the track when it is None.
    let sps = vec![0x67, 0x42, 0x00, 0x1E];
    let pps = vec![0x68, 0xCE, 0x38, 0x80];

    let without = generate_av_sdp(&sps, &pps, None, 8000, None);
    assert!(!without.contains("m=audio"));

    let asc = aac_audio_specific_config(8000, 1);
    let with = generate_av_sdp(&sps, &pps, Some(&asc), 8000, None);
    assert!(with.contains("m=audio"));
    assert!(with.contains("config=1588"));
}
```

Place this in `helpers.rs` tests if `generate_av_sdp` is more naturally reachable there — the point is asserting the gate, not the file.

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib set_audio_config
```

Expected: FAIL on the missing `aac_audio_specific_config` import or the missing setter, depending on where you placed it.

**Step 3: Implement the setter**

In `bridge.rs`, on the same impl block that has `cached_params`:

```rust
    /// Publish the AAC AudioSpecificConfig, enabling the audio track.
    ///
    /// This is the single gate on audio: while it is `None`, `generate_av_sdp`
    /// omits the `m=audio` line and the FLV path emits no audio sequence header,
    /// so no client negotiates audio even if frames are arriving.
    pub fn set_audio_config(&self, config: Vec<u8>) {
        tracing::info!(
            config_len = config.len(),
            sample_rate = self.audio_sample_rate,
            "Publishing audio config; audio track is now advertised"
        );
        *self.audio_config.write() = Some(config);
    }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: all pass.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/streaming/bridge.rs \
            cross-compile/onvif-rust/src/streaming/helpers.rs
rtk git commit -m "feat(streaming): add bridge.set_audio_config

audio_config was constructed as None with no setter, so generate_av_sdp always
omitted the m=audio line. This is the single gate that enables the audio track
across SDP and FLV simultaneously."
```

---

## Task 5: IPC client methods

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs`

**Step 1: Write the failing test**

Follow the existing pattern at `mod.rs:2819` (`test_start_push_encodes_stream_handle_and_stream_id`) — it captures sent requests via the existing test transport. Read that test before writing this one.

```rust
#[test]
fn test_start_audio_push_encodes_rate_channels_and_interval() {
    let (ipc, captured) = /* same harness as test_start_push_encodes_... */;

    ipc.start_audio_push(8000, 1).unwrap();

    assert_eq!(captured[0].0, CMD_AUDIO_START_PUSH);
    let req = &captured[0].1;
    assert_eq!(u32::from_le_bytes(req[0..4].try_into().unwrap()), 8000);
    assert_eq!(u32::from_le_bytes(req[4..8].try_into().unwrap()), 1);
    // AAC is 1024 samples/frame: 1024 * 1000 / 8000 = 128 ms
    assert_eq!(u32::from_le_bytes(req[8..12].try_into().unwrap()), 128);
}

#[test]
fn test_start_audio_push_derives_64ms_interval_at_16khz() {
    let (ipc, captured) = /* ... */;
    ipc.start_audio_push(16000, 1).unwrap();
    assert_eq!(u32::from_le_bytes(captured[0].1[8..12].try_into().unwrap()), 64);
}

#[test]
fn test_start_audio_push_stale_epoch_is_hardware_unavailable() {
    // Mirror test_start_push_stale_epoch_is_hardware_unavailable at mod.rs:3190.
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib start_audio_push
```

Expected: FAIL, no method `start_audio_push`.

**Step 3: Implement**

Add the command constants next to `CMD_VENC_START_PUSH` at `mod.rs:262`:

```rust
const CMD_AUDIO_START_PUSH: i32 = 57;
const CMD_AUDIO_STOP_PUSH: i32 = 58;
```

Add them to the command-name match at `mod.rs:547`:

```rust
            CMD_AUDIO_START_PUSH => "AUDIO_START_PUSH",
            CMD_AUDIO_STOP_PUSH => "AUDIO_STOP_PUSH",
```

Add the methods next to `start_push`:

```rust
    /// Start push-based audio delivery from the daemon.
    ///
    /// Carries no handles: the daemon owns the whole `ak_ai_open` →
    /// `ak_aenc_request_stream` chain, because the bound stream handle must live
    /// exactly as long as the push thread.
    pub fn start_audio_push(&self, sample_rate: u32, channels: u32) -> PlatformResult<()> {
        // AAC-LC emits one frame per 1024 samples, so the ADC frame interval is
        // determined by the codec, not chosen: 128 ms at 8 kHz, 64 ms at 16 kHz.
        let interval_ms = 1024 * 1000 / sample_rate.max(1);

        tracing::info!(
            event = "audio_push_start_request",
            diag_monotonic_ms = monotonic_millis(),
            sample_rate,
            channels,
            interval_ms,
            "requesting daemon audio push start"
        );

        let mut req = [0u8; 12];
        req[0..4].copy_from_slice(&sample_rate.to_le_bytes());
        req[4..8].copy_from_slice(&channels.to_le_bytes());
        req[8..12].copy_from_slice(&interval_ms.to_le_bytes());

        let (status, _) = self.send_request(CMD_AUDIO_START_PUSH, &req)?;
        if status == VD_STATUS_STALE_EPOCH {
            return Err(Self::stale_epoch_error(CMD_AUDIO_START_PUSH));
        }
        if status != AK_SUCCESS_I32 {
            warn!(
                event = "audio_push_start_failed",
                diag_monotonic_ms = monotonic_millis(),
                status,
                "daemon rejected audio push start"
            );
            return Err(PlatformError::HardwareFailure(
                "start_audio_push failed".into(),
            ));
        }
        Ok(())
    }

    /// Stop push-based audio delivery.
    pub fn stop_audio_push(&self) -> PlatformResult<()> {
        let (status, _) = self.send_request(CMD_AUDIO_STOP_PUSH, &[])?;
        if status == VD_STATUS_STALE_EPOCH {
            return Err(Self::stale_epoch_error(CMD_AUDIO_STOP_PUSH));
        }
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure(
                "stop_audio_push failed".into(),
            ));
        }
        Ok(())
    }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib start_audio_push
```

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
rtk git commit -m "feat(hal): add start_audio_push/stop_audio_push IPC methods

The ADC frame interval is derived from the codec rather than configured: AAC-LC
is 1024 samples per frame, giving 128 ms at 8 kHz and 64 ms at 16 kHz."
```

---

## Task 6: Wire up the platform layer

Where the feature becomes real. `AnykaAudioEncoder` stops being a config-holder.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/audio_encoder.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/video_encoder.rs` (near line 817)
- Modify: `cross-compile/onvif-rust/src/config/types.rs:967,990`

**Step 1: Flip the config default to AAC**

`config/types.rs` currently defaults `audio_encoding: "G711"` on both profiles, which makes ONVIF misreport. Change both to `"AAC"`.

Update the assertions in the existing config tests (`types.rs:1194` and nearby) that cover these defaults.

**Step 2: Write the failing test**

In `audio_encoder.rs`:

```rust
#[tokio::test]
async fn test_start_publishes_audio_config_and_requests_push() {
    // Audio must not be advertised until the daemon has accepted the push
    // request: an SDP that promises a track the camera never sends leaves
    // clients waiting on RTP that never arrives.
    let mut mock = MockAudioHalTrait::new();
    // ... expectations per the existing mockall pattern in this file
    let encoder = AnykaAudioEncoder::with_ffi(Arc::new(mock));

    let bridge = /* test bridge */;
    encoder.start(&bridge, 8000, 1).await.unwrap();

    assert_eq!(
        bridge.audio_config.read().as_deref(),
        Some([0x15, 0x88].as_slice())
    );
}

#[tokio::test]
async fn test_start_leaves_audio_config_none_when_daemon_rejects() {
    // Audio is strictly additive; a failed mic must never take video down and
    // must never advertise a track.
    // ... mock start_audio_push to fail
    assert!(bridge.audio_config.read().is_none());
}
```

**Step 3: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib audio_encoder
```

**Step 4: Implement `start`**

```rust
    /// Start microphone capture and advertise the audio track.
    ///
    /// Ordering matters: the ASC is published only after the daemon accepts the
    /// push request, so the SDP never promises a track the camera is not
    /// actually sending.
    pub(super) async fn start(
        &self,
        bridge: &Arc<StreamBridge>,
        sample_rate: u32,
        channels: u32,
    ) -> PlatformResult<()> {
        let ipc = crate::hal::anyka::ipc::AnykaIpc::new()?;
        ipc.start_audio_push(sample_rate, channels)?;

        bridge.set_audio_config(crate::streaming::helpers::aac_audio_specific_config(
            sample_rate,
            channels,
        ));

        tracing::info!(sample_rate, channels, "Audio capture started");
        Ok(())
    }
```

Adapt the `AnykaIpc` acquisition to however the surrounding code obtains it — reuse the existing handle rather than opening a second connection if one is already available on `self`.

**Step 5: Call it from startup**

In `video_encoder.rs`, after the video push calls succeed (~line 830):

```rust
    // Audio is strictly additive: a failure here must never stop video.
    if audio_enabled {
        if let Err(e) = audio_encoder.start(&bridge, audio_sample_rate, 1).await {
            tracing::error!(error = %e, "Audio capture failed to start; continuing video-only");
        }
    }
```

Thread `audio_enabled` and `audio_sample_rate` from `stream_profile_1` through to this call site.

**Step 6: Verify**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

All three must pass. Then confirm the ARM build, which CI does *not* cover on PRs:

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust && $CARGO build --release
```

**Step 7: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/
rtk git commit -m "feat(platform): start audio capture and advertise the track

AnykaAudioEncoder stops being a config-holder that reported an encoder the
hardware never ran. The ASC is published only after the daemon accepts the push
request, so the SDP never promises a track the camera is not sending, and a
failed mic logs and continues video-only.

Also flips the audio_encoding default from G711 to AAC so ONVIF stops
misreporting the codec."
```

---

## Task 7: Device validation

Nothing before this proves audio actually reaches a client. Do not skip it and do not report the feature working on the strength of Tasks 1–6.

**Step 1: Deploy**

Use the `anyka-firmware-upgrade` skill to build and upload a bundle to `192.168.2.198`.

**Step 2: Confirm the daemon started the audio chain**

```bash
cd /home/kmk/dev/anyka-dev
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 \
  "grep -E 'audio_push_lifecycle|\[audio\]' /mnt/logs/*.log | tail -20"
```

Expected: `event=audio_push_lifecycle state=start` and `[audio] push started rate=8000 ch=1 interval=128ms`.

If `ak_ai_open` failed, you get `[audio] ak_ai_open failed` and video keeps working — that is the designed behavior, not a pass.

**Step 3: Confirm the track is advertised and carries data**

```bash
ffprobe -v error -show_entries stream=index,codec_name,codec_type \
  -of csv=p=0 rtsp://192.168.2.198/main
```

Expected: two rows, one `h264,video` and one `aac,audio`.

**Step 4: Confirm the audio is not silent**

A track that exists but carries silence is the failure mode a track-exists check misses — it is exactly what a wrong `AENC_AAC_CUT_FRAME_HEAD` setting or a dead mic looks like.

```bash
ffmpeg -v error -rtsp_transport tcp -i rtsp://192.168.2.198/main \
  -t 10 -vn -af volumedetect -f null - 2>&1 | grep mean_volume
```

Expected: a `mean_volume` well above `-91 dB` (which is digital silence). Room tone should land somewhere around -40 to -60 dB.

**Step 5: Check A/V sync — the flagged main risk**

```bash
ffprobe -v error -show_entries packet=stream_index,pts_time \
  -select_streams a -read_intervals %+5 -of csv=p=0 rtsp://192.168.2.198/main | head
ffprobe -v error -show_entries packet=stream_index,pts_time \
  -select_streams v -read_intervals %+5 -of csv=p=0 rtsp://192.168.2.198/main | head
```

Audio and video `pts_time` should start near the same value. A large constant offset means the timestamp anchoring in Task 2 is wrong — revisit `g_push_streams[0].first_timestamp_ms` before doing anything else.

**Step 6: Confirm HTTP-FLV and the browser preview**

Load the WebUI live preview and confirm audio plays. This is the MSE path that motivated choosing AAC over G.711, so it is the payoff check.

**Step 7: Confirm video did not regress**

```bash
cd /home/kmk/dev/anyka-dev/validation
# run the existing RTSP validation harness and compare against baseline
```

Use the `anyka-validation` skill. Ring pressure went up ~20 % with audio sharing the slots; this is where that would show as video frame drops.

**Step 8: Commit any fixes, then request review**

Use the `superpowers:requesting-code-review` skill before merging.

---

## Notes for the implementer

**Do not trust these code blocks blindly.** They are written against the code as it reads today, but two things need checking on contact:
1. The exact list-walking macro available for `aenc_entry` (Task 2, Step 4). Guessing here walks freed memory.
2. How `dispatcher.c` registers handlers — table vs switch. Match the existing pattern.

**Known-unverified assumption:** `ak_ai_set_frame_interval(128)` is outside the documented [10, 125] ms range. `aenc_demo` does it and produced valid AAC in the measurements behind this design, so it is expected to work — but if audio comes out corrupted or the interval is silently clamped, switching `stream_profile_1.audio_sample_rate` to `16000` yields a compliant 64 ms and better quality at negligible extra CPU. That is a config change, not a code change.

**Deployment consequence, stated once:** `stream_profile_1.audio_enabled` defaults to `true`, so deploying this starts capturing room audio on every camera it reaches.

---

## Implementation status (2026-08-24)

All 7 tasks complete and validated on 192.168.2.198; branch is `feat/live-audio`.

**Device validation results:**
- Daemon starts the audio chain: `[audio] push started rate=8000 ch=1 interval=128ms`.
- RTSP negotiates two tracks: `h264,video` + `aac,audio`; HTTP-FLV carries audio too.
- Non-silent: `n_samples 65536`, `mean_volume -30.9 dB` (8 kHz, 8 s).
- A/V synced: both tracks start at PTS 0 in the SR-synced mux; audio advances 1024/frame, video 9000/frame.
- Video unregressed: 51 frames / 5 s decode, no stall.

**Fixes beyond the plan's code (found by device validation):**
1. The audio push loop busy-spun on `ak_aenc_get_stream` returning 0 with an empty list, starving the SDK's `read_pcm_thread` (capture stalled after one frame at both 8 and 16 kHz). Pacing every iteration fixed it. The 128 ms out-of-range interval was NOT the cause.
2. `handle_pushed_frame` dropped `StreamId::Audio` frames before the bridge — every audio frame was discarded.
3. Audio RTP timestamps were milliseconds treated as sample units (RFC 3640 wants samples) — audio ran ~16× slow.
4. RTP-Info advertised a random `init_timestamp` that never matched the first RTP packet (0-based), so strict clients stalled.
5. The RTCP SR always reported RTP timestamp 0 because the pull path never fed sent packets into the track's `RtcpContext`.

Each fix has a dedicated commit on the branch. The Task 6 call site deviates from the plan (app `start_streaming` after bridge registration, via the `AudioEncoder` trait) because the plan's proposed sync `venc-read` thread cannot host an async bridge-aware call.
