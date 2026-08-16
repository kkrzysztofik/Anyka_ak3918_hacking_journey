# PTZ Position Tracking — Design

Date: 2026-08-16
Scope: `onvif-rust` (`platform/anyka/ptz_actor.rs`, `ptz_control.rs`, `onvif/ptz/`,
`config/types.rs`). No `www` changes.
Follows `2026-08-16-ptz-diagnostics-pane-design.md`, which surfaced the tracked position
in the WebUI and thereby made it obvious that the tracked position is always `0,0`.

## Problem

Position *is* tracked — but only on the code path nobody uses.

`do_move` (absolute) dead-reckons correctly: it commits the clamped per-axis target once
`ptz_wait_turn` returns un-interrupted (`ptz_actor.rs:306-309`). `do_continuous` commits
nothing — it sets velocity, drains the axis waits, records `step_pos`, and returns
(`ptz_actor.rs:380-392`). The WebUI only ever calls `continuousMove` + `stop`;
`ptzService.ts` has no `absoluteMove` at all.

So every real user interaction moves the lens and updates zero state. Three consequences
from that one root cause:

- `GetStatus` reports `0,0` forever.
- Presets are silently broken: `set_preset` snapshots the tracked position, so all three
  preset slots in `LiveViewPage.tsx` store the same point and every `GotoPreset` is a
  no-op.
- A restart re-centers the lens, losing whatever view the camera was aimed at.

### There is no hardware readback

`TurnOutcome.step_pos` looks like feedback but is decorative. It comes from
`MOTOR_GET_STATUS`, which on V500 hardware returns success and writes nothing (see the
`v500-motor-ioctls-are-a-silent-noop` finding), so it reads back the zeroed
`MotorMessage` every time. On the interrupt path our own driver makes it worse:
`wait_event_interruptible` returns `NotifyData::default()` (`driver.rs:362`), discarding
`remain_steps` before any caller can look at it.

The one real absolute reference the hardware offers is the limit switch. `calibrate()`
already drives each motor to its physical stop (HIT event) and back to mid-travel; it
runs at every `open()` via `check_self`. That is a homing routine, currently used only as
a noisy self-check.

Everything below is therefore dead reckoning from commanded motion, anchored by homing.

## 1. One integration helper, two callers

`ptz_actor.rs` gains a single helper: for each axis started by a command, add
`elapsed.as_secs_f32() * deg_per_sec * sign` to `state.position`, clamped to the existing
`PTZ_{MIN,MAX}_{PAN,TILT}_DEGREES`.

Two call sites, because they are the same bug:

- `do_continuous` — `started_at` before the first `ptz_start_turn`, `stopped_at` after
  the last `ptz_wait_turn` returns, then integrate.
- `drain_started_turns` — on `interrupted`, integrate instead of `break`ing with the
  position left untouched. Today an interrupted absolute move loses all the motion that
  actually happened.

**One `Instant` pair per command, not per axis.** When two axes are jogged together the
waits drain sequentially: motor B physically runs for the whole of A's wait, and B's own
`ptz_wait_turn` then returns almost immediately, so timing each axis separately would
credit B with ~0°. Both motors issue `TURN_STOP` within one loop iteration of each other,
so a shared window is both the accurate model and the smaller code.

**Wall-clock at the client would be wrong; this clock is free.** The stop path is click →
SOAP → `ptz_interrupt` → up to one 100 ms driver poll tick → `TURN_STOP`. Measuring from
the ONVIF layer would smear that latency into every jog. `do_continuous` already sits on
both sides of the exact HAL calls, and `ptz_wait_turn` returns *after* the driver issued
`TURN_STOP`, so the window is motor-on time measured on the actor thread with no new
plumbing.

## 2. `PtzCommand::Home`

New actor command: reply on acceptance, then run `ffi.ptz_check_self(...)` (the existing
limit-switch sweep), then commit `position = HOME`, `velocity = STOP`.

Replying on acceptance is required, not stylistic: the sweep runs to a physical limit and
back and can outrun `PTZ_CMD_TIMEOUT` (15 s), handing the client a bogus `Timeout` fault.
`MoveTo` already replies on acceptance for the same reason, and ONVIF treats these moves
as asynchronous.

`PTZControl` gains `async fn home()`. `StubPTZControl` snaps to `HOME`.

## 3. `GotoHomePosition` re-homes

`ops/status.rs::goto_home_position` calls `ptz.home()` first, then keeps its existing
`move_to_position(stored_home)`.

This resolves a spec conflict rather than papering over it: ONVIF also has
`SetHomePosition`, so home is not necessarily `0,0`. Sweeping to true center and *then*
dead-reckoning to the stored home both re-zeros drift and honours the spec — and it is
the most accurate way to reach that stored point, because the dead-reckoned leg starts
from a just-calibrated origin. When home is the default `0,0` the second leg falls below
`PTZ_MIN_MOVE_THRESHOLD` and costs nothing.

`GetStatus` needs no change: it already calls `sync_position_from_platform`, so it becomes
truthful the moment the platform layer does.

## 4. Calibration constants

Two fields in the existing `[ptz]` config section (`config/types.rs:599`), beside
`pan_speed`/`tilt_speed`: `pan_degrees_per_sec`, `tilt_degrees_per_sec`, validated
positive, threaded into `PtzActorState`.

Measuring them needs no stopwatch and no visual judgement: issue an absolute move of a
known angle and time `start_turn` → `wait_turn` return. The motor runs a commanded step
count and the kernel signals completion, so `deg/s = commanded_degrees / elapsed`. One
telnet session, two numbers, both axes.

`ponytail:` comment naming the ceiling — the driver sets speed once
(`DEFAULT_SPEED = 100`) and nothing varies it, so a constant is exactly as expressive as
the hardware currently is. If variable speed ever lands, the constant becomes a function
of `speed_step`.

## 5. Position persistence

**File.** `ptz_position.toml` holding `pan`/`tilt` in degrees, in the directory the app
already resolved its config file to. No new config knob, no deploy change; it inherits
whatever the A/B slot convention already does for config. Consequence, accepted: if
config is per-slot, a firmware upgrade loses the saved position — an upgrade reboots and
re-homes anyway, so the cost is one re-jog on a rare event.

**Writes happen on the actor thread, after each completed movement command.** No
debounce, no dirty flag, no background task. The actor is a dedicated OS thread that
already blocks on motor waits, so blocking file I/O there is free, and `dispatch_batch`'s
supersede semantics already collapse a burst of jog clicks into one completed command —
writes track user actions, not clicks. Temp-file-plus-rename for atomicity, since power
loss mid-write is the realistic corruption path on this hardware.
`ponytail:` comment naming debounce as the upgrade path.

**Restore** goes in the async platform init, after `init_ptz_control` reports success:
read the file, and if the target exceeds `PTZ_MIN_MOVE_THRESHOLD`, issue an ordinary
`move_to_position`. No special-case path — it is the same dead-reckoned move as any
other, and the most accurate one available, because `check_self` just established true
center as its origin.

**Every failure is non-fatal.** Missing, unparseable, or out-of-range → log, clamp or
ignore, stay centered. Boot never fails because of a state file. If PTZ never opened,
nothing is read and nothing is written.

Ordering is load-bearing but free: the restore read happens before any motion, and the
first write can only follow a completed command, so no window exists where boot state
clobbers the saved target. Pinned by a test, because a future "write initial state at
startup" change would silently break exactly this.

Error does not compound across reboots: every boot re-establishes true center physically
before restoring, so the error is one session's drift plus one restore move, and the next
boot wipes both.

## 6. Presets, end to end

The frontend is already complete — `LiveViewPage.tsx` has three preset slots wired
through TanStack Query to `getPresets`/`setPreset`/`gotoPreset`/`removePreset` with cache
invalidation on mutation. It has been sitting on top of a broken backend. No `www` work.

Three backend gaps:

**a) Stale position at snapshot time.** `set_preset` snapshots the *state manager's*
cached position. Only `absolute_move`, `relative_move` and `GetStatus` call
`sync_position_from_platform`; `continuous_move` and `stop` do not
(`ops/movement.rs:161,206`). Without this, jog → "save preset" snapshots a stale value
even after §1 lands, and the whole fix looks like it did not work.

Fix: one `sync_position_from_platform` call in `stop` — where motion ends, repairing
every downstream reader at once, rather than in `set_preset` alone.

This is race-free for a non-obvious reason worth a test: when `Stop` arrives the actor is
blocked inside `do_continuous`'s wait; the interrupt unwinds it, `do_continuous`
integrates and commits, and *only then* does the actor dequeue the `Stop` batch and
reply. By the time `stop()` returns to the ONVIF layer the integrated position is already
committed.

**b) `PresetStore` is RAM-only.** Its module doc claims "persistent preset management"
and then admits "in-memory only… for potential persistence to non-volatile storage in the
future" (`store.rs:1-7`). Presets die on every restart. Folded into this design rather
than deferred: a camera that resumes its exact view after a reboot but has forgotten its
three presets is a worse half-state than either extreme. Same shape as §5 — serde to
`ptz_presets.toml` beside the config, atomic temp-and-rename, write on mutation, load at
startup, all failures non-fatal.

**c) Dead code.** `AnykaPTZControl` carries its own `presets: HashMap` + `next_preset_id`
that nothing on the ONVIF path reaches; `ops/presets.rs:35` notes `ptz_control` is
"accepted but not yet used". Two preset stores, one unreachable. Deleted, so a later fix
cannot land on the wrong one.

## Non-goals

- **The `remain_steps` probe.** Whether the kernel posts a stop event carrying usable
  `remain_steps` after `TURN_STOP` is unknown; if it does, `commanded − remain` would
  beat any timing constant. Speculative against a "few degrees" accuracy bar. Revisit
  only if measurement misses that bar — the design is unchanged either way, only the
  source of the delta.
- **Variable motor speed.** See §4.
- **Hardware-step → degree reconciliation.** Impossible while `MOTOR_GET_STATUS` is a
  silent no-op.

## Accuracy budget

Target: return to roughly the same view; a few degrees of drift acceptable.

- Boot: physically homed by `check_self`, error zero.
- Per jog: the accel ramp at start and decel tail after `TURN_STOP` are unmodelled; they
  partly cancel and the per-axis constant absorbs the average.
- Accumulation: within a session only. `GotoHomePosition` (§3) is the reset; a reboot is
  the other one.

## Verification

- Mock-HAL unit tests where `ptz_wait_turn` sleeps a known duration; assert position
  advanced by `rate × duration` within tolerance.
- Test pinning that an interrupted absolute move now commits partial motion.
- Test pinning the restore-before-first-write ordering (§5).
- Test pinning that jog → `stop` → `set_preset` stores the jogged position, not `0,0`.
- On hardware: the two-number `deg/s` measurement (§4), then jog → save preset → jog away
  → goto preset → confirm the view returns; reboot → confirm the view is restored.
