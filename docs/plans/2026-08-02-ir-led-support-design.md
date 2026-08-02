# IR Cut Filter and LED Illumination Support — Design

Date: 2026-08-02
Status: approved (design), implementation plan pending

## Problem

Night vision is plumbed end to end and connected to nothing. Every layer of the
stack declares the state, persists it, and drops it before it reaches a pin.

| # | Defect | Location |
|---|--------|----------|
| D1 | `CMD_ISP_SET_IR_FILTER` only calls `ak_vi_switch_mode`. It changes ISP colour processing; the mechanical filter never moves and the lamp never lights | `vendor-daemon/src/handlers_isp.c:80` |
| D2 | `ImagingSettings.ir_led` is declared in the platform traits, defaulted in three places, and never written to hardware by any code path | `platform/common/traits.rs:288` |
| D3 | `ImagingConfig.ir_cut_filter` is a `bool`, so `AUTO` — a third state — is unrepresentable at the config layer | `config/types.rs:588` |
| D4 | `ImagingOptions` hardcodes `ir_cut_filter_supported: true, ir_led_supported: true` without probing for the nodes | `platform/common/traits.rs:324-325` |
| D5 | `SendAuxiliaryCommand` parses the command string, logs it, and returns success without acting — silent success for hardware that may not exist | `onvif/ptz/ops/auxiliary.rs:79` |
| D6 | The WebUI IR-cut filter UI is complete and shipped, talking to a backend that discards the value | `www/src/pages/settings/ImagingPage.tsx:347-399` |

D6 is the shape of the whole task: this is a wiring job, not an architecture
job. The abstractions exist and were abandoned before the last mile.

## Hardware ground truth

Captured live from the vendor firmware, `orig/sys/`:

```
/sys/user-gpio/    ircut_a = 0   ircut_b = 0   IR_LED = 1
                   WHITE_LED = 0  SPK_PA = 0   wifi_en = 0

/sys/kernel/ain/   ain0 = 306    ain1 = 769    bat = 3375

/sys/class/leds/   (empty)
```

Five facts follow, each of which contradicts a vendor reference source:

**H1 — node names differ from every reference.** The nodes are `ircut_a` and
`ircut_b`. `ak_drv_ir.c:14` uses `gpio-ircut_a`, `ak_misc.c:23` uses
`gpio-light_ctrl` for the lamp. Reference paths must not be copied.

**H2 — `gpio-rf_feed` is absent.** `ak_drv_ir_get_input_level` prefers that node
and falls back to `/sys/kernel/ain/ain0` (`ak_drv_ir.c:140`). On this board only
the fallback exists, so the ain path is the sole light source. The preferred
branch is dead code here and is not implemented.

**H3 — there are no status LEDs.** No `PWR_LED`, no `AP_LED`, and
`/sys/class/leds` is empty so no `wps_led`. `orig/usr/sbin/led.sh` is generic
Anyka boilerplate for a different board. The only visible-light node is
`WHITE_LED`. Status-LED support is out of scope: there is no hardware.

**H4 — the IR cut filter is a two-line H-bridge.** Both `ircut_a` and `ircut_b`
are present, and both read `0` in the capture. That is the post-pulse idle
state, observed on running hardware. Driving one line without returning both to
`0` leaves a solenoid coil energised.

**H5 — the vendor's own thresholds return an error at this board's reading.**
`get_ain_threshold` (`ak_drv_ir.c:100-110`) with vendor defaults, given
`ain0 = 306`:

```
306 > day.min (1100)                     ->  no
306 > night.min (2) && < night.max (300) ->  no, exceeds 300 by 6
306 == feature (1 or 0)                  ->  no
                                         ->  returns -1 (indeterminate)
```

An unhandled dead zone spans 300..1100 and this board sits inside it. Yet
`IR_LED = 1` at capture time, so the camera *was* in night mode and the correct
answer was "night".

The `[autoir]` block in `anyka_cfg.ini` explains why nobody noticed:
`day_to_night_lum = 6400`, `night_to_day_lum = 2048` are ISP luminance values on
a different scale from `ain0` entirely. The vendor app used frame luma and left
`ak_drv_ir`'s thresholds unexercised.

Consequence: thresholds cannot be copied from either vendor source. They must be
measured on the camera, and `-1` must mean "hold the last mode", never "day".

**H6 — do not use the SDK's own IR setter.** `camera_set_ircut`
(`ak_drv_ir.c:44`) shells out via `ak_cmd_exec` and its comment reads *"must run
cmd_serverd to receive shell-cmd"*. That daemon is not in our boot path. Direct
`fs::write` is both simpler and removes the dependency.

## Scope

In scope: IR cut filter, IR illuminator, white floodlight.

Out of scope, with reasons:

- **Status LEDs** — no hardware on this board (H3).
- **Floodlight brightness** — `WHITE_LED` is a binary `user-gpio` node and there
  is no kernel LED class, so no PWM and no `brightness` attribute.
- **Custom WebUI endpoint** — ONVIF auxiliary commands express everything the
  hardware can do, so it would carry nothing.
- **Floodlight scheduling** — pure software, unrequested.

## Architecture

One new file. Everything else is an edit to an existing hollow hook.

```
onvif-rust ---> night_mode.rs ---> fs::write  /sys/user-gpio/ircut_a, ircut_b
                              ---> fs::write  /sys/user-gpio/IR_LED
                              ---> fs::write  /sys/user-gpio/WHITE_LED
                              ---> fs::read   /sys/kernel/ain/ain0
                              ---> IPC 104    ak_vi_switch_mode
```

The vendor daemon appears exactly once, for the one operation that genuinely
needs it: `ak_vi_switch_mode` takes the VI handle the daemon owns. Every other
operation is sysfs, which any process may write.

No new IPC opcodes. No new HAL trait methods. No new modules for file I/O —
`std::fs::write` is the whole abstraction, matching `wifi.rs:486`.

### Components

| Piece | Location | Status |
|---|---|---|
| Night-mode controller | `platform/anyka/night_mode.rs` | new, ~120 lines |
| Imaging forwards intent | `platform/anyka/imaging.rs:92` | edit |
| Auxiliary command parsing | `onvif/ptz/ops/auxiliary.rs:79` | edit |
| Advertise aux commands | `onvif/ptz/ops/config.rs:58` | edit |
| Capability probe | `platform/common/traits.rs:324-325` | edit |
| Config widening | `config/types.rs:588` | edit |
| `sendAuxiliaryCommand` | `www/src/services/ptzService.ts` | new function |
| Illumination card | `www/src/pages/settings/ImagingPage.tsx` | new card |

Mutual exclusion is one `tokio::sync::Mutex` around `apply(mode)`. The AUTO
tick, ONVIF `SetImagingSettings`, and `SendAuxiliaryCommand` all pass through
it. An actor was considered and rejected: `ptz_actor.rs` needs one because PTZ
has queued motion with mid-flight cancellation. Night mode has two states and a
timer, and a mutex gives identical exclusion in one line.

## Data flow

### Read path — AUTO tick, 2 s

```
/sys/kernel/ain/ain0  ->  raw value  ->  threshold compare  ->  day | night | indeterminate
```

Indeterminate holds the current mode. Poll interval is a constant, not config:
at 2 s the ~12 ms scheduler cost of an await on this SoC is noise, and the lock
window dominates responsiveness regardless.

### Decision

Raw reading -> hysteresis -> `lock_time_ms` guard -> target mode. No transition
occurs while locked. This is the only thing preventing a camera that oscillates
at dusk.

### Write path

Order is the vendor's (`main.c:740-752`) and is load-bearing:

```
to DAY:                    to NIGHT:
  1. ircut -> day            1. IR_LED on
  2. ISP   -> DAY            2. ISP -> NIGHT
  3. IR_LED off              3. ircut -> night
  4. settle 300 ms           4. settle 300 ms
```

The lamp turns on before the ISP switches and off after it, so no frame is
captured dark.

GPIO writes precede the IPC call for a second reason: GPIO state is durable and
survives a daemon restart, ISP state is not. A daemon bounce then costs one tick
of wrong-looking image rather than a stuck filter.

### `ircut` write

The two-line pulse, confirmed by H4:

```
write(ircut_a, level); write(ircut_b, !level);
sleep(10 ms);
write(ircut_a, 0); write(ircut_b, 0);
```

The trailing zero-writes are coil protection, not cleanup.

### Polarity

`ak_misc_set_video_day_night` computes `ir_switch_val` in all four `day_ctrl`
branches (`main.c:720-732`) and never reads it. The vendor's four-mode
HH/HL/LH/LL table therefore collapses to two distinct behaviours, with the
second polarity bit computed and discarded.

Two booleans instead — same knob count, both live:

```toml
[imaging]
ir_cut_filter = "AUTO"        # widened from bool; IrCutFilterMode: ON | OFF | AUTO
ir_led = false

[imaging.night]
ldr_high_is_day = true
ircut_high_is_night = true
day_threshold = 1100          # MUST be calibrated on hardware, see H5
night_threshold = 300         # MUST be calibrated on hardware, see H5
lock_time_ms = 900000
```

`ir_cut_filter` widens from `bool` to the existing `IrCutFilterMode`
(`onvif/types/common.rs:1833`), which already derives `Serialize`,
`Deserialize`, and `Default = AUTO` with no `tt:` renames on its variants, so it
is TOML-clean as-is. The bool is precisely why `AUTO` was never implementable;
widening it keeps one source of truth rather than adding a parallel `mode` key.

`settle_ms` and `poll_interval_ms` are constants. Polarity and thresholds vary
per board; a solenoid settle time does not.

## Error handling

### Capability probe

One `stat()` of both `ircut` nodes at init answers three questions, replacing
the hardcoded `true`s at `traits.rs:324-325`:

| Nodes present | Result |
|---|---|
| `ircut_a` + `ircut_b` | two-line mode |
| either one | one-line mode |
| neither | `ir_cut_filter_supported = false` |
| `IR_LED` absent | `ir_led_supported = false` |

This board reports two-line (H4). The probe is kept rather than hardcoded
because it is the same `stat()` the capability report needs anyway.

### The one failure that damages hardware

In two-line mode the coil is energised between the pulse write and the idle
write. **No `?` operator in that window.** The two zero-writes run
unconditionally and errors are propagated afterward. Not a `Drop` guard, not a
`scopeguard` dependency — simply no early return across four statements.

### Everything else fails soft

| Failure | Response |
|---|---|
| `ain0` read fails or returns indeterminate | hold current mode, `warn`, retry next tick — never default to day |
| IPC 104 returns `VD_STATUS_STALE_EPOCH` | daemon restarted; GPIOs already correct, retry ISP next tick |
| GPIO write `ENOENT` | mark that capability unsupported, stop retrying, log once |
| `SendAuxiliaryCommand` for absent hardware | ONVIF fault — the current silent success is D5 |

## WebUI

The IR cut filter UI is already complete: `IrCutFilterMode` type
(`imagingService.ts:9`), parsing (`:138`), serialization (`:171`), options
(`:307`), and a Moon-icon "Infrared Settings" card with Day/Night/Auto labels
(`ImagingPage.tsx:347-399`). Wiring the backend switches it on with no frontend
change.

One three-line fix: `ImagingPage.tsx:389` falls back to showing all three modes
when `irCutFilterModes` is absent. Once the capability probe can report
unsupported, distinguish empty array (hide the card) from undefined (fallback).

New work is one service function and one card:

- `ptzService.ts` gains `sendAuxiliaryCommand(profileToken, cmd)`, matching the
  shape of the existing calls in that file.
- `ImagingPage.tsx` gains an "Illumination" card with two switches, IR Lamp and
  White Light. `@radix-ui/react-switch` is already a dependency and
  `SettingsCard` is used six times in that file already.

Commands are the ONVIF-standard strings: `tt:IRLamp|On`, `tt:IRLamp|Off`,
`tt:IRLamp|Auto`, `tt:WhiteLight|On`, `tt:WhiteLight|Off`.

## Testing

`plan(target, cfg) -> Vec<Step>` is pure and separate from `execute(steps,
&paths)`. The split removes the need for a recorder or filesystem mock, so it is
net fewer lines than mocking, and it makes ordering directly assertable.

Path injection follows `wifi.rs:334` — two base directories, `user_gpio` and
`kernel_ain`, joined with fixed node names. Tests point them at a
`tempfile::tempdir()`, as `monitor.rs:242` and `storm.rs:163` already do.

| Test | Kind | Asserts |
|---|---|---|
| transition ordering | pure | `plan(Night)` equals the full expected `Vec`, including trailing zeros |
| polarity inversion | pure | flipping `ircut_high_is_night` flips exactly that step |
| hysteresis and lock | pure, injected clock | no transition inside `lock_time_ms`; dusk flicker yields one transition |
| threshold mapping | pure | raw `ain0` values map to day/night/indeterminate, including the H5 dead zone |
| line-mode detection | tempdir | 0/1/2 nodes give unsupported / one-line / two-line |
| execute writes | tempdir | final file contents match the plan |
| read failure | tempdir | missing `ain0` holds mode, does not reset to day |

No hardware is required for any of these, and `mockall` is not involved because
nothing new crosses the HAL trait.

### Hardware verification

None of the above proves the board is wired as assumed. On `192.168.2.198:24`:

1. `cat /sys/kernel/ain/ain0` covered and uncovered — **record both values and
   set `day_threshold` / `night_threshold` from them.** Per H5 the vendor
   defaults are wrong for this board. This step is a prerequisite for AUTO, not
   a check on it.
2. `ls /sys/user-gpio/ircut_*` — confirm two-line before trusting the probe.
3. Force `ir_cut_filter = "OFF"` — the filter audibly clicks and the IR LEDs are
   visible through a phone camera.
4. Force `ir_cut_filter = "ON"` — it clicks back; confirm no node is left at `1`.
5. `ir_cut_filter = "AUTO"`, cover the sensor — one transition, not oscillation.

Step 1 gates the feature. Step 5 is the one that cannot be faked in tests.

`orig/` is a flattened capture and its symlinks were lost, so steps 1 and 2
verify against the live camera before any code trusts a path. The vendor's own
`stat()`-based detection exists precisely because these nodes vary per board.

### Hardware verification results (2026-08-02)

| Check | Result |
|---|---|
| GPIO nodes | `ircut_a`, `ircut_b`, `IR_LED`, `WHITE_LED` present (two-line) |
| Dark-box `ain0` | ≈648 (covering only the camera lens barely moved the reading; LDR is not behind the optics) |
| Room-uncovered `ain0` | ≈670 (evening lighting; earlier afternoon samples were ≈710) |
| Thresholds shipped | `day_threshold = 662`, `night_threshold = 652` |
| Force OFF / ON | IR LED tracks night/day; `ircut_a`/`ircut_b` idle at `0` after each pulse |
| Aux lamps | `tt:WhiteLight\|On/Off`, `tt:IRLamp\|On/Off` OK; `tt:Wiper\|On` faults |
| AUTO dark → night | Observed (`IR_LED=1` at `ain0≈648`) |
| AUTO unlock → day | Observed after `lock_time_ms` (`IR_LED=0` at `ain0≈673`) |
| ISP `ak_vi_switch_mode` | Returns `-1` over IPC; GPIO path still completes |

The ADC span between dark and room light on this board is narrow (~20 counts).
Retune if ambient lighting changes substantially.

## Rejected alternatives

| Alternative | Reason |
|---|---|
| `NightModeActor` with channel and command enum | ~140 lines for exclusion a `Mutex` gives in one; `ptz_actor.rs` needs an actor for queued cancellable motion, which night mode has none of |
| `CMD_ISP_GET_LDR_LEVEL` daemon opcode | `ak_drv_ir_get_input_level` is `fopen` on sysfs (`ak_drv_ir.c:140`); the IPC round-trip, trait method, stub, and mock were all justified by a false premise |
| `platform/anyka/gpio.rs` helper module | `std::fs::write` is the abstraction; `wifi.rs:486` already does this inline |
| Calling `ak_drv_ir_set_ircut` | Shells out through `cmd_serverd`, which is not in our boot path (H6) |
| Frame-luma day/night detection | Fights AGC, and the IR cut filter changes the luma being measured |
| Shared GPIO crate for `anyka-init` | Moot: status LEDs are out of scope (H3) |
