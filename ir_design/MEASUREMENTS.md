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
| 3 | `IR` line asserted | | | expect 3.3 V; sets U1 CTRL tolerance |
| 3 | `HB` line asserted | | **CRITICAL PATH** — drives Q1's gate directly. 3.3 V or 5 V fine; 1.8 V will not switch it |
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

## Mechanical (calipers — outstanding)

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
