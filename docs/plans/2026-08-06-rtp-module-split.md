# RTP Analysis Module Split Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split `validation/rust/src/rtsp.rs` (4,317 lines) into a measured `rtp/` module plus `probe.rs`, leaving the harness as `harness.rs`, with zero behaviour change.

**Architecture:** Pure code-move refactor along boundaries that were measured, not guessed (see `docs/plans/2026-08-06-rtp-module-split-design.md`). `rtp/` is a four-file layered DAG (`rows` <- `payload` <- `streams`; `rows` <- `pacing`) needing exactly three `pub(super)` widenings. `probe.rs` has zero coupling to the harness. The harness is deliberately **not** split further — measurement showed 51 cross-group edges and cycles.

**Tech Stack:** Rust 2024 edition, vendored toolchain at `toolchain/arm-anykav200-crosstool-ng/bin/cargo`, host target `x86_64-unknown-linux-gnu` (set in `validation/rust/.cargo/config.toml`, so no `--target` flag needed).

---

## READ THIS FIRST — how this plan differs from a normal TDD plan

This is a **refactor, not a feature**. There is no new behaviour to test-drive.

- **All 226 tests already exist and must pass unchanged after every task.**
- **Never rewrite, reword, or "improve" a test while moving it.** A test that needed editing to pass is evidence the move changed behaviour — stop and investigate.
- The test count must stay exactly **226** at every commit. A drop means a test was lost in the move; a rise means one was duplicated.
- Work **by item name, never by line number.** Line numbers in the design doc were accurate when written and drift the moment the first item moves.

**Setup — run once before Task 1:**

```bash
cd /home/kmk/dev/anyka-dev/validation/rust
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
cargo test 2>&1 | tail -3
```

Expected: `test result: ok.` totalling **226 passed**. If not 226, stop — the baseline is wrong.

---

## Task 1: Create `rtp/rows.rs` (the dependency base)

`rows` depends on nothing else being moved, so it goes first.

**Files:**
- Create: `validation/rust/src/rtp/mod.rs`
- Create: `validation/rust/src/rtp/rows.rs`
- Modify: `validation/rust/src/lib.rs`
- Modify: `validation/rust/src/rtsp.rs`

**Step 1: Create `src/rtp/mod.rs`**

```rust
//! Pure pcap/RTP analysis: tshark row parsing, RFC payload validation,
//! stream grouping and packet loss, frame pacing.
//!
//! Nothing here touches `EffectiveConfig`, `TestResult`, ffmpeg or tokio —
//! it takes rows and numbers and returns statistics. Keep it that way.

pub(crate) mod rows;

pub(crate) use rows::{RtpTsharkRow, tshark_extract_rtp_rows};
```

**Step 2: Register the module in `src/lib.rs`**

Add `pub mod rtp;` to the existing `pub mod` list, keeping alphabetical order (after `report`, before `rtsp`).

**Step 3: Move these 7 items from `rtsp.rs` into `src/rtp/rows.rs`**

Cut each item **with its doc comments and `#[derive(...)]` attributes**:

`RtpTsharkRow`, `parse_tshark_hex_bytes`, `parse_tshark_u32`, `parse_tshark_u16`, `parse_tshark_f64`, `parse_tshark_rtp_row_line`, `tshark_extract_rtp_rows`

Prepend this exact header to `rows.rs`:

```rust
//! Parsing `tshark -T fields` output into RTP rows.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::util::tail_lossy;
```

Then widen visibility on the items `rtsp.rs` and the sibling modules still need:

```rust
pub(crate) struct RtpTsharkRow { ... }        // was: struct
pub(crate) fn tshark_extract_rtp_rows(...)    // was: fn
```

Leave `parse_tshark_hex_bytes`, `parse_tshark_u32`, `parse_tshark_u16`, `parse_tshark_f64`, `parse_tshark_rtp_row_line` **private** — they are only used inside `rows.rs` and by its own tests.

`RtpTsharkRow`'s fields must also become `pub(crate)` — sibling modules read them.

**Step 4: Fix `rtsp.rs`**

Add to the `use` block:

```rust
use crate::rtp::{RtpTsharkRow, tshark_extract_rtp_rows};
```

`rtsp.rs` still holds `payload`/`streams`/`pacing` items that reference `RtpTsharkRow`; this import satisfies them until Tasks 2–4 move them.

**Step 5: Move the 12 `rows` tests**

Create at the bottom of `rows.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ... the 12 moved test fns, verbatim ...
}
```

Move these, **unchanged**, out of `rtsp.rs`'s `mod tests`:

```
test_parse_tshark_hex_bytes_empty_ok
test_parse_tshark_hex_bytes_colon_separated
test_parse_tshark_hex_bytes_contiguous
test_parse_tshark_rtp_row_line_contiguous_payload_and_hex_ssrc
test_parse_tshark_u32_accepts_hex_and_decimal
test_parse_tshark_u32_invalid_and_empty_return_none
test_parse_tshark_rtp_row_line_insufficient_fields_returns_none
test_parse_tshark_rtp_row_line_invalid_numeric_fields_return_none
test_parse_tshark_rtp_row_line_invalid_payload_hex_returns_err
test_parse_tshark_rtp_row_line_empty_payload_returns_none
test_parse_tshark_rtp_row_line_with_epoch
test_parse_tshark_rtp_row_line_missing_epoch_is_none
```

Remove the now-orphaned names from `rtsp.rs`'s `mod tests` `use super::{...}` list.

**Step 6: Build and test**

```bash
cargo build 2>&1 | tail -20
```

Expected: clean. If you see `private type ... in public interface` or `field is private`, a `pub(crate)` from Step 3 was missed.

```bash
cargo test 2>&1 | tail -3
```

Expected: **226 passed**. Not 225, not 227.

**Step 7: Commit**

```bash
git add validation/rust/src/lib.rs validation/rust/src/rtp/ validation/rust/src/rtsp.rs
git commit -m "refactor(validation): extract rtp::rows from rtsp.rs"
```

---

## Task 2: Create `rtp/payload.rs`

**Files:**
- Create: `validation/rust/src/rtp/payload.rs`
- Modify: `validation/rust/src/rtp/mod.rs`, `validation/rust/src/rtsp.rs`

**Step 1: Move these 8 items into `src/rtp/payload.rs`**

`RtpPcapRfc6184Stats`, `RtpPcapRfc3640Stats`, `is_h264_vcl_nal_type`, `validate_h264_rtp_payload_rfc6184`, `validate_aac_rtp_payload_rfc3640`, `pick_best_payload_type`, `analyze_h264_rfc6184_from_rows`, `analyze_aac_rfc3640_from_rows`

Header:

```rust
//! RTP payload conformance: RFC 6184 (H.264) and RFC 3640 (AAC).

use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::HashMap;

use super::rows::RtpTsharkRow;
```

Visibility: `RtpPcapRfc6184Stats`, `RtpPcapRfc3640Stats`, `analyze_h264_rfc6184_from_rows`, `analyze_aac_rfc3640_from_rows` become `pub(crate)` (used by `rtsp.rs`). `validate_h264_rtp_payload_rfc6184` and `validate_aac_rtp_payload_rfc3640` become `pub(super)` (used by `streams.rs` in Task 3). The stats structs' fields become `pub(crate)`.

**Step 2: Extend `rtp/mod.rs`**

```rust
pub(crate) mod payload;
pub(crate) mod rows;

pub(crate) use payload::{
    RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, analyze_aac_rfc3640_from_rows,
    analyze_h264_rfc6184_from_rows,
};
pub(crate) use rows::{RtpTsharkRow, tshark_extract_rtp_rows};
```

**Step 3: Fix `rtsp.rs` imports**

Extend the `use crate::rtp::{...}` line with the four newly re-exported names.

**Step 4: Move the 19 `payload` tests verbatim**

```
test_validate_h264_rfc6184_single_nal_ok
test_validate_h264_rfc6184_stap_a_ok
test_validate_h264_rfc6184_fu_a_ok
test_validate_h264_rfc6184_marker_violation_non_vcl_single_nal
test_validate_h264_rfc6184_fu_a_marker_violation_when_not_end
test_validate_h264_rfc6184_fu_a_malformed_short_and_invalid_header
test_validate_h264_rfc6184_stap_a_malformed_variants
test_validate_aac_rfc3640_single_au_ok
test_validate_aac_rfc3640_malformed_au_header
test_validate_aac_rfc3640_zero_au_size_is_invalid
test_validate_aac_rfc3640_au_size_too_large
test_analyze_h264_rfc6184_from_rows_no_rows_returns_error
test_analyze_h264_rfc6184_from_rows_insufficient_packets_returns_error
test_analyze_h264_rfc6184_from_rows_low_valid_ratio_returns_error
test_analyze_h264_rfc6184_from_rows_collects_violation_counters
test_analyze_aac_rfc3640_from_rows_no_rows_returns_error
test_analyze_aac_rfc3640_from_rows_insufficient_packets_returns_error
test_analyze_aac_rfc3640_from_rows_low_valid_ratio_returns_error
test_analyze_aac_rfc3640_from_rows_collects_error_and_timestamp_counters
```

The `analyze_*` tests construct `RtpTsharkRow` values. If `rtsp.rs`'s `mod tests` has a row-building helper those tests use, move it too and make it `pub(super)` in `rows.rs`'s test module or duplicate it locally — check before assuming.

**Step 5: Build, test, commit**

```bash
cargo build 2>&1 | tail -20 && cargo test 2>&1 | tail -3
```

Expected: **226 passed**.

```bash
git add validation/rust/src/rtp/ validation/rust/src/rtsp.rs
git commit -m "refactor(validation): extract rtp::payload from rtsp.rs"
```

---

## Task 3: Create `rtp/streams.rs`

**Files:**
- Create: `validation/rust/src/rtp/streams.rs`
- Modify: `validation/rust/src/rtp/mod.rs`, `validation/rust/src/rtsp.rs`

**Step 1: Move these 10 items**

`RtpStreamKey`, `RtpStreamStats`, `HarnessRtpLossMetric`, `compute_packet_loss_from_seqs`, `compute_stream_loss_metric`, `is_reasonably_h264`, `is_reasonably_aac`, `pick_primary_video_stream`, `pick_primary_audio_stream`, `group_rtp_rows_by_stream`

Header — note `serde::Serialize` **is** required here (`HarnessRtpLossMetric` derives it):

```rust
//! Grouping RTP rows into streams, selecting the primary video/audio stream,
//! and computing packet loss.

use serde::Serialize;
use std::collections::HashMap;

use super::payload::{validate_aac_rtp_payload_rfc3640, validate_h264_rtp_payload_rfc6184};
use super::rows::RtpTsharkRow;
```

Visibility: `HarnessRtpLossMetric` (+ fields), `RtpStreamStats` (+ `key`, `rows` fields), `group_rtp_rows_by_stream`, `pick_primary_video_stream`, `pick_primary_audio_stream`, `compute_stream_loss_metric` become `pub(crate)`. `RtpStreamKey` becomes `pub(crate)` (it is `RtpStreamStats::key`'s type). The rest stay private.

**`HarnessRtpLossMetric` keeps its name.** It is serialised into the report JSON under that shape; renaming is a behaviour change. Its name is now slightly odd (no "harness" in this module) — that is deliberate and recorded in the design doc.

**Step 2: Extend `rtp/mod.rs`** with `pub(crate) mod streams;` and re-export the six `pub(crate)` items.

**Step 3: Fix `rtsp.rs` imports.**

**Step 4: Move the 1 `streams` test:** `test_packet_loss_computation_does_not_mix_streams`.

**Step 5: Build, test, commit**

Expected: **226 passed**.

```bash
git commit -m "refactor(validation): extract rtp::streams from rtsp.rs"
```

---

## Task 4: Create `rtp/pacing.rs`

**Files:**
- Create: `validation/rust/src/rtp/pacing.rs`
- Modify: `validation/rust/src/rtp/mod.rs`, `validation/rust/src/rtsp.rs`

**Step 1: Move these 9 items**

`VIDEO_RTP_CLOCK_HZ`, `GapStats`, `FramePacing`, `delay_threshold_ms`, `percentile`, `gap_stats`, `encoder_deltas_ms`, `arrival_deltas_ms`, `compute_pacing`

Header:

```rust
//! Frame pacing: encoder cadence (RTP timestamp deltas) and arrival cadence
//! (wall clock), summarised as gap percentiles and a delay-event rate.

use serde::Serialize;

use super::rows::RtpTsharkRow;
```

> **Do NOT add `use super::streams::compute_packet_loss_from_seqs`.** A dependency
> scan reports that edge, but it is a **false positive**: the name appears only in a
> doc comment on `encoder_deltas_ms` ("same assumption as
> `compute_packet_loss_from_seqs`"), not in code. `pacing` depends on `rows` alone.
> Leave the comment as prose — it is still accurate.

Visibility: `GapStats` (+ fields), `FramePacing` (+ fields), `compute_pacing` become `pub(crate)`. `VIDEO_RTP_CLOCK_HZ`, `percentile`, `delay_threshold_ms`, `gap_stats`, `encoder_deltas_ms`, `arrival_deltas_ms` stay private.

**Step 2: Extend `rtp/mod.rs`.** Final form:

```rust
pub(crate) mod pacing;
pub(crate) mod payload;
pub(crate) mod rows;
pub(crate) mod streams;

pub(crate) use pacing::{FramePacing, GapStats, compute_pacing};
pub(crate) use payload::{
    RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, analyze_aac_rfc3640_from_rows,
    analyze_h264_rfc6184_from_rows,
};
pub(crate) use rows::{RtpTsharkRow, tshark_extract_rtp_rows};
pub(crate) use streams::{
    HarnessRtpLossMetric, compute_stream_loss_metric, group_rtp_rows_by_stream,
    pick_primary_audio_stream, pick_primary_video_stream,
};
```

**Step 3: Fix `rtsp.rs` imports.**

**Step 4: Move the 10 `pacing` tests verbatim**

```
test_delay_threshold_ms_fps_scaling
test_encoder_deltas_ms_basic
test_encoder_deltas_ms_frames_with_multiple_packets_collapse
test_encoder_deltas_ms_wrap_around
test_arrival_deltas_ms
test_compute_pacing_skips_arrival_when_epochs_missing
test_compute_pacing_no_data_returns_none
test_gap_stats_delay_rule_boundary
test_gap_stats_percentiles
test_compute_pacing_encoder_and_arrival_stats
```

**Step 5: Verify the whole gate, then commit**

```bash
cargo fmt
cargo clippy --all-targets 2>&1 | tail -10
cargo test 2>&1 | tail -3
```

Expected: clippy **clean** (it is clean as of `aeee6416` — any new warning is from this refactor), **226 passed**.

Sanity-check the split landed as designed:

```bash
wc -l src/rtp/*.rs src/rtsp.rs
```

Expected roughly: `rows` ~250, `payload` ~450, `streams` ~180, `pacing` ~260, `mod` ~40, `rtsp.rs` ~2,930 (all including tests).

```bash
git commit -m "refactor(validation): extract rtp::pacing, completing the rtp module"
```

---

## Task 5: Extract `probe.rs`

The Retina live probe has **zero** reference edges to the harness — verified by measurement.

**Files:**
- Create: `validation/rust/src/probe.rs`
- Modify: `validation/rust/src/lib.rs`, `validation/rust/src/rtsp.rs`

**Step 1: Move these 8 items into `src/probe.rs`**

`run_validation`, `build_sdp_test_results`, `validate_h264_length_prefixed_nals`, `empty_report`, `critical_proto_failed`, `to_retina_transport`, `to_retina_initial_timestamp_policy`, `stream_info_from_retina`

> `validate_h264_length_prefixed_nals` moves here **despite the H.264 name** — it
> validates length-prefixed NAL framing on frames from the Retina live path, not
> from a pcap. It does not belong in `rtp/`.

Header:

```rust
//! Retina-based live RTSP probe: DESCRIBE/SETUP/PLAY sequencing, SDP structure,
//! and first-frame timing measured in-process.

use anyhow::{Context, Result, bail};
use chrono::Utc;
use futures_util::StreamExt;
use retina::client::{
    Credentials, InitialTimestampPolicy, PlayOptions, Session, SessionOptions, SetupOptions,
    Transport,
};
use retina::codec::{CodecItem, ParametersRef};
use tokio::time::{Instant, timeout};
use tracing::{debug, info, trace, warn};
use url::Url;

use crate::config::{Args, EffectiveConfig, InitialTimestampPolicyArg, TransportArg};
use crate::report::{StreamInfo, TestResult, TestRun, ValidationReport, result_ok};
```

Also move the const `PROBE_DEMUX_ERROR_TOLERANCE`.

**Step 2: Fix the duplicated URL string**

`run_validation` opens with:

```rust
let url_str = format!(
    "rtsp://{}:{}{}",
    effective.rtsp_host, effective.rtsp_port, effective.rtsp_stream
);
```

That is a fourth copy of the format in `rtsp_url`. Replace with:

```rust
let url_str = crate::rtsp::rtsp_url(
    &effective.rtsp_host,
    effective.rtsp_port,
    &effective.rtsp_stream,
);
```

(After Task 6 this becomes `crate::harness::rtsp_url`.) Ensure `rtsp_url` is `pub(crate)` — it already is.

**Step 3: Update `src/lib.rs`**

```rust
pub mod probe;
```

and move the re-export:

```rust
pub use probe::{critical_proto_failed, run_validation};
pub use rtsp::run_harness;
```

**Step 4: Update `src/main.rs`**

`main.rs` imports `critical_proto_failed` and `run_validation` from `rtsp_validation_tool::rtsp`. Repoint to `::probe`. `run_harness` stays.

**Step 5: Move the probe tests**

Move every test whose subject is one of the eight moved items — the `test_build_sdp_test_results_*`, `test_critical_proto_failed_*`, `test_validate_h264_length_prefixed_nals_*`, `test_empty_report`, `test_to_retina_transport`, `test_to_retina_initial_timestamp_policy` families. Verbatim.

**Step 6: Build, test, commit**

```bash
cargo build 2>&1 | tail -20 && cargo test 2>&1 | tail -3
```

Expected: **226 passed**.

```bash
git add validation/rust/src/
git commit -m "refactor(validation): extract the Retina live probe into probe.rs"
```

---

## Task 6: Rename `rtsp.rs` -> `harness.rs`

After Task 5 the file contains only the external-tool harness. The name is now misleading — `probe.rs` is equally "rtsp".

**Files:**
- Rename: `validation/rust/src/rtsp.rs` -> `validation/rust/src/harness.rs`
- Modify: `validation/rust/src/lib.rs`, `validation/rust/src/main.rs`, `validation/rust/src/probe.rs`

**Step 1: Rename with history preserved**

```bash
git mv validation/rust/src/rtsp.rs validation/rust/src/harness.rs
```

**Step 2: Update the module declaration in `lib.rs`**

`pub mod rtsp;` -> `pub mod harness;` (keep alphabetical order: after `device`, before `httpflv`). Update `pub use rtsp::run_harness;` -> `pub use harness::run_harness;`.

**Step 3: Update every `crate::rtsp::` / `rtsp_validation_tool::rtsp::` reference**

```bash
grep -rn "crate::rtsp\|rtsp_validation_tool::rtsp\|::rtsp::" validation/rust/src/
```

Repoint each to `harness`. Includes the `crate::rtsp::rtsp_url` call added in Task 5 Step 2.

**Step 4: Update the module doc comment**

Change `harness.rs`'s `//!` header to describe the harness only — the live-probe half is gone.

**Step 5: Full gate**

```bash
cargo fmt
cargo clippy --all-targets 2>&1 | tail -10
cargo test 2>&1 | tail -3
wc -l src/*.rs src/rtp/*.rs
```

Expected: clippy clean, **226 passed**, and `harness.rs` ~2,200 lines. If `harness.rs` is much larger than that, a probe item was left behind.

**Step 6: Commit**

```bash
git commit -m "refactor(validation): rename rtsp.rs to harness.rs now that probe.rs is split out"
```

---

## Task 7: Final verification

**Step 1: Confirm no behaviour drift in the report shape**

No serialised type was renamed and no `serde` attribute touched. Confirm:

```bash
git diff aeee6416..HEAD -- validation/rust/src/ | grep -E "^[+-].*#\[serde" | sort | uniq -c
```

Expected: every `+` line has a matching `-` line (pure moves, no attribute changes).

**Step 2: Confirm the test count never moved**

```bash
git log --oneline aeee6416..HEAD
```

For each commit, the message should correspond to a green 226-test run.

**Step 3: Confirm the dependency direction holds**

```bash
grep -n "^use " src/rtp/rows.rs
```

Expected: **no `use super::`** — `rows` is the base and must depend on no sibling.

```bash
grep -n "use super::" src/rtp/pacing.rs
```

Expected: only `use super::rows::RtpTsharkRow;` — if `streams` appears, the false-positive warning in Task 4 was ignored.

**Step 4: Update the design doc status**

Change `Status: approved, not yet implemented` to `Status: implemented` in `docs/plans/2026-08-06-rtp-module-split-design.md`, and commit.

---

## Rollback

Each task is one commit on a measured seam. To abandon at any point:

```bash
git reset --hard aeee6416   # pre-refactor state, clippy-clean, 226 tests green
```

Stopping after Task 4 is a coherent end state (`rtp/` extracted, `rtsp.rs` intact). Stopping after Task 5 without Task 6 is also coherent, just with a misleadingly-named file.

## Known follow-up (not in scope)

`rtp/streams.rs` will hold 10 items and 1 test. Invisible in a 4,317-line file, glaring in a 180-line module. Worth a coverage pass afterwards.
