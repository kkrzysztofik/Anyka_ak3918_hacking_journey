# RTP Analysis Module Split — Design

Date: 2026-08-06
Status: approved, not yet implemented
Target: `validation/rust` (rtsp-validation-tool)

## Problem

`src/rtsp.rs` is 4,317 lines (2,926 code + 1,391 tests). It holds three unrelated
subsystems: a Retina-based live RTSP probe, an external-tool harness driving
ffmpeg/ffprobe/tshark, and a pure pcap/RTP analysis layer. Navigating or reviewing
any one of them means scrolling past the other two.

The goal is navigability only. No behaviour change, no API for external reuse, no
new crate.

## Method

Every proposed boundary was measured before being accepted: cross-group reference
edges in the code, and how the 90 tests partition. Boundaries that turned out to be
meshes rather than layers were rejected, including one this design originally
proposed.

| boundary | edges | items needing wider visibility | shape | verdict |
|---|---:|---:|---|---|
| `rtp/` internals | 10 | 3 | layered DAG | split |
| `probe` ↔ `harness` | 0 | 0 | disjoint | split |
| `harness/` internals | 51 | 30 | cyclic mesh | **do not split** |

## Final structure

```
src/probe.rs        ~479 code    Retina live probe
src/harness.rs    ~1,676 code    was rtsp.rs; run_harness, scenarios,
                                 appenders, ffmpeg/tshark plumbing
src/rtp/mod.rs        ~40        re-exports only, no logic
src/rtp/rows.rs       186        tshark field-row parsing -> RtpTsharkRow
src/rtp/payload.rs    286        RFC 6184 (H.264) / RFC 3640 (AAC) validation
src/rtp/streams.rs    136        grouping by (pt, ssrc, port), primary
                                 selection, packet loss
src/rtp/pacing.rs     163        encoder + arrival cadence, gap percentiles
```

### `rtp/` internal dependencies

```
rows  <-- payload <-- streams
  ^
  +-------- pacing
```

`rows` depends on nothing. No cycles. Exactly three items widen to `pub(super)`:
`RtpTsharkRow`, `validate_h264_rtp_payload_rfc6184`, `validate_aac_rtp_payload_rfc3640`.

`rtp/mod.rs` re-exports as `pub(crate)` only what `harness.rs` calls:
`tshark_extract_rtp_rows`, `RtpTsharkRow`, `group_rtp_rows_by_stream`,
`pick_primary_video_stream`, `pick_primary_audio_stream`, `compute_stream_loss_metric`,
`analyze_h264_rfc6184_from_rows`, `analyze_aac_rfc3640_from_rows`, `compute_pacing`,
`HarnessRtpLossMetric`, `RtpPcapRfc6184Stats`, `RtpPcapRfc3640Stats`, `FramePacing`,
`GapStats`.

### Staying in `harness.rs`

Three items sit inside the moved line range but do not belong to the RTP layer:

- `HarnessPacketLossResult` — harness aggregate that *holds* the moved types.
- `stream_info_from_retina` — Retina, moves to `probe.rs`.
- `validate_h264_length_prefixed_nals` — used by the Retina live path, not pcap,
  despite the H.264 name. Moves to `probe.rs`.

Also staying: `parse_status_code`, `tshark_rtsp_sequence_stats`, `TsharkCapture`,
`drain_ffmpeg` — RTSP-sequence parsing and capture orchestration, out of scope.

`HarnessRtpLossMetric` keeps its name after moving into `rtp::streams`. It is
serialised into the report JSON under that shape; renaming would be a behaviour
change for no gain.

## Rejected: splitting `harness/`

An earlier draft proposed `harness/{mod,scenarios,results,tools}.rs`. Measurement
rejected it: 51 cross-group edges, 30 items needing `pub(super)`, and cycles in both
directions (`results -> scenarios` 7, `scenarios -> results` 1; `thresh -> tools`,
`results -> thresh`). The groups were named before the graph was measured, and the
names implied a layering the code does not have.

Consequence: `harness.rs` remains ~2,200 lines including tests. That is the price of
not inventing seams. The trigger to revisit is the coupling count falling, not the
line count rising.

## Incidental fix

`run_validation` re-inlines `format!("rtsp://{}:{}{}", ...)` instead of calling
`rtsp_url` — a fourth copy of that string. Folded into the `probe.rs` extraction.

## Sequencing

Two commits, each on a measured seam and gated by the full test suite:

1. Extract `rtp/` — pure move plus three visibility widenings.
2. Extract `probe.rs`, rename `rtsp.rs` -> `harness.rs`, fix the inlined URL.

## Verification

Behaviour must not change. Per step:

- `cargo test` — 226 tests, all must pass unchanged. Tests move with their code;
  none are rewritten. The 42 `rtp/` tests partition 12/19/1/10 across the four
  submodules with zero cross-group dependencies.
- `cargo clippy --all-targets` — must stay clean (it is clean as of `aeee6416`).
- `cargo fmt`.
- The report JSON shape is unchanged: no serialised type is renamed and no
  `serde` attribute is touched.

## Known coverage gap surfaced

`rtp/streams.rs` will hold 10 items and 1 test. Invisible in a 4,317-line file,
glaring in a 136-line module. Worth a follow-up; not a blocker for this move.
