# Replacement IR LED ring board

Date: 2026-09-21
Status: design approved; U1/passives locked, mechanical measured, no layout started

Supersedes the locked decisions in `ir_design/SPEC.md` where the two conflict.
Surviving from the spec: the mechanical envelope, the all-IR channel choice
and the 100 mA target. **Superseded:** the aluminium substrate (the stock board
is FR4 and we go two-layer FR4), the 5 V assumption (measured 5.3 V), and the
whole electrical section — 2S2P strings, AL8860 buck, per-branch ballast
resistors, two independent drivers.

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

**One SY7200A boost LED driver, 8 emitters in a single series string, with a
switched sense resistor selecting 50 mA or 100 mA.** Emitters on the front
under the stock lens array; converter on the back of a two-layer FR4 board.

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
Eight in series is 10.4–16.0 V at the measured Vf, comfortably above the
measured 5.3 V rail, duty ≈ 0.67.

A single series string also makes current matching exact by construction. That
removes the four per-branch ballast resistors the spec called for, and with
them the current-hogging failure mode those resistors existed to prevent.

`PAM2803 is ruled out` on its own numbers: SW pin absolute maximum is 6 V,
below the string voltage.

### Operating modes

Revised 2026-09-22 against the measured 5.3 V rail and a measured Vf of ~1.5 V
at 100 mA (read from the emitter's I-V curve; the widely quoted 1.4–1.6 V is
binned at 20 mA and does not apply here).

| Mode | `IR` | `HB` | String | Per-emitter | Vstring | LED power | Rail draw |
|---|---|---|---|---|---|---|---|
| Off | 0 | × | — | — | — | 0 | ~0 |
| Half | 1 | 0 | 8 LEDs | 50 mA | 10.4–13.6 V | ~0.5 W | ~135 mA |
| Full | 1 | 1 | 8 LEDs | 100 mA | 10.4–16.0 V | ~1.1 W | ~271 mA |

All eight emitters are in circuit in both modes. Only the regulated current
changes, so the string voltage never approaches the rail.

### Half power — by current, not by shortening the string

Revised 2026-09-22. **All eight emitters stay in series in every mode.** Half
power halves the regulated current rather than bypassing emitters.

- **R1a**, permanent, FB to ground, sized for 50 mA → half power default
- **R1b**, same value, from FB through **Q1** to ground. Q1 on parallels the
  pair to ~2.0 Ω → 100 mA full power
- **Q1**, logic-level N-MOSFET, gate driven directly from `HB` through R3 with
  an R4 pulldown. Source at ground, so no level shifting and no inverter

`HB` high → Q1 on → **full power**. `HB` low → Q1 off → **half power**.

#### Why not the emitter bypass this design originally specified

The superseded scheme shorted LEDs 5–8 to halve output. It does not work. At
Tj ≈ 105 °C the emitter Vf falls to ~1.34 V (−2 mV/°C per junction from a
measured 1.5 V at 100 mA), so a 4-emitter string is **5.36 V typical and
4.96 V on a low Vf bin — at or below the 5.3 V rail.** A boost cannot regulate
there.

That is the same marginal-headroom failure this design rejected the 2S2P buck
for, reappearing in half-power mode — and per *Firmware consequence* below,
half power is the mode unmodified firmware boots into. Bypassing only two
emitters would have kept 2.14 V of headroom, but switching the sense resistor
removes the failure entirely instead of shrinking it, keeps the ring evenly
lit rather than leaving two emitters dark, and is one part fewer.

Deleted by this change: **Q2** (the inverter is unnecessary with Q1 at
ground), **R2** (its pull-up), and **R5** (which existed only to limit the
output-cap dump into a shortened string — there is no shortened string now).
**C2 is freed** from the 1 µF that the inrush limit forced, and moves to
4.7 µF, off the bottom of TI's 1–10 µF range where below-range operation is
warned to be potentially unstable.

Failure modes stay benign: Q1 shorted leaves the board stuck at full power,
Q1 open leaves it stuck at half power. Neither damages anything.

Q1 is an **AO3400A**: `Rds(on)` ≤ 48 mΩ at Vgs 2.5 V, Vgs(th) 1.45 V max. It
sits in series with R1b and skews the parallel value by **−1.09 % at 25 °C,
−1.43 % hot** — inside the ~2 % threshold, so R1b is not trimmed. It is not
the dominant error anyway: Vfb ±2 % and the resistors ±1 % already give ±3 %.

**R3 is 1 kΩ, not 10 kΩ.** A MOSFET gate is a DC open, so R3 and R4 form a
plain divider — unlike the superseded BJT arrangement, where the base junction
clamped at 0.7 V and R4 sank only ~7 µA, making the divider irrelevant. At
10 kΩ the gate would see `HB × 100/110` = **3.0 V from a 3.3 V line, a 9 %
loss**. That still works, but 1 kΩ gives `× 100/101` = 3.27 V for the same
money.

**Q1's gate drive now depends on measurement 3, which has therefore moved onto
the critical path.** The old inverter referenced the gate to the 5.3 V rail, so
`HB`'s own level barely mattered. Driving the gate directly, 3.3 V and 5 V are
both fine — but **1.8 V is not**: a 1.64 V gate against a 1.45 V maximum
threshold does not reliably turn Q1 on, and the board would be stuck at half
power. Measure `HB` before ordering.

## Firmware consequence

`cross-compile/onvif-rust/src/platform/anyka/night_mode.rs:849` — `plan()`
writes only `Node::IrLed` and the two ircut nodes. `Node::WhiteLed`, which is
the `HB` line, is reachable only through `set_white_light()` at
`platform/anyka/imaging.rs:355`, exposed as the ONVIF `tt:WhiteLight` auxiliary
command. Nothing calls it on a day/night transition.

With the chosen polarity, that means **unmodified firmware runs the board at
half power** — 8 emitters at 50 mA, which is 2× the stock IR array rather than
the designed 4×.

Reaching the designed output therefore requires adding `Step::Write` entries
for `Node::WhiteLed` to `plan()`. This is a small change the codebase is
already shaped for, and the objection previously recorded against it — that a
white floodlight is not a sane default on an outdoor camera — no longer
applies, because the second bank is now infrared.

This firmware change is a required deliverable of this design, not an optional
follow-up.

## Bill of materials

**21 designators.** `ir_design/PARTS.md` is authoritative for LCSC numbers and
datasheet evidence; this table is the summary.

(Arithmetic, since this count has been wrong twice: 13 singles + D2–D9 + J1 =
22 before the rework, not the 21 originally claimed here. Then +C4, −Q2, −R2,
−R5, +R1b = **21**.)

| Ref | Part | Note |
|---|---|---|
| U1 | **Silergy SY7200A**, SOT23-6, `C107309` | 1 MHz fixed, Vref 200 mV, ILIM 2 A, open-LED clamp 28/30/33 V, **plain EN** (1.5 V rising). Replaced TPS61165 2026-09-22 |
| L1 | **Sunlord SWPA4030S330MT**, `C83470`, 33 µH shielded 4x4x3 | Isat 1.10 A, Irms 0.84 A, DCR 0.33/0.43 Ω — ~3x margin on both currents. Custom footprint from Sunlord Table 4-1 |
| D1 | Schottky **≥ 40 V**, SOD-123 | must exceed the 33 V open-LED clamp |
| C1 | 10 µF 25 V 0805 | input; datasheet wants ≥ 4.7 µF |
| C2 | **4.7 µF 50 V X7R 1206**, `C29823` | output; datasheet wants ≥ 2.2 µF. 50 V covers the 33 V clamp. **1206 since 2026-09-23**: an 0805 was estimated at only ~2–2.8 µF left at 12–16 V, too close to the floor |
| C3 | 100 nF 0805 | input bypass |
| R2 | **1 MΩ** 0805 | **EN pulldown.** Silergy layout note 6: required where the driving pin is high-impedance at shutdown — exactly a camera GPIO before its port is initialised. Without it the illuminator can light at boot |
| R1a | Sense, `Vfb / 0.05 A` ≈ 4.0 Ω 1 % | permanent — sets the half-power default |
| R1b | Same value, through Q1 | parallels R1a to ~2.0 Ω for full power |
| Q1 | Logic-level N-MOSFET, 30 V, SOT-23 | source at ground; `Rds(on)` skews R1b, trim if > ~2 % |
| R3 | **1 kΩ** | Q1 gate, from `HB`. **Not 10 kΩ** — see below |
| R4 | 100 kΩ | Q1 gate pulldown |
| D2–D9 | 850 nm IR, **3535** | 10.8 mm radial room, 7 mm dome. **Height limit lifted 2026-09-22** — the operator will raise the lens array rather than constrain the emitter. Same die family as the 2835 alternative, so ratings are unchanged |
| U2 | **Vishay TEMT6200FX01** `C143695`, custom 0805 footprint with collector marked | **populates the stock LDR position.** 2 x 1.25 x **0.85 mm** — fits the 1.00 mm clearance. **Must be IR-filtered, see below** |
| R6 | ~100 kΩ, start value | LDR divider bottom leg, emitter to GND. Final value set on hardware |
| R7 | **100 kΩ**, start value | **collector limiter, added 2026-09-22.** Caps the `LDR` node at `5.3 V x R6/(R6+R7)` ≈ 2.65 V, so the 5.3 V rail can never reach the SoC's ADC pin — see below |
| TP1–TP3 | Test pads, 1.5 mm, back | **added 2026-09-23.** TP1 = VOUT, sitting on D2's thermal island (VOUT copper); TP2 = FB and TP3 = GND beside D9. Not in the BOM |
| J1 | **Molex PicoBlade 1.25 mm, 5-pos, right-angle THT**, body on the back | Pitch measured 1.246 mm from the scan (the 6.0 mm caliper figure was edge to edge). **Pad 5 = 5V ... pad 1 = WL_EN**, reversed against the stock `+ - LDR IR HB` order by the back-side flip. Net names on the back silk |

**Back-side layout, revised 2026-09-23.** All small passives are 0805 now (0402 before), for hand
rework. Only the switching loop (U1, D1, C2, L1, C3, R2, C1) stays in the top gap, with
courtyard gaps of at least 0.3 mm (they were 0.03–0.1 mm, and L1, C1 and C2 cut into
the D2/D9 thermal islands). R1a/R1b sit beside it. The half-power switch (Q1, R3, R4) carries
only DC, so it moved to the D7–D8 gap. R6/R7 stay on the front beside U2. The power stage is pre-routed, not left
to the autorouter: the loop closes on B.Cu, the GND return runs between C2's pads, and VOUT
crosses the SW node under D1's body, then through a via to D2's island. `09_place.py`
asserts island, screw-head and courtyard clearances.

~~Layout rule: no RC on the CTRL net.~~ **Deleted 2026-09-22 with the move to
SY7200A**, whose EN is a plain enable (1.5 V rising / 0.4 V falling) with no
one-wire protocol to fall into. Our measured 3.3 V GPIO drives it directly.
This is the main reason the part was swapped: the hazard is designed out rather
than documented around.

**Board: two-layer FR4, 1.0 mm** (1.05 mm measured on the stock board), 2 oz
copper.

### The light sensor must be IR-blind, or it oscillates

The camera names J1 pin 3 `LDR` and the stock footprint for it is unpopulated,
which appears to be why `ain0` cannot see daylight — see
`ir_design/MEASUREMENTS.md`. This design populates it.

**The sensor must reject near-IR.** An IR-sensitive part — most cheap 940 nm
phototransistors, and a bare CdS cell — would be illuminated by *our own
850 nm array* and report daylight. That closes a positive feedback loop:
dark -> illuminator on -> sensor sees IR -> "day" -> illuminator off -> dark.
The camera would hunt at dusk instead of switching once.

**Vishay TEMT6200FX01** is specified with a near-IR suppression filter and a
450-610 nm photopic response, which is exactly the requirement. 0805,
**0.85 mm tall**, inside the 1.00 mm clearance.

Polarity must satisfy the firmware's `ldr_high_is_day = true`, so the reading
has to *rise* with light: collector to the rail, emitter to the node, **R6 from
node to ground**, node to J1 pad 3. With ~39 uA of light current, R6 starts
around 100 kΩ and is trimmed on hardware.

**R7 protects the SoC.** With the collector tied straight to the 5.3 V rail, a
phototransistor in daylight saturates and pulls the `LDR` node to ~5 V. That
node is J1 pin 3, which goes straight to an ADC input on the AK3918. Its
full-scale voltage is unknown, but it is almost certainly not 5 V. R7 in series
with the collector turns saturation into a fixed divider: the node can never
exceed `5.3 x R6/(R6+R7)` ≈ **2.65 V** at equal values, whatever the light.
One resistor, and it is the safe default at a trust boundary we cannot see
into.

It is also a calibration knob. The phototransistor saturates at about
`5.3 V / (R6+R7)` ≈ 26 µA, so bright indoor light and daylight both read as
"day". The useful range, from dusk to a few lux, falls in the linear region.
Raise R6 for more dusk sensitivity, and keep R7 >= R6 so the ceiling stays
under half the rail.

**One measurement finalises both values:** the ADC's full-scale voltage.
Read `ain0` and measure pin 3 with a meter at the same moment; counts over
volts gives the scale. The camera already reads ~660 counts with nothing
fitted, so something biases that line at the camera end. Fit the sensor, read
`ain0` covered and uncovered, and trim R6/R7 for the widest swing below full
scale.

Inductor sizing against the **measured** 5.3 V rail: `D = 1 − 5.3/16 = 0.67`,
input current `16 × 0.1 / (0.85 × 5.3) = 355 mA`, `ΔI` at 30 % = 107 mA, so
`L = Vin·D / (f·ΔI)` at TPS61165's **1.2 MHz** = 5.3 × 0.67 / (1.2 MHz ×
107 mA) ≈ 27.6 µH — but **TI caps the inductor at 22 µH** (recommended range
10–22), so **L1 = 22 µH** and ripple runs 34–42 % with a worst-case peak of
504 mA, inside the 1.2 A switch limit. The earlier 33 µH assumed 1 MHz and an
uncapped range; both were wrong.

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
electrical) + ~250 mW converter loss ≈ **1.1 W**, giving ~44 °C above internal
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
   the back.** Converter placement is unconstrained, so the 22 µH inductor
   fits. The fallback to a linear current sink is off the table.
2. ~~**Rail voltage.**~~ **RESOLVED 2026-09-22: 5.3 V measured at the header.**
   The boost topology is confirmed. A 12 V result would have inverted it back
   to a buck; it did not.
3. **Rail current headroom.** **Eased 2026-09-22 to ~271 mA** at full power
   (was 355 mA) once the emitter Vf turned out to be ~1.5 V rather than the
   assumed 1.7–2.0 V. Still worth checking against measurement 4, since the
   stock board lit one 4-emitter channel at a time. Browning out that rail
   reboots the SoC.
4. **Switching noise near the image sensor.** **Largely retired by the FR4
   decision** — a two-layer board gives the SW node a return plane, which was
   the missing mitigation. The converter also now sits on the opposite face
   from the emitters, with the plane between it and the sensor. Two residuals:
   keep the C1–U1–D1–C2 loop as one tight cluster on the back, and note that
   **inductor ripple is worst in the mode the board boots into.** `ΔI` is
   load-independent, so halving the LED current does not halve it: ripple runs
   **84 % of input current at half power versus 42 % at full** (ΔI 111 mA on
   131 mA, trough 76 mA — still continuous conduction). The superseded bypass
   had the opposite property, since 4 emitters meant D = 0.145 and ~21 %
   ripple. This is the one thing the current-switching trade gives up, and it
   is a second argument against dropping L1 below 22 µH.
5. **Single string, single point of failure.** One open emitter kills all
   eight. U1's OVP must shut down cleanly rather than running the output away.
6. ~~**Driver enable semantics.**~~ **CLOSED 2026-09-22, then designed out.**
   The EasyScale analysis showed a static GPIO could not enter the protocol,
   but the swap to SY7200A removes the mechanism altogether — plain EN, 1.5 V
   rising threshold, driven directly by the measured 3.3 V line. No residual
   layout rule.
8. ~~**Emitter height.**~~ **Resolved 2026-09-22 by decision, not by part
   selection:** stock clearance under the lens array is 1.00 mm and most 3535
   emitters are 1.4-2.0 mm, so the operator will **raise the dome** instead of
   restricting the package. 3535 stands.

   Two consequences to carry into mechanical work. Raising the array increases
   the emitter-to-dome standoff, so a dome that collimated a 0.7 mm-tall part
   will focus differently on a ~1.9 mm one — **expect a changed beam pattern,
   probably wider**. And the 9th smaller dome over the light sensor rises with
   it, which is harmless for a 0.85 mm sensor but should not be allowed to
   shadow it.
7. **Emitter junction temperature is now the binding constraint.** With the
   ballast resistors deleted by the topology, the dies are what run hot:
   ~44 °C board-wide rise from 1.1 W plus ~13 °C locally, over ~40 °C internal
   ambient ≈ **Tj ~97 °C** against a typical 850 nm rating of 110–125 °C.
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
