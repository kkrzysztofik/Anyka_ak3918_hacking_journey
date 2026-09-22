# Measured values — RZ-XHR(08SG)-C4 stock ring

Camera: AK3918 dome, stock ring board removed for measurement.
Instrument: multimeter + tape measure (calipers still to come).

Photos in `photos/`.

## Electrical

| # | Measurement | Value | Date | Notes |
|---|---|---|---|---|
| 1 | Header rail | **5.3 V** | 2026-09-22 | **GATE PASSED.** Boost topology confirmed; 12 V would have forced a buck |
| 1 | Silkscreen pin order | `- + IR HB` | 2026-09-22 | read from photo `000211`; confirm against the cable before layout |
| 2 | Back-side clearance | **adequate everywhere** | 2026-09-22 | **GATE PASSED.** No local restriction — converter placement is unconstrained on the back face |
| 2a | Substrate is aluminium? | **No — FR4** | 2026-09-22 | two-point resistance on the bare back reads open. See "Substrate" below |
| 3 | `IR` line asserted | **3.3 V** | 2026-09-22 | within TPS61165 CTRL rating |
| 3 | `HB` line asserted | **3.3 V** | 2026-09-22 | **GATE PASSED.** With R3 = 1 k the gate sees 3.27 V vs AO3400A's 1.45 V max threshold — 2.3x margin |
| 4 | Board current, IR channel on | | | headroom check against the new board's 271 mA |
| 5 | Vf D1..D8 (diode mode) | | | resolves `SPEC.md` open item 2 — are the stock emitters degraded? |
| 6 | Striped component, room light | | | resolves `SPEC.md` open item 3 |
| 6 | Striped component, covered | | | no change ⇒ not a photoresistor, drop it from the new design |

## Substrate — the stock board is single-layer FR4

Two scratched points on the bare back read **open circuit**. Not an aluminium
MCPCB. Photo `000142` supports it: the back carries no copper, no silkscreen
and no components at all, which is single-layer FR4.

Two consequences:

1. **The redesign is an upgrade, not a tradeoff.** Moving to two-layer FR4 with
   2 oz pours and thermal vias is strictly better than what ships today. The
   earlier concern about "giving up the aluminium core" was arguing against a
   substrate that was never there.
2. **`SPEC.md`'s founding suspicion now has a mechanism.** Ballast resistors
   dissipating ~0.12 W each in 0805 packages, on single-layer FR4, with no pour
   and no back copper to spread into. The new topology — one series string on a
   constant-current driver — has no ballast resistors at all, so the root cause
   is removed by the topology rather than by the substrate.

## Optics — the IR lens array is a separate part

Photo `000236` is a clear plastic disc, separate from the board. Photo `000211`
is the board with it fitted (the four "domes" belong to the array, not to the
LEDs); photo `000315` is the same board with it removed, showing **eight flat
SMD packages** — four white with yellow phosphor, four IR.

This is why the converter goes on the back: the array covers the whole emitter
ring, so the front is unavailable regardless of clearance above it.

## Thermal — the emitters are now the binding constraint

With the ballast resistors gone, the limit moves to the dies:

| | Stock | This design |
|---|---|---|
| IR emitters | 4 @ 50 mA (~90 mW each) | 8 @ 100 mA (~180 mW each) |
| Total board dissipation | ~0.36 W (IR channel) | **~1.1 W** |
| Substrate | 1-layer FR4, no pour | 2-layer FR4, 2 oz, thermal vias |

Estimated junction temperature: ~44 °C board-wide rise from 1.1 W, plus ~13 °C
local at each die, over ~40 °C internal ambient ≈ **Tj ~97 °C** against a
typical 850 nm rating of 110–125 °C.

Workable but not generous. **R1 is the knob**: if the evaluation rig shows
adequate output at 80 mA, dropping there buys roughly 18 °C of margin for a
~20 % output cost. Decide this against measured radiant output, not on paper.

## Scan-derived geometry (2026-09-22)

Source: `photos/img20260922_10485355.jpg` (front) and `img20260922_10535866.jpg`
(back), flatbed at **3200 dpi = 31.75 um/px** at the 1/4 working scale.

**Scale independently verified.** The ruler's 1/16-inch ticks measure exactly
50.00 px at 1/4 scale = 1.5875 mm against a true 1.58750 mm. Two further
cross-checks agree: the tape-measure photos gave ~37 mm across, and a 16.8 mm
bore passes an M12 lens barrel, which a 10.6 mm bore (the figure implied by the
alternative scale reading) could not.

### ⚠ Coordinate frames are mirrored between the two scans

Back-scan angles are **mirrored** relative to front-scan angles:
`front = 360 - back`. Both are quoted below in their own frame and labelled.
Angles run from +x with **+y downward in the scan image**. Fix one datum before
`Edge.Cuts` and convert everything into it.

### Centre bore

| Method | Value |
|---|---|
| Back scan, circle fit on boundary | **16.806 mm** |
| Back scan, area-based | 16.827 mm |
| Front scan, circle fit | 16.471 mm |

The back scan is bare laminate with a hard edge; the front reads ~0.34 mm
smaller because solder mask and silkscreen overhang the hole. **Use 16.8 mm for
mechanical clearance, 16.5 mm for keep-out.** Circle-fit residual 235 um mean
means the bore is routed, not drilled, and is not perfectly round.

### Outer profile (back-scan frame, from bore centre)

Median radius **20.01 mm**; extent **~37.9 mm** across both X and Y. The
outline is irregular, not a disc:

| Sector (back frame) | Min radius | Depth inside nominal |
|---|---|---|
| 31-60 deg | 15.65 mm | 4.36 mm |
| 77-103 deg | 16.63 mm | 3.39 mm |
| 150-194 deg | 17.81 mm | 2.20 mm |
| 329-359 deg | 18.01 mm | 2.00 mm |

Max radius 21.89 mm at 285 deg. Full 1-degree profile is reproducible from the
scans; it will be sampled directly into `Edge.Cuts` rather than retyped.

### Mounting holes - there are THREE, not two

`SPEC.md` assumed two. The back scan shows three, and they sit on a common bolt
circle:

| # | Diameter | Bolt-circle radius | Angle (back frame) |
|---|---|---|---|
| 1 | 1.82 mm | 16.16 mm | 130.3 deg |
| 2 | 1.86 mm | 16.03 mm | 330.6 deg |
| 3 | 1.86 mm | 16.12 mm | 209.7 deg |

Bolt-circle radius is consistent to **0.13 mm** across all three, so
**16.10 mm** is a sound nominal. Spacing is *not* symmetric: 79.4 / 120.9 /
159.7 degrees. Hole diameter ~1.85 mm suits an M1.6 screw with clearance.

A fourth enclosed feature sits at r = 12.71 mm, 179.2 deg, 1.35 mm across -
smaller and off the bolt circle. Identify it before treating it as a hole.

### Emitters (front-scan frame, from bore centre)

Eight found, mean radius **13.12 mm** (spread 12.79-13.55 mm):

| Angle (front frame) | Radius |
|---|---|
| 38.6 | 12.79 |
| 80.1 | 13.08 |
| 121.2 | 12.80 |
| 163.6 | 13.03 |
| 203.1 | 13.43 |
| 240.6 | 13.55 |
| 319.4 | 13.22 |
| 358.3 | 13.10 |

**Spacing is not 45 degrees.** Seven gaps of ~40 degrees plus one gap of
**78.8 degrees** centred near 280 degrees, where the passives and the
`RZ-XHR(08SG)-C4` silkscreen sit. The new layout has fewer passives, so that
gap can close - but the lens array's dome positions must be honoured, so this
is constrained by measurement 16, not free.

Package measures **2.7-3.0 x 3.5-4.0 mm** = **2835**, some placed rotated 90
degrees.

### Emitter package decision: 3535 - RESOLVED

Radial room from bore edge (8.24 mm) to the nearest outer edge (~19 mm) is
**~10.8 mm**, well past the ~8 mm threshold `SPEC.md` set for choosing 3535.
The operator confirms the lens array dome is **7 mm diameter**, which covers a
3.5 mm package comfortably.

**Use 3535.** Per the Task 4 sourcing the shortlisted 2835 and 3535 parts are
the same die family with identical electricals, so this changes the footprint
only - no circuit rework.

### Header J1 - RESOLVED

**Five pad positions**, outer-to-outer **6.0 mm** measured with calipers
(2026-09-22) = 4 gaps = **1.50 mm pitch**. JST ZH class, not the 1.25 mm GH I
guessed from solder fillets - fillet centroids are not pad centres, and that
method was 13 % low.

**Five conductors** in the stock cable (2026-09-22), so all five positions are
used against four silkscreen labels. Operator reports `+` and `-` land directly
on the white emitter string. Exact pad-to-signal mapping still to be rung out —
see "Outstanding" below. This matters: J1's pin order must match the stock
cable or the board is unusable without rewiring.

### Board thickness

**1.05 mm** measured. Order **1.0 mm** FR4 (standard JLCPCB option; 1.05 is the
measurement including solder mask).

### ⚠ Lens array domes sit directly over the existing emitters

Operator confirms (2026-09-22) the domes are **directly over the current
diodes**. Two consequences, both binding:

1. **Emitter positions are frozen**, including the uneven spacing measured from
   the scans — seven gaps of ~40 deg and one of 78.8 deg. The earlier note that
   the wide gap "can close now there are fewer passives" is **withdrawn**. New
   emitters must land on the old centres or they sit off-axis under their dome.
2. The stock array has **4 domes over the former IR positions** and flat
   windows over the 4 white positions. Going all-IR means four emitters get a
   focused dome and four get a flat window — a **mixed beam pattern**, narrow
   throw from four, wide flood from four. Not necessarily bad, but it is a
   change in beam shape, not just brightness. Confirm the dome count before
   layout.

### ⚠ Dome clearance - 1.00 mm, and it constrains the emitter

Clearance between the board face and the underside of the lens array is
**1.0 mm**. All converter parts are on the back, so this binds on one thing
only: **the emitters must be no taller than ~1.0 mm including any integral
lens.**

This was *not* a criterion in the Task 4 emitter shortlist and must be applied
retroactively. Many 3535 infrared emitters are ~1.9 mm tall with an integral
dome and **will not fit**; flat-top 3535 parts run ~0.6-0.8 mm and will. The
stock 2835 is ~0.7 mm, which is the existence proof that the envelope is
workable.

**Hard selection criterion: emitter height <= 1.0 mm, flat-top package.**

## Stock circuit, traced 2026-09-22

### J1 pinout (5 positions, 1.50 mm pitch)

| Pad | Function | Goes to |
|---|---|---|
| 1 | `+` rail | anodes of all 4 white emitters **and** the 2 "bottom" IR emitters |
| 2 | `-` return | emitters of Q2 and Q3 |
| 3 | **light-sensor return** | unpopulated R6 and the unpopulated top position |
| 4 | IR enable | Q2 base via a resistor beside R5 |
| 5 | white enable | Q3 base via R5 |

Q2 sinks the cathodes of the 2 "top" IR emitters; Q3 sinks the white string.

### The IR channel is 2S2P — SPEC.md was right

`+` reaches only 2 of the 4 IR emitters, and Q2 sinks the other 2. So the IR
array is **two series pairs in parallel**: `+` -> bottom IR -> top IR ->
ballast -> Q2 -> `-`. That is exactly the topology `SPEC.md` described, now
confirmed by continuity rather than inferred.

The whites are separate: all 4 anodes on `+`, cathodes sunk by Q3.

### SPEC.md open item 2 — RESOLVED: the emitters are healthy

Diode-mode forward voltages, all eight:

| | Vf (meter test current) |
|---|---|
| IR (x4) | **1.23-1.24 V** |
| White (x4) | **2.54 V** |

Tightly matched within each group and consistent with healthy parts — 1.23 V at
~1 mA is normal for an 850 nm AlGaAs die. **No emitter is degraded.** The dark
night image is not a failed array; it is an undersized one, which is what
`[[white-led-is-the-usable-night-illuminator]]` concluded from luma
measurements. Two independent lines now agree.

### SPEC.md open item 3 — RESOLVED: it is an unpopulated light sensor

The "striped component near the top-centre pads" is an **unpopulated
footprint**, not a fitted part. Its divider partner R6 is also unpopulated, and
R1 in that section is a **0 ohm link**. Pad 3 of J1 is the return path back to
the camera.

So the stock board *supports* a light sensor and this variant does not fit one.
Nothing to replicate — but see the design question this raises.

### Lens array — 8 domes, plus a 9th

**8 domes, one per emitter, plus 1 smaller dome** over the unpopulated
light-sensor position.

This **withdraws the mixed-beam concern** recorded above: every emitter gets a
dome, so an all-IR board keeps a uniform beam pattern. It also confirms the
array was designed for a populated light sensor.

Emitter positions remain frozen to the existing centres.

## Still outstanding

| # | Item | Why it matters |
|---|---|---|
| 4 | Stock board current draw | **not measurable with available tools.** Estimable from the ballast resistor values instead — read the codes off R2/R13 and compute. Informational only |
| — | Whether J1 pad 3 reaches the SoC ADC (`ain0`) | decides whether to populate a light sensor — see below |

## Mechanical (calipers — superseded by the scans above)

Photo-derived estimate only, **not for layout**: outer diameter ~36–40 mm,
centre bore ~13–14 mm. Measure properly before `Edge.Cuts`.

| # | Measurement | Value | Notes |
|---|---|---|---|
| 7 | Outer diameter | | circle or flats? |
| 8 | Notch 1: angle / width / depth | | datum notch — declare which one |
| 8 | Notch 2: angle / width / depth | | |
| 9 | Centre bore diameter | | |
| 10 | Mounting hole diameter | | |
| 10 | Mounting hole centre-to-centre | | |
| 10 | Mounting hole radius from centre | | |
| 11 | Header pitch | | |
| 11 | Header through-hole or SMD | | |
| 11 | Header distance from centre / angle | | |
| 11 | Cable exit direction | | |
| 12 | Board thickness | | |
| 13 | LED pad radius from centre | | |
| 13 | LED angular positions (×8) | | |
| 14 | Radial lobe width at an LED | | decides 2835 vs 3535 |
| 15 | Largest clear area on the **back** | | note mounting bosses and cable landing |
| 16 | Lens array: OD, locating features, dome positions, standoff height | | decides front keep-out and whether a 3535 sits correctly under a stock-sized dome |
