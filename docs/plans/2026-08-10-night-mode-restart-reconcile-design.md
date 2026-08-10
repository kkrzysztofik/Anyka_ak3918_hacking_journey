# Night-Mode Restart Reconcile — Design

Date: 2026-08-10
Status: approved; implementation plan to follow
Branch context: `fix/night-mode-restart-reconcile` (off `main`)

## Problem

At 23:12 local on `192.168.30.121`, with `ir_cut_filter = "AUTO"` and a healthy AE
luma feed, night mode had not triggered. No day/night transition had been applied
since the process restarted at 19:52:53.

`NightModeController::new` seeds its state from the `IR_LED` GPIO
(`read_initial_state`, added in `79b68be1`). A leftover `IR_LED = 1` seeded the
state to `Night`. `spawn_auto_loop` performs no start-up apply in AUTO, and
`decide` returns `None` whenever `target == state.current`. So the controller
concluded "already at night" and never called `apply` — the only path to
`set_ir_filter` → `CMD_ISP_SET_IR_FILTER` → `ak_vi_switch_mode`
(`handlers_isp.c:113`, the sole call site).

**GPIO state survives a restart; the vendor daemon's ISP state does not.** The
daemon restarted alongside `onvif-rust` (PID 562) and came back in day mode.
`VI_MODE_DAY` is the zero value of `enum video_daynight_mode` (`ak_vi.h:25-28`)
and `handle_vi_open` never calls `ak_vi_switch_mode`, so a fresh VI sits at the
SDK default. Nothing ever told it otherwise.

The seed was correct about the lamp and wrong about everything else.

### How the lamp got stuck on

| Time | Event |
|---|---|
| 18:20:53 | `watch121.log`: `ir=0 ain=613` — daylight, filter in day position |
| 18:38:26–18:38:50 | three consecutive `get_ae_luma IPC failed error=Operation timed out` |
| ~18:39 | streak hits `AE_FAIL_STREAK_MAX` → falls back to `ain0` |
| | config still carried `day_threshold = 662` / `night_threshold = 652` — **`.198`'s values**. `.121` reads 548–639, so `raw <= 652` always → forced Night → `IR_LED = 1` |
| 19:12 | thresholds commented out (correct fix), but the lamp was already lit |
| 19:12 → 19:52 | seven restart cycles, venc unsafe teardown, `st=Code(1)` |
| 19:52:53 | final restart seeds `Night` from the stale GPIO; idle ever since |

### Why it stayed invisible

Three silent paths compounded:

- `get_ae_luma`'s `Ok(_) => None` arm (`imaging.rs:94`) swallows a daemon
  `STATUS_ERROR` or a short payload with no log at all. Only the transport `Err`
  arm logs, so a quiet log does not mean AE is healthy.
- `warn!("AE luma unavailable; falling back to ain0")` is invisible under
  `.121`'s `logging.level = "error"`.
- The luma value and the AUTO decision are logged nowhere. `wiki/IR-Night-Mode-
  Calibration.md:57` instructs the operator to *edit source* and add a
  `tracing::info!` to calibrate.

## Decisions

| # | Choice |
|---|---|
| D1 | `AutoState.current` becomes `Option<DayNight>`; `None` until the controller has driven the hardware itself |
| D2 | Delete `read_initial_state` and the `IR_LED` seed |
| D3 | No start-up apply in AUTO; the first determinate reading reconciles GPIO and ISP together |
| D4 | Rate-limited `info!` sampling: on classification change, or every ~10 min |
| D5 | `error!` on `get_ae_luma`'s silent non-success arm, matching the sibling `Err` arm and surviving `level = "error"` |
| D6 | One-shot `warn!` if still unsynced after the first few ticks |
| D7 | `EnvFilter` directive pins the night-mode target to `info` regardless of `logging.level` |

Ponytail cuts: no boot-time "safe default" apply; no indeterminate-timeout
policy; no `isp_synced` flag; no new config knobs; no diagnostics endpoint.

## Architecture

```text
new(AUTO):   state.current = None          # was: read_initial_state(IR_LED)
spawn_auto_loop(AUTO): no start-up apply   # unchanged

tick():
  luma = ffi.get_ae_luma()
  reading = classify(...)                  # Some(Day) | Some(Night) | None
  log_sample(luma, src, reading)           # rate-limited
  target = decide(state, reading, now, lock)
    -> reading?                            # None  => hold
    -> Some(target) == state.current?      # None  => hold; None current never matches
    -> within lock window?                 # None  => hold
    -> Some(target)
  apply(target)                            # GPIO plan + ISP switch, together
```

`decide` needs no new branch. `Some(target) != None` is already true for every
`target`, so a `None` current makes the first determinate reading transition
unconditionally — which is exactly the reconcile.

## Components

| Piece | Change |
|---|---|
| `night_mode::AutoState` | `current: Option<DayNight>`; `new()` takes `Option` |
| `night_mode::decide` | comparison becomes `Some(target) == state.current`; no other logic change |
| `night_mode::read_initial_state` | **deleted**, with its test |
| `night_mode::NightModeController` | `current_mode() -> Option<DayNight>`; add `last_sample_log: Mutex<Option<Instant>>`, `last_class`, `unsynced_warned: AtomicBool` |
| `night_mode::tick` | sample logging, one-shot unsynced warning |
| `night_mode::apply` | `info!` on transition and on the `isp` return code (currently logged only on failure) |
| `hal::anyka::ipc::imaging::get_ae_luma` | `warn!` on the `Ok(_)` non-success arm |
| `platform::anyka::imaging.rs:342` | handle `Option` from `current_mode()` |
| `logging::init_logging_impl` | `.add_directive()` pinning the night-mode target to `info` |

## Error handling

Unchanged where it already holds the mode. `None` from `get_ae_luma` still
increments the fail streak and falls back to `ain0` after three; an uncalibrated
board (both `ain0` thresholds `None`) still holds rather than guessing.

New: an indeterminate reading with `current == None` logs once at `warn!` and
keeps holding. It never guesses a mode — that is what put `.198`'s thresholds on
`.121`.

A failed `apply` still does not record the change, so the next tick retries.

## Testing

**Unit (host, `--target x86_64-unknown-linux-gnu`):**

- `decide` with `current: None` and a determinate reading returns `Some(target)` —
  the reconcile, and the direct regression test for this bug
- `decide` with `current: None` and an indeterminate reading returns `None`
- a restart with `IR_LED = 1` and a Night reading still calls `set_ir_filter`
  (mock `expect_set_ir_filter().times(1)`) — fails against today's code
- forced `ON`/`OFF` start-up apply unchanged
- sample logging fires on class change and suppresses within the window
- `get_ae_luma` non-success status emits a warning

**On `.198`:**

1. Force night, kill `onvif-rust.bin`, let `anyka-init` respawn → confirm one
   `set_ir_filter` on the first determinate tick with the lamp already on
2. Confirm `ak_vi_switch_mode` returns `0` (the stale `isp=-1` caveat in
   `wiki/IR-Night-Mode-Calibration.md:166` predates the `isp_first_vi` fix)
3. Confirm the heartbeat appears under `logging.level = "error"`

**On `.121`:** deploy, then read the heartbeat across one natural dusk. The
sample lines are the raw data for the deferred `ain0`/AE calibration.

## Out of scope

Tracked separately, from the same investigation:

- **P2** — `.121` has no `ain0` calibration; the AE-failure fallback is dead there
- **P3** — `.121`'s `ain0` polarity looks inverted (613 daylight → 726 dark) against
  `ldr_high_is_day = true`. Measure with the lamp off; it may be lamp bleed
- **P5** — a deployed config carrying another board's thresholds. `ec4da8ad` fixed
  the *default*; the deployed file still carried the values
- **P6** — the venc unsafe-teardown restart loop that exposed all of this

## Rejected alternatives

| Alternative | Why not |
|---|---|
| Start-up apply of the seeded state (`forced = Some(current_mode())`) | `apply` calls `record_change`, arming `lock_time_ms`. A daylight boot with a stale lamp drives to Night and locks it there 15 minutes — the "IR on at noon" failure, re-entered through another door |
| `isp_synced: bool` forcing the first apply | Same outcome as D1, but keeps two sources of truth plus a flag whose only job is to compensate for the first being untrustworthy |
| Boot-time floor: apply Day at start-up | Pulses the coil and toggles the lamp on every boot. `.121` restarted 7 times in 40 minutes tonight — 7 pulses to fix a case that may never occur |
| Indeterminate timeout, then fall to Day | A magic `N` and a policy knob for a case we have no evidence of |
| Per-tick `debug!` behind a live switch | Requires someone present at failure time. `set_log_level` (`logging/mod.rs:93`) can already hot-reload the filter but has no caller; wiring a trigger is a larger change than a rate-limited heartbeat |
| Read ISP day/night state back from the daemon | No such SDK getter; would need a new IPC command to avoid a one-line fix |
