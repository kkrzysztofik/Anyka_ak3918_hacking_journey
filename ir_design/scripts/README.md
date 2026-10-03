# Geometry pipeline

Regenerates `ir_design/geometry.json` and `ir_design/kicad/ir-ring.kicad_pcb`
from the 3200 dpi scans in `../photos/`. Run from the repo root with
`/usr/bin/python3` — **not** `python3 -c` and **not** `uv run`, because the
hook on this machine rejects the former and the latter cannot see the system
`pcbnew` module.

| Script | Does |
|---|---|
| `01_extract_front.py` | bore, emitters, outline from the front scan |
| `02_extract_back.py` | outline from the back scan (clean silhouette), saved as `ir_design/back_profile.json` for 04 |
| `03_register.py` | solves the mirror + rotation between the two scans, and **verifies** it by predicting the mounting holes; exits 1 if they do not land. Uses `scan.py` |
| `04_merge.py` | merges both into one front-frame `geometry.json`; the outline goes in as `outline_polar_front_raw` |
| `05_outline_to_kicad.py` | reconstructs the connector sector and smooths `outline_polar_front_raw` into `outline_polar_front`, which 09 uses. The raw outline is never overwritten, so re-running 05 gives the same result. Writes only `geometry.json`: 09 builds the board, Edge.Cuts included |
| `07_schematic.py` | writes `ir-ring.kicad_sch` from one `NETS` table, with deterministic UUIDs |
| `08_verify_netlist.py` | exports KiCad's netlist and diffs it against `NETS` — the check that catches a label on the wrong pin |
| `09_place.py` | builds the whole board: outline, footprints, zones, pre-routes, stitching; asserts its geometry; exports DSN, or with `--ses` imports routes |
| `10_fill.py` | fills zones in a separate process (see below); exits 1 without saving if any copper zone stays unfilled |
| `11_fab.py` | JLCPCB order files in `fab/`: gerbers zip, BOM, CPL, `hand_assembly.txt`. Refuses to run unless DRC with schematic parity is clean, then cross-checks BOM, CPL and drill hits against the board. `fab/ORDER.md` is hand-written and kept |
| `scan.py`, `sexp.py` | helpers: scan tracing for 03, s-expression parse/dump for 07 and 08 |

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

## Board build: place -> route -> fill -> check

```sh
/usr/bin/python3 ir_design/scripts/07_schematic.py && /usr/bin/python3 ir_design/scripts/08_verify_netlist.py
/usr/bin/python3 ir_design/scripts/09_place.py                       # writes ir-ring.dsn
(cd ir_design/kicad && java -jar ~/.local/share/freerouting/freerouting-2.4.1.jar \
    -de ir-ring.dsn -do ir-ring.ses -mp 80 --gui.enabled=false)   # only when re-routing
/usr/bin/python3 ir_design/scripts/09_place.py --ses ir_design/kicad/ir-ring.ses
/usr/bin/python3 ir_design/scripts/10_fill.py ir_design/kicad/ir-ring.kicad_pcb
kicad-cli pcb drc ir_design/kicad/ir-ring.kicad_pcb --schematic-parity \
    --severity-error --exit-code-violations -o /tmp/drc.rpt           # must exit 0
/usr/bin/python3 ir_design/scripts/11_fab.py                         # fab/: gerbers zip, JLCPCB BOM + CPL
```

The DRC rules are JLCPCB's **2 oz** limits, not KiCad's defaults. Track and clearance are 0.16 mm (set in `09_place.py`) and the PTH ring is 0.254 mm (`kicad/ir-ring.kicad_dru`). A clean DRC therefore means the 2 oz order is legal as drawn.

`ir-ring.ses` is committed because Freerouting's output varies run to run. It is
the routing. The `--ses` rebuild is deterministic, so it matches the DSN it was
routed on. Freerouting 2.4.1, jar sha256 `251101c3...c6aa9`, checked against the
GitHub release digest.

Freerouting's summary always lists ~17 "unrouted" connections, and none of them are real.
They are each emitter's anode thermal vias (joined by the pad and the island zone), TP1 (joined by D2's island), and every link `09_place.py` pre-routes (the
power stage, STR7, the VOUT via). Freerouting does not count protected wires as
connections. The DRC `unconnected pads` count is the one to trust.

## KiCad 9.0.8 scripting traps met here

- **`pcbnew.LoadBoard()` corrupts the SWIG type table.** After it,
  `FootprintLoad` and `GetDesignSettings` return bare `SwigPyObject`s and the
  process segfaults. `09_place.py` therefore never loads a board: it builds from
  `CreateEmptyBoard()` every time.
- **`ZONE_FILLER.Fill()` segfaults on a `CreateEmptyBoard()` board**, even a
  trivial one, but works after `LoadBoard()`. Hence the separate `10_fill.py`.
  The two bugs point in opposite directions, and each step uses the path that
  works for it.
- **`kicad-cli pcb drc` has no refill option in 9.0.8**, so zones must be filled
  before DRC or before Gerber export.
- **Pre-existing wiring is exported to DSN as `(type route)`**, which Freerouting
  rips up. `09_place.py` rewrites it to `(type protect)`.
- **Freerouting undershoots clearance by ~1 um** (0.1488 against a 0.15 rule).
  The DSN gets +10 um of margin.
- **`FootprintLoad(path)` leaves the library nickname empty**, which fails
  `--schematic-parity`. `SetFPID(LIB_ID(lib, name))` fixes it.
- **No `duplicate_pad_numbers_are_jumpers` in 9.0.8** (it arrives in 10). The
  emitter's two anode pads, joined inside the package, are linked with a short
  F.Cu track instead.
