# Geometry pipeline

Regenerates `ir_design/geometry.json` and `ir_design/kicad/ir-ring.kicad_pcb`
from the 3200 dpi scans in `../photos/`. Run from the repo root with
`/usr/bin/python3` — **not** `python3 -c` and **not** `uv run`, because the
hook on this machine rejects the former and the latter cannot see the system
`pcbnew` module.

| Script | Does |
|---|---|
| `01_extract_front.py` | bore, emitters, outline from the front scan |
| `02_extract_back.py` | outline from the back scan (clean silhouette) |
| `03_register.py` | solves the mirror + rotation between the two scans, and **verifies** it by predicting the mounting holes |
| `04_merge.py` | merges both into one front-frame `geometry.json` |
| `05_outline_to_kicad.py` | smooths, reconstructs the connector sector, writes `Edge.Cuts` |
| `06_visual_check.py` | renders `/tmp/check.png` for eyeballing |

## Two things that are easy to get wrong

**The scans are mirrored relative to each other**, and the board was placed at a
different rotation each time. The transform is
`back_angle = (190.50 - front_angle) mod 360`. It is not assumed — script 03
solves it by outline correlation and then verifies it by predicting where the
three back-scan mounting holes should appear on the front scan. All three land
within 1.81 degrees. If you rescan, re-run 03 and check that verification still
passes before trusting anything downstream.

**The connector sector reads short.** A ray cast from the bore centre stops at
the connector body, which is light and merges with the background — not at the
board edge. That sector (86.0-114.5 deg in the front frame) is reconstructed by
interpolating between clean neighbours. It is the one part of the outline not
measured, and it is why the 1:1 print check is not optional.
