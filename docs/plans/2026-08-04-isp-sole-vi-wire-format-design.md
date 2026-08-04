# ISP Sole-VI Wire Format — Design

Date: 2026-08-04
Status: approved; implementation plan at `docs/plans/2026-08-04-isp-sole-vi-wire-format.md`
Branch context: `feat/ir-led-support` (fixes ISP day/night / imaging effects after IR GPIO landed)

## Problem

GPIO night works (`IR_LED=1`, ircut pulse) but the stream stays day-coloured under
IR. Logs show `ISP day/night switch failed … isp=-1`.

Root cause: `CMD_ISP_SET_IR_FILTER` (and shared ISP effect cmds) expect
`[u64 vi_token][i32 value]` (12 bytes). Rust `ImagingHalTrait` already sends only
the `i32` (4 bytes). The daemon rejects short payloads / never resolves a VI, so
`ak_vi_switch_mode` never runs. Brightness/contrast/saturation/sharpness share
the same broken 12-byte expectation via `handle_isp_effect`.

## Decisions

| # | Choice |
|---|---|
| D1 | Fix **all** ISP setters that use VI (effects + IR), not IR alone |
| D2 | Daemon **sole-VI** lookup (same pattern as `CMD_ISP_GET_AE_LUMA`) |
| D3 | **Hard cut** wire format to `[i32 value/mode]` only — no dual-accept |
| D4 | **Daemon-only** functional fix — Rust payloads already correct |
| D5 | Leave `set_wdr` no-op; no platform / `night_mode` / HAL trait changes |

Ponytail cuts: no VI threading through Rust; no `vd_obj_first` in `globals`;
no dual wire format; no protocol.h churn; no C unit harness; no multi-VI.

## Architecture

```
Rust ImagingHalTrait (unchanged payloads)
  set_*effect(value) / set_ir_filter → i32 LE (4 bytes)
       │
       ▼
vendor-daemon handlers_isp
  req_len < 4 → STATUS_ERROR
  vi = first live VD_OBJ_KIND_VI  // file-local helper shared w/ get_ae_luma
  vi == NULL → STATUS_ERROR
  else → ak_vpss_effect_set / ak_vi_switch_mode → status
```

GPIO ordering in `night_mode::apply` unchanged (GPIO first, ISP best-effort).

## Components

| Piece | Change |
|---|---|
| `cross-compile/vendor-daemon/src/handlers_isp.c` | Hard-cut `i32` + sole-VI in `handle_isp_effect` and `handle_isp_set_ir_filter`; update wire-format comments; optional file-local `isp_first_vi` folded into `get_ae_luma` |
| Rust / platform / HAL / WDR | No change |

`# ponytail: sole-VI, pass token only if multi-VI appears.`

## Error handling

| Case | Response |
|---|---|
| `req_len < 4` | `STATUS_ERROR` |
| no live VI | `STATUS_ERROR` (+ warn) |
| SDK non-zero | pass through as status (unchanged) |

No retries; no token/epoch path on these cmds after the hard cut.

## Testing

**Host:** `make -C cross-compile/vendor-daemon release`.

**On `.198`:** deploy vendor-daemon; force `IrCutFilter` OFF/ON on `VideoSource_1`;
confirm `IR_LED` tracks **and** stream looks night/day; optional brightness smoke.

**Success:** no `isp=-1` on healthy attach with a live VI; video mode matches GPIO night/day.

## Out of scope

- Multi-VI tokens / Rust VI plumbing
- WDR enablement
- IPC attach-flap / "not attached" races
- WebUI
- C daemon unit harness
