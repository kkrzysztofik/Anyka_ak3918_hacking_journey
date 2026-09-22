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
| U2 | TEMT6200FX01 ambient light sensor | `Resistor_SMD:R_0805_2012Metric` — the part uses a standard 0805 footprint; **verify against the Vishay drawing before committing** |

`_Handsoldering` variants chosen for U1 and the passives where available: this
is a prototype run and the larger pads cost nothing on a board with this much
free copper.

## Still to source — L1

L1 is 33 µH, Isat ≥ 500 mA, DCR < 50 mΩ (Silergy's guidance). The footprint
follows the part, not the other way round. Stock library has credible
candidates once the part is picked — `L_Vishay_IHLP-5050`,
`L_Chilisin_BWVS00505030`, `L_Changjiang_FTC404030S`, `L_Wuerth_MAPI-4020`.

Height is unconstrained on the back side, so prefer the lowest DCR that fits
the clear area from measurement 15.

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
| 1 | cathode | 0.6 x 3.3 mm, plus 0.3 x 0.6 outward tab | x = -1.30 |
| 2 | anode, thermal | 1.0 x 3.3 mm | x = 0 |
| 2 | anode | 0.6 x 3.3 mm, plus 0.3 x 0.6 outward tab | x = +1.30 |

Package pins 2 and 3 are common inside the part, so every anode pad is
numbered `2` and a plain two-pin LED symbol works. Loads cleanly in
pcbnew 9.0.8.

**Added beyond the vendor drawing:** three 0.3 mm thermal vias in the anode
pad, at 1.0 mm pitch. They are the thermal design on a board with no metal
core. They are separate pads with no paste layer, so they do not starve the
joint.

#### ⚠ Verify pin-1 handedness before ordering — a mirror shorts the LED

The datasheet's top view and bottom view **both** put pin 1 on the left.
Both cannot be true. Most likely the "bottom" view is drawn see-through from
the top, which is common in these datasheets, and the footprint follows the
top view (pin 1 left, looking down on the board). That is a reading of the
drawing, not a certainty.

**The failure is not benign.** If the footprint is mirrored, the anode net
lands on the cathode pad and on the anode centre pad at the same time, which
shorts every emitter in the string. Check before ordering:

- against JLCPCB's own library footprint for `C22447930` (EasyEDA view), or
- by putting a meter in diode mode across a physical part, or
- by the corner chamfer: the top view puts it at the pin-2 (anode) corner.

A pin-1 dot is on the silkscreen, so the assembled orientation can be
inspected.

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
