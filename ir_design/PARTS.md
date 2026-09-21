# Parts selection — replacement IR LED ring board

Date: 2026-09-22
Status: U1, R1 and L1 locked. Emitter package still blocked on measurement 14.

Implements Task 4 of `docs/plans/2026-09-21-ir-led-ring-design.md`. That design
doc is authoritative for topology; this file is authoritative for part numbers
and for the five numbers that picked them.

Everything below that is marked **verified** was read out of a datasheet PDF
downloaded during this session and converted with `pdftotext`. Anything not so
marked is called out as unverified. Vendor search-result summaries were *not*
treated as evidence — they were wrong about this part family more than once.

---

## Decision: U1 = TPS61165DBVR

**Texas Instruments TPS61165, SOT-23-6, LCSC/JLCPCB `C58756`, Extended part.**

Decisive reason: it is the only candidate whose numbers could be verified at
all. The HT7938A alternative is **out of stock at JLCPCB (0 units)**, is
parametrically listed at **20 mA** output — a sixth of what this board needs —
and its datasheet could not be retrieved from any source. The TPS61165 clears
every requirement with margin, and its one known hazard (the EasyScale dimming
protocol on CTRL) is provably not reachable from a static GPIO.

Datasheet: <https://www.ti.com/lit/ds/symlink/tps61165.pdf> (SLVS790E, April 2019)
LCSC: <https://www.lcsc.com/product-detail/LED-Drivers_Texas-Instruments-Texas-Instruments-TPS61165DBVR_C58756.html>

### Comparison

| Parameter | Requirement | **TPS61165** (verified) | HT7938A | PAM2803 |
|---|---|---|---|---|
| `VIN` range | must include 5.3 V | **3–18 V** ✅ — §7.2 Recommended Operating Conditions, p.4 | unverified | ruled out |
| Max `VOUT` / OVP | ≥ 20 V, OVP mandatory | **`VOUT` max 38 V; `Vovp` on SW 37 / 38 / 39 V** ✅ — §7.6 Electrical Characteristics, p.5. Latching shutdown, §8.3.2 p.9 | unverified | ruled out |
| Feedback reference | sets `R1` | **200 mV, 196/200/204 mV (±2 %)** ✅ — §7.6, p.5 | unverified (family is ~200 mV per the HT7939A blurb — *not* evidence) | ruled out |
| Switch current limit | > ~500 mA peak | **0.96 / 1.2 / 1.44 A** at `D = Dmax`; **0.7 A** start-up limit ✅ — §7.6, p.5 | unverified | **SW abs max 6 V — below the 13.6–19.4 V string.** Settled, not re-opened |
| Switching frequency | sets `L1` | **1.2 MHz fixed** ✅ — §3 Description, p.1 | 800 kHz–1.6 MHz (LCSC parametric only, not a datasheet) | — |
| Enable semantics | plain static GPIO | CTRL = enable + PWM dim + EasyScale one-wire. **Safe, see below** — §7.3 pin table p.3, §9.2.1.2.1 p.12 | unverified | — |
| JLCPCB | must be buildable | `C58756`, **Extended**, stocked (LCSC 4 722) | `C259955`, Extended, **stock 0** | — |

No third candidate was adopted. The TPS61165 satisfies every criterion, and
adding a fourth unverifiable part would not have improved the evidence.

### What could not be verified

**The HT7938A datasheet was never obtained.** Attempts, all failing:

- `WebFetch` of the LCSC URL in the design doc → 301 to
  `www.lcsc.com/datasheet/C259955.pdf`, which serves "The document link is invalid."
- `curl` with a browser UA and Referer against four LCSC mirror paths
  (`datasheet.lcsc.com/lcsc/`, `datasheet.lcsc.com/szlcsc/`,
  `wmsc.lcsc.com/.../pdf/v2/lcsc/`, `www.lcsc.com/datasheet/lcsc_datasheet_...`)
  → 8.6 kB HTML error pages or a 92-byte JSON 404. The same `wmsc` path
  pattern *did* return real PDFs for the emitters below, so the file simply is
  not there.
- Holtek's own site: `holtek.com` and `holtek.com.tw` product pages return a
  953-byte JS shell with no document links; guessed `webapi/` paths 404.
- alldatasheet.com and datasheet4u.com → HTTP 403.

So every HT7938A figure in this document is either absent or from a
distributor parametric field. It is **rejected on availability (0 stock) and on
the 20 mA parametric**, not on a datasheet comparison, and that distinction is
recorded deliberately: if it is ever restocked, the datasheet still has to be
read before it can be reconsidered.

---

## Enable semantics — risk 6 is closed

This was the item that could have thrown the part out. It does not.

**Finding: a static GPIO high cannot enter EasyScale mode. The failure is not
merely unlikely, it is unreachable — one of the three required conditions is
never produced by a level that does not move.**

TI §9.2.1.2.1, p.12. The default mode after *every* enable is PWM dimming, in
which a DC high is 100 % duty, i.e. full brightness. Entering EasyScale requires
all three of:

1. CTRL rising edge — enables the part and starts the detection window.
2. After `tes_delay` = 100 µs, CTRL driven **low for more than `tes_det` = 260 µs**.
3. That low must complete before `tes_win` = 1 ms expires, measured from the
   first low→high transition.

TI states the negative case in so many words:

> "To ensure not to enter EasyScale mode, please make sure CTRL pin is never
> held low for more than 160us."

`IR_LED` goes low→high and then stays high for hours. It emits no low pulse at
all inside the 1 ms window, so condition 2 is never met and the device settles
in PWM dimming at 100 % duty.

Three corollaries that are now **layout and firmware rules**, not observations:

- **Do not put an RC on the CTRL net.** The only route into EasyScale is a
  glitch on the rising edge lasting 260 µs–1 ms. A series resistor alone is
  fine; a series resistor *with* capacitance, or a soft pull-up fighting the
  GPIO, could manufacture exactly that glitch. Drive CTRL from the GPIO
  directly.
- **Turn-off needs CTRL low for ≥ 2.5 ms** (§9.2.1.2.1) before the device
  shuts down and the mode latch clears. A GPIO that stays low until the next
  day/night transition satisfies this by orders of magnitude.
- **The mode latch is per-startup.** Even the worst case — one boot
  misinterpreted — self-clears on the next off/on cycle. It cannot become a
  stuck-dim illuminator.

Verified against §7.3 (pin table, p.3), §7.5 (EasyScale timing, p.5) and
§9.2.1.2.1 (p.12).

---

## R1 — both values, ready to go

`R1 = Vfb / I_LED` with `Vfb = 200 mV` (verified, §7.6).

| Target | `R1` | LCSC | Actual current | Dissipation |
|---|---|---|---|---|
| **100 mA** | **2.00 Ω** 1 % 0805 | `C17606` (UNI-ROYAL 0805W8F200KT5E, 16 011 stock) | 100.0 mA | 20 mW in a 125 mW package |
| **80 mA** | **2.49 Ω** 1 % 0805 (nearest E96 to 2.50) | `C17525` (UNI-ROYAL 0805W8F249KT5E, 3 098 stock) | 80.3 mA | 16 mW |

Both are Extended parts; no Basic 0805 exists at either value.

Accuracy stack: `Vfb` ±2 % plus `R1` ±1 % gives **±3 % on LED current**, which
is well inside what the evaluation rig can distinguish. Fit 2.00 Ω for the
first build, keep 2.49 Ω on the bench, and decide on measured radiant output
per design-doc risk 7.

Junction-temperature context, now with a real number: the chosen emitters spec
**`Tj` max = 115 °C** (verified, JNJ p.4), which sits at the bottom of the
110–125 °C band the design doc assumed. The 20 °C that 80 mA buys is therefore
worth more than the doc implied, not less.

---

## L1 — 22 µH, not 33 µH

The design doc's 33 µH was derived at an assumed `f` = 1 MHz. The real number is
**1.2 MHz fixed** (verified). Recomputing the doc's own formula:

```
L = Vin·D / (f·ΔI) = 5.3 × 0.67 / (1.2 MHz × 107 mA) = 27.6 µH
```

But **TI §7.2 Recommended Operating Conditions caps the inductor at 22 µH**
(range 10–22 µH, p.4). 27.6 µH and 33 µH are both outside it. So:

**`L1` = 22 µH.**

Resulting ripple at 22 µH / 1.2 MHz, using the *measured-from-datasheet* Vf
(see the next section — it is lower than the doc assumed, which raises duty
less and helps here):

| Case | `Vout` | `D` | `ΔI` | `Iin` | ripple |
|---|---|---|---|---|---|
| typical string | 12.2 V | 0.566 | 114 mA | 271 mA | 42 % |
| worst-case high bin | 19.4 V | 0.727 | 146 mA | 431 mA | 34 % |

Worst-case peak inductor current **504 mA**. Normal boost practice is 30–50 %
ripple, so this is unremarkable; the 30 % target in the design doc was a
preference, the 22 µH ceiling is a constraint.

**Part: `C2849503` — DMBJ PNLS5040-220M, 22 µH ±20 %, 5 × 5 × 4.0 mm,
magnetically shielded, Isat/Irms 1.6 A, DCR 130 mΩ, 1 455 stock, Extended.**

Chosen over the 4 × 4 mm PNLS4018 (`C2849465`, 800 mA, 630 mΩ) because
**1.6 A exceeds the TPS61165's 1.44 A maximum switch current limit** — it does
not saturate even during a hard current-limit event, which matters given
design-doc risk 5. Back-side clearance is unconstrained (measurement 2), so the
5 × 5 footprint costs nothing. DCR loss is 24 mW at the worst-case 431 mA.
Shielded construction also serves risk 4 — this part sits on the same board as
the image sensor.

---

## Emitters D2–D9 — shortlist in both packages

Package is still blocked on **measurement 14** (radial lobe width) and
**measurement 16** (whether a 3535 body sits correctly under a stock-sized
dome). Both packages are shortlisted below.

**Convenient result: the two recommended parts are the same JNJ die family and
have byte-identical electrical specifications.** Vf, `Tj`, current rating and
radiant power are the same; only the body and beam differ. So measurement 14
changes the footprint and nothing else — no electrical rework, no re-check of
`R1`, `L1` or OVP headroom.

| | **2835 pick** | **3535 pick** | 2835 alt | 3535 alt |
|---|---|---|---|---|
| LCSC | **`C7500098`** | **`C7529167`** | `C22466172` | `C22466178` |
| Part | JNJ-LTJI0108W90/20mil/850NM | JNJ-LEJI0106W60 | Silverlight P2835P1IRS7G12 | Silverlight M3535E1IRS6G12 |
| Body | 3.5 × 2.8 mm (verified, p.3) | 3.5 × 3.5 mm | 3.5 × 2.8 mm | 3.5 × 3.5 mm |
| Beam | 90° | 60° | 120° | 120° |
| Vf spec | 1.4–2.1 V **@350 mA** | 1.4–2.1 V **@350 mA** | **1.4–2.4 V @100 mA** | 1.4–2.1 V @350 mA |
| **Vf at 100 mA** | **~1.5 V typ** (I-V curve Fig.2 p.5; Fig.4 gives 1.60 V @250 mA) | **~1.5 V typ** (same curves) | ~1.4–2.4 V, bins 1.6–2.0 / 2.0–2.2 / 2.2–2.4 | as 3535 pick |
| Continuous IF | **350 mA** (3.5× headroom) | **350 mA** | **150 mA max** (1.5× headroom) | 350 mA peak |
| `Tj` max | **115 °C** | **115 °C** | not specified | not specified |
| Rth | 100 °C/W derating curve, Fig.6 | 100 °C/W curve | 20 °C/W | 4.5 °C/W |
| Po @350 mA | 210 mW | 200 mW | 60 mW max optical | >100 mW |
| Stock | 626 | 536 | 3 908 | **49 — unbuildable** |
| **8S string** | **10.4–16.0 V**, typ 12.0 V | **10.4–16.0 V**, typ 12.0 V | **12.8–19.2 V** | 12.8–19.2 V |

Datasheets (all downloaded and read this session):
- `C7500098` <https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2309141627_JNJ-OPTOELECTRONICS-JNJ-LTJI0108W90-20mil-850NM_C7500098.pdf>
- `C7529167` <https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2308251147_JNJ-OPTOELECTRONICS-JNJ-LEJI0106W60_C7529167.pdf>
- `C22466172` <https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2406181656_Silverlight-P2835P1IRS7G12-850nm_C22466172.pdf>
- `C22466178` <https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2406181659_Silverlight-M3535E1IRS6G12-850NM_C22466178.pdf>

### OVP headroom — no candidate is close

Worst 8S string across *every* shortlisted part is the Silverlight 2.2–2.4 V
bin: `8 × 2.4 + 0.2` (the `R1` drop) = **19.4 V**. Minimum OVP is **37 V**.

**Headroom 1.9×. Nothing needs flagging.** The design doc's own worst case
(`8 × 2.2 = 17.6 V`) was, if anything, pessimistic.

### Why JNJ over Silverlight

- **Headroom.** 350 mA continuous against our 100 mA is 3.5×. The Silverlight
  2835 is rated 150 mA continuous *and* 150 mA peak — running it at 100 mA is
  running it at two-thirds of absolute maximum on a board whose whole premise
  is that the emitters now run hot (measurements, "Thermal").
- **Evidence quality.** JNJ publishes `Tj` max, an I-V curve, a flux-vs-current
  curve and an ambient-derating curve. Silverlight publishes a bin table whose
  tolerance note says `±0.05V@IF=350mA` **on the 100 mA part as well** — a
  copy-paste error in the vendor document, and a reason to distrust its other
  numbers.
- Silverlight's 3535 has 49 units in stock; it cannot build a board needing 8.

Against that: Silverlight's 2835 is the only part that specifies Vf *at* 100 mA
directly, and it has 3 908 in stock versus JNJ's 626. If the run size exceeds
78 boards, revisit.

Beam: 90° (2835) or 60° (3535) — the stock lens array's domes sit over the
emitters and will re-shape the output, so this is a measurement-16 question,
not a datasheet one.

---

## The design doc's Vf assumption is wrong, and it matters twice

The design doc says "datasheet Vf is 1.7–2.0 V at [100 mA]". **Both JNJ
datasheets put typical Vf at 100 mA near 1.5 V** — Fig.2 (If vs Vf, log axis,
p.5) reads ≈1.5 V at 100 mA, and Fig.4 gives 1.60 V at 250 mA / 25 °C.

Once, this is good news:

- Typical 8S string is **12.2 V**, not 16 V. Rail draw at full power is
  **271 mA**, not 355 mA — design-doc **risk 3 gets easier**, not harder.
- Per-emitter dissipation is **150 mW**, not 180 mW, so the thermal budget
  drops from ~1.25 W to roughly **1.1 W**.

Once, it is a problem. See below.

### ⚠ The 4-of-8 bypass does not have enough headroom

Half power puts **4** emitters in circuit. Required: string voltage must stay
above the 5.3 V rail, or a boost converter cannot regulate at all.

| Condition | 4-emitter string + `R1` | vs 5.3 V rail |
|---|---|---|
| typical Vf, 25 °C | 4 × 1.5 + 0.2 = **6.2 V** | +0.9 V |
| typical Vf, Tj ≈ 105 °C (−2 mV/°C/junction) | 4 × 1.34 + 0.2 = **5.56 V** | **+0.26 V** |
| low bin, hot | 4 × 1.14 + 0.2 = **4.76 V** | **−0.54 V — below the rail** |

This is the same marginal-headroom failure the design doc rejected the 2S2P
buck topology for, reappearing in half-power mode. It matters more than it
looks, because per the doc's own "Firmware consequence" section **half power is
the state unmodified firmware boots into** — it is the shipping default until
the `plan()` change lands, not a corner case.

The `Q1`/`Q2`/`R5` part choices below are unaffected either way, so this does
not block ordering. Options, for the design owner rather than for this file to
settle:

- **Bypass 2 emitters instead of 4** (6 in circuit): hot low-bin string 7.04 V,
  **1.74 V** of headroom. Safe. "Half power" becomes 75 % power.
- **Bypass 3** (5 in circuit): hot low-bin 5.9 V, +0.6 V. Still thin.
- **Drop the half-power mode** and let `R1` be the only output knob.

Bypassing 2 is the cheapest fix — it changes which node `Q1` lands on and
nothing else.

---

## Bill of materials

22 designators, not 21 — see `C4`.

| Ref | Value | LCSC | Package | JLC | Stock | Datasheet |
|---|---|---|---|---|---|---|
| **U1** | TPS61165DBVR boost LED driver | `C58756` | SOT-23-6 | **Extended** | 4 722 | [TI SLVS790E](https://www.ti.com/lit/ds/symlink/tps61165.pdf) |
| **L1** | 22 µH shielded, Isat 1.6 A, 130 mΩ | `C2849503` | SMD 5 × 5 × 4.0 mm | Extended | 1 455 | [LCSC](https://www.lcsc.com/product-detail/C2849503.html) |
| **D1** | Schottky 60 V 1 A, `Vf` 580 mV @1 A | `C77343` | SOD-123 | Extended | 45 265 | [LCSC](https://www.lcsc.com/product-detail/C77343.html) |
| **C1** | 10 µF 25 V X5R input | `C15850` | 0805 | **Basic** | 5.9 M | [LCSC](https://www.lcsc.com/product-detail/C15850.html) |
| **C2** | **1 µF 50 V** X7R output | `C28323` | 0805 | **Basic** | 2.8 M | [LCSC](https://www.lcsc.com/product-detail/C28323.html) |
| **C3** | 100 nF 50 V X7R input bypass | `C307331` | 0402 | **Basic** | 13 M | [LCSC](https://www.lcsc.com/product-detail/C307331.html) |
| **C4** | **220 nF 16 V X7R — COMP compensation, NEW** | `C16772` | 0402 | **Basic** | 2.8 M | [LCSC](https://www.lcsc.com/product-detail/C16772.html) |
| **R1** | **2.00 Ω 1 %** (100 mA) / 2.49 Ω (80 mA) | `C17606` / `C17525` | 0805 | Extended | 16 011 / 3 098 | [LCSC](https://www.lcsc.com/product-detail/C17606.html) |
| **Q1** | AO3400A N-MOSFET 30 V, `Vgs(th)` 1.45 V max, 48 mΩ @2.5 V | `C20917` | SOT-23 | **Basic** | 830 590 | [LCSC](https://www.lcsc.com/product-detail/C20917.html) |
| **Q2** | MMBT3904 NPN 40 V, hFE 100–300 | `C20526` | SOT-23 | **Basic** | 250 471 | [LCSC](https://www.lcsc.com/product-detail/C20526.html) |
| **R2** | 100 kΩ 1 % gate pull-up to 5.3 V | `C25741` | 0402 | **Basic** | 9.7 M | [LCSC](https://www.lcsc.com/product-detail/C25741.html) |
| **R3** | 10 kΩ 1 % Q2 base, from `HB` | `C25744` | 0402 | **Basic** | 25.5 M | [LCSC](https://www.lcsc.com/product-detail/C25744.html) |
| **R4** | 100 kΩ 1 % Q2 base pull-down | `C25741` | 0402 | **Basic** | 9.7 M | as R2 |
| **R5** | 4.7 Ω 1 % bypass inrush limiter, 47 mW | `C17675` | 0805 | **Basic** | 80 222 | [LCSC](https://www.lcsc.com/product-detail/C17675.html) |
| **D2–D9** | 850 nm IR ×8 — **2835** | `C7500098` | SMD 3.5 × 2.8 mm, 90° | Extended | 626 | JNJ, link above |
| | 850 nm IR ×8 — **3535** | `C7529167` | SMD 3.5 × 3.5 mm, 60° | Extended | 536 | JNJ, link above |
| **J1** | 4-pin header, `+ − IR HB` | **PENDING** | **PENDING — measurement 11** | — | — | — |

**Extended-part loading fees: 5 distinct parts** (`U1`, `L1`, `D1`, `R1`,
`D2–D9`). Everything else is Basic. Deliberate: the three parts where a Basic
substitute would have cost real margin are the driver, the inductor and the
emitters, and those are exactly the three where no Basic option exists anyway.

`J1` is left blank on purpose. Its pitch, mounting style, radial position and
cable-exit direction are all measurement 11, and the whole point of the header
is that the stock cable still plugs into it — a guessed pitch produces a board
that does not connect.

### Alternates, one line each

- **D1** → `C14996` (SS210, 100 V 2 A, SMA, **Basic**, 869 k stock) saves one
  loading fee; costs a 5.0 × 2.6 mm footprint in the `C1`–`U1`–`D1`–`C2` loop
  that design-doc risk 4 wants kept tight. SOD-123 was kept for the loop.
- **L1** → `C2849465` (PNLS4018, 22 µH, 4 × 4 mm, 800 mA) if the back is
  tighter than measurement 2 suggests; it saturates under a current-limit event.
- **D2–D9** → `C22466172` (Silverlight 2835, 3 908 stock) if the run exceeds
  78 boards.

---

## Three corrections to the design doc's BOM

Sourcing turned up three places where the doc's part values are wrong against
the driver that was actually selected. All three follow from one fact: **OVP
trips at 37–39 V on the SW node**, so on an open-string fault the output rail
reaches ~38 V for 8 switching cycles before the latch fires (§8.3.2, verified).

1. **`C2` must be 50 V, not 25 V.** It sits on the output node and sees the OVP
   excursion. `C28323` (50 V, Basic) fixes it at no cost.
2. **`D1` must be ≥ 40 V, not 30 V.** Same reason; TI §9.1.3 says outright that
   "the reverse breakdown voltage of the diode must exceed the open LED
   protection voltage", and TI's own reference design uses a 40 V MBR0540.
   60 V specified here for margin.
3. **`C4` is missing entirely.** SOT-23-6 pin 5 is **COMP**, the
   transconductance error amplifier output, and TI §9.1.4 requires a
   compensation capacitor to ground — "a 220 nF ceramic capacitor is suitable
   for most applications". The doc's 21-designator BOM has no such part. The
   board is 22 designators.

Plus the two already covered above: `L1` is 22 µH (TI's 10–22 µH recommended
range), and the 4-of-8 bypass needs rethinking against a real 1.5 V Vf.

### One thing deliberately left as-is

`C2` = 1 µF is the **bottom** of TI's recommended 1–10 µF output-cap range, and
TI warns that "if the output capacitor is below the range, the boost regulator
can potentially become unstable" (§9.1.5). The design doc chose 1 µF on purpose
to cap bypass inrush, and 1 µF is *inside* the range, so it stands. But this is
a build-and-measure item, not a settled one: check loop stability on the first
board before committing, and remember that raising `C2` raises the `R5`
inrush energy it was sized against.
