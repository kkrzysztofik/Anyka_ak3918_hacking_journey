# IR / Night-Mode Calibration

How to measure `/sys/kernel/ain/ain0` thresholds and verify IR-cut / IR-lamp /
white-light behaviour on an Anyka AK3918 board. Vendor defaults
(`day_threshold = 1100`, `night_threshold = 300`) leave this camera in a dead
band and must not be copied.

Config lives under `[imaging.night]` in
`SD_card_contents/anyka_hack/onvif/config.toml` (device path
`/mnt/anyka_hack/onvif/config.toml`).

## Why calibrate

AUTO prefers ISP AE `current_calc_avg_lumi` (`ae_*_threshold`). After three
consecutive AE read failures it falls back to `ain0` (`day_threshold` /
`night_threshold`).

`ain0` is a board-specific ADC reading from the light sensor (LDR). It is **not**
behind the camera optics — covering only the lens barely moves the value. Put
the whole front of the camera in a dark box (or cup) for the night sample. AE
luma *does* follow the lens, so a dark box over the optics is enough for AE.

With high = day (AE always; `ain0` when `ldr_high_is_day = true`):

| Condition | Meaning |
|---|---|
| value `>= *_day_threshold` | Day |
| value `<= *_night_threshold` | Night |
| otherwise | Indeterminate — hold last mode |

Leave a gap between the two thresholds for hysteresis. `lock_time_ms` blocks
another transition until that many milliseconds have elapsed (templates ship
`900000` = 15 minutes; use a few seconds only while testing).

## Prerequisites

- Camera reachable: recovery telnet on port **24**, or
  `uv run --no-project python3 scripts/debugging/cam_exec.py '…'`
- Deployed `onvif-rust` that includes night-mode support
- Nodes present (two-line IR cut on the verified board):

```sh
ls /sys/user-gpio/
# expect: ircut_a  ircut_b  IR_LED  WHITE_LED  …

ls /sys/kernel/ain/
# expect: ain0  ain1  bat
```

If `ircut_b` is missing, the probe selects one-line mode — note that before
trusting two-line pulse behaviour.

## Measure thresholds

### AE luma (preferred)

The AUTO loop already logs every light sample: `night sample raw=<n> src="ae"
mode=<Day|Night>` lines in the daily tracing log (`/tmp/onvif.log.YYYY-MM-DD` on
the `.198` layout, `/mnt/logs/onvif-debug.log.YYYY-MM-DD` on `.121`). Cover /
uncover and read those lines — no source patch needed.

Set:

- `ae_day_threshold` slightly **below** the bright reading
- `ae_night_threshold` slightly **above** the dark reading

Example from the lab board (`192.168.2.198`, 2026-08-04):

| Sample | AE luma |
|---|---|
| Dark box (lens covered) | ≈0..1 |
| Room uncovered | ≈34 |
| Shipped thresholds | `ae_day_threshold = 28`, `ae_night_threshold = 8` |

### ain0 (fallback)

```sh
# Room light (uncovered):
cat /sys/kernel/ain/ain0

# Dark box over the whole front; wait ~5 s, then:
cat /sys/kernel/ain/ain0
```

Set `day_threshold` / `night_threshold` the same way (below bright / above dark).

Example (`192.168.2.198`, 2026-08-02):

| Sample | `ain0` |
|---|---|
| Dark box | ≈648 |
| Room uncovered (evening) | ≈670 |
| Shipped thresholds | `day_threshold = 662`, `night_threshold = 652` |

Edit on the device (busybox `sed -i` is fine), or update the tracked template and
redeploy:

```toml
[imaging.night]
ldr_high_is_day = true
ircut_high_is_night = true
day_threshold = 662
night_threshold = 652
lock_time_ms = 900000
ae_day_threshold = 28
ae_night_threshold = 8
```

Restart `onvif-rust` after changing config (`killall onvif-rust.bin`;
`anyka-init` respawns it).

## Verify forced day / night

Use ONVIF `SetImagingSettings` with video source token **`VideoSource_1`**
(imaging store token; media profiles may advertise `VideoSource_0`).

| Mode | Expected |
|---|---|
| `IrCutFilter = OFF` | Night: `IR_LED` → `1`; after the pulse, `ircut_a` and `ircut_b` both `0` |
| `IrCutFilter = ON` | Day: `IR_LED` → `0`; coils idle at `0` / `0` |

```sh
cat /sys/user-gpio/ircut_a /sys/user-gpio/ircut_b /sys/user-gpio/IR_LED
```

**Anything other than `0` on both ircut lines after a transition means the coil
guard is broken — stop and fix before enabling AUTO.**

Optional: audible filter click; faint purple IR glow through a phone camera in
night mode.

## Verify AUTO

1. Set `ir_cut_filter = "AUTO"` (config or ONVIF), with measured thresholds.
2. Dark-box the camera → within a few poll intervals (`~2 s`), expect exactly one
   transition to night (`IR_LED = 1`).
3. Uncover → no transition until `lock_time_ms` elapses, then day (`IR_LED = 0`)
   once AE luma `>= ae_day_threshold` (or `ain0 >= day_threshold` on fallback).

If uncover stays in the indeterminate band, ambient light is too close to the
dark reading. Do **not** widen the threshold gap — that only makes the
indeterminate band wider and the reading stays stuck. Instead lower the
relevant `*_day_threshold` to just below the measured uncovered value so the
reading classifies as Day, keeping `*_night_threshold` above the dark reading
(or retune both thresholds together).

- `ain0` fallback: lower `day_threshold` below the uncovered `ain0` reading.
- AE luma: lower `ae_day_threshold` below the uncovered luma reading.

The lab board’s `ain0` span is only ~20 counts; AE luma has a wider dark/room
gap (~1 vs ~34) on the same board.

## Auxiliary lamps

From an ONVIF client (PTZ `SendAuxiliaryCommand`):

| Command | Effect |
|---|---|
| `tt:IRLamp\|On` / `Off` | IR illuminator |
| `tt:IRLamp\|Auto` | Re-enable AUTO polling |
| `tt:WhiteLight\|On` / `Off` | White floodlight (no Auto) |
| `tt:Wiper\|On` | Must fault — not silent success |

WebUI: Imaging → Illumination card.

## Known caveats (lab board)

- Imaging SOAP uses token `VideoSource_1` even when media reports
  `VideoSource_0`.
- Do not overwrite the whole device `config.toml` from the repo copy — it
  diverges (logging, streams). Patch `[imaging.night]` in place.

## See Also

- [[ONVIF-Rust-Implementation]] — server build and services
- [[Boot-Runtime-Supervisor]] — `anyka-init`, telnet `:24`
- [[Troubleshooting]] — common ONVIF / imaging issues
- Design notes: `docs/plans/2026-08-02-ir-led-support-design.md`
