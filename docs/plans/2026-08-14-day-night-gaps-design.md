# Day/night: closing the gap to the stock firmware — design

Date: 2026-08-14
Status: approved, ready for implementation planning
Background: `docs/reference/vendor-day-night-implementation.md`

## Problem

Two distinct problems came out of the 2026-08-13/14 investigation, and only one of them
was what we set out to find.

1. **The night image is worse than the stock firmware's.** On the vendor software the dark
   scene is clearly visible; on ours it is not. This is the problem that matters.
2. **Switching-logic gaps.** Our AUTO loop is missing four mechanisms the vendor has: an
   AWB gate, multi-sample voting, a stability check, and an early lock release. It also has
   no ISP mode read-back, and discards most of the AE run-info struct.

3. **Amended 2026-08-14 (evening): gap (2) is a live defect, not a theoretical shortfall.**
   `.127` oscillates day↔night on a 30-minute cycle all night, spending half of it in Day
   mode at night, i.e. showing black frames. Cause: the AE luma we switch on is a regulated
   servo output, and in Night mode our own IR lamp drives it back to its setpoint (~50, well
   above `ae_day_threshold`), so the camera reads "day", switches, goes dark, reads "night",
   and switches back. Full evidence in
   `docs/reference/vendor-day-night-implementation.md` §10.

These are independent. **None of the (2) gaps would make the night image brighter** — they
govern *when* we switch, not what the image looks like afterwards. That distinction drives
the whole design: two tracks, sharing one new capability.

Finding (3) does not change the design; it changes the priority and the justification.
Track B stops being "catch up with the vendor" and becomes the fix for a reproducible
defect on every board whose IR lamp works. It also settles two questions the design left
open: **B1 is the mechanism that matters** (chroma collapses under IR, so the colour bins
refuse consent however bright the frame looks), and **B2 voting is irrelevant to this bug**
(all N samples agree on the same lie). Threshold tuning cannot help either — no threshold
separates a servo output pinned at its setpoint in daylight from the same output pinned at
its setpoint under IR.

## Constraints established before designing

- `ak_vi_switch_mode` already applies the full ISP profile. `isp_switch_mode` walks all 25
  modules in the block, and re-applies user effects (hue/brightness/saturation/contrast/
  sharp) afterwards. **There is no per-parameter switch work we are failing to do.**
- The stock app links the low-level `AK_ISP_*` API directly and keeps programming the ISP
  after the switch. We stop at the switch. That asymmetry is the leading hypothesis for (1).
- `struct vpss_isp_ae_attr` — which we already ship in `ak_vpss.h` — carries
  `a_gain_max`, `exp_time_max`, `d_gain_max`, `isp_d_gain_max` and `target_lumiance`,
  i.e. every prime suspect. Both `ak_vpss_isp_get_ae_attr` and `ak_vpss_isp_set_ae_attr`
  are exported by the `libplat_vpss.so` we already ship.
- `isp_get_cur_lum_factor` and `isp_get_statinfo` are exported by the `libplat_vi.so` we
  already ship.
- **No lib swap is needed for any of this.** A richer `libplat_vpss.so` exists (75 exports
  vs our 38, strict superset, already present on the camera at
  `/mnt/anyka_hack/snapshot/lib/`) but reports `V1.2.1` against our `V2.1.03`. Mixing lib
  versions is what broke `.121`, and the risk buys nothing we cannot reach otherwise.

## Decisions

| decision | choice | rationale |
|---|---|---|
| scope | two tracks | image quality and switching gaps are unrelated problems |
| track A first step | dump ISP state, then decide | one unexplained observation, three plausible causes; reading beats guessing |
| ground truth | live ISP state vs the parsed conf block | no camera runs stock any more; the conf block is a usable reference |
| vendor algorithm | reimplement on our libs | avoids the mixed-lib-set hazard; keeps it testable |
| layering | C reads the SDK, Rust decides | policy belongs where the test suite is |

## Architecture

```text
                    ┌──────────────────────────────────────┐
                    │ vendor-daemon (C) — SDK access only  │
  CMD_ISP_GET_AE_   ├─ ak_vpss_isp_get_ae_attr             │
       ATTR   (A1)  │                                      │
  CMD_ISP_SET_AE_   ├─ ak_vpss_isp_set_ae_attr             │
       ATTR   (A3)  │                                      │
  CMD_ISP_GET_AWB_  ├─ isp_get_statinfo(ISP_AWBSTAT)       │
       STAT   (B1)  │                                      │
  CMD_ISP_GET_LUM_  ├─ isp_get_cur_lum_factor   (built)    │
       FACTOR       └──────────────────┬───────────────────┘
                                       │ IPC
                    ┌──────────────────▼───────────────────┐
                    │ onvif-rust — all policy, all tests   │
                    │  NightModeController: gate + voting  │
                    │  diagnostics: ISP snapshot           │
                    └──────────────────────────────────────┘
```

The daemon gains no policy. The one standing exception is `get_lum_factor`, where the
`* 40 / avg_lumi` formula is tied to SDK internals and belongs next to them.

## Track A — night image quality

### A1: read what the ISP actually holds

`CMD_ISP_GET_AE_ATTR`, empty request, response is the raw `struct vpss_isp_ae_attr` bytes.
C copies the struct out and does not interpret it; Rust decodes and exposes it through the
existing diagnostics endpoint.

Start with the AE attr alone. `3d_nr_attr` and `awb_attr` (also in `ak_vpss.h`) are added
only if the AE diff fails to explain the image.

**Deferred, not scoped:** vendoring `ak_isp_sdk.h` from `anyka_reference` to reach
sharp / WDR / BLC / CCM / NR1 / NR2 attrs. That is the escape hatch if A2 says the AE attr
is unremarkable.

### A2: the diff — a real gate

A host-side script captures the attr in day and in night mode on `.146` and prints it
beside the values parsed from the conf's `ISP_EXP` module. Three outcomes, each leading
somewhere different:

| outcome | meaning | next |
|---|---|---|
| attr matches the night conf | switch is complete; the profile is the ceiling | A3 raises it deliberately |
| attr matches the *day* conf while we believe we are in night | the switch is not sticking | different bug; stop and investigate |
| attr matches neither | something reprograms AE behind us | find the writer first |

**A3 does not begin until A2 has run.**

### A3: the fix, evidence-led

Contingent on A2. If the ceiling is the cause: `CMD_ISP_SET_AE_ATTR` plus a `[night]`
config block overriding only named fields (`a_gain_max`, `exp_time_max`), re-applied after
each night transition because `isp_switch_mode` reloads the module.

Read-modify-write only. Never construct a fresh `vpss_isp_ae_attr` — that would zero
`hist_weight`, `envi_gain_range` and `target_lumiance`, which is worse than a dark image.

## Track B — switching gaps

Both items are config-gated and **default off**, so a deploy cannot regress switching that
currently works. They are enabled one at a time on hardware.

### B1: AWB gate

`CMD_ISP_GET_AWB_STAT` returns the `total_cnt[]` bins from `isp_get_statinfo(ISP_AWBSTAT)`.
`NightModeController` holds the comparison; `night_cnt` / `day_cnt` thresholds go in
`NightConfig`, defaulting to the vendor's `[autoir]` values (1200 and 600000).

A classification commits only if the colour statistics agree. This is the vendor's second
opinion, and the reason it tolerates a `-1` luminance factor where we cannot.

**Amended 2026-08-14: measure the bins before gating on them.** The vendor's `night_cnt`
(1200) and `day_cnt` (600000) are calibration for the vendor's AWB tuning, and nothing in
the plan ever logged a bin. Shipping the gate straight onto those numbers risks the
opposite defect from the one it fixes: if the night ISP profile leaves AWB idle, the bins
never rise, the gate never permits night→day, and the camera sticks in night. Stuck-in-night
and flapping look nothing alike, but both come from the same unmeasured number, and each
wrong guess costs a full night to observe.

So Task 6 lands the IPC command *and* a log line — the ten bins on the existing rate-limited
`night sample` line, plus the same bins in `VisionDiagnostics` — with no gate and no
behaviour change. One night on `.127` then answers both questions at once: whether AWB is
alive under IR, and what the thresholds should be. Task 7 sets them from that data.

**Contingency if the bins do not separate — the lamp-off peek.** If AWB proves idle or
indeterminate under IR, the fallback is to remove the contamination rather than gate around
it: before honouring a night→day decision, switch the illuminators off, poll the source
every 200 ms until two consecutive reads differ by less than 2 (or a `peek_max_ms` ceiling
elapses), restore the lamps, and decide on the peeked value. Rate-limited to once per
`peek_interval_ms` (default `lock_time_ms`), it costs one short dark blink per lock window
and needs no vendor symbols at all. It is deliberately *not* being built now: it duplicates
what B1 is designed to do, and it is only justified if B1's measurement says the colour
statistics cannot carry the decision.

### B2/B3: voting, with the lock release folded in

`NightModeController` samples N times per decision (default 3, the vendor's `CHECK_TIME`)
and commits only on unanimity. Unanimity doubles as the lock's early release: N agreeing
samples contradicting the current mode release the lock before `lock_time_ms` expires.

**Not built:** `wait_move_stable()` and a separate `night_status_change()` escape. Voting
subsumes both — "the image is stable" and "the scene really changed" both reduce to N
consecutive agreeing samples, and that is one mechanism to test instead of three.

## Data flow

```text
tick()
 ├─ auto_enabled? ──no──> return
 ├─ vote: N × { get_lum_factor → classify }              (B2)
 │     any sample unavailable  ──> inconclusive ──> hold
 │     not unanimous           ──> hold
 ├─ AWB gate: get_awb_stat → bins agree?                 (B1)
 │     unavailable or disagree ──> hold
 ├─ decide(state, reading, lock)                         (B3 may release lock early)
 └─ apply(target)
       ├─ GPIO pre-ISP → set_ir_filter → GPIO post-ISP → IDR
       └─ re-apply [night] AE overrides                  (A3)
```

The source is chosen once per tick and then that source votes. Mixing a lum-factor sample
with an AE-luma sample inside one vote would compare two scales of opposite polarity.

## Error handling

**The rule:** every signal source has exactly one failure representation — `None` — and
`None` always means *hold the current mode*. Never a value that classifies.

This is enforced structurally, not by comment. Each source converts to `Option` at the HAL
boundary, so `classify` is only reached with a value the source has vouched for, and a
source added later inherits the guarantee.

The rule exists because it was violated on 2026-08-13: `get_lum_factor` forwarded the SDK's
`-1`, which sits below every day threshold and would have switched night vision off at
night. `read_light_sensor` carries a comment warning of exactly that hazard, and the comment
did not prevent it.

Consequences:

- **Vote** — an unavailable sample makes the vote inconclusive. It is not skipped and not
  counted as dissent; three samples where one failed is not evidence.
- **AWB gate** — unavailable statistics hold, matching the vendor, whose
  `night_mode_cmp_awb` returns `AK_FAILED` and so fails its `== STATE_DAY` test.
- **AE override** — a failed read means no write. A failed write logs and retries on the
  next transition, mirroring the existing `isp_pending` ISP retry.
- **Config** — new `[night]` keys join the ordering validation; an override outside the day
  profile's own range is rejected at load, not clamped silently.

## Testing

The layering rule *is* the test strategy. Each new C handler is ~15 lines of "call the SDK,
copy the struct, send it" with no branching beyond failure guards — it either links and
returns bytes or it does not. No C test harness is proposed; that would be a larger
commitment than the code it protects. All logic that can be subtly wrong lives in Rust.

Rust unit tests (mockall):

| behaviour | why it earns a test |
|---|---|
| vote unanimous → commits | happy path |
| vote split → holds | an off-by-one silently disables voting |
| vote with one unavailable sample → inconclusive | the tempting bug is to vote on the other two |
| AWB bins disagree → holds despite confident luma | the gate is the point of B1 |
| AWB unavailable → holds | the `-1` class of bug, one layer up |
| unanimity releases the lock early | otherwise B3 is dead code that looks alive |
| `vpss_isp_ae_attr` decode | a field-offset slip reads `a_gain_max` as `d_gain_min` |
| AE override preserves untouched fields | guards the read-modify-write rule |

Host-side: the conf-diff script carries one `assert`-based self-check that parsing the
shipped `isp_gc1084.conf` yields 25 modules ending at the block boundary — the invariant
that established the schema, and what breaks if a camera ships a different conf layout.

Hardware: A2 is itself the validation gate for track A. Track B deploys with both flags
off, flags flipped one at a time, `night sample` lines watched across one dusk.

**Not tested:** the vendor's numeric thresholds. `night_cnt = 1200` and `day_cnt = 600000`
are the vendor's calibration for this sensor; asserting them would encode a second copy of
the config, which is already the source of truth.

## Out of scope

- `WHITE_LED` as a night illuminator — excluded by request, tracked separately.
- The `profiles.toml` fps mismatch (declares 30, sensor delivers 15 day / 10 night).
  Pre-existing and orthogonal; noted in the reference doc.
- Swapping `libplat_vpss.so`.
- Porting the full stock ISP programming surface.
