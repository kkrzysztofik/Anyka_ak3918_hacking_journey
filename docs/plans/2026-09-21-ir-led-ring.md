# IR LED Ring Board Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Produce a JLCPCB-ready **two-layer FR4** replacement for the camera's `RZ-XHR(08SG)-C4` IR ring board — 8 × 850 nm emitters in one series string at 100 mA driven by a single boost LED driver, with a switched sense resistor for half power — plus the `onvif-rust` change that lets the night transition assert the `HB` line.

**Architecture:** One TPS61165 boost converter steps the **measured 5.3 V** header rail up to the 10.4–16.0 V that eight series 850 nm emitters need at 100 mA. Series wiring makes current matching exact, so there are no ballast resistors. An N-MOSFET gated from `HB` parallels a second sense resistor to switch between 50 mA and 100 mA — all eight emitters stay lit in both modes. Emitters on the front under the stock lens array; converter cluster on the back. Firmware gains one config key and one pure step-mirroring function so `WHITE_LED` follows `IR_LED` on boards where that line drives infrared.

**Tech Stack:** KiCad 9.0.8 (Ubuntu archive), JLCPCB two-layer FR4 with 2 oz copper, Rust 1.x via the vendored `arm-anykav200-crosstool-ng` toolchain.

> **Revision note (2026-09-22):** the original plan specified a single-layer aluminium MCPCB and a MOSFET bypass across four emitters. Both are superseded — the stock board turned out to be FR4, and the 4-emitter bypass cannot regulate at operating temperature. See the design doc.

**Design doc:** `docs/plans/2026-09-21-ir-led-ring-design.md`

---

## Task ordering

Tasks 1 and 2 are **blocking gates**: their results can invalidate the whole
topology, so nothing in Phase C or later starts until they pass. Task 10
(firmware) depends on nothing and can be done at any point — start it first if
the camera is not available to measure.

| Phase | Tasks | Blocked by |
|---|---|---|
| A — Measure | 1, 2 | nothing (needs camera + calipers) |
| B — Tooling | 3 | nothing |
| C — Part selection | 4 | 1, 3 |
| D — CAD | 5, 6, 7, 8, 9 | 2, 4 |
| E — Firmware | 10 | nothing |

---

## Phase A — Measurement gates

### Task 1: Electrical measurements

**Files:**
- Create: `ir_design/MEASUREMENTS.md`

These are physical measurements. Record every number, including the ones that
look boring — a later task reads this file rather than re-measuring.

**Step 1: Create the results file**

```markdown
# Measured values — RZ-XHR(08SG)-C4 stock ring

Date:
Camera:
Instrument:

## Electrical
| # | Measurement | Value | Notes |
|---|---|---|---|
| 1 | Header rail | **5.3** | V — measured 2026-09-22, gate PASSED |
| 1 | Silkscreen pin order | `- + IR HB` | read from photo; confirm against cable |
| 2 | Back-side clearance | | mm — GATE, must be ≥3.5 |
| 2 | Location of most generous back region | | |
| 3 | `IR` line asserted | | V |
| 3 | `HB` line asserted | | V — **CRITICAL**: drives Q1 gate directly; 1.8 V will not switch it |
| 4 | Board current, IR channel on | | mA |
| 5 | Vf D1..D8 (diode mode) | | V each |
| 6 | Striped component, room light | | Ω |
| 6 | Striped component, covered | | Ω |
```

**Step 2: Measure the rail (gate) — DONE 2026-09-22, PASSED**

Measured **5.3 V** at the header. The boost topology is confirmed; a 12 V
result would have sent it back to a buck. L1 recomputed against 5.3 V:
`D = 1 − 5.3/16 = 0.67`, input 271 mA at the measured Vf; L1 settled in Task 4 at **22 µH**, TI's cap.

Nothing further to do in this step.

**Step 3: Measure BACK-side clearance (gate) — DONE 2026-09-22, PASSED**

Clearance is adequate across the whole back face, so converter placement is
unconstrained and the 22 µH inductor fits. The fallback to a linear current
sink is off the table.

Also settled in the same session: **the stock board is FR4, not aluminium** —
two scratched points on the bare back read open circuit. The redesign is
therefore an upgrade on the stock substrate rather than a tradeoff against a
metal core, and `SPEC.md`'s resistor-thermal suspicion gains a mechanism
(~0.12 W in an 0805 on single-layer FR4 with no pour). The new topology has no
ballast resistors at all.

Nothing further to do in this step.

**Step 4: Measure the GPIO logic levels**

Drive the lines from the camera over telnet and probe each at the header.
See @.claude/skills/anyka-remote-debugging for the shell access pattern.

Expect 3.3 V. A 5 V result means R3/R4, the Q1 gate network, needs rechecking,
and U1's `CTRL` input must tolerate 5 V (TPS61165 `CTRL` is rated to 7 V, so
this is fine either way — but confirm rather than assume).

**Step 5: Measure stock current draw**

Break the `+` wire and meter it in series with the IR channel on.

This is the headroom check against the new board's 271 mA at full power. If the
stock board draws ~200 mA and the supply has no margin, note it — it becomes a
constraint on whether full power is usable continuously.

**Step 6: Measure each emitter's Vf and the striped component**

Multimeter in diode mode across each of the 8 emitters individually. A dead or
badly mismatched emitter confirms `SPEC.md` open item 2.

Then the striped component near the top-centre pads: resistance in room light
versus covered. **No change** → it is not a photoresistor and can be dropped
from the new design. **Changes** → find what consumes the signal before
replicating its position, because 4 header pins leave no obvious return path.

**Step 7: Commit**

```bash
rtk git add ir_design/MEASUREMENTS.md
rtk git commit -m "docs(ir): electrical measurements from the stock ring board"
```

---

### Task 2: Mechanical measurements

**Files:**
- Modify: `ir_design/MEASUREMENTS.md`

**Step 1: Append the mechanical table**

```markdown
## Mechanical (calipers, board removed)
| # | Measurement | Value | Notes |
|---|---|---|---|
| 7 | Outer diameter | | mm; circle or flats? |
| 8 | Notch 1: angle / width / depth | | ° / mm / mm |
| 8 | Notch 2: angle / width / depth | | ° / mm / mm |
| 9 | Centre bore diameter | | mm |
| 10 | Mounting hole diameter | | mm |
| 10 | Mounting hole centre-to-centre | | mm |
| 10 | Mounting hole radius from centre | | mm |
| 11 | Header pitch | | mm |
| 11 | Header through-hole or SMD | | |
| 11 | Header distance from centre / angle | | mm / ° |
| 11 | Cable exit direction | | |
| 12 | Board thickness | | mm |
| 13 | LED pad radius from centre | | mm |
| 13 | LED angular positions (×8) | | ° |
| 14 | Radial lobe width at an LED | | mm |
| 15 | Largest clear area on the BACK (W × H) and position | | mm; note mounting bosses and cable landing |
| 16 | Lens array: OD, locating features, dome positions, standoff | | decides front keep-out and whether a 3535 fits under a stock-sized dome |
```

**Step 2: Measure, choosing one notch as the 0° datum**

Every angle in the table is referenced to the same datum. State which notch it
is at the top of the section — an outline built against the wrong datum is
mirrored and will not fit.

**Step 3: Decide the emitter package**

From measurement 14:

- **> ~8 mm of radial room** → **3535**. 100 mA is a small fraction of a
  3535's 300–500 mA continuous rating, and the larger thermal pad is free
  margin.
- **≤ ~8 mm** → **2835**, matching the stock footprint. Note that 100 mA is
  close to a 2835's 100–150 mA ceiling, so the thermal vias and pours are doing
  real work.

Record the decision and the reasoning in the file.

**Step 4: Commit**

```bash
rtk git add ir_design/MEASUREMENTS.md
rtk git commit -m "docs(ir): mechanical envelope of the stock ring board"
```

---

## Phase B — Tooling

### Task 3: Install KiCad

**Step 1: Install**

```bash
sudo apt update && sudo apt install -y kicad kicad-footprints kicad-symbols kicad-libraries
```

Ubuntu 26.04 ships KiCad 9.0.8 in the archive — no PPA is needed.

**Step 2: Verify the CLI and the Python API**

```bash
kicad-cli version
python3 -c "import pcbnew; print(pcbnew.Version())"
```

Expected: a `9.0.x` version string from both. The `pcbnew` import is what
later tasks use for scripted footprint generation and fab output; if it fails,
the package `kicad` did not bring in the Python bindings and
`python3-pcbnew` is needed separately.

**Step 3: Verify fab-output options**

```bash
kicad-cli pcb export gerbers --help | head -20
```

Expected: usage text listing `--layers` and `--output`. Single-layer MCPCB fab
needs `F.Cu`, `F.Mask`, `F.Paste`, `F.Silkscreen`, `Edge.Cuts` only.

No commit — nothing in the repo changed.

---

## Phase C — Part selection

### Task 4: Lock the driver IC, R1 and L1

**Files:**
- Create: `ir_design/PARTS.md`

**Step 1: Pull the candidate datasheets**

Primary candidates, both SOT-23-6 boost LED drivers:

- **HT7938A** — Holtek, JLCPCB `C259955`, `https://datasheet.lcsc.com/lcsc/1912111437_Holtek-Semicon-HT7938A_C259955.pdf`
- **TPS61165** — TI, `VIN` 3–18 V, 38 V OVP, 200 mV reference

**PAM2803 is already ruled out** — its SW pin absolute maximum is 6 V, below
our 13.6–16.0 V string. Do not re-evaluate it.

**Step 2: Extract the five numbers that decide it**

For each candidate record:

| Parameter | Why it matters |
|---|---|
| `VIN` operating range | must include the measured rail from Task 1 |
| Max output / OVP threshold | must exceed 16 V with margin; **OVP is mandatory** — an open string on a boost runs the output away |
| Feedback reference voltage | sets `R1 = Vfb / 0.1 A` |
| Switch current limit | must exceed the ~450 mA peak inductor current |
| EN / CTRL semantics | see step 3 |

**Step 3: Check the enable semantics specifically**

TPS61165's CTRL pin implements a one-wire dimming protocol in which a plain DC
high means full brightness, but a toggling input can in principle enter the
protocol. The camera drives `IR` as a bare GPIO. Either confirm from the
datasheet that a static high is unambiguous, or prefer a part with a plain EN
pin. Record the finding — this is a real failure mode, not a formality.

**Step 4: Compute R1 and L1 from the chosen part**

```
R1 = Vfb / 0.100 A          # 2.0 Ω for a 200 mV reference
D  = 1 − Vin / Vstring      # ≈ 0.69 at 5 V in, 16 V out
ΔI = 0.30 × (Vstring × 0.1 A) / (0.85 × Vin)   # ≈ 113 mA at 30 % ripple
L1 = Vin × D / (f × ΔI)     # ≈ 30 µH at 1 MHz
```

Round L1 to a stocked value (TI caps it at 22 µH) with `Isat ≥ 600 mA`, and **check its
height against measurement 2** before accepting it. Use a 1 % resistor for R1.

**Step 5: Record the full BOM with LCSC part numbers**

Write `ir_design/PARTS.md` with one row per designator from the design doc's
BOM table, each carrying a concrete LCSC number, package, and the datasheet
URL. Flag which are JLCPCB Basic versus Extended parts — Extended parts carry
a per-part loading fee.

**Step 6: Commit**

```bash
rtk git add ir_design/PARTS.md
rtk git commit -m "docs(ir): lock the boost driver, sense resistor and inductor"
```

---

## Phase D — CAD

### Task 5: KiCad project and footprints

**Files:**
- Create: `ir_design/kicad/ir-ring.kicad_pro`, `ir-ring.kicad_sch`, `ir-ring.kicad_pcb`
- Create: `ir_design/kicad/ir-ring.pretty/` (custom footprint library)

**Step 1: Create the project**

```bash
mkdir -p ir_design/kicad/ir-ring.pretty
kicad-cli project new ir_design/kicad/ir-ring  # or create via GUI if unsupported
```

**Step 2: Identify which footprints already exist**

Standard library covers SOT-23-6, SOT-23, SOD-123, 0402/0805/1206 chip
packages. Do **not** hand-draw these.

```bash
ls /usr/share/kicad/footprints/Package_TO_SOT_SMD.pretty/ | grep -i sot-23
ls /usr/share/kicad/footprints/Resistor_SMD.pretty/ | grep -i 0805
```

Expected: `SOT-23-6.kicad_mod`, `R_0805_2012Metric.kicad_mod` and siblings
present. Anything found here is reused as-is.

**Step 3: Create only the footprints that do not exist**

Two are genuinely custom:

1. **The emitter** (2835 or 3535 per Task 2 step 3) — pad geometry and thermal
   pad from the chosen part's datasheet, not from a generic template.
2. **J1, the 4-pin header** — from measurement 11. Pitch, hole or pad size, and
   orientation must match the stock connector exactly or the camera's cable
   will not mate.

Draw both in the footprint editor, save into `ir-ring.pretty`.

**Step 4: Verify the footprints against the measurements**

```bash
kicad-cli fp export svg ir_design/kicad/ir-ring.pretty -o /tmp/fp
```

Open the SVGs and check J1's pitch and the emitter's pad spacing against the
numbers in `MEASUREMENTS.md`. A footprint that is wrong here is a scrapped
board later.

**Step 5: Commit**

```bash
rtk git add ir_design/kicad
rtk git commit -m "feat(ir): KiCad project and custom footprints"
```

---

### Task 6: Board outline

**Files:**
- Modify: `ir_design/kicad/ir-ring.kicad_pcb`

**Step 1: Generate Edge.Cuts from the measurements**

Build the outline from `MEASUREMENTS.md` items 7–12, every angle referenced to
the datum notch declared in Task 2 step 2:

- Outer circle at measurement 7's radius
- Both notches at their measured angles, widths and depths
- Centre bore at measurement 9's radius
- Both mounting holes at measurement 10's radius and angles

A ring outline is regular enough to script — a short Python file using `pcbnew`
to place arcs and circles is more reliable and more reviewable than clicking
it. Keep that script in the repo so the outline can be regenerated when a
measurement is corrected.

**Step 2: Verify the outline against the physical board**

```bash
kicad-cli pcb export svg ir_design/kicad/ir-ring.kicad_pcb \
  --layers Edge.Cuts --output /tmp/outline.svg --exclude-drawing-sheet
```

Print `/tmp/outline.svg` at exactly 1:1 scale and lay the stock board on top.
Notches, bore and both mounting holes must line up. This is the single
cheapest check in the whole plan and it catches datum errors, unit errors and
mirroring.

**Step 3: Commit**

```bash
rtk git add ir_design/kicad
rtk git commit -m "feat(ir): board outline traced from the stock envelope"
```

---

### Task 7: Schematic — DONE 2026-09-22

Generated by `ir_design/scripts/07_schematic.py`, which is label-connected and
sources its netlist from one table (`NETS`). `08_verify_netlist.py` exports the
netlist KiCad computed and diffs it against that table. ERC is 0 violations
at `--severity-all`, and the diff is identical. The diff check was proven
load-bearing: a deliberately mislabelled gate passed ERC (exit 0) and failed
the diff (exit 1). The steps below are the original plan, kept for reference.


**Files:**
- Modify: `ir_design/kicad/ir-ring.kicad_sch`

U1 is **TPS61165DBVR** in SOT-23-6. Pins: VIN, SW, FB, GND, CTRL, **COMP**.

**Step 1: Draw the power and driver section**

J1 `+`/`−` → C1 (10 µF) and C3 (100 nF) → U1 `VIN`. L1 (22 µH) from `VIN` to
`SW`, D1 (Schottky **≥ 40 V**) from `SW` to the string anode, C2 (4.7 µF,
**50 V** — it sees the ~38 V OVP excursion) from string anode to ground.
**C4 (220 nF) from `COMP` to ground** — required by TI, and keep it off the
switching loop on quiet analogue ground. U1's `CTRL` from J1 `IR`.

**Step 2: Draw the string and the current switch**

D2…D9 in series, anode of D2 at the boost output, cathode of D9 to the sense
node, which ties to U1's `FB`.

**All eight emitters stay in circuit in both modes.** Half power halves the
current, it does not shorten the string:

- **R1a** (≈ 4.0 Ω 1 %) from the sense node to ground — permanent, sets 50 mA
- **R1b** (same value) from the sense node to Q1's drain; Q1's source to ground
- **R3** (1 kΩ — not 10 kΩ; a MOSFET gate is a DC open, so R3/R4 divide) from J1 `HB` to Q1's gate; **R4** (100 kΩ) gate to ground

Resulting logic:
- `HB` low → Q1 off → 4.0 Ω → 50 mA → **half power**
- `HB` high → Q1 on → R1a‖R1b ≈ 2.0 Ω → 100 mA → **full power**

No inverter and no level shifting: Q1's source is at ground, so `HB` drives the
gate directly. There is no Q2, R2 or R5 in this design.

**Step 3: Annotate the CTRL net in the schematic text**

Add a text note beside U1's `CTRL` pin: *"No RC on this net. A 260 µs–1 ms low
pulse after a rising edge enters TI's EasyScale dimming protocol and the
illuminator comes up at an arbitrary level. A static GPIO cannot do this; an RC
can."*

(The superseded design also had an R5 inrush limiter needing a note. R5 no
longer exists — there is no output-cap dump without a shortened string.)

**Step 4: Run ERC**

```bash
kicad-cli sch erc ir_design/kicad/ir-ring.kicad_sch \
  --output /tmp/erc.rpt --severity-error --exit-code-violations
echo "exit=$?"
```

Expected: `exit=0`. Read `/tmp/erc.rpt` even on success — warnings about
unconnected pins are how a missed net shows up.

**Step 5: Commit**

```bash
rtk git add ir_design/kicad
rtk git commit -m "feat(ir): schematic for the boost driver, string and current switch"
```

---

### Task 8: Layout — DONE 2026-09-22 (scripted placement + Freerouting)

Built by `ir_design/scripts/09_place.py`, routed by Freerouting 2.4.1, and
filled by `10_fill.py`. **DRC with `--schematic-parity` exits 0 at error
severity: 0 violations, 0 unconnected pads, 0 footprint errors.** Remaining
warnings are 2 copper slivers in the F.Cu pour and silkscreen cosmetics (JLCPCB
clips silk at mask openings). See `ir_design/scripts/README.md` for the pipeline
and the KiCad 9.0.8 scripting traps it works around. The steps below are the
original plan, kept for reference.


**Files:**
- Modify: `ir_design/kicad/ir-ring.kicad_pcb`

Routing is interactive work in pcbnew. These are the constraints it must
satisfy, not a click-by-click script.

Substrate is **two-layer FR4**: emitters on `F.Cu`, converter on `B.Cu`.

**Step 1: Place the eight emitters, front side**

At measurement 13's radius and angles. These positions are fixed by the optics
and the lens array; everything else works around them. Check every placement
against measurement 16 — a pad that drifts puts its emitter off-axis under its
dome.

**Step 2: Thermal vias under every emitter pad**

This is the entire thermal design, and it is not optional. A grid of
0.3 mm vias through each emitter's thermal pad into the back-side pour. The
board dissipates ~1.1 W with no conduction path to any heatsink, which is
**3.5× the stock board's IR dissipation** — the vias and the two pours are the
entire thermal design.

**Step 3: Place the converter cluster, back side**

Into the clear back area from measurement 15. **C1 → U1 → D1 → C2 must form
one tight loop.** Keep it on `B.Cu` over an unbroken ground pour — that plane
is the mitigation that the single-layer design could not have, and it now also
sits between the switcher and the sensor.

**Step 4: Size the copper and pour both layers**

| Net | Current | Minimum width |
|---|---|---|
| `VIN` from J1 | 271 mA | 0.4 mm |
| `SW` node | ~450 mA peak | 0.5 mm, kept short |
| LED string | 100 mA | 0.3 mm |
| Signal (`HB`, gate, base) | negligible | 0.2 mm |

Ground pour on both layers, stitched. Do not let the pour under the converter
be split by a signal trace — a slot in the return path under the `SW` node
undoes the reason for choosing FR4.

**Step 4: Run DRC**

```bash
kicad-cli pcb drc ir_design/kicad/ir-ring.kicad_pcb \
  --output /tmp/drc.rpt --severity-error --exit-code-violations
echo "exit=$?"
```

Expected: `exit=0`. Set the design rules to JLCPCB's standard two-layer FR4
minimums first — 0.127 mm track/clearance, 0.3 mm via with 0.2 mm drill. Vias
are now both allowed and load-bearing, which was not true of the superseded
aluminium process.

**Step 5: Verify the 1:1 print again with components placed**

Same print-and-overlay check as Task 6, this time confirming that nothing
placed overlaps a mounting hole, the bore, or a notch, and that L1's footprint
sits inside the clear region from measurement 15.

**Step 6: Commit**

```bash
rtk git add ir_design/kicad
rtk git commit -m "feat(ir): two-layer FR4 layout, emitters front, converter back"
```

---

### Task 9: Fab package — DONE 2026-09-23 (`scripts/11_fab.py` → `ir_design/fab/`)

Deviations from the steps below:
- **Board rules raised to JLCPCB's 2 oz limits:** 0.16/0.16 mm track and space, and a 0.254 mm PTH ring in `kicad/ir-ring.kicad_dru`. The board was drawn to 0.15 mm, which is legal only at 1 oz.
- **J1 got a custom footprint** (`..._Ring0.275`). KiCad's stock PicoBlade pads leave a 0.15 mm ring on the 0.5 mm hole. That is under JLCPCB's floor even for 1 oz (0.18 mm).
- **J1 is hand-soldered.** No LCSC listing was found for 53048-0510.
- **BOM from `parts.json`, not `sch export bom`:** JLCPCB wants grouped rows with its own column names. The CPL is KiCad's placement file with the columns renamed.
- **gerbview is GUI-only.** The script cross-checks the drill hits against the board instead. The visual check happens in JLCPCB's upload preview (see `fab/ORDER.md`).


**Files:**
- Create: `ir_design/fab/` (gerbers, drill, BOM, CPL)

**Step 1: Export gerbers for a two-layer board**

```bash
mkdir -p ir_design/fab
kicad-cli pcb export gerbers ir_design/kicad/ir-ring.kicad_pcb \
  --output ir_design/fab \
  --layers F.Cu,F.Mask,F.Paste,F.Silkscreen,B.Cu,B.Mask,B.Paste,B.Silkscreen,Edge.Cuts
kicad-cli pcb export drill ir_design/kicad/ir-ring.kicad_pcb \
  --output ir_design/fab --format excellon
```

Both sides now — the back carries the converter. The drill file is also
load-bearing this time: it holds the thermal vias, not just the mounting holes.

**Step 2: Export BOM and placement**

```bash
kicad-cli sch export bom ir_design/kicad/ir-ring.kicad_sch \
  --output ir_design/fab/bom.csv \
  --fields "Reference,Value,Footprint,LCSC"
kicad-cli pcb export pos ir_design/kicad/ir-ring.kicad_pcb \
  --output ir_design/fab/cpl.csv --format csv --units mm --side both
```

`--side both` — assembly needs placements for both faces. Exporting `front`
only would silently drop the entire converter from the CPL and you would get
boards back with eight emitters and nothing to drive them.

**Step 3: Verify the gerbers before ordering**

```bash
ls ir_design/fab
```

Expected: one file per exported layer plus the drill file. Open them in a
gerber viewer — `gerbview` ships with KiCad — and confirm the outline, that
copper does not run into the bore or the mounting holes, and that the emitter
pads are where the optics need them.

**Step 4: Cross-check the BOM against PARTS.md**

Every designator in `bom.csv` must carry the LCSC number recorded in Task 4. A
blank LCSC field means JLCPCB will not place that part.

**Step 5: Note the fab options in the order**

Two-layer FR4, **2 oz copper** (the pours are the thermal design — do not
accept the 1 oz default), thickness from measurement 12, white soldermask on
the front for reflectivity. Assembly on **both sides**.

**Step 6: Commit**

```bash
rtk git add ir_design/fab
rtk git commit -m "feat(ir): JLCPCB two-layer FR4 fab package"
```

---

## Phase E — Firmware

### Task 10: Let the night transition drive `WHITE_LED`

**Why:** `plan()` at `night_mode.rs:849` writes only `Node::IrLed`.
`Node::WhiteLed` — the `HB` line — is reachable only via `set_white_light()`
at `imaging.rs:355`, which nothing calls on a transition. With the chosen
polarity chosen, unmodified firmware would run the new board permanently at
half power.

On a **stock** board `WHITE_LED` is a visible floodlight, so this must be
opt-in per camera. `NightConfig` is `#[serde(default)]` without
`deny_unknown_fields`, so adding a key is safe for an older `onvif-rust` in the
other A/B slot — it ignores the unknown key rather than failing to parse.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs` (`NightConfig`, around line 731)
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs` (new helper; `apply()` around line 499)

Use @.claude/skills/anyka-rust-testing for test conventions and
@.claude/skills/anyka-embedded-build for the toolchain invocation.

**Step 1: Write the failing tests**

Append to the `tests` module in `night_mode.rs`:

```rust
#[test]
fn test_mirror_lamp_duplicates_each_ir_write_onto_white() {
    let mut steps = plan(DayNight::Night, pol(), true);
    mirror_lamp_to_white(&mut steps);

    let ir_at = steps
        .iter()
        .position(|s| matches!(s, Step::Write { node: Node::IrLed, value: 1 }))
        .expect("night plan writes IrLed=1");
    assert_eq!(
        steps[ir_at + 1],
        Step::Write { node: Node::WhiteLed, value: 1 },
        "the white write must immediately follow the IR write so it keeps \
         plan()'s ordering around the ISP switch"
    );

    let isp_at = steps.iter().position(|s| *s == Step::IspMode).unwrap();
    assert!(ir_at + 1 < isp_at, "lamp on before the ISP switches to night");
}

#[test]
fn test_mirror_lamp_follows_the_ir_value_off_at_day() {
    let mut steps = plan(DayNight::Day, pol(), true);
    mirror_lamp_to_white(&mut steps);

    let ir_at = steps
        .iter()
        .position(|s| matches!(s, Step::Write { node: Node::IrLed, value: 0 }))
        .expect("day plan writes IrLed=0");
    assert_eq!(
        steps[ir_at + 1],
        Step::Write { node: Node::WhiteLed, value: 0 }
    );
}

#[test]
fn test_mirror_lamp_leaves_a_plan_without_lamp_writes_alone() {
    let mut steps = vec![Step::IspMode, Step::Sleep(SETTLE)];
    let before = steps.clone();
    mirror_lamp_to_white(&mut steps);
    assert_eq!(steps, before);
}
```

And in `config/types.rs` tests:

```rust
#[test]
fn test_night_config_defaults_to_treating_white_led_as_visible() {
    // A stock ring board's WHITE_LED is a floodlight. Driving it on every
    // night transition is only correct on the replacement all-IR board, so
    // the default must be off.
    let cfg = NightConfig::default();
    assert!(!cfg.white_led_is_ir);
}
```

**Step 2: Run the tests to verify they fail**

The vendored toolchain lives at the **repo root**, not under `cross-compile/`:

```bash
cd cross-compile && \
  ../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu \
  -p onvif-rust night_mode::tests::test_mirror_lamp 2>&1 | tail -20
```

Expected: FAIL — `cannot find function 'mirror_lamp_to_white'` and
`no field 'white_led_is_ir'`.

**Step 3: Add the config key**

In `NightConfig` (`config/types.rs:731`):

```rust
    /// `true` when the `WHITE_LED` line drives an infrared emitter rather than
    /// a visible lamp, so the night transition may assert it.
    ///
    /// The replacement IR ring wires its half/full-power bypass to `HB`, which
    /// the kernel exposes as `WHITE_LED`; on that board both lines are IR. On
    /// a stock `RZ-XHR(08SG)-C4` the same line is a visible floodlight, which
    /// is why this defaults to `false`.
    pub white_led_is_ir: bool,
```

And in its `Default` impl:

```rust
            white_led_is_ir: false,
```

**Step 4: Add the mirroring helper**

In `night_mode.rs`, immediately after `plan()`:

```rust
/// Duplicate every `IrLed` write onto `WhiteLed`, in place.
///
/// Inserting each mirror directly after its source preserves `plan()`'s
/// ordering guarantee — the lamp turns on before the ISP switches to night and
/// off after it switches to day, so no frame is captured dark.
fn mirror_lamp_to_white(steps: &mut Vec<Step>) {
    let mut out = Vec::with_capacity(steps.len() + 2);
    for step in steps.drain(..) {
        let mirrored = match &step {
            Step::Write {
                node: Node::IrLed,
                value,
            } => Some(*value),
            _ => None,
        };
        out.push(step);
        if let Some(value) = mirrored {
            out.push(Step::Write {
                node: Node::WhiteLed,
                value,
            });
        }
    }
    *steps = out;
}
```

**Step 5: Run the tests to verify they pass**

```bash
cd cross-compile && \
  ../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu \
  -p onvif-rust mirror_lamp 2>&1 | tail -20
```

Expected: PASS, 3 tests.

**Step 6: Wire it into `apply()`**

In `apply()`, immediately after the existing `if !self.caps.ir_led { … }`
retain block (around line 499):

```rust
        // The replacement IR ring drives its bypass from HB, which the kernel
        // calls WHITE_LED. Gated twice: the node must exist, and the operator
        // must have declared that this camera's white line is infrared.
        if self.cfg.white_led_is_ir && self.caps.white_led {
            mirror_lamp_to_white(&mut steps);
        }
```

Placing it after the retain means a board with no `IrLed` node has no writes
left to mirror, which is the correct outcome.

**Step 7: Run the full suite and clippy**

```bash
cd cross-compile && \
  ../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust 2>&1 | tail -20
cd cross-compile && \
  PATH="$(git rev-parse --show-toplevel)/toolchain/arm-anykav200-crosstool-ng/bin:$PATH" \
  cargo clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings; echo "exit=$?"
```

Expected: all tests pass, `exit=0` from clippy. Two non-negotiables here:

- The `PATH` prefix on clippy is required — without it the vendored toolchain
  dies with `E0514`.
- Run clippy **raw, not through `rtk`**, and read `$?`. The RTK filter has been
  observed printing a success verdict over a real exit-1. Never trust a
  filtered pass/fail verdict.

**Step 8: Document the key**

Add `white_led_is_ir` to the night-mode configuration reference in `wiki/`
alongside the other `[night]` keys, stating plainly that enabling it on a
stock board turns on a visible floodlight every night.

**Step 9: Commit**

```bash
rtk git add cross-compile/onvif-rust/src wiki
rtk git commit -m "feat(night): let the night transition drive WHITE_LED on all-IR rings"
```

---

## Final verification

Before calling this done, confirm each of these and quote the output:

1. `ir_design/MEASUREMENTS.md` has a value in every row — no blanks.
2. Task 1's rail gate and Task 1's clearance gate both passed, with the
   measured numbers stated.
3. `kicad-cli sch erc` and `kicad-cli pcb drc` both exit 0.
4. The 1:1 print overlays the physical board correctly.
5. Every BOM row carries an LCSC part number.
6. `cargo test -p onvif-rust` passes and `cargo clippy -- -D warnings` is
   clean.

See @.claude/skills/superpowers/verification-before-completion — evidence
before assertions.

## Deliberately out of scope

- Any change to the ISP night profile's gain cap. That is a separate
  constraint on night image quality, documented but not addressed here.
- Restoring a white-light night mode. The all-IR decision was made with the
  36× luma measurement on the table and is recorded in the design doc.
- A prototype or eval-rig build. `SPEC.md`'s LM317 + BPW34 rig stands as
  written and is used in Task 4 only to confirm Vf at 100 mA.
