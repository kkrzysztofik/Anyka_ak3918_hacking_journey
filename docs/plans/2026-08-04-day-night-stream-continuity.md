# Day/Night Stream Continuity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep live RTSP viewers playing across ISP day/night by publishing continuous SHM timestamps from `push.c` and requesting IDR on every `night_mode::apply`.

**Architecture:** Offset-correct large forward `vs.ts` jumps in the daemon push path (sole clock fix for all consumers). Wire `Weak<AnykaVideoEncoder>` into `NightModeController` so forced and AUTO transitions both request main+sub IDR. No RTP clamp in this pass.

**Tech Stack:** vendor-daemon C (`push.c`), onvif-rust (`night_mode.rs`, `imaging.rs`), deploy to `192.168.2.198`.

**Design:** `docs/plans/2026-08-04-day-night-stream-continuity-design.md`

---

### Task 1: Forward-clamp state in `push_stream_state`

**Files:**
- Modify: `cross-compile/vendor-daemon/src/globals.h` (`struct push_stream_state`)
- Modify: `cross-compile/vendor-daemon/src/push.c` (normalize path + push-start reset)

**Step 1: Extend state**

In `struct push_stream_state`, after `timestamp_initialized`, add:

```c
uint32_t last_raw_ts_ms;
uint32_t last_out_ts_ms;
uint32_t last_sane_interval_ms; /* init 66; updated when delta in 16..1000 */
int64_t  ts_corr_ms;            /* added into normalized out; keeps continuity after clamps */
```

**Step 2: Reset on push start**

Where `timestamp_initialized = 0` today (~push.c:513), also zero `last_*`, set `last_sane_interval_ms = 66`, `ts_corr_ms = 0`.

**Step 3: Clamp after existing first-anchor / wrap**

After computing `timestamp_ms = raw - first` (with wrap), apply correction and clamp:

```c
#define TS_MAX_FORWARD_MS 250u
#define TS_SANE_MIN_MS 16u
#define TS_SANE_MAX_MS 1000u

/* after first-anchor / wrap into timestamp_ms: */
uint64_t out64 = (uint64_t)timestamp_ms + (uint64_t)state->ts_corr_ms;
/* handle corr negative carefully if using signed corr — prefer unsigned corr_boost only */

if (state->timestamp_initialized /* already past first frame */) {
    uint32_t last_out = state->last_out_ts_ms;
    uint32_t cand = /* timestamp_ms + corr, saturating to u32 */;
    uint32_t delta = cand - last_out; /* only if cand >= last_out; else existing wrap/regress */

    if (cand >= last_out && delta > TS_MAX_FORWARD_MS) {
        uint32_t step = state->last_sane_interval_ms;
        if (step < TS_SANE_MIN_MS || step > TS_MAX_FORWARD_MS)
            step = TS_MAX_FORWARD_MS;
        uint32_t clamped = last_out + step;
        state->ts_corr_ms += (int64_t)clamped - (int64_t)cand;
        timestamp_ms = clamped;
        log_warn("event=timestamp_forward_clamp stream=%u raw=%u cand=%u out=%u step=%u",
                 state->stream_id, raw_timestamp_ms, cand, clamped, step);
    } else {
        timestamp_ms = cand;
        if (cand >= last_out) {
            uint32_t d = cand - last_out;
            if (d >= TS_SANE_MIN_MS && d <= TS_SANE_MAX_MS)
                state->last_sane_interval_ms = d;
        }
    }
}
state->last_raw_ts_ms = raw_timestamp_ms;
state->last_out_ts_ms = timestamp_ms;
```

Refine first-frame path so the first published frame still starts at 0 and initializes `last_out`.

`# ponytail: 250ms forward cap; lower if VLC still hiccups after confirm log.`

**Step 4: Build**

```bash
source ./setenv.sh && make -C cross-compile/vendor-daemon release
```

Expected: clean build.

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/globals.h cross-compile/vendor-daemon/src/push.c
git commit -m "fix(vendor-daemon): clamp forward capture-ts jumps in push"
```

---

### Task 2: IDR on every `night_mode::apply`

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs`
- Test: extend existing `night_mode` unit tests in the same file / module tests

**Step 1: Failing test**

Add a test that `apply(DayNight::Night)` calls IDR when an encoder weak is set.
Reuse existing mock FFI patterns; use a test double or count `request_idr_frame` via a thin callback if `AnykaVideoEncoder` is heavy to mock:

Prefer: optional `idr_hook: Option<Arc<dyn Fn() + Send + Sync>>` **only if** Weak encoder is awkward in unit tests — ponytail prefers Weak + one integration-style test with mockall on a trait if already present.

Simplest ponytail path: store `Option<Weak<AnykaVideoEncoder>>`, set from imaging; unit-test with `video_encoder: None` still succeeds; add test that documents hook is invoked by extracting:

```rust
fn request_idr_best_effort(enc: &Option<Weak<AnykaVideoEncoder>>) {
    if let Some(e) = enc.as_ref().and_then(Weak::upgrade) {
        let _ = e.request_idr_frame(true);
        let _ = e.request_idr_frame(false);
    }
}
```

Call at end of `apply` after ISP (even if isp != 0 — still want keyframe for GPIO change).

**Step 2: Wire from imaging**

In `AnykaImagingControl::with_ffi_and_video_encoder`, after setting `control.video_encoder`, also `control.night.set_video_encoder(Arc::downgrade(&video_encoder))`.

Add `NightModeController::set_video_encoder(&self, Weak<AnykaVideoEncoder>)` (Mutex/OnceLock around Option).

**Step 3: Host tests**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu night_mode --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

**Step 4: Commit**

```bash
git commit -m "fix(night_mode): request IDR on every day/night apply"
```

---

### Task 3: Deploy and verify on `.198`

**Files:** none (binaries only; do not commit SD card bins)

**Step 1: Build + install to SD tree**

```bash
source ./setenv.sh
make -C cross-compile/vendor-daemon release install
./cross-compile/onvif-rust/scripts/build.sh --release
```

**Step 2: Transfer + restart both services** (nc + `camera_shell.py` as prior deploy)

Confirm new PIDs; leave `ir_cut_filter` as needed for test.

**Step 3: Live VLC check**

1. Open VLC on `rtsp://192.168.2.198:554/stream` (admin/admin); leave playing.
2. `SetImagingSettings` IrCutFilter=OFF then ON on `VideoSource_1`.
3. Pass: no VLC timestamp conversion spam; session stays up; `event=timestamp_forward_clamp` may appear once per switch in `vendor_daemon.log`.
4. Confirm IDR path: no hard requirement to decode bitstream; optional brief clean picture after switch.

**Step 4: Handoff note** — no binary commit.

---

### Task 4: Ponytail-review

Diff vs design. Cut RTP clamp, extra abstractions, FPS hacks. Commit shrinks if any.

---

## Execution handoff

After plan is saved, use **executing-plans** (or task-by-task). Do not add RTP clamp unless Task 3 fails with continuous push timestamps.
