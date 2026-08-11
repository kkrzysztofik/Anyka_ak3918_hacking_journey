# Diagnostics Vision + Network Text — Design

Date: 2026-08-11
Status: Approved

## Problem

Diagnostics already charts network throughput from `network.{rx,tx}_bps`, but Device
Information has no textual rates. Day/night hardware state (AE luma, ain0 photosensor,
IR LED / IR-CUT / white LED) is owned by `NightModeController` and is not exposed to
the WebUI.

## Goals

1. Show **download / upload** as text in Device Information (same units as the chart).
2. Add a separate **Day / Night Vision** section with live readings:
   - AE luma
   - ain0 photosensor
   - IR LED, IR-CUT A, IR-CUT B, White LED (full lamp set)
   - Current day/night mode when known

## Non-goals

- Changing night-mode AUTO behaviour or thresholds
- Historical sparklines for AE/ain0
- New auth levels or extra HTTP routes

## Decisions

| Topic | Choice |
| --- | --- |
| Data path | Extend `GET /api/diagnostics` with `vision` |
| Freshness | Live read on every poll (AE FFI + ain0/GPIO sysfs) |
| Photosensor | `ain0` (not a separate “aic0” node) |
| Lamps | Full set: IR LED, IR-CUT A/B, White LED |
| Missing HW / errors | Field `null`; `supported` flags capability |
| Stub / no imaging | `vision: null` |

## API shape

```json
{
  "network": { "rx_bps": 1234, "tx_bps": 567 },
  "vision": {
    "mode": "day",
    "ae_luma": 42,
    "ain0": 306,
    "ir_led": true,
    "ircut_a": false,
    "ircut_b": true,
    "white_led": false,
    "supported": {
      "ir_led": true,
      "ircut": true,
      "white_led": true
    }
  }
}
```

- `mode`: `"day"` | `"night"` | `null` (never driven yet)
- Lamp bools: `null` when node absent or unreadable; otherwise sysfs value ≠ 0 → `true`
- `supported.*`: probe results (node existence), independent of current on/off

## Backend

1. `NightModeController::live_diagnostics()` — async:
   - `ffi.get_ae_luma()`
   - `read_light_sensor(&paths)` for ain0
   - read each GPIO node when `caps` says present
   - `current_mode()` for mode
2. `ImagingControl::vision_diagnostics()` — default `Ok(None)`; Anyka imaging
   delegates to the night-mode controller; stub returns `Ok(None)`.
3. `DiagnosticsState::snapshot` becomes async; awaits imaging vision when platform
   has `imaging_control()`.
4. `handle_diagnostics` already async — just await snapshot.

Keep types serializable next to other snapshot structs (or in `platform/common` if
shared by the trait). Prefer one `Vision` struct used by both to avoid drift.

## Frontend

1. Device Information: Download / Upload rows from `data.network` (format kbps like
   the chart; `—` when `network` is null).
2. New card **Day / Night Vision** beside/under Device Information / Stream Health:
   mode, AE, ain0, each lamp (unsupported → `n/a`).
3. Extend `Diagnostics` TypeScript type + Vitest coverage.

## Testing

- Rust: night-mode live_diagnostics with tempdir GPIO/ain0 + mocked AE FFI
- Rust: snapshot includes `vision: null` without imaging
- WWW: Device Information network rows; vision card render / n/a paths

## Deploy

WWW rebuild + FTP to `.198` after implementation; ARM binary needed because vision is
server-side live reads.
