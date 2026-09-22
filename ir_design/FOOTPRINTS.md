# Footprint assignment — Task 5

Library survey run against KiCad 9.0.8 stock libraries (`/usr/share/kicad/footprints`).

## Reused from the standard library — no work needed

| Ref | Part | Footprint |
|---|---|---|
| U1 | SY7200A, SOT23-6 | `Package_TO_SOT_SMD:SOT-23-6_Handsoldering` |
| Q1 | AO3400A, SOT-23 | `Package_TO_SOT_SMD:SOT-23` |
| D1 | Schottky ≥40 V, SOD-123 | `Diode_SMD:D_SOD-123` |
| C1 | 10 µF 25 V | `Capacitor_SMD:C_0805_2012Metric` |
| C2 | 4.7 µF 50 V | `Capacitor_SMD:C_0805_2012Metric` |
| C3 | 100 nF | `Capacitor_SMD:C_0402_1005Metric` |
| R1a, R1b | 4.02 Ω 1 % | `Resistor_SMD:R_0805_2012Metric` |
| R2 | 1 MΩ EN pulldown | `Resistor_SMD:R_0402_1005Metric` |
| R3 | 1 kΩ | `Resistor_SMD:R_0402_1005Metric` |
| R4 | 100 kΩ | `Resistor_SMD:R_0402_1005Metric` |
| R6 | ~100 kΩ sensor divider | `Resistor_SMD:R_0402_1005Metric` |
| U2 | TEMT6200FX01 ambient light sensor, `C143695` | **custom `ir-ring:TEMT6200_0805`** — see below |

`_Handsoldering` variants chosen for U1 and the passives where available: this
is a prototype run and the larger pads cost nothing on a board with this much
free copper.

## L1 — RESOLVED: Sunlord SWPA4030S330MT (`C83470`)

Spec `ir_design/swpa4030s.pdf` (SWPA1102230000 rev 11), Appendix A row:
33 µH ±20 %, DCR **0.330 Ω typ / 0.429 Ω max**, Isat **1.10 A**, Irms
**0.84 A**, SRF 10 MHz. Shielded, 4.0 x 4.0 x 3.0 mm max.

Against this design (~0.36 A peak, ~0.27 A input): **~3x margin on both
saturation and heating current**. Worst-case DCR loss ~31 mW. SRF sits 10x above
the 1 MHz switching frequency. Silergy's "DCR < 50 mΩ" guidance targets their
2 A applications and does not bind at our current; a 5 x 5 mm part would
only add board area.

KiCad ships no SWPA footprint. `ir-ring:L_Sunlord_SWPA4030S` is drawn from
Table 4-1's recommended reflow pattern: pads 1.10 x 3.70 mm with a 1.90 mm
inner gap, 3.00 mm centre to centre. Verified against the spec to 0.01 mm.

## U2 — custom footprint, not generic 0805

`ir-ring:TEMT6200_0805` follows Vishay's recommended solder pad
(`ir_design/temt6200.pdf`, doc 81317 rev 1.9 p.5): two 1.0 x 1.45 mm pads with
a **0.6 mm gap**. KiCad's `R_0805` has ~0.8 mm and no polarity marking. The
marking is the real reason: **VECO max is 1.5 V**, so a part fitted backwards
does not work. **Pad 1 = collector** (matches `Device:Q_Photo_NPN` pin 1), with a
"C" on the silkscreen. On the part, the die sits toward the collector end,
visible through the clear body, 0.82 mm from that edge. That gives a check at
incoming inspection.

VCEO max is **6 V**, and the collector sits at most at the 5.3 V rail
through R7. That is inside the rating, close enough to note, and it rules out
running this sensor from any higher rail.

## Custom — must be drawn

### D2–D9 — RESOLVED: JNJ-L-3535EW120-85035D-SL-J2 (`C22447930`)

**Correction first.** An earlier revision of this file attributed
`C22447934` to Task 4. That was wrong — I picked that code out of a search
result and misattributed it. Task 4 actually recommended `C7529167` (3535,
845-860 nm, Vf 1.4-1.8 V, 60 deg) and `C7500098` (2835, 840-870 nm,
Vf 1.5-2.0 V, 90 deg). **Task 4's Vf figure was correct**; the 810 nm /
2.5 V / 30 deg complaint was against a part it never chose. What survived
from that review is the *beam angle* criterion, which the shortlist genuinely
never applied, and thin stock on both (535 / 570 units).

#### Candidates evaluated 2026-09-22

| Code | Part | Vf | Angle | If max | Chip | Verdict |
|---|---|---|---|---|---|---|
| **C22447930** | JNJ-L-3535EW120-85035D-SL-J2 | **1.4-1.6 V** | **120 deg** | 1 A | 35 mil | **CHOSEN** |
| C2988736 | XYC-HIRC19C120-35 | 1.4-2.0 V | 120 deg | 1.2 A | 35 mil | good alternate, looser Vf bin |
| C2988738 | XYC-HIRC19C120-28 | 1.4-2.0 V | 120 deg | 750 mA | 28 mil | smaller die, no benefit at 100 mA |
| C25170649 | JNJ-L-3535EW90-85028C-SL-Q2 | 1.4-1.5 V | 90 deg | 700 mA | 28 mil | viable if 120 deg proves too wide |
| C22447925 | JNJ-L-3535AG60-85028D-SL-J2 | 1.3-1.4 V | 60 deg | 700 mA | 28 mil | 180 mW/sr — throw, not coverage |
| C2988744 | XYC-HIRC19C120-42 | **2.8-3.2 V** | 120 deg | 1.5 A | 42 mil | **DISQUALIFIED** |

All are 850 nm and SMD3535-3P.

#### Why C2988744 is out

`Vf` 2.8-3.2 V is a **multi-junction die** — roughly three junctions in one
package. Eight in series is **22.4-25.6 V**, against SY7200A's open-LED clamp
of 28/30/33 V. The clamp would sit barely above the normal operating string
voltage, so a cold high-Vf bin could trip protection during normal running.
Everything else on the list is single-junction at 1.3-2.0 V.

#### Why 120 degrees, not 60

The emitters all point forward on parallel axes, so the array's beam is the
single-emitter beam — the ring does not splay it. **And the lens array dome
narrows whatever it is given.** Starting at 120 deg and letting the dome
collimate lands near the camera's field of view; starting at 60 deg would
compound into a spot.

The failure modes are asymmetric. Too wide costs range but keeps coverage.
Too narrow produces a bright centre that the auto-exposure meters on, which
then *darkens* the corners — the classic cheap-IR-camera look, and precisely
what this board exists to fix. `C22447925`'s 180 mW/sr at 350 mA versus
`C22447930`'s 70 is concentration, not more light.

#### Why this part over the other two 120 deg options

Its **Vf bin is 1.4-1.6 V**, tighter than the 1.4-2.0 V of both NEWOPTO parts.
String voltage predictability is worth real money here: it sets duty cycle,
rail draw and the OVP margin. The 35 mil die also runs cooler than the 28 mil
at equal current.

#### The power budget survives

At 100 mA the string is roughly 11-13 V, so duty ~0.56, rail draw ~266 mA and
LED dissipation ~1.2 W. **These are within a few percent of the 271 mA and
1.1 W already in the design doc** — nothing downstream needs recomputing. We
are also running at **10 % of the part's 1 A rating**, so junction temperature
has enormous margin.

**Footprint drawn 2026-09-22:** `ir-ring:LED_3535_JNJ_EW120`
(`ir_design/kicad/ir-ring.pretty/`), from the vendor's recommended pad
drawing, datasheet rev A page 3. Dimensions read against the drawing's own
labels (1.0 mm centre pad = 80 px, 3.3 mm height = 263 px, consistent to
0.3 %):

| Pad | Net | Size | Position |
|---|---|---|---|
| 1 | cathode | 0.6 x 3.3 mm, plus 0.3 x 0.6 outward tab | x = **+1.30** |
| 2 | anode, thermal | 1.0 x 3.3 mm | x = 0 |
| 2 | anode | 0.6 x 3.3 mm, plus 0.3 x 0.6 outward tab | x = **-1.30** |

Package pins 2 and 3 are common inside the part, so every anode pad is
numbered `2` and a plain two-pin LED symbol works. Loads cleanly in
pcbnew 9.0.8.

**Added beyond the vendor drawing:** three 0.3 mm thermal vias in the anode
pad, at 1.0 mm pitch. They are the thermal design on a board with no metal
core. They are separate pads with no paste layer, so they do not starve the
joint.

#### Pin-1 handedness: RESOLVED

The datasheet has **one** pad view, and it is a bottom view, with pin 1 on
the left. The drawing on the same page labelled like a top view shows the
lens face, not the pads, so the two do not conflict. My first revision drew
pin 1 on the left and was mirrored. Fixed on 2026-09-22: looking down on the
board, **pin 1 (cathode) is on the right**. The pin-1 dot on the silkscreen
moved with it.

Seen from the component side, the part's corner chamfer should sit on the
anode (left) side. That gives a visual check at incoming inspection.

#### ⚠ The thermal pad IS the anode — layout consequence for Task 8

On this part the big centre pad is not electrically isolated. It is the
anode. In a series string every emitter's anode sits at a **different
potential**, from ~1.4 V up to ~13 V. So:

- **each emitter's back-side thermal copper must be its own island**, sized
  as large as the space allows, and
- those islands **cannot join the ground pour** or each other.

This limits heat spreading per emitter to its own island. At 100 mA, 10 % of
the part's rating, that is acceptable, but it rules out the "one big back pour
for heat" picture the substrate section assumed. The ground pour lives under
the converter only.
`ir_design/C22447934.pdf` is the same family and its part-number scheme now
decodes cleanly — `3535{series}{angle}-{wavelength}{chip}{bin}` — so
`3535EW120-85035D` reads as 120 deg, 850 nm, 35 mil.

### J1, 5-position 1.50 mm — RESOLVED, use the stock library

Operator confirms the connector is **pass-through (through-hole)**. With a
1.50 mm pitch and 5 positions that is the **JST ZH** family, and KiCad ships
the footprint:

`Connector_JST:JST_ZH_B5B-ZR_1x05_P1.50mm_Vertical`

**No custom drawing needed.** Verify pin-1 orientation and the body outline
against the stock board on the 1:1 print before routing — the *pitch* is
certain, the *handedness* is not.

Placement note: the body mounts on the back, which is also where the converter
cluster lives. Keep them apart during Task 8.
