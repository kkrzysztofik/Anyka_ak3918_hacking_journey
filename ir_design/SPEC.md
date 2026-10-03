# IR LED ring replacement — design spec

Source hardware: LED ring board silkscreened `RZ-XHR(08SG)-C4`, part of the
Anyka AK3918 camera project. Original board has 8 emitters (4× white "B+",
4× IR "H+" 850nm), no current regulation — just an NPN low-side switch
(S8050, marked J3Y) plus a ballast resistor per string. Suspected resistor
thermal margin issue at the package's rated power; redesigning from scratch
rather than repairing.

## Locked decisions

- **Input rail: 5 V**, taken from the existing 4-pin header (`+ − IR HB`).
  Not yet measured directly at the pins — verify before finalizing, but
  design against 5 V for now.
- **PCB: aluminum-core (MCPCB)**, single layer, routed entirely on top
  copper. Chosen over FR4 for thermal margin — LED die temperature is the
  actual constraint on how hard these can be driven, not the driver
  topology.
- **No white LED channel.** Both channels are now IR. This removes any
  full-color / white-light night mode the camera may have had — confirm
  this tradeoff is acceptable before committing to layout.
- **Two independent IR banks**, each switched by one of the camera's
  existing enable lines (bank A on `IR`, bank B on `HB`). This preserves
  the camera firmware's existing GPIO control and gives a natural
  half-power / full-power mode by firing one or both banks.
- **Mechanical envelope unchanged**: same outer profile (including the
  locating notches), same center bore diameter (clears the lens barrel),
  same two mounting-hole positions, same header footprint and cable exit,
  so the stock cable and dome/lens assembly still fit.

## Electrical design per bank

- 4× IR LEDs per bank, wired 2 series × 2 parallel (2S2P). Series pairing
  forces the two LEDs in a pair to share current inherently; the 2 branches
  in parallel need a small ballast resistor each (see below) to avoid the
  original board's failure mode, where the lower-Vf die in a pair hogs
  current.
- Driver IC: **AL8860** (SOT23-6 buck LED driver, wide input range,
  internal 100 mV current-sense reference, EN pin accepts a digital
  enable directly — no level shifting needed from the camera's IR/HB
  lines).
- Target current: **100 mA per LED** (2× the ~50 mA the stock IR LEDs
  were measured running at). Combined with double the LED count (8 vs 4
  total), this is roughly 4× the stock array's radiant output.
- Bank sense resistor: `R = Vref / I_total = 0.1 V / 0.2 A = 0.5 Ω`
  (two 1 Ω 0805 1% resistors in parallel — 0.5 Ω isn't a standard single
  value). 200 mA per bank = 2 parallel branches × 100 mA.
- Per-branch ballast resistor: **1 Ω**, in series with each of the 2
  parallel branches in a bank (4 total across both banks). Forces even
  current sharing between the two LEDs in a branch; voltage drop is
  negligible (~0.1 V) at the target current.
- Per bank passive support: 1× inductor (~10 µH, per AL8860 reference
  design), 1× Schottky diode, input/output caps per datasheet reference
  design.
- LED emitter: 850 nm, 2835 package to match the stock footprint —
  **pending confirmation of available radial lobe width** (see open
  items). If there's more than ~8 mm of radial room per lobe (bore edge
  to outer edge, minus clearance), 3535 package is worth it for the extra
  thermal pad area, since 100 mA is a small fraction of a 3535's typical
  300–500 mA continuous rating vs. being close to a 2835's 100–150 mA
  ceiling.

## Open items — resolve before finalizing layout

1. **Measure actual rail voltage** at the header pins directly; 5 V is
   inferred from stock resistor values, not measured.
2. **Confirm the stock LEDs are actually degraded**, not just correctly
   following day/night switching — probe each emitter's Vf individually
   (diode mode) rather than assuming failure from the photo evidence.
3. **Identify the striped component near the top-center pads.** Tentatively
   called a CdS photoresistor, but with only 4 header pins (`+ − IR HB`)
   there's no obvious return path for an analog light reading back to the
   SoC, which argues against that identification. Measure its resistance
   in room light vs. covered — if it doesn't change, it's something else
   and can be dropped from the new design; if it does, figure out what
   consumes that signal before replicating its position.
4. **Measure radial lobe width** (bore edge to outer edge of one LED
   position, minus clearance) to decide 2835 vs 3535 package.
5. Confirm the current target (100 mA/LED) against real Vf/output
   measurements from the LED evaluation rig (LM317 or AL8860 breakout +
   BPW34 photodiode in photovoltaic mode) before locking the sense
   resistor values.

## Testing/eval approach already worked out

- Simple adjustable CC source for characterizing candidate LEDs: LM317 +
  single resistor (`R = 1.25 / I_target`), or an AL8860 breakout if
  available, rather than a generic buck CC module (those are typically
  inaccurate below ~200 mA).
- Relative radiant output measurement: BPW34 photodiode in photovoltaic
  mode into a multimeter on µA range, fixed distance, dark enclosure.
  Gives relative comparison between candidate emitters, not absolute
  mW/sr — sufficient for ranking parts and checking linearity vs. current.
- 850 nm confirmed via phone camera IR bloom (visible purple glow); 940 nm
  would be much dimmer to a phone sensor.

## What's next (suggested Claude Code tasks)

- Set up a KiCad project; schematic capture for one bank (AL8860 + LEDs +
  passives), then duplicate for the second bank.
- Source or build KiCad footprints for the AL8860 (SOT23-6), the chosen
  LED package (2835 or 3535, pending item 4), and passives.
- Draw the board outline to match the stock mechanical envelope (profile,
  bore, mounting holes, header position) — likely needs the stock board's
  outline traced from photos/calipers rather than a datasheet.
- Generate BOM and a JLCPCB-ready aluminum MCPCB fab package once the
  layout is placed and routed.
- Note: PCB copper placement/routing itself is still best done
  interactively in the KiCad GUI — Claude Code is most useful here for
  schematic/netlist scripting, footprint generation, BOM management, and
  DRC/fab-output automation, not freehand trace routing.

