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

### D2–D9, 3535 IR emitter — BLOCKED

Needs the chosen part's recommended pad layout, including the thermal pad.
Task 4 shortlisted JNJ `C22447934`; its datasheet has not been read. A 3535
package is 3.45 × 3.45 mm nominal but anode/cathode/thermal pad geometry
varies between manufacturers and **must not be guessed** — eight wrong
footprints is a scrapped board.

### J1, 5-position 1.50 mm — BLOCKED

Pitch (1.50 mm) and position count (5) are measured. What is not settled:

- **Through-hole or SMD**, and **which side the body mounts on.** The back scan
  shows the white housing with five contacts at the board edge; the front scan
  shows five solder fillets. That is consistent with either a through-hole part
  bodied on the back and soldered on the front, *or* an edge-mount SMD part on
  the back. The scan cannot separate them.
- Pin 1 orientation relative to the board datum.

This matters beyond the footprint: our converter cluster also lives on the
back, so if the connector bodies there, the two must not collide.
