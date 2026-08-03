# AE Luma Day/Night AUTO — Design

Date: 2026-08-04
Status: approved; implementation plan at `docs/plans/2026-08-04-ae-luma-day-night.md`
Branch context: `feat/ir-led-support` (extends IR night-mode; replaces weak `ain0`-only AUTO sensing)

## Problem

AUTO day/night reads `/sys/kernel/ain/ain0`. On `.198` that LDR ADC has a ~20-count
span, drifts over days (306 → 648 → 730), and stays firmly in “day” under normal
room light with the shipped thresholds. Forced ON/OFF via GPIO works; AUTO does not
reliably trigger.

Stock Anyka soft-IR uses ISP AE luminance (`ak_vpss_isp_get_ae_run_info` /
`current_calc_avg_lumi`), already present in shipped `libplat_vpss.so`. We do not
expose it over IPC today.

## Decisions

| # | Choice |
|---|---|
| D1 | Prefer ISP AE luma; keep `ain0` as fallback |
| D2 | Hard lock only after transitions (`lock_time_ms`, existing `decide`) |
| D3 | Metric: `current_calc_avg_lumi` (u8) via `ak_vpss_isp_get_ae_run_info` |
| D4 | Fallback after hard AE failure streak (`N=3` ticks), not hybrid voting |
| D5 | Architecture A: thin IPC get + classify/decide in Rust |

Ponytail cuts applied: no SensorSource layer; no gain/exp payload; no AE polarity
flag; no configurable N; no VI token in this change (single-VI daemon lookup);
no C daemon test harness; no `set_ir_filter` VI-token fix in this design.

## Architecture

```
tick():
  luma = ffi.get_ae_luma()          # IPC → daemon → AE run info → u8
  if Some(luma):
    streak = 0; classify(luma, ae_thresholds)
  else:
    streak += 1
    if streak < 3: return           # hold
    else: classify(ain0, ain0_thresholds)  # existing path; None → hold
  decide + lock_time_ms → apply()   # unchanged (GPIO then ISP)
```

Forced ON/OFF / IRLamp / WhiteLight unchanged.

## Components

| Piece | Change |
|---|---|
| `vendor-daemon` | `CMD_ISP_GET_AE_LUMA = 106`, empty request; resolve sole `VD_OBJ_KIND_VI`; call `ak_vpss_isp_get_ae_run_info`; respond status + 1-byte luma |
| `ImagingHalTrait` | `async fn get_ae_luma(&self) -> Option<u8>` (`None` = any failure) |
| `night_mode::tick` | AE-first + fail streak + ain0 fallback |
| `NightConfig` | Add `ae_day_threshold`, `ae_night_threshold`; keep existing ain0 fields + `lock_time_ms` |
| `config.toml` | Populate AE thresholds after on-device measure |

**Wire format:** empty request body. Response payload: one `u8` on success.

**# ponytail:** single-VI slot lookup in the daemon. Pass a VI token only if
multi-VI ever appears. Out of scope: fixing `set_ir_filter` client omitting the
VI handle (GPIO path already succeeds; ISP returns `-1`).

## Error handling

```
Some(luma) → streak = 0; AE classify
None       → streak += 1; if streak < 3 hold; else ain0 path
decide / lock / apply unchanged
```

HAL maps IPC errors, bad status, missing VI, and short payloads to `None`.

## Testing

**Unit (host):** mock `get_ae_luma` — `Some` uses AE thresholds; 3× `None` falls
back to ain0; later `Some` clears streak. Fake-daemon round-trip for opcode 106 →
one luma byte.

**On `.198`:**
1. Measure AE luma bright vs dark-box → set `ae_*` thresholds
2. AUTO dark → night (`IR_LED=1`); after `lock_time_ms` uncover → day
3. Forced IrCut ON/OFF still works

## Out of scope

- `gpio-rf_feed` (absent on this board)
- Vendor `get_auto_day_night_level` / AWB soft-IR
- Frame-luma from encoded RTSP
- Fixing ISP `set_ir_filter` VI-token mismatch
- WebUI changes

## Rejected alternatives

| Alternative | Why not |
|---|---|
| Replace ain0 entirely | No fallback when daemon/AE down |
| Hybrid voting | Extra policy; user chose prefer+fallback |
| Call `get_auto_day_night_level` | Blocks daemon; cedes GPIO ordering |
| Frame luma in Rust | CPU + AGC feedback; no new ISP IPC |
| Pass VI token from night_mode now | Larger HAL/platform change; single-VI board |

## Calibration note

AE thresholds are board-specific (0–255). Measure on hardware before trusting
AUTO. Keep ain0 thresholds for fallback; they remain fragile but better than
nothing when AE is unavailable.
