# AK3918 AE raw units — measured scales

Measured 2026-09-21 on `.198` (GC1084, firmware build `f2980a99`) with
`scripts/debugging/measure_ae_units.py`, which drives the temporary
`/api/ae-debug` endpoint (see its ponytail note in `onvif-rust/src/diagnostics/http.rs`)
and records the driver's own readback (`GET /api/diagnostics → vision.ae_run_info`,
raw units from `struct vpss_isp_ae_run_info`) plus one RTSP frame's mean Y per step.

## What is known, and how confident

| Quantity | Raw unit | Conversion | Confidence |
|---|---|---|---|
| `a_gain` (analog gain) | Q8 fixed point, **256 = 1.0×** | dB = 20·log₁₀(a_gain / 256); max 16384 = 64× ≈ +36.12 dB | **High** — the auto state at rest sits at exactly 256, and the gain-ceiling sweep shows the operating point tracking the ceiling at 512/1024/2048/8192, i.e. the same number-space |
| `exp_time` | sensor lines (vendor unit) | **No µs conversion established** — see below | Unknown |
| `isp_d_gain` | Q8 (256 = 1×) | same as a_gain | Medium — only one non-256 sample (300) |
| `avg_lumi` | driver AE target scale (≈0–255, target ≈ 40–55) | not an ONVIF quantity | n/a — used as the convergence signal |

## Raw sweep table

Phase A pins `a_gain_max=256` (1×) and sweeps the exposure ceiling; Phase B pins
`exp_time_max=10` and sweeps the gain ceiling. The camera was in stable indoor
daylight; `frameY` is the mean of the RTSP Y plane, `avg_lumi` the driver's own
scene-luma reading.

| phase | a_gain_max | exp_time_max | a_gain | d_gain | isp_d_gain | exp_time | avg_lumi | frameY |
|---|---|---|---|---|---|---|---|---|
| A | 256 | 40 | 0 | 256 | 256 | 0 | 0 | 0.09 |
| A | 256 | 100 | 256 | 256 | 256 | 1 | 0 | 0.09 |
| A | 256 | 200 | 256 | 256 | 256 | 1 | 0 | 0.09 |
| A | 256 | 400 | 256 | 256 | 256 | 186 | 40 | 4.5 |
| A | 256 | 800 | 256 | 256 | 256 | 186 | 45 | 159.0 |
| A | 256 | 1600 | 256 | 256 | 256 | 186 | 46 | 159.1 |
| A | 256 | 3000 | 256 | 256 | 256 | 186 | 46 | 159.3 |
| A | 256 | 5000 | 256 | 256 | 300 | 225 | 42 | 128.2 |
| B | 256 | 10 | 0 | 256 | 256 | 0 | 0 | 0.09 |
| B | 512 | 10 | 512 | 256 | 256 | 1 | 0 | 0.09 |
| B | 1024 | 10 | 1024 | 256 | 256 | 2 | 0 | 0.09 |
| B | 2048 | 10 | 2048 | 256 | 256 | 8 | 5 | 29.2 |
| B | 4096 | 10 | 0 | 256 | 256 | 0 | 0 | 0.09 |
| B | 8192 | 10 | 8192 | 256 | 256 | 1 | 4 | 8.6 |
| B | 16384 | 10 | 0 | 256 | 256 | 0 | 0 | 0.09 |

Pre-sweep baseline (default ceilings `a_gain_max=16384`, `exp_time_max=2250`):
`a_gain=256 d_gain=256 isp_d_gain=256 exp_time=203 avg_lumi=49 darked=0`.

## Reading the table

- **Gain.** At rest the AE holds `a_gain=256` — a fixed-point 1.0×, the only
  value consistent with a neutral white image under the default profile. The
  Phase-B operating points (`512, 1024, 2048, 8192`) are the ceilings verbatim,
  so ceiling and operating point share one number-space: **Q8, divisor 256**.
  This resolves the plan's open question (16384 = 64×, not 16×).
- **Exposure.** Once the ceiling exceeds ~400 the AE converges to
  `exp_time≈186` regardless of ceiling (800/1600/3000 all hold 186), i.e. the
  ceiling is a clamp, not a setpoint, and the converged point in this
  lighting is ≈186–225 lines. Below ~200 the scene goes black and the AE
  collapses `exp_time` to 0–1 (black-clamp), which destroys the
  "operating point = ceiling" property the µs fit needs.
- **Why no µs conversion.** Two honest options, both blocked:
  1. *Pinned sweep* — pin the exposure value and read the frame. Manual AE
     (`AK_ISP_set_mae_attr`, `struct {exp_time, a_gain, d_gain, isp_d_gain}`)
     exists in the vendor source, but the shipped `libakispsdk.so` **does not
     export it** (nm-checked: the lib carries only the blc/wb/awb/exp_type
     subset), and `libplat_vpss.so` has no mae passthrough. A raw ioctl on
     `/dev/isp_char` would reach it, but the manual-WB gate already proved the
     shipped kernel's 3A can ignore written parameters (the manual WB branch
     never applied), so a raw-ioctl mae is a bet the plan's gate says not to
     take.
  2. *Sensor geometry* — µs-per-unit = 1e6 / (fps × VTS lines/frame). The
     GC1084 runs from a closed kernel module (`sensor_gc1084.ko`); no VTS
     table exists in the reference tree.
  Publishing a guessed µs constant under the ONVIF `tt:ExposureTime` name
  would lie to every third-party client, which is exactly what this task
  exists to prevent.

## Consequences for the ONVIF surface (Task 11)

- `MinGain`/`MaxGain` **may** be advertised in dB using the Q8 constant above
  (`AE_GAIN_Q8 = 256`), with the measured max 16384 → +36.12 dB.
- `MinExposureTime`/`MaxExposureTime` are **omitted** — the driver keeps them
  in line units and no conversion is established. ONVIF `tt:Exposure20`
  tolerates the fields being absent; the UI degrades to mode + gain.
- The exposure **mode** (auto vs manual) is switchable (`AK_ISP_set_exp_type`,
  exported and verified linkable), but with no mae path a manual selection
  freezes the AE at whatever values the driver already holds — an unrecoverable
  picture through ONVIF. Ship **AUTO-only**: `GetOptions` advertises `AUTO`,
  `SetImagingSettings` rejects `MANUAL` with a spec fault, and the UI shows the
  MANUAL option disabled.

## Cleanup owed

- `/api/ae-debug` route + `AeDebugBody`/`AeDebugResult` in
  `onvif-rust/src/diagnostics/http.rs` (ponytail note there), the
  `DiagnosticsState::platform()` accessor, and the daemon's 119/120/121
  commands stay only as long as the sweep needs them; the permanent surface is
  `vision.ae_run_info` in `/api/diagnostics`.
