# Frame Pacing Validation — Design

Date: 2026-08-06
Status: implemented
Target: `validation/rust` (rtsp-validation-tool)

## Problem

The RTSP validation tool measures startup latency, average bitrate/fps, and packet
loss — but **not frame pacing**. A camera that bursts frames then stalls passes all
existing checks: average FPS stays "fine", loss stays ~0, while the stream is
visibly choppy.

Two distinct cadences matter:

- **Encoder cadence (A)** — the sender's frame generation regularity, read from
  RTP timestamp deltas between consecutive frames (90 kHz media clock). Catches
  encoder frame drops/duplicates, variable GOP bursts, sensor stalls.
- **Arrival cadence (B)** — when frames actually arrive at the client (wall
  clock). Catches network jitter and sender burst-then-stall. Requires a
  wall-clock per packet, which the current tshark extraction does **not** capture.

## Goals

1. Add a `frame_pacing` measurement to the validation report covering both
   cadences, with pass/fail verdict metrics usable in CI and baseline comparison.
2. Keep it pcap-based (approach approved over live in-process measurement):
   one code path, works for all launch modes, works retroactively on saved pcaps.
3. Follow the project's ponytail discipline: no new abstractions, no histogram,
   no RFC 3550 jitter metric, single config source.

## Decisions

| # | Choice |
|---|---|
| D1 | Approach: pcap-based, post-hoc analysis in `rtsp.rs` |
| D2 | Delay rule: gap is a delay when `gap_ms ≥ max(nominal_ms × delay-multiple, delay-floor-ms)` where `nominal_ms = 1000 / expected-fps` |
| D3 | Nominal interval comes from config `[pacing] expected-fps` only — no SDP fallback, no coupling to `[thresholds.expected] fps` |
| D4 | Scope: primary video (H.264) stream only; audio skipped (existing `timestamp_anomalies` covers AAC structural sanity) |
| D5 | Output: verdict metrics (delay-percent) + diagnostics (min/median/p90/p99/max gaps); both cadences |
| D6 | Frame assembly minimized: single-pass delta computation over existing `RtpStreamStats.rows`; **no `Vec<Frame>` intermediate struct** |
| D7 | Arrival measurement requires adding `frame.time_epoch` to tshark extraction (optional field; legacy pcaps degrade to A-only) |
| D8 | Baseline: track delay-percent + max-gap for both cadences (direction: lower is better) |
| D9 | No CLI flags for pacing; config-only tuning |

Ponytail cuts applied: no `Frame` struct, no packet-count/byte-size diagnostics,
no histogram buckets, no RFC 3550 jitter, no fallback chain for nominal interval,
no CLI overrides.

## Architecture

```text
tshark_extract_rtp_rows()  +--- frame.time_epoch field added
  -> RtpTsharkRow { ..., time_epoch_sec: Option<f64> }   (optional; legacy ok)

pick_primary_video_stream() -> RtpStreamStats (existing)

compute_pacing(rows: &[RtpTsharkRow], expected_fps, multiple, floor_ms)
  A: rows in pcap order (media time is monotonic in arrival order for a live
     stream, keeping 32-bit wrap-around well-defined); consecutive distinct
     rtp.timestamp -> gap_ms = wrapping_sub(ts2, ts1) / 90_000 * 1000
  B: rows in pcap order; when rtp.timestamp changes at row i, the previous
     frame's arrival epoch = row (i-1)'s time_epoch_sec (the packet that closed
     the frame); gap_ms = (epoch_i-1 - prev_frame_epoch) * 1000
     (B skipped if any time_epoch_sec is None)
  both: delay event if gap_ms >= max(nominal_ms * multiple, floor_ms)
  -> FramePacing { encoder: GapStats, arrival: Option<GapStats> }
     GapStats { count, min_ms, median_ms, p90_ms, p99_ms, max_ms,
                delay_count, delay_percent }

Harness result path: run pacing when a pcap exists; emit
  frame_pacing_encoder_delay_percent   (verdict, fail if > delay-tolerance-percent)
  frame_pacing_arrival_delay_percent   (verdict, fail if > delay-tolerance-percent)
  frame_pacing stats object in report JSON
```

## Components

| Piece | Change |
|---|---|
| `rtsp.rs` `tshark_extract_rtp_rows` / `parse_tshark_rtp_row_line` | Add `frame.time_epoch` to field list + parse into `RtpTsharkRow.time_epoch_sec: Option<f64>` |
| `rtsp.rs` | New pure `compute_pacing(...)` + `GapStats`/`FramePacing` types + delay-rule helpers |
| `rtsp.rs` harness result path | Integrate pacing metrics + stats object (mirrors existing RFC 6184/3640 integration) |
| `config.rs` | New `[pacing]` section: `expected-fps` (default 25), `delay-multiple` (2.0), `delay-floor-ms` (150), `delay-tolerance-percent` (5) |
| `baseline.rs` | Track `frame_pacing_encoder_delay_percent`, `frame_pacing_arrival_delay_percent`, `frame_pacing_encoder_max_gap_ms`, `frame_pacing_arrival_max_gap_ms` (direction: lower) |
| `report.rs` | Serialize `frame_pacing` stats object (if any) |

## Testing plan

Unit tests (pure functions; no tshark/ffmpeg I/O):

| Case | Assert |
|---|---|
| A: distinct-timestamp extraction | FU-A multi-packet frames collapse to one boundary |
| A: 32-bit wrap-around | wrapping arithmetic correct |
| A: media-order sort | deltas from media order, not pcap order |
| B: pcap-order iteration | arrival deltas use wall-clock boundaries |
| B: missing epochs | `time_epoch_sec = None` -> B reported skipped, A still works |
| Delay rule: boundary | gap == floor counts as delay (>=) |
| Delay rule: fps scaling | 15fps: floor 150ms governs (2x = 133ms); 30fps: floor still governs (2x = 66.7ms); 5fps: multiple governs (2x = 400ms) |
| Verdict | delay_percent vs tolerance threshold |
| tshark parser | new field parses; legacy line without it -> `None` |
| Config | `[pacing]` parse + defaults when absent |

Integration: harness result path includes pacing when pcap exists; JSON report
serializes `frame_pacing`. Manual: one real-mode device run to eyeball numbers.

### Verification

```bash
source ./setenv.sh
cd validation/rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

## Error handling / non-goals

- Out of scope: audio pacing, histogram, RFC 3550 jitter, SDP-framerate fallback,
  live in-process measurement (retina probe loop), per-stream metrics in real mode.
- Legacy pcaps without `frame.time_epoch` degrade gracefully (B skipped, noted in report).
- If the pcap has no usable video rows, pacing produces no metrics (matches existing
  packet-loss behavior when no stream is found).

## Success criteria

- [x] `frame_pacing_encoder_delay_percent` / `frame_pacing_arrival_delay_percent` verdict metrics in report
- [x] `frame_pacing` stats object with gap diagnostics for both cadences
- [x] `[pacing]` config section honored; defaults sane when absent
- [x] Baseline comparison covers the four new values
- [x] Unit tests above pass; clippy/fmt clean
- [ ] Manual device run produces plausible numbers
