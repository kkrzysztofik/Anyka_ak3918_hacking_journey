---
name: anyka-validation
description: Use when running or extending RTSP/HTTP-FLV protocol conformance and performance validation (validation/rust tool, baseline compare/update, launch-on-device, ffmpeg/ffprobe/tshark harness, frame pacing, telemetry, performance regression checks).
version: 1.0.0
---

# Anyka Validation Tool (RTSP/HTTP-FLV)

Host-side validation of RTSP/HTTP-FLV performance and protocol conformance for onvif-rust. One Rust binary (`validation/`) launches onvif-rust (optional), runs protocol checks (Retina) + harness scenarios (ffmpeg, ffprobe, tshark), and writes a single JSON report. Full reference: `wiki/RTSP-Validation-Tool.md`.

## Build

```bash
source ./setenv.sh                       # vendored toolchain

# Host build of onvif-rust (needed for local --h264-file launches)
cd cross-compile/onvif-rust
$CARGO build --target x86_64-unknown-linux-gnu

# Build the validator — places binary at validation/rtsp_validation_tool
./validation/build_rtsp_validation_tool.sh
```

Prereqs on the host: `ffmpeg`, `tshark` (`sudo apt-get install ffmpeg tshark`; tshark may need the `wireshark` group or sudo).

## Quick Start

```bash
# Against an already-running server
./validation/rtsp_validation_tool --no-launch --rtsp-port 8554 --rtsp-stream /stream1

# Launch onvif-rust locally with a test H.264 file
./validation/rtsp_validation_tool --h264-file test_video.h264 --rtsp-port 8554

# Against the camera via SSH (config-driven)
./validation/rtsp_validation_tool -c validation/config/rtsp_validation.toml

# View latest report
LATEST_RUN="$(ls -1dt rtsp_results/runs/* | head -n1)"
jq . "${LATEST_RUN}/rtsp_validation.json"

# Convert to Markdown
python3 validation/scripts/rtsp_validation_json_to_md.py -o rtsp_validation.md
```

## Config

Config search order: `--config <path>` → env `RTSP_VALIDATION_CONFIG` → `./rtsp_validation.toml` → `validation/config/rtsp_validation.toml`. CLI overrides config.

Example configs in `validation/config/`:
- `rtsp_validation.toml` — device connection (default; `launch-on-device = true`)
- `rtsp_validation_device_real.toml` — real-mode camera sensor (multi-stream `main`/`sub`)
- `rtsp_validation_external.toml` — external/third-party RTSP server

Key sections: `[run]` (no-launch / launch-on-device / h264-file / update-baseline / compare-baseline), `[rtsp]` (host/port/stream, `initial-timestamp-policy = "permissive"` for MediaMTX), `[thresholds]` (startup latencies, bitrate/fps/packet-loss tolerances), `[pacing]` (expected-fps, delay-multiple, delay-floor-ms, delay-tolerance-percent), `[device]` (SSH host/port/user/password, telemetry, h264-file/aac-file, real-mode), `[artifacts]`, `[logging]`. Note kebab-case keys (`deny_unknown_fields` — `bitrate-kbps` not `bitrate_kbps`).

## Device (camera) Validation

Requires SSH on the device (default port 22) and `onvif-rust` + `config.toml` present at `/mnt/anyka_hack/onvif/` on the device.

```bash
# Password via env var (recommended, avoids secrets on the CLI)
RTSP_VALIDATION_DEVICE_PASSWORD=www123 \
  ./validation/rtsp_validation_tool -c validation/config/rtsp_validation.toml \
  --launch-on-device --no-launch

# File playback (validation-mode) with device-side H.264 + AAC
./validation/rtsp_validation_tool --launch-on-device --no-launch \
  --device-h264-file /mnt/anyka_hack/onvif/test.h264 \
  --device-aac-file /mnt/anyka_hack/onvif/test.aac --device-loop-playback

# Real-mode (camera sensor instead of file) — implies --launch-on-device
./validation/rtsp_validation_tool -c validation/config/rtsp_validation_device_real.toml --device-password secret
```

- `--device-real-mode` and `--device-h264-file` are mutually exclusive.
- `--validation-mode` is a runtime flag on onvif-rust, **not** a cargo feature — a normal host build already supports it.
- If auth is enabled, validation-mode needs a provisioned `users.toml` in the config dir or startup aborts.
- Report may include a `telemetry` object (RAM/CPU/load average, `onvif_rss_kib`, `onvif_vmsize_kib`, `onvif_pid`). Skip with `--no-telemetry`.

## Baselines (Performance Regression)

Baselines live under `rtsp_results/baselines/`.

```bash
# Record a baseline
./validation/rtsp_validation_tool --no-launch --rtsp-port 8554 --update-baseline

# Compare a run against baseline — deviations become baseline_regression_* failures
./validation/rtsp_validation_tool --no-launch --rtsp-port 8554 --compare-baseline
```

Tracked metrics: harness_startup_latency_ms, harness_bitrate_kbps, harness_fps, harness_packet_loss_percent, telemetry_mem_*, telemetry_load_avg_*, telemetry_onvif_rss_kib, telemetry_onvif_vmsize_kib.

## Key Metrics & Thresholds

- `first_video_frame_latency_ms` — protocol path first-frame (threshold `video-startup-latency-ms`, default 1500).
- `harness_startup_latency_ms` — ffmpeg to first frame (threshold `harness-startup-latency-ms`; 2500–4000 ms normal for external FFmpeg/MediaMTX).
- `harness_bitrate_kbps`, `harness_fps` — steady-state; sanity minimums ≥1 kbps / ≥5 fps, then expected values or baselines.
- `harness_packet_loss_percent` — RTP loss from capture (tolerance 1%).
- `harness_pcap_rfc6184_h264` / `harness_pcap_rfc3640_aac` — RTP payload structural conformance (H.264 Single/STAP-A/FU-A, AAC AU headers).
- `frame_pacing_encoder_delay_percent` / `frame_pacing_arrival_delay_percent` — % of frame gaps over the pacing rule (gap ≥ max(1000/expected-fps × delay-multiple, delay-floor-ms)); fail above `delay-tolerance-percent` (default 5).
- `harness_protocol_sequence` — DESCRIBE/SETUP/PLAY/TEARDOWN counts + status codes; 401s tracked separately and don't fail.

## Code Layout (`validation/rust/src/`)

| Module | Role |
|--------|------|
| `main.rs` | CLI entry, run orchestration, device SSH lifecycle |
| `config.rs` | TOML schema + CLI parsing → `EffectiveConfig` |
| `probe.rs` | Retina probe: DESCRIBE/SETUP/PLAY, SDP, first-frame timing |
| `harness.rs` | ffmpeg/ffprobe/tshark scenarios |
| `httpflv.rs` | FLV container parse + ffmpeg harness |
| `rtp/` | Pure analysis: rows (tshark fields), payload (RFC 6184/3640), streams (loss), pacing |
| `baseline.rs` | Baseline write + regression comparison |
| `device.rs` | Device SSH, telemetry, log collection |
| `report.rs` | `TestResult`, `Summary`, report types |

`rtp/` is pure (no config/test-type/ffmpeg/tokio deps); `probe` and `harness` don't reference each other.

## Testing the Tool

```bash
source ./setenv.sh
cd validation/rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

Integration test: `validation/rust/tests/httpflv_integration.rs`.

## Helper Scripts

- `validation/scripts/rtsp_validation_json_to_md.py -o out.md` — report → Markdown.
- `validation/scripts/ws_discovery_validator.py --timeout 10 --verbose` — WS-Discovery probes to validate ONVIF device responses on the local network.

## Troubleshooting

- **ffmpeg/tshark not found** → `sudo apt-get install ffmpeg tshark`; tshark needs `wireshark` group or sudo.
- **Server not starting** → check the H.264 file path; `--validation-mode` is a runtime flag, no cargo feature.
- **Port in use** → `--rtsp-port` override.
- **Device unreachable** → verify `--device-host`, SSH enabled, creds, and `/mnt/anyka_hack/onvif/onvif-rust` + `config.toml` exist on device.
- **rtptime missing on PLAY** → `initial-timestamp-policy = "permissive"` (default; MediaMTX).
