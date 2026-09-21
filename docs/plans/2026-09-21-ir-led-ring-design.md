# Replacement IR LED ring board

Date: 2026-09-21
Status: design approved, no layout started

Supersedes the locked decisions in `ir_design/SPEC.md` where the two conflict.
The spec's mechanical envelope, 5 V assumption, aluminium substrate, all-IR
channel choice and 100 mA target survive unchanged. Its electrical section —
2S2P strings, AL8860 buck, per-branch ballast resistors, two independent
drivers — does not.

## Problem

The stock ring board (`RZ-XHR(08SG)-C4`) carries 8 emitters: 4 white on `HB`
and 4 IR at 850 nm on `IR`, each string switched by an S8050 NPN low-side
transistor through a ballast resistor. There is no current regulation. The
resistors are suspected of running at their package's thermal limit.

The camera's night image is dark. The measured reason is not a broken board —
it is an undersized array. See `[[white-led-is-the-usable-night-illuminator]]`
and `[[isp-night-profile-lowers-gain]]`: exposure headroom is already spent
(`ae_exp_time_max` is ~94 % of frame time in both ISP profiles), the night
profile caps analogue gain at 10 against day's 24, and there is no software
lever that raises IR output — `IR_LED` is a binary GPIO already at 100 % and
these boards have no `/sys/class/pwm`.

So the only remaining lever is the illuminator itself.

## Accepted risk: all-IR is the weaker channel

The design drops the white channel and makes all 8 emitters IR. This is a
deliberate choice for covert night operation, made with the following
measurement on the table.

Mean frame luma (`ffmpeg signalstats YAVG`, 0–255), per camera:

| camera | baseline | `IR_LED=1` | `WHITE_LED=1` |
|---|---|---|---|
| `.146` kitchen, indoor | 0.73–0.90 | 2.86–3.04 | **110** |
| `.198` room, some ambient | ~22 | ~27 (+4.5) | **~100 (+78)** |

Same emitter count, same rough drive current. The white channel is roughly 36×
more effective at raising frame luma than the IR channel on this sensor, which
is a GC1084 whose quantum efficiency at 850 nm is a fraction of its visible
peak.

This board delivers 4× the stock IR array's radiant output (8 emitters ×
100 mA versus 4 × 50 mA). Against the `.146` numbers that projects to roughly
12 YAVG where the existing white channel already delivers 110. The covertness
argument is real and the tradeoff was accepted explicitly; it is recorded here
so a later reader does not mistake it for an oversight.

## Topology

**One boost LED driver, 8 emitters in a single series string at 100 mA, with a
switchable bypass across four of them for half power.**

### Why not the spec's 2S2P buck

A 2-series pair of 850 nm emitters is 3.4–4.0 V at 100 mA (datasheet Vf is
1.7–2.0 V at that current; the commonly quoted 1.4–1.6 V figure is binned at
20 mA). Vf falls about 2 mV/°C per junction, so a hot string sags to ~3.9 V.
Against a 5 V rail pulled to ~4.85 V under load, the entire regulating element
gets **0.9–1.1 V of headroom**. That is marginal for a buck, marginal for a
linear regulator, and lets Vf binning swing the current ±25 % through a plain
resistor. The AL8860's own characteristic curves are taken at VIN = 16 V with
3 LEDs; 5 V in / 4 V out is the far corner of its envelope.

Going the other way — one emitter per branch — is worse: 3.0–3.3 V dropped at
100 mA across 8 LEDs burns **2.6 W** in ballast, more than the LEDs consume.

### Why series, and why 8

A boost converter can only regulate when the string voltage exceeds the input
voltage. This is explicit in the PAM2803 datasheet as `VIN max = VF − 0.2 V`.
Eight emitters in series is 13.6–16.0 V, comfortably above 5 V, duty ≈ 0.69.

A single series string also makes current matching exact by construction. That
removes the four per-branch ballast resistors the spec called for, and with
them the current-hogging failure mode those resistors existed to prevent.

`PAM2803 is ruled out` on its own numbers: SW pin absolute maximum is 6 V,
below the string voltage.

### Operating modes

| Mode | `IR` | `HB` | String | Vstring | LED power | Rail draw |
|---|---|---|---|---|---|---|
| Off | 0 | × | — | — | 0 | ~0 |
| Half | 1 | 0 | 4 LEDs | 6.8–8.0 V | 0.8 W | ~188 mA |
| Full | 1 | 1 | 8 LEDs | 13.6–16.0 V | 1.6 W | ~376 mA |

Per-emitter current is 100 mA in both modes; the driver regulates it and the
bypass only changes how many emitters are in circuit.

### The bypass

An N-MOSFET (Q1) shorts LEDs 5–8, the four nearest the sense resistor, so its
source sits ~0.1 V above ground and needs no floating gate drive. An NPN (Q2)
inverts `HB` onto that gate, with a 100 k pull-up to the 5 V rail:

- `HB` low → Q2 off → gate at 5 V → Q1 on → 4 LEDs bypassed → **half power**
- `HB` high → Q2 on → gate at ~0.1 V → Q1 off → 8 LEDs lit → **full power**

A P-FET would avoid the inverter but its source would float at ~8 V, out of
reach of a ground-referenced 3.3 V GPIO.

**R5, 4.7 Ω in series with Q1, is not optional.** Turning Q1 on while the
converter runs dumps C2 from 16 V into a 4-LED string: `½ × 1 µF ×
(16² − 8²)` ≈ 96 µJ in a few microseconds, a multi-amp spike through the
emitters on every transition. R5 caps it, and costs 47 mW in half power and
nothing in full power.

Failure modes are benign in both directions: Q1 shorted leaves the board stuck
at half power, Q1 open leaves it stuck at full power. Neither damages anything.

## Firmware consequence

`cross-compile/onvif-rust/src/platform/anyka/night_mode.rs:849` — `plan()`
writes only `Node::IrLed` and the two ircut nodes. `Node::WhiteLed`, which is
the `HB` line, is reachable only through `set_white_light()` at
`platform/anyka/imaging.rs:355`, exposed as the ONVIF `tt:WhiteLight` auxiliary
command. Nothing calls it on a day/night transition.

With the inverted bypass polarity, that means **unmodified firmware runs the
board at half power** — 4 emitters at 100 mA, which is 2× the stock IR array
rather than the designed 4×.

Reaching the designed output therefore requires adding `Step::Write` entries
for `Node::WhiteLed` to `plan()`. This is a small change the codebase is
already shaped for, and the objection previously recorded against it — that a
white floodlight is not a sane default on an outdoor camera — no longer
applies, because the second bank is now infrared.

This firmware change is a required deliverable of this design, not an optional
follow-up.

## Bill of materials

21 designators.

| Ref | Part | Note |
|---|---|---|
| U1 | Boost LED driver, SOT-23-6, Vout ≥ 20 V, **OVP required** | TPS61165 or HT7938A (JLCPCB C259955); datasheet verification outstanding |
| L1 | 33 µH shielded, Isat ≥ 600 mA | back side; height against the body, not the dome |
| D1 | Schottky 30 V 0.5 A, SOD-123 | |
| C1 | 10 µF 25 V 0805 | input |
| C3 | 100 nF 0402 | input bypass |
| C2 | 1 µF 25 V 0805 | output; deliberately small to cap bypass inrush. Ripple `I·D/(f·C)` = 69 mV |
| R1 | Sense, `Vfb / 0.1 A` (2 Ω for a 200 mV reference) | sets current for all 8 emitters |
| Q1 | Logic-level N-MOSFET, 30 V, SOT-23 | bypass |
| Q2 | NPN, SOT-23 | inverter |
| R2 | 100 kΩ | gate pull-up to 5 V |
| R3 | 10 kΩ | Q2 base, from `HB` |
| R4 | 100 kΩ | Q2 base pull-down |
| R5 | 4.7 Ω 0805 | bypass inrush limiter |
| D2–D9 | 850 nm IR, 2835 or 3535 | package pending measurement 14 |
| J1 | 4-pin header | stock footprint, `+ − IR HB` |

Inductor sizing against the **measured** 5.3 V rail: `D = 1 − 5.3/16 = 0.67`,
input current `16 × 0.1 / (0.85 × 5.3) = 355 mA`, `ΔI` at 30 % = 107 mA, so
`L = Vin·D / (f·ΔI)` = 5.3 × 0.67 / (1 MHz × 107 mA) ≈ **33 µH**.

## Substrate and mechanical

**Two-layer FR4, emitters on the front, converter cluster on the back.**
Superseded the spec's single-layer aluminium MCPCB on 2026-09-22.

**The stock board is not aluminium.** Two scratched points on its bare back
read open circuit, and photo `photos/20260922_000142.jpg` shows a back with no
copper, silkscreen or components — single-layer FR4. So this is an upgrade over
what ships today, not a tradeoff against a metal core that was never there.

The aluminium case was weak anyway. The ring bolts to a plastic housing and
sits in still air inside a dome, with no conduction path to any heatsink. The
dominant thermal resistance is therefore **board-to-air**, roughly 40 K/W on a
~40 mm disc regardless of substrate. What an aluminium core actually buys is
*spreading* — no hot spot under each die — not a lower total rise.

Budget: 8 emitters × ~130 mW of heat (at ~30 % wall-plug efficiency on 180 mW
electrical) + ~250 mW converter loss ≈ **1.25 W**, giving ~50 °C above internal
ambient on either substrate. Note this is **3.5× the stock board's IR
dissipation** — the stock ring runs 4 IR at 50 mA, about 0.36 W. This is a
materially warmer board than the one it replaces.

FR4 recovers most of the spreading with thermal vias under every emitter pad
and copper pours on both layers, and buys something aluminium cannot have at
any price: **a ground plane**. That retires risk 4 below.

Two-sided is not a layout preference, it is forced: an aluminium MCPCB is
copper / dielectric / solid aluminium, so its back *is* the metal core and
cannot carry components at all. Choosing to populate the back is choosing FR4.

Mechanical envelope matches the stock board exactly: outer profile with its
locating notches, centre bore, both mounting holes, header footprint and cable
exit, so the stock cable and dome assembly still fit.

### The lens array

The stock IR optics are a **separate clear plastic lens array** that sits over
the board — not integrated dome emitters. The board itself carries eight flat
SMD packages; the array provides domes over the four IR positions. It is a
full-disc cover over the emitter ring.

Two consequences: it defines the front-side keep-out (which is why the
converter goes on the back regardless of how much height exists above the
front), and it constrains emitter selection, since its domes must sit correctly
over whatever package is chosen.

## Known risks

1. ~~**Back-side clearance.**~~ **RESOLVED 2026-09-22: adequate everywhere on
   the back.** Converter placement is unconstrained, so the 33 µH inductor
   fits. The fallback to a linear current sink is off the table.
2. ~~**Rail voltage.**~~ **RESOLVED 2026-09-22: 5.3 V measured at the header.**
   The boost topology is confirmed. A 12 V result would have inverted it back
   to a buck; it did not.
3. **Rail current headroom.** 355 mA continuous at full power, versus a stock
   board that lit one 4-emitter channel at a time. Browning out that rail
   reboots the SoC.
4. ~~**Switching noise near the image sensor.**~~ **Largely retired by the FR4
   decision** — a two-layer board gives the SW node a return plane, which was
   the missing mitigation. Residual: keep the C1–U1–D1–C2 loop as one tight
   cluster on the back, and remember that a single converter is not
   automatically quieter than two, since the higher output voltage raises dV/dt
   even as the converter count drops. The converter now also sits on the
   opposite face from the emitters, with the plane between it and the sensor.
5. **Single string, single point of failure.** One open emitter kills all
   eight. U1's OVP must shut down cleanly rather than running the output away.
6. **Driver enable semantics.** TPS61165's CTRL pin uses a one-wire dimming
   protocol where a plain DC high means full brightness; a toggling GPIO could
   in principle enter that protocol. Verify against the datasheet, or prefer a
   part with a plain EN pin.
7. **Emitter junction temperature is now the binding constraint.** With the
   ballast resistors deleted by the topology, the dies are what run hot:
   ~50 °C board-wide rise from 1.25 W plus ~15 °C locally, over ~40 °C internal
   ambient ≈ **Tj ~105 °C** against a typical 850 nm rating of 110–125 °C.
   Workable, not generous. **R1 is the knob** — if the evaluation rig shows
   adequate output at 80 mA, going there buys roughly 20 °C for a ~20 % output
   cost. Decide on measured radiant output, not on paper.

## Measurements required before layout

Electrical, camera running, board still connected:

1. ~~Header rail voltage~~ — **DONE 2026-09-22: 5.3 V.** Silkscreen pin order
   `− + IR HB` confirmed from photos; verify against the cable before layout.
2. **Back-side clearance** between the board's back face and whatever it mounts
   against, and where the most generous region is. (Was "height above the
   board" — the lens array makes the front unusable.)
3. `IR` and `HB` logic levels when asserted — 3.3 V or 5 V. Drive from telnet
   via the `IR_LED` and `WHITE_LED` nodes.
4. Stock board total current draw with the IR channel on.
5. Each stock emitter's Vf in diode mode, individually — resolves whether the
   stock emitters are actually degraded (`SPEC.md` open item 2).
6. The striped component's resistance, room light versus covered — resolves
   `SPEC.md` open item 3.

Mechanical, board out, calipers:

7. Outer diameter, and whether the profile is a true circle or has flats.
8. Notches: angular position from a datum, width, depth.
9. Centre bore diameter.
10. Mounting holes: diameter, centre-to-centre, distance from board centre.
11. Header: pitch, through-hole or SMD, distance from centre, angular
    position, cable exit direction.
12. Board thickness.
13. LED pad centres: radius from centre and angular position of all eight.
14. Radial lobe width, bore edge to outer edge at an LED position — decides
    2835 versus 3535 (`SPEC.md` open item 4).
15. Largest contiguous clear area **on the back** for the converter cluster,
    with dimensions — noting where the mounting bosses and the cable land.
16. The lens array: outer diameter, how it locates on the board, its internal
    dome positions, and the standoff height between board face and dome. This
    decides both the front keep-out and whether a 3535 emitter still sits
    correctly under a dome sized for the stock package.

## Evaluation rig

Unchanged from `SPEC.md`. LM317 plus a single resistor (`R = 1.25 / I_target`)
as an adjustable constant-current source for characterising candidate
emitters, and a BPW34 photodiode in photovoltaic mode into a multimeter on the
µA range at fixed distance in a dark enclosure for relative radiant output.
Relative comparison only, sufficient for ranking parts and checking linearity
against current.

Use the rig to confirm Vf at 100 mA on the actual emitters received before
committing R1's value.

## Deliverables

- KiCad installed locally, with `kicad-cli` and `pcbnew` scripting available.
- U1 selected against its datasheet; R1 and L1 locked to concrete values.
- Footprints: SOT-23-6, the chosen LED package, passives, the stock header.
- `Edge.Cuts` outline from measurements 7–12.
- Schematic, then layout: emitters front, converter back, ground pour on both
  layers, thermal vias under every emitter pad.
- JLCPCB **two-layer FR4** fab package and BOM with LCSC part numbers.
- `plan()` change in `onvif-rust` to drive `Node::WhiteLed` on the night
  transition.
