# The Imaging tab's sliders have never worked

Date: 2026-09-21
Status: design approved, implementation not started

## Problem

`www/src/pages/settings/ImagingPage.tsx` renders eight cards. Four of its six
live controls cannot apply a value, two more are cosmetic, and two cards are
greyed out as "Unavailable" against hardware that supports them.

The headline defect is not a missing feature. Moving the Brightness, Contrast,
Saturation or Sharpness slider to anything at or above 20% makes
`SetImagingSettings` fail with a SOAP fault. Confirmed on `.198`:

```
$ SetImagingSettings Brightness=60
  → s:Receiver / ter:HardwareFailure / "Hardware query failed"

/mnt/logs/vendor_daemon.log:
  [isp_set_effect:1753] value range [-50, 50], cur value: 153
```

### Root cause

`hal/common/imaging.rs:122` maps the ONVIF 0–100 scale onto 0–255:

```rust
pub(crate) fn onvif_to_sdk_value(onvif_value: f32, sdk_max: i32) -> i32 {
    ((onvif_value / ONVIF_MAX) * sdk_max as f32).round() as i32
}
```

with `SDK_MAX_VALUE = 255`. The SDK's contract is different, and it does not
clamp — it hard-rejects (`libplat/src/vi/isp_basic.c:2052`):

```c
int isp_set_effect(enum vpss_effect_type type, int value)
{
	if (value > 50  || value < -50) {
		ak_print_error_ex("value range [-50, 50], cur value: %d\n", value);
		return AK_FAILED;
	}
```

`ak_vpss.h` documents the same range for `VPSS_EFFECT_HUE` … `VPSS_EFFECT_WDR`
and adds the part that matters: **"0 means use the value in ISP config file"**.
So the scale is not 0-to-max, it is a signed offset from the tuning profile.

The vendor's own ONVIF layer gets this right (`libapp/src/onvif/ja_onvif.c:299`):

```c
ak_vpss_effect_set(..., VPSS_EFFECT_BRIGHTNESS, color->brightness - IMG_EFFECT_DEF_VAL);
```

with `IMG_EFFECT_DEF_VAL = 50` (`ja_media.h:10`). ONVIF 0–100 maps to −50…+50 by
subtracting 50.

| ONVIF slider | we send | SDK verdict |
|---|---|---|
| 0 | 0 | accepted — means "conf default", not "darkest" |
| 19 | 48 | accepted |
| **20** | **51** | **AK_FAILED** |
| **50** (UI default) | **128** | **AK_FAILED** |
| **100** | **255** | **AK_FAILED** |

### Why this survived

`config/types.rs:715-718` defaults all four to `50.0`, and the page renders
whatever it reads back. The UI shows 50, submits 50, and
`platform/anyka/imaging.rs:216` skips the IPC call as redundant via
`approximately_equal`. The round trip is config → UI → config with the hardware
never consulted, so the tab looks healthy. Only a *changed* value reaches the
SDK, and any change ≥20 fails.

Three unit tests cover `onvif_to_sdk_value` and all three assert the wrong
contract (`0→0`, `50→128`, `100→255`). The real contract lived only in a vendor
header comment and a vendor `.c` file.

## What the hardware actually supports

The daemon's command table is not the ceiling. `readelf` on the libs we already
ship and already link:

- `libplat_vpss.so` exports `ak_vpss_isp_set_exp_type`, `set_ae_attr`,
  `set_wb_type`, `set_mwb_attr`, `set_awb_attr`, `set_3d_nr_attr`.
- `libakispsdk.so` exports 98 low-level `AK_ISP_*` symbols, including
  `set_blc_attr`, `set_wdr_attr`, `set_hue_attr`, `set_rgb_gamma_attr`,
  `set_frame_rate`, `set_nr1_attr`, `set_nr2_attr`.
- `libplat_vi.so` holds 188 undefined `AK_ISP_*` references and the daemon
  already links `-lakispsdk` (`vendor-daemon/Makefile:96`), so the ISP SDK is
  loaded and initialised in-process. **No second `AK_ISP_sdk_init`, and no new
  library for any phase of this work.**
- `handle_isp_effect` (`handlers_isp.c:26`) is already generic over
  `enum vpss_effect_type`. The dispatcher hardwires 4 of the 8 values.

The 2026-08-14 day/night design called `ak_isp_sdk.h` "the escape hatch" and
never took it, because that investigation's root cause turned out to be the IR
lamp emitting nothing rather than AE ceilings. `CMD_ISP_SET_AE_ATTR = 109` was
cancelled for the same reason and its slot is still reserved.

## The gaps

| # | Gap | Location |
|---|-----|----------|
| 1 | Brightness/contrast/saturation/sharpness reject every value ≥20 | `hal/common/imaging.rs:122` |
| 2 | `handle_isp_set_wdr` is a `log_debug("no-op")` that returns OK | `handlers_isp.c:47` |
| 3 | Backlight compensation is persisted and never reaches the SDK | `imaging/store.rs:559` |
| 4 | `set_settings` uses `?` between knobs — a mid-batch fault leaves the camera partially applied and persists nothing | `platform/anyka/imaging.rs:216-242` |
| 5 | White Balance card greyed out; backend hardcodes `white_balance: None` | `ops/settings.rs:234`, `ImagingPage.tsx:423` |
| 6 | Exposure card greyed out; backend hardcodes `exposure: None` | `ops/settings.rs:232`, `ImagingPage.tsx:453` |
| 7 | Hue, anti-flicker Hz and style unreachable despite the generic handler | `dispatcher.c:332-351` |
| 8 | Illumination switches are `useState(false)` — always OFF after reload | `ImagingPage.tsx:86-87` |
| 9 | No image visible while tuning; "Focus & Sharpness" promises focus the device lacks | `ImagingPage.tsx:350` |

Gaps 1–4 are the dangerous ones: the tab actively lies about applying settings.

## Decisions

| decision | choice | rationale |
|---|---|---|
| slider scale | keep ONVIF 0–100, map `v - 50` | matches the vendor; 50 = conf default = today's actual image, so no migration and no visual jump |
| WDR transport | `ak_vpss_effect_set(VPSS_EFFECT_WDR)` | same already-linked path as the other effects; no new command |
| BLC transport | new `CMD_ISP_SET_BLC` over `AK_ISP_*` | only knob with no `ak_vpss` equivalent |
| attr writes | read-modify-write, always | a fresh `vpss_isp_ae_attr` zeroes `hist_weight`, `envi_gain_range`, `target_lumiance` |
| exposure/WB surface | ONVIF SOAP | `Exposure20`/`WhiteBalance20` and their options types already exist; third-party clients get them free |
| hue/Hz/style surface | REST `/api/imaging` | ONVIF does not model them; follows the `/api/network` precedent |
| apply trigger | `onValueCommit`, not on drag | every apply requests an IDR, and a 184KB main IDR does not fit the 131008-byte ring slot |

## Architecture

```text
  WebUI ImagingPage
    ├── SOAP  /onvif/imaging_service ── ONVIF-modelled knobs
    │     brightness, contrast, saturation, sharpness, IrCutFilter,
    │     WideDynamicRange, BacklightCompensation, WhiteBalance20, Exposure20
    │
    └── REST  /api/imaging ──────────── non-ONVIF knobs
          hue, power_hz, style_id

  onvif-rust  ── all mapping and validation, all tests
       │ IPC
  vendor-daemon (C) ── SDK access only, no policy
       ├── ak_vpss_effect_set        (brightness…sharp, wdr, hue, hz, style)
       ├── ak_vpss_isp_set_wb_type / set_mwb_attr
       ├── ak_vpss_isp_set_exp_type / set_ae_attr
       └── AK_ISP_get_blc_attr / AK_ISP_set_blc_attr
```

## Phase 1 — make the existing six work

Replace `onvif_to_sdk_value(v, sdk_max)` with an offset mapping, drop the now
meaningless `sdk_max` parameter and `SDK_MAX_VALUE`. Four call sites, one file.
Rewrite the three tests that encode the old contract.

Wire WDR through the effect path. Add `CMD_ISP_SET_BLC` with read-modify-write.

Restructure `set_settings` to validate all values before applying any, so a
rejected value cannot leave the camera half-configured.

**Gate:** on hardware, `SetImagingSettings` with brightness 0, 50 and 100 all
succeed, and 0 vs 100 are visibly different frames.

## Phase 2a — White Balance

`WhiteBalance20{Mode, CrGain, CbGain}` ↔ `ak_vpss_isp_set_wb_type`
(`WB_OPS_TYPE_MANU=0`, `WB_OPS_TYPE_AUTO=1`) plus `set_mwb_attr`. CrGain→`r_gain`,
CbGain→`b_gain`, `g_gain` and all three offsets preserved by read-modify-write.
ONVIF gains and SDK gains are both unitless multipliers, so no calibration.

Backend stops returning `white_balance: None`; `GetOptions` advertises the modes.
The card loses its "Unavailable" treatment.

## Phase 2b — Exposure, with calibration

`set_exp_type` (AUTO/MANUAL) is trivial. The numeric limits are not: ONVIF
mandates `MinExposureTime`/`MaxExposureTime` in **microseconds** and
`MinGain`/`MaxGain` in **decibels**, while the live device reports raw
fixed-point and line units. From `.198` via `/api/diagnostics`:

```json
{ "ae_a_gain_max": 16384, "ae_exp_time_max": 2250, "ae_target_luminance": 55 }
```

`16384` is a fixed-point gain (Q8 → 64×, or Q10 → 16×) and `2250` is lines or
0.1 ms, not µs. Publishing either under an ONVIF field name would lie to every
third-party client.

So this phase leads with measurement:

1. Sweep `exp_time_max` and `a_gain_max` over `CMD_ISP_SET_AE_ATTR` (slot 109,
   still reserved) and record `ae_run_info.current_exp_time`,
   `current_a_gain` and achieved luma at each step.
2. Derive µs-per-unit from frame timing at a known sensor fps, and dB from the
   gain ratio, via `Ak_ISP_Get_Sensor_Fps`.
3. Only then implement the ONVIF conversion, with the derived constants named
   and documented next to the mapping.

Ship Mode AUTO/MANUAL as soon as it passes; hold the numeric limits behind the
calibration result. If the sweep cannot produce a consistent conversion, the
limits stay out of ONVIF rather than shipping wrong units.

`ak_vpss_isp_get_ae_run_info` also carries `current_darked_flag`, the vendor's
own day/night bit. Worth reading into diagnostics while we are here — it is a
second opinion on a decision the `ae-luma-cannot-see-dusk` finding showed we
make poorly.

## Phase 3 — hue, anti-flicker, style

`GET`/`PUT /api/imaging` carrying three integers. Two dispatcher lines per knob
against the existing generic handler:

- hue, `VPSS_EFFECT_HUE`, UI 0–100 → −50…50, same offset as Phase 1
- `power_hz`, `VPSS_POWER_HZ`, 50 or 60 only — the lib rejects anything else,
  and 50 Hz matters for mains lighting here
- `style_id`, `VPSS_STYLE_ID`, 0–2, must match the ISP config file

Values persist in `[imaging]` config. Per the
`anyka-toml-config-key-is-a-rollback-hazard` finding, these get code defaults
rather than a new required section, so the older `anyka-init` in the other A/B
slot still parses the file.

## Phase 4 — live preview and honest cards

Reuse `components/common/LiveVideoPlayer.tsx` beside the controls. Apply on
Radix `onValueCommit` so one drag produces one apply, not one per tick.

Fold Sharpness into Color & Brightness and drop the "Focus &" half of the title.
Give the illumination switches real state by reading `ir_led` / `white_led`,
which `/api/diagnostics` already returns.

## Testing

- Host-side unit tests for every mapping, including the boundaries that
  currently fail: 0, 19, 20, 50, 100.
- A test asserting `onvif_to_sdk_value(50.0) == 0`, i.e. that neutral means
  "conf default". This is the assertion whose absence hid the bug.
- Daemon-side: BLC and AE attr read-modify-write preserve untouched fields.
- Vitest for the imaging page: commit-on-release fires one mutation per drag,
  lamp switches reflect fetched state, WB/exposure cards render live.
- Hardware gate per phase. A green host test suite is what we already had while
  the tab was broken.

## Risks

| risk | mitigation |
|---|---|
| Fixing the range changes the image on cameras whose stored value is not 50 | 50 is the config default everywhere; audit the fleet's `anyka.toml` before rollout |
| `AK_ISP_set_blc_attr` behaves differently from the source we read | the `vendored-sdk-source-is-not-the-shipped-lib` finding applies: verify on hardware, trust the daemon counters over the source |
| Exposure calibration yields no clean conversion | Phase 2b ships Mode only; limits stay unexposed |
| An apply storm hits the main stream | commit-on-release, and the `main-keyframes-exceed-shm-slot` ceiling is a known separate defect |
| A new `[imaging]` key breaks A/B rollback | code defaults, no new required section |
