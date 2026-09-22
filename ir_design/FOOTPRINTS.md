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

### D2–D9, 3535 IR emitter — STILL BLOCKED, and the shortlisted part is wrong

`ir_design/C22447934.pdf` was read (JNJ-L-3535AW30-805xx-SL-J2-D3, rev A/2).
**It is not the part this design needs.** Three deviations, all from the
datasheet's own Optical Characteristics table:

| | Datasheet | Design needs |
|---|---|---|
| Wavelength | **810 nm** | 850 nm |
| `VF` | **2.5 V typ / 3.2 V max @ 350 mA** | Task 4 recorded ~1.5 V @ 100 mA — not supported by this document |
| Viewing angle | **30 deg** | wide — the lens array dome does the focusing |

Matching Task 4: Tj 115 °C, 700 mA max continuous, 30 mil chip.

**Why each one matters**

*810 nm is more visible to the eye than 850 nm*, not less. It glows a
noticeable dull red. The all-IR decision was justified on covertness, and this
part erodes exactly that. (It is also brighter to the sensor, since silicon QE
is higher at 810 nm — a real trade, but not the one that was chosen.)

*`VF` drives the whole power budget.* At 2.5 V typ the string is ~17.6 V at
100 mA and up to ~23 V on a cold high bin, against SY7200A's 28/30/33 V
open-LED clamp — workable but nothing like the 1.9x headroom Task 4 claimed.
Rail draw becomes ~390 mA, not 271 mA, and board dissipation ~2.1 W, not
~1.1 W. The thermal estimate in the design doc is sized on the lower figure.

*30 deg is the wrong beam for this optical stack.* The stock lens array already
collimates; feeding it a narrow emitter compounds the focusing and gives a hot
centre with dark edges. The stock 2835 parts are almost certainly wide-angle
(~120 deg) with the dome doing the work. **Emitter beam angle is a selection
criterion Task 4 never applied.**

**What is needed:** the 850 nm variant of this family (part code likely
`...-850xx-...`), or an equivalent 850 nm 3535 with a wide native beam. Its
own `VF` curve then feeds back into the rail-draw and thermal numbers.

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
