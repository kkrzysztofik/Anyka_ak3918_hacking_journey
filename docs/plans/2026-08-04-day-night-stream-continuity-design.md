# Day/Night Stream Continuity — Design

Date: 2026-08-04
Status: approved; implementation plan at `docs/plans/2026-08-04-day-night-stream-continuity.md`
Branch context: `feat/ir-led-support` (live RTSP must survive ISP day/night)

## Problem

Forcing or AUTO day↔night via `ak_vi_switch_mode` leaves the encoder running, but
VLC open on an existing RTSP session fails with `Timestamp conversion failed`
(bound ~9s). Root cause is a **large forward jump** in published capture
timestamps, not a dead pipeline and not the night FPS drop (~15→~10).

Pipeline today:

```
vs.ts → push.c (raw - first only) → SHM timestamp_ms
     → bridge pass-through → RtpTimestampNormalizer (backward-only)
     → RTP → VLC
```

`FramePacer` caps sleep at 200ms but still emits the jumped source timestamp.
`night_mode::apply` does not request IDR (ONVIF `set_settings` does).

## Decisions

| # | Choice |
|---|---|
| D1 | Success = live RTSP stays up + IDR on every mode switch (forced + AUTO) + continuous ts for all SHM consumers |
| D2 | Timestamp continuity once in **`push.c`** — not RTP-only, not dual-layer |
| D3 | **Offset / forward-clamp** on published `timestamp_ms` (Approach A) |
| D4 | Night FPS drop is **expected**; no FPS “fix” |
| D5 | IDR from `NightModeController::apply` via existing `Weak<AnykaVideoEncoder>` pattern |

Ponytail cuts: no RTP forward clamp (unless push deploy still fails VLC); no
bridge clock; no stream restart on mode switch; no C unit harness; no wall-clock
replacement of `vs.ts`.

## Architecture

```
ak_vi_switch_mode / ISP stall
       │
       ▼
push.c  — after first-anchor + wrap:
          if forward delta > 5000ms → publish last_out + bounded_step
          bounded_step = clamp(last_sane_interval, 16..=250)
          keep correction so later frames stay continuous
       │
       ▼
SHM timestamp_ms (continuous for RTSP, HTTP-FLV, all consumers)
       │
night_mode::apply
  GPIO → ISP → request_idr main+sub (best-effort)
```

`# ponytail: 5s forward cap (TS_MAX_FORWARD_MS); lower if VLC still hiccups after confirm log.`

## Components

| Piece | Change |
|---|---|
| `vendor-daemon/src/globals.h` + `push.c` | State fields + forward-clamp; `event=timestamp_forward_clamp` warn |
| `platform/anyka/night_mode.rs` | Optional `Weak` encoder; IDR after apply |
| `platform/anyka/imaging.rs` | Wire encoder weak into night controller when bound |

## Error handling

| Case | Behavior |
|---|---|
| Forward jump > 5000ms (`TS_MAX_FORWARD_MS`) | Publish `last_out + clamp(last_sane_interval, 16..=250)`; rate-limited warn |
| Encoder missing / IDR fail | Best-effort; do not fail `apply` |
| u32 wrap | Existing push wrap path unchanged |

## Testing

**Host:** `make -C cross-compile/vendor-daemon release`; Rust night_mode IDR test.

**On `.198`:** VLC playing → IrCut OFF/ON; no timestamp spam / no reconnect;
optional clamp warn in `vendor_daemon.log`.

## Out of scope

- RTP-layer forward clamp (follow-up only if needed)
- Sensor FPS forcing
- Restarting push/venc on mode switch
- C daemon unit framework
