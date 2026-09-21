# Parts selection — replacement IR LED ring board

Date: 2026-09-22
Status: U1, R1a/R1b and L1 locked. Emitter package still blocked on
measurement 14; `Q1` gate drive now also blocked on measurement 3.

Implements Task 4 of `docs/plans/2026-09-21-ir-led-ring-design.md`. That design
doc is authoritative for topology; this file is authoritative for part numbers
and for the five numbers that picked them.

**Revision 2026-09-22b.** Half power is now produced by **switching the sense
resistor**, not by bypassing four emitters. All 8 emitters stay in series in
every mode. This deletes `Q2`, `R2` and `R5`, adds `R1b`, and moves `C2` to
4.7 µF. See "Half power" below for why the bypass scheme was abandoned.

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

## Half power — switched sense resistor, not a bypass

All 8 emitters are permanently in series. Half power halves the **regulated
current** instead of shortening the string, so the string voltage never
approaches the 5.3 V rail in any mode.

```
R1a          FB → GND, permanent          sized for the HALF current
R1b + Q1     FB → Q1 → GND                parallels in for FULL current
Q1 gate      HB → R3 → gate, R4 to GND    source at ground, no level shift
```

Polarity: **`HB` high → Q1 on → ~2 Ω → full power. `HB` low → Q1 off → 4 Ω →
half power.** Unmodified firmware therefore boots into half power, which is
expected and is handled by the `plan()` change shipped in Task 10.

### Why the 4-of-8 bypass was abandoned

Recorded here because it is the reason this board looks the way it does.

The bypass scheme put **4** emitters in circuit at half power. A boost
converter cannot regulate unless the string voltage exceeds its input:

| Condition | 4-emitter string + `R1` | vs 5.3 V rail |
|---|---|---|
| typical Vf, 25 °C | 4 × 1.5 + 0.2 = **6.2 V** | +0.9 V |
| typical Vf, `Tj` ≈ 105 °C (−2 mV/°C/junction) | 4 × 1.34 + 0.2 = **5.56 V** | **+0.26 V** |
| low bin, hot | 4 × 1.14 + 0.2 = **4.76 V** | **−0.54 V — below the rail** |

That is the same marginal-headroom failure the design doc rejected the 2S2P
buck topology for, reappearing in half-power mode — and since half power is
the boot default, it was the shipping state, not a corner case. The switched
sense resistor **designs the failure out** rather than mitigating it: at half
power the string is still 8 emitters, ≈11.8 V against a 5.3 V rail.

### R1a / R1b values

`I_LED = Vfb / R_total` with `Vfb = 200 mV` (verified, §7.6). Q1's `Rds(on)`
sits in series with `R1b`, so the full-power leg is `R1b + Rds(on)`.

| Option | `R1a` = `R1b` | LCSC | Stock | Half (Q1 off) | Full (Q1 on) |
|---|---|---|---|---|---|
| **100 / 50 mA** | **4.02 Ω** 1 % 0805 | `C367870` (Walsin WR08W4R02FTL) | 925 | **49.75 mA** (−0.50 %) | **98.92 mA** (−1.09 %) |
| **80 / 40 mA** | **4.99 Ω** 1 % 0805 | `C25273` (UNI-ROYAL 0805W8F499KT5E) | 36 227 | **40.08 mA** (+0.20 %) | **79.78 mA** (−0.28 %) |

`R1a` and `R1b` are the **same value** in both options — one line item, one
reel, one Extended loading fee. Worked example for the 100 mA option:

```
half:  I = 0.200 / 4.02                      = 49.75 mA
full:  leg      = 4.02 + 0.048 (Rds(on))     = 4.068 Ω
       parallel = 4.02 × 4.068 / 8.088       = 2.0219 Ω
       I        = 0.200 / 2.0219             = 98.92 mA
```

**No trim on `R1b` is required.** The brief set ~2 % as the threshold; the
worst case is **−1.43 %**, at the 100 mA option with `Rds(on)` hot (see Q1
below). The 80 mA option is better still at −0.56 %. Both are inside the ±3 %
that `Vfb` (±2 %) and the resistors (±1 %) already contribute, so `Rds(on)` is
not the dominant error term and trimming it out would be chasing noise.

Dissipation is trivial: at full power the 100 mA splits ~50/50, so each leg
runs `0.05² × 4.02` = **10 mW** in a 125 mW 0805.

**Volume fallback:** 4.02 Ω has only 925 units (462 boards at 2 each). 3.9 Ω
(`C17615`, UNI-ROYAL, **46 416 stock**) is the high-volume substitute, but it
errs the *wrong* way — 51.28 mA / 101.94 mA, i.e. **+1.9 % at full power**,
which with the ±3 % tolerance stack reaches 104.9 mA. Against a `Tj` budget
already at ~105 °C of a **115 °C** rating that is margin this board cannot
spare, so 4.02 Ω is primary despite the thinner stock. Use 3.9 Ω only with the
80 mA option or after the evaluation rig says there is headroom.

Junction-temperature context, now with a real number: the chosen emitters spec
**`Tj` max = 115 °C** (verified, JNJ p.4), which sits at the bottom of the
110–125 °C band the design doc assumed. The 20 °C that 80 mA buys is therefore
worth more than the doc implied, not less. `R1a`/`R1b` remain the thermal knob.

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

This is also what killed the 4-of-8 bypass — see "Half power" above, where the
numbers are worked. With the switched sense resistor the string is 8 emitters
in every mode and the rail is never approached.

### One consequence of halving the current instead of the string

Inductor ripple `ΔI = Vin·D / (f·L)` does **not** depend on load, so halving
the LED current does not halve it — it doubles the *relative* ripple:

| Mode | string | `D` | `ΔI` | `Iin` | ripple |
|---|---|---|---|---|---|
| full, 100 mA | 12.2 V | 0.566 | 114 mA | 271 mA | 42 % |
| **half, 50 mA** | 11.8 V (Vf ≈1.45 V at 50 mA) | 0.551 | 111 mA | **131 mA** | **84 %** |

Trough current is 131 − 55 = **76 mA**, so the converter stays in continuous
conduction — this is not a regulation problem. But it is worth recording for
design-doc risk 4, because **the relatively noisiest mode is also the boot
default**. The old bypass scheme had the opposite property (4 emitters meant
`D` = 0.145 and only ~21 % ripple at half power); that is the one thing given
up in this trade, and it is the right trade against losing regulation entirely.

It is also a second argument against ever dropping `L1` below 22 µH.

---

## Bill of materials

**21 designators.** Arithmetic, since the count has moved twice:

```
design doc's BOM, enumerated   13 singles + D2–D9 (8) + J1  = 22
                               (the doc says 21; it undercounts by one)
+ C4, the missing COMP cap                                  = 23
− Q2, R2, R5  (deleted by the switched-sense-resistor scheme) = 20
+ R1b         (R1 becomes R1a + R1b)                        = 21
```

| Ref | Value | LCSC | Package | JLC | Stock | Datasheet |
|---|---|---|---|---|---|---|
| **U1** | TPS61165DBVR boost LED driver | `C58756` | SOT-23-6 | **Extended** | 4 722 | [TI SLVS790E](https://www.ti.com/lit/ds/symlink/tps61165.pdf) |
| **L1** | 22 µH shielded, Isat 1.6 A, 130 mΩ | `C2849503` | SMD 5 × 5 × 4.0 mm | Extended | 1 455 | [LCSC](https://www.lcsc.com/product-detail/C2849503.html) |
| **D1** | Schottky 60 V 1 A, `Vf` 580 mV @1 A | `C77343` | SOD-123 | Extended | 45 265 | [LCSC](https://www.lcsc.com/product-detail/C77343.html) |
| **C1** | 10 µF 25 V X5R input | `C15850` | 0805 | **Basic** | 5.9 M | [LCSC](https://www.lcsc.com/product-detail/C15850.html) |
| **C2** | **4.7 µF 50 V** X5R output | `C98192` | 0805 | Extended | 442 501 | [LCSC](https://www.lcsc.com/product-detail/C98192.html) |
| **C3** | 100 nF 50 V X7R input bypass | `C307331` | 0402 | **Basic** | 13 M | [LCSC](https://www.lcsc.com/product-detail/C307331.html) |
| **C4** | **220 nF 16 V X7R — COMP compensation** | `C16772` | 0402 | **Basic** | 2.8 M | [LCSC](https://www.lcsc.com/product-detail/C16772.html) |
| **R1a** | **4.02 Ω 1 %** (50 mA) / 4.99 Ω (40 mA) | `C367870` / `C25273` | 0805 | Extended | 925 / 36 227 | [LCSC](https://www.lcsc.com/product-detail/C367870.html) |
| **R1b** | same value as `R1a` | `C367870` / `C25273` | 0805 | Extended | — | as R1a |
| **Q1** | AO3400A N-MOSFET 30 V, `Vgs(th)` ≤1.45 V, `Rds(on)` ≤48 mΩ @2.5 V | `C20917` | SOT-23 | **Basic** | 830 584 | [AO datasheet](https://www.lcsc.com/datasheet/lcsc_datasheet_1811081213_Alpha---Omega-Semicon-AO3400A_C20917.pdf) |
| **R3** | 10 kΩ 1 % Q1 gate series, from `HB` — *see note* | `C25744` | 0402 | **Basic** | 25.5 M | [LCSC](https://www.lcsc.com/product-detail/C25744.html) |
| **R4** | 100 kΩ 1 % Q1 gate pull-down | `C25741` | 0402 | **Basic** | 9.7 M | [LCSC](https://www.lcsc.com/product-detail/C25741.html) |
| **D2–D9** | 850 nm IR ×8 — **2835** | `C7500098` | SMD 3.5 × 2.8 mm, 90° | Extended | 626 | JNJ, link above |
| | 850 nm IR ×8 — **3535** | `C7529167` | SMD 3.5 × 3.5 mm, 60° | Extended | 536 | JNJ, link above |
| **J1** | 4-pin header, `+ − IR HB` | **PENDING** | **PENDING — measurement 11** | — | — | — |

**Deleted in this revision:** `Q2` (MMBT3904 inverter) — Q1's source is now at
ground with the correct sense, so nothing needs inverting. `R2` (gate pull-up
to 5.3 V) — the gate is driven directly. `R5` (4.7 Ω inrush limiter) — it
existed only to limit the `C2` dump into a shortened string, and there is no
shortened string any more.

**Extended-part loading fees: 6 distinct parts** (`U1`, `L1`, `D1`, `C2`,
`R1a`/`R1b`, `D2–D9`) — one more than before, because 4.7 µF 50 V 0805 has no
Basic equivalent. `R1a` and `R1b` share one fee by sharing a value.

### Q1 — gate drive and the resulting current error

Verified from the Alpha & Omega AO3400A datasheet (Rev. 2, p.2 Static
Parameters): `Rds(on)` ≤ **48 mΩ** at `Vgs` = 2.5 V (typ 24 mΩ), ≤32 mΩ at
4.5 V; `Vgs(th)` 0.65 / 1.05 / **1.45 V max**; `Vgs` absolute max ±12 V. The
Rds(on) figures are specified at 3–5 A; at our ~50 mA the channel is nowhere
near pinch-off, so 48 mΩ is a conservative bound.

**Current error contributed by `Rds(on)`, at the 100 mA option:**

| | `Rds(on)` | full-power current | error |
|---|---|---|---|
| 25 °C | 48 mΩ | 98.92 mA | −1.09 % |
| ~90 °C (`Rds(on)` ≈ ×1.6) | 77 mΩ | 98.57 mA | **−1.43 %** |

Worst case **−1.43 %**, inside the ~2 % threshold, so **`R1b` is not trimmed**.

**⚠ `R3`/`R4` now form a voltage divider on the gate — this is new.** With the
old MMBT3904, `R3` was a *base* resistor: the junction clamps at ~0.7 V and
`R4` sank only ~7 µA, so the divider was irrelevant. A MOSFET gate is a DC
open circuit, so `R3` and `R4` divide:

```
Vgate = HB × 100k / (100k + 10k) = 0.909 × HB
```

At `HB` = 3.3 V that is **3.00 V** — above the 2.5 V spec point, so the ≤48 mΩ
bound holds and there is 2.07× margin over the 1.45 V maximum threshold. It
works. But it throws away 9 % of the gate drive for no remaining benefit.

**Recommendation: `R3` = 1 kΩ (`C11702`, UNI-ROYAL, Basic, 10.4 M stock)**,
giving `0.990 × HB` = 3.27 V. Same package, same Basic status, no cost. The
10 kΩ in the table above is what the brief specified and is safe; this is a
free improvement, not a correction.

**⚠ `Q1`'s gate drive now depends on measurement 3, which is outstanding.**
Previously `Q2` + `R2` referenced the gate to the 5.3 V rail, so `HB` only had
to clear a BJT's `Vbe` and its actual level barely mattered. Now `HB` drives
the gate directly. At 3.3 V or 5 V this is fine. **At 1.8 V it would not be** —
`0.909 × 1.8` = 1.64 V against a 1.45 V maximum threshold is not a working
design. Measurement 3 has moved onto the critical path.

### Why deleting R5 is safe

Switching `R1b` in and out is a **setpoint change on a running converter**, not
a capacitor dump — the multi-amp spike `R5` existed to limit cannot occur
because no capacitor is ever connected across a shortened string.

When Q1 turns on, FB drops instantaneously from 200 mV to 100 mV and the loop
ramps the current up, damped by the 220 nF `C4` compensation capacitor and the
`tREF` = 180 µs VREF filter (§7.6). When Q1 turns off, FB rises to 400 mV —
well under the **3 V FB absolute maximum** (§7.1 p.4). Both directions are
soft.

Note the corollary: `C4`, the capacitor the design doc's BOM was missing
entirely, is now doing double duty as compensation *and* as the damping that
makes `R5` unnecessary.

### Failure modes are still benign in both directions

`Q1` shorted → board stuck at full power. `Q1` open → board stuck at half
power. Neither damages anything, which preserves the property the design doc
claimed for the old bypass. (The doc's wording describes the bypass; the
property survives the topology change.)

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
- **R1a/R1b** → `C17615` (3.9 Ω, 46 416 stock) for volume, at +1.9 % on full
  power. See the thermal caveat under "R1a / R1b values".
- **R3** → `C11702` (1 kΩ, Basic) — recommended, recovers 9 % of gate drive.

---

## Layout rules derived here — carry into Task 8

1. **No RC on the `CTRL` net.** The only route into the EasyScale dimming
   protocol is a 260 µs–1 ms low glitch on the rising edge. A series resistor
   alone is fine; a series resistor *with* capacitance, or a soft pull-up
   fighting the GPIO, could manufacture exactly that glitch. Drive `CTRL`
   straight from `IR_LED`. Full reasoning under "Enable semantics" above.
2. Keep the `C1`–`U1`–`D1`–`C2` loop as one tight cluster on the back
   (design-doc risk 4). `D1` was kept in SOD-123 rather than SMA for this.
3. `L1` is magnetically shielded specifically because it shares a board with
   the image sensor. Do not substitute an unshielded drum-core part.

---

## Corrections to the design doc's BOM

Sourcing turned up four wrong part values plus one missing part. Three of them
follow from one fact: **OVP trips at 37–39 V on the SW node**, so on an
open-string fault the output rail reaches ~38 V for 8 switching cycles before
the latch fires (§8.3.2, verified).

1. **`C2` must be 50 V, not 25 V.** It sits on the output node and sees the OVP
   excursion.
2. **`D1` must be ≥ 40 V, not 30 V.** Same reason; TI §9.1.3 says outright that
   "the reverse breakdown voltage of the diode must exceed the open LED
   protection voltage", and TI's own reference design uses a 40 V MBR0540.
   60 V specified here for margin.
3. **`C4` is missing entirely.** SOT-23-6 pin 5 is **COMP**, the
   transconductance error amplifier output, and TI §9.1.4 requires a
   compensation capacitor to ground — "a 220 nF ceramic capacitor is suitable
   for most applications".
4. **`L1` is 22 µH, not 33 µH** — TI's recommended range is 10–22 µH (§7.2).
5. **Vf is ~1.5 V at 100 mA, not 1.7–2.0 V**, which is what invalidated the
   4-of-8 bypass and forced the switched-sense-resistor scheme.

### C2 = 4.7 µF — and why 1 µF was worse than it looked

`C2` was pinned at 1 µF purely to cap the bypass inrush energy. With the bypass
gone that constraint is gone, so `C2` moves mid-range into TI's recommended
1–10 µF (§9.1.5, which warns that below-range "the boost regulator can
potentially become unstable").

This matters more than a tidy-up, because of DC bias. A 50 V 0805 MLCC loses a
large fraction of its rated capacitance at 12–19 V of applied DC:

- old `C2`, 1 µF X7R 50 V 0805 → **plausibly ~0.6–0.7 µF effective, i.e.
  *below* TI's 1 µF floor**. The stability concern flagged in the previous
  revision was probably understated.
- new `C2`, 4.7 µF X5R 50 V 0805 (`C98192`) → roughly 2–2.8 µF effective,
  comfortably inside the range.

**⚠ Those derating figures are an engineering estimate, not a verified
number.** No DC-bias curve for `C98192` was read — Samsung publishes one via
their characterisation tool and it was not retrieved in this session. Pull it
(or measure the fitted part) before relying on the exact value. The *direction*
is not in doubt; the magnitude is.

Output ripple improves as a side effect: the design doc's `I·D/(f·C)` gives
~23 mV at 2.4 µF effective, against the 69 mV it quoted for 1 µF at 1 MHz.
