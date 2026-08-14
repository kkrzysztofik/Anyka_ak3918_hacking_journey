# Measuring the IR / white illuminator on the bench

Written 2026-08-15, after an evening of remote measurement established that the dark night
image on this fleet is **not** a firmware fault. Everything software-side checks out; what
remains can only be settled with a meter on the ring PCB.

Aimed at the LED ring in these dome cameras: **4 IR + 4 white emitters interleaved** around
the lens, on two independent circuits. The single most valuable number is **current through
the LED string**, not voltage.

## What has already been ruled out (don't re-debug it)

Measured on `.198` and `.121`, 2026-08-14/15 — see
`vendor-day-night-implementation.md` §12 and the fleet numbers below:

- **The ISP profile really switches.** Night fingerprint is `ae_exp_time_max=2250` and
  `ae_target_lumiance=55` (day is 1500/35), the encoder drops to 10 fps, and the daemon logs
  `switching isp mode -> night`. All readable live from `/api/diagnostics`.
- **The IR-cut solenoid really moves.** A manual coil pulse produces audible clicking on
  `.198`.
- **The IR LEDs really emit.** A phone camera shows a strong purple wash. A half-dark ring in
  an IR photo is *normal* — those are the white LEDs, not dead emitters.
- **Exposure headroom is spent.** `exp_time_max` is ~94% of frame time in both modes
  (1500 lines x ~41.7 us against a 66.7 ms frame at 15 fps; 2250 against 100 ms at 10 fps).
- **No software lever raises IR output.** `IR_LED` is a binary GPIO already at 100%, there is
  no `/sys/class/pwm` on these boards, and PWM could only ever dim.

## Reference numbers — what each camera contributes

Mean frame luma out of 255, `ffmpeg signalstats YAVG`, via RTSP:

| camera | baseline | `IR_LED=1` | `WHITE_LED=1` |
|---|---|---|---|
| `.146` kitchen, indoor, pitch black | 0.73–0.90 | 2.86–3.04 | **110** |
| `.198` room with ambient | ~22 | ~27 (**+4.5**) | ~100 (**+78**) |
| `.121` | ~19.4 | ~20.2 (**+0.8**) | ~22.1 (**+1.9**) |

`.198` is the known-good reference. `.121` is the unit under suspicion: **both** its lamps do
essentially nothing, and because an IR-cut filter passes visible light, a stuck filter cannot
explain a *white* lamp adding only 1.9. The fault there is upstream of the optics.

## Before you open anything

Force the lamp on from software so you are probing a live circuit. The GPIO holds its value
until the next day/night transition:

```bash
# .198 (direct)
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 \
  'echo 1 > /sys/user-gpio/IR_LED; echo 0 > /sys/user-gpio/WHITE_LED'

# .121 (via SSH tunnel on 12421, or the jumphost root@192.168.3.137)
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12421 \
  'echo 1 > /sys/user-gpio/IR_LED; echo 0 > /sys/user-gpio/WHITE_LED'
```

To stop AUTO flipping the lamp mid-measurement, set `ir_cut_filter = "OFF"` in
`/mnt/anyka_hack/onvif/config.toml` and restart onvif-rust, or simply re-issue the write
before each reading. **Measure the IR and white circuits separately** — they are independent.

## Tools

Multimeter with diode-test and mA ranges. A phone (most sensors see near-IR). A TV remote as
a known-good IR source, to prove the phone actually sees IR before trusting a negative
result. Fine-tip probes. Optional but excellent: `.198` on the bench alongside, to measure
the same points on hardware known to work.

## Safety

Low voltage throughout (5 V or 12 V), so shock risk is minimal — but keep the mains adapter
unplugged while probing anything on its side. **Do not put your eye close to a powered IR
array**: 850 nm is invisible, the blink reflex never fires, and these emit real optical power
at close range. Anti-static basics before touching the PCB.

## Disassembly notes

The front bezel unscrews or unclips; the ring PCB sits around the lens barrel on a small
2- or 3-pin connector. Two things to protect:

- the **ircut solenoid's thin twin wires**, which tear easily;
- the **lens focus** — if the barrel is threaded, mark its position first or you will be
  refocusing afterwards.

Telling emitters apart: white LEDs have a visible yellow phosphor dome; IR emitters look
clear, dark grey, or faintly blue-black.

## Step 1 — unpowered checks

Power off, then:

| check | method | good | bad |
|---|---|---|---|
| Each IR LED | **diode test** across the package | ~0.9–1.3 V forward | `OL` = open (dead); ~0 V = shorted |
| Each white LED | diode test | ~2.4–3.2 V (many meters read `OL` — normal, they cannot reach Vf) | ~0 V = shorted |
| Series resistor | measure across it | matches its marking (often 2–20 Ω) | open, or well out of tolerance |
| String continuity | continuity mode along the string | beeps end to end | a break locates the open LED |

In-circuit readings can be skewed by parallel paths. If something looks odd, lift one leg
before believing it.

## Step 2 — powered checks (the ones that matter)

Camera powered and booted, `IR_LED=1`:

1. **Supply rail at the ring PCB** — expect a steady 5 V or 12 V. If it sags when the lamp
   switches on, the supply or driver is the limit, not the LEDs.
2. **Voltage across the current-limiting resistor**, then compute **I = V / R**. This is the
   diagnostic. A small 4-LED array should draw roughly **20–80 mA**. A few mA means the
   driver is not turning on; ~0 mA with the rail present means an open string.
3. **Voltage across the whole string** — roughly `n x Vf`, so ~4.8 V for four IR LEDs in
   series. Much higher *with no current* points to an open LED.
4. **Across the switching transistor/MOSFET** (Vce or Vds) — a few tenths of a volt when on.
   A large drop means it is not being driven fully; check its gate/base against the GPIO.
5. **Temperature** — after a minute at full current the LEDs and resistor should be
   noticeably warm. **Stone cold with the GPIO high means no current is flowing**, and that
   answers the question without a meter.

Repeat with `WHITE_LED=1`, `IR_LED=0`.

## Step 3 — interpreting the result

| finding | conclusion |
|---|---|
| Expected current, LEDs warm, phone sees the glow | Array healthy — the problem is optical (undersized array, or scene too far). Matches `.198`. |
| Rail present, ~0 current | Open string or dead driver — locate the open LED by diode test |
| Current present but well below expectation | Wrong resistor, degraded LEDs, or a driver not fully switching |
| No rail at the ring PCB | Fault is on the main board or the connector, not the lamp |
| GPIO reads 1 but the transistor gate never moves | The sysfs node is not wired to this driver on this board revision — a real possibility that could not be ruled out remotely |

## Do this first

Measure **`.198`'s ring at the same points before touching `.121`**. We have proven `.198`
emits properly, so a known-good reference turns every ambiguous reading on `.121` into a
comparison instead of a judgement call — which is exactly what was missing during the remote
session.

If `.121`'s current turns out healthy while its scene stays black, the answer is geometry
rather than electronics: the lamp works and simply has nothing near enough to illuminate.
That closes the question.

## If the array is healthy and the image is still dark

The remaining levers, in order of effort:

1. **`WHITE_LED` as an opt-in night illuminator** — worth ~16x the IR array on `.198`.
   `Node::WhiteLed`, the caps probe and `set_white_light` already exist; only `plan()` never
   writes it. Needs a `[night]` config bool plus two `Step::Write`s.
2. **Halve the night frame rate** (10 → 5 fps) for ~2x exposure, paid for in motion blur.
3. **External illuminator**, or a hardware change to the LED drive current. An undersized
   onboard array is not something firmware can fix.
