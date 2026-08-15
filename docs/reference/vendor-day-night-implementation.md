# How the vendor firmware does day/night, and what we copied wrong

Investigation of 2026-08-13/14, against the binaries actually running on the fleet.

**Short version:** there are two vendor implementations, not one. We ported the weaker
one. The stock firmware uses an ISP-side algorithm driven by a signal we do not expose
over IPC, with thresholds that are already sitting in every camera's `anyka_cfg.ini`.

## 1. The two implementations

| | `libre_anyka_app` (the SDK sample in `cross-compile/anyka_reference/`) | `anyka_ipc` (shipped with the camera) |
|---|---|---|
| light signal | `ak_drv_ir_get_input_level()` — digital `gpio-rf_feed`, else `ain0` | ISP luminance ratio + AWB statistics |
| thresholds | hardcoded in `ak_drv_ir.c` | `/etc/jffs2/anyka_cfg.ini` `[autoir]` |
| hysteresis | none | luma band + AWB gate + lock timer |
| ircut drive | one line, direct sysfs write | `ak_yi_set_ircut_{one,two}_line` with runtime probe |
| our port follows | **this one** | — |

`libre_anyka_app` imports none of the ISP soft-IR symbols. Reading it as "the vendor
reference" hid the fact that a second, better path existed and was the one actually used
on this hardware.

## 2. Binary provenance — `orig/` is not the fleet's firmware

| source | md5 | version string |
|---|---|---|
| `orig/usr/bin/anyka_ipc` | `50b654b4…` | `6.0.24.10_202401091113` (9 Jan 2024) |
| `.121` and `.146`, `/usr/bin/anyka_ipc` | `00993ae9…` | `6.0.24.10_202311281724` (28 Nov 2023) |

Same version number, different build, 124 differing strings (error-voice playback and
wifi info). The day/night machinery is identical between them, but that had to be
checked rather than assumed. All analysis below is against the camera's own binary.

To pull a file off a camera — FTP requires a password we do not have:

```bash
# on the camera, via scripts/debugging/cam_exec.py
nc -l -p 9931 < /usr/bin/anyka_ipc &
# on the jumphost
nc 192.168.30.146 9931 > /tmp/aipc_146
```

Disassembly needs the vendored ARM objdump; the host one has no ARM support:

```text
toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-objdump
```

## 3. Which path the stock app takes — the `hw.conf` gate

At `0x383a0`:

```asm
3839c: bl   ak_drv_ir_set_threshold       ; LDR thresholds set unconditionally
383a0: bl   ak_config_get_camera_info     ; -> r6
383a8: bl   ak_config_get_auto_day_night  ; -> r0/r5   ([autoir] block)
383ac: ldrb r3, [r7, #3]                  ; r7 = &yi_hwconfig (.bss 0x519ff0)
383b0: cmp  r3, #5
383b8: bne  38420                         ; != 5 -> hardware LDR path, ps_mode = 0
                                          ; == 5: marshal [autoir] onto the stack
383bc: ldr r3,[r0,#8]   -> [sp,#80]       ; day_to_night_lum
383c8: ldr r3,[r0,#12]  -> [sp,#84]       ; night_to_day_lum
383e0: loop            -> [sp,#88..104]   ; night_cnt[5]
383fc: loop            -> [sp,#108..144]  ; day_cnt[10]
383d4: ldr r3,[r0,#76] -> [sp,#148]       ; lock_time
38414: bl   ak_vpss_isp_set_auto_day_night_param
38418: mov  r0, #1                        ; AUTO_PHOTOSENSITIVE
38430: ldr  r1, [r6, #96]                 ; day_night_mode
38434: bl   ak_misc_start_photosensitive_switch_ex(1, day_night_mode)
```

The stack layout matches `struct ak_auto_day_night_threshold` field for field, including
`lock_time` at offset 68 (`sp+80+68 = sp+148`).

`yi_hwconfig` is filled by `yi_hwconfig_init` at `0x29ec8`, which `fgets` one line of
`/etc/jffs2/hw.conf` into `sp+16` and then does, for each field:

```asm
ldrb r3, [sp, #19+i] ; sub r3, #48 ; strb r3, [r4, #i]
```

so the decode rule is:

```text
yi_hwconfig[i] = hw.conf_line[3 + i] - '0'
```

i.e. index `i` of the digit string after the `HW=` prefix. This is the same convention as
the known wifi decode (`tail -c +4`, then index 51).

### hw.conf offsets decoded so far

| index | meaning | source |
|---|---|---|
| 3 | day/night sensing strategy; `5` selects the ISP soft-IR path | `anyka_ipc` `0x383ac` |
| 51 | wifi chip (`h` = ssv6355_ble, `g` = zt9101) | `wifi_driver.sh` |
| 52 | wifi gpio polarity (`2` = high_low) | `wifi_driver.sh` |

There is no schema for this file anywhere. Each offset costs a disassembly session, so
add to this table whenever one is decoded.

### What the fleet reports

| camera | `hw.conf` | index 3 |
|---|---|---|
| `.121` | `111513155011100180020000000000000000000000020000003h229000000000` | `5` |
| `.146` | *identical to `.121`* | `5` |
| `.198` | `111513155011100180020000000000000000000000020000003h200000000000` | `5` |
| `.127` | `111513175121100180010081602802812035021024021007011g229000000000` | `5` |

All four take the ISP soft-IR branch. **The vendor firmware never read `ain0` on any of
these boards** — consistent with `ain0` sitting at ~510–990, inside the vendor's own
unhandled 300–1100 dead zone.

## 4. The algorithm

`calc_cur_lumi()`, `anyka_reference/platform/libplat/src/vpss/ak_vpss_isp.c:205`:

```c
avg_lumi   = run_info.current_calc_avg_lumi;   // the byte our CMD_ISP_GET_AE_LUMA returns
avg_lumi   = (avg_lumi == 0 ? 40 : avg_lumi);
lum_factor = isp_get_cur_lum_factor() * 40 / avg_lumi;
```

The vendor does **not** use `current_calc_avg_lumi` as a light meter — it uses it as a
divisor. The numerator is the exposure/gain *effort* the AE is spending; dividing by the
luma it achieved gives effort-per-unit-brightness. **Higher means darker.** That is why
`ak_vpss_isp_get_auto_day_night_level` (`ak_vpss_isp.c:1011`) tests

- `cur_lum_factor > day_to_night_lum` → night
- `cur_lum_factor < night_to_day_lum` → day

with the gap between the two as the hysteresis band.

This is exactly the un-regulated gain/shutter signal that
`docs/`-adjacent analysis of the dusk lag concluded we needed. It is computed from
`isp_get_cur_lum_factor()`, which we do not expose.

Three more layers we do not have:

1. **An independent AWB gate.** `night_mode_cmp_awb()` / `day_mode_cmp_awb()`
   (`ak_vpss_isp.c:122` / `:77`) sample `ISP_AWBSTAT` twice, average the per-bin
   `total_cnt[]`, and require them to cross `night_cnt[5]` / `day_cnt[10]`. Luminance
   alone never flips the mode. This is what separates "lights off" from "dark grey
   daytime scene": chroma content collapses under IR.
2. **`wait_move_stable()`** before judging, and `CHECK_TIME = 3` voting samples per call.
3. **A night lock with an escape hatch** — `lock_time`, released early when
   `night_status_change()` is true, rather than our flat unconditional timer.

## 5. The configuration is already on every camera

`/etc/jffs2/anyka_cfg.ini`:

```ini
[camera]
day_ctrl              = 2        ; DAY_LEVEL_HL — the polarity our two booleans encode

[autoir]
auto_day_night_enable = 1
day_night_mode        = 2        ; SET_AUTO_MODE
day_to_night_lum      = 6400
night_to_day_lum      = 2048
lock_time             = 900000
quick_switch_mode     = 0
night_cnt0..4         = 1200
day_cnt0..9           = 600000
```

The range notation abbreviates the file: it carries `night_cnt0` through `night_cnt4` and
`day_cnt0` through `day_cnt9`, each set to the value shown.

Our `NightConfig::lock_time_ms` default of `900_000` came from this file. We inherited the
vendor's tuning for a *different signal*: 15 minutes is reasonable for a ratio that has an
AWB gate and an early-release condition, and far too blunt for a raw AE byte with neither.

## 6. Actuation — the part we did port correctly

`ak_misc_set_video_day_night` (`libre_anyka_app/main.c:709`), used by both vendor apps:

```c
if (day_val) {                                 // day
    camera_set_ir(!ir_val, IRCUT_A_FILE_NAME);
    ret = ak_vi_switch_mode(vi_handle, VI_MODE_DAY);
    camera_set_ir(!ir_val, IRLED_FILE_NAME);   // lamp off last
} else {                                        // night
    camera_set_ir(!ir_val, IRLED_FILE_NAME);   // lamp on first
    ret = ak_vi_switch_mode(vi_handle, VI_MODE_NIGHT);
    camera_set_ir(!ir_val, IRCUT_A_FILE_NAME);
}
ak_sleep_ms(300);
```

Our `plan()` and `SETTLE = 300ms` match this. Two deliberate improvements: we drive both
ircut lines with the 10 ms coil pulse and return-to-idle from `ak_drv_ir.c:239-252` (the
sample writes one line and leaves the coil energised), and we write the sysfs nodes
directly instead of `system("echo %d > %s")`.

Note the vendor's `ak_drv_ir` paths are all `gpio-` prefixed
(`/sys/user-gpio/gpio-ircut_a`), which do not exist on this board — our node names
(`ircut_a`, `ircut_b`, `IR_LED`) are correct and the vendor driver is inert here. This is
also why `ptz_daemon_dyn`, which links the entire `ak_drv_ir` API, can never touch the
filter: `ak_drv_ir_init()` stats the prefixed names, fails, and every
`ak_drv_ir_set_ircut()` returns at its uninitialised guard.

## 7. ISP profile content

The night switch is real and loud in `/mnt/logs/vendor_daemon.log`. Both profiles come
from `/data/sensor/isp_gc1084.conf`:

| profile | frame_rate | max_exp_time | low_light_gain |
|---|---|---|---|
| day | 15 | 1500 | **24** |
| night | 10 | 2250 | **10** |

Night buys 1.5x exposure and half the frame rate but gives up 2.4x of gain. On `.146` the
AE rails at luma 2–3 against a ~40 setpoint all night, so the ISP is asking for more gain
than the night block permits. Saturation is handled by the profile — settled night frames
measure `SATAVG = 0`, true monochrome, with no action needed from us.

### What else the profile changes

`isp_gc1084.conf` is **three mode blocks of exactly 34,746 bytes** (3 × 34746 = 104238),
same layout, different content. `isp_cfg_file_load(mode, …)` picks one by index, so
`VI_MODE_DAY` = block 0 and `VI_MODE_NIGHT` = block 1. All three carry the same
`3.05_032401` version at offset 4; each also carries the tuning engineer's own note:

| block | note strings |
|---|---|
| 0 — day | `0324`, `v0.4_release_GC1084_DVP_base27M@pclk=36Mhz_1280x720_30fps`, `V2 YY change day/night` |
| 1 — night | `5.22 blc sharp wdr yuveffect`, `8.4 sharp wdr denoise 3dnr`, `8.24 contrast` |
| 2 — third mode | `V2 change day ccm sharp /night lum noise` |

So the fps / exposure / gain triple the SDK prints on every switch is the small visible
part: **26.5 % of the block differs between day and night.**

The block is self-describing. From `0x0200` onward it is a flat sequence of
`[u16 module_id][u16 module_len][data]` records, walked by `isp_switch_mode`
(`isp_basic.c:1918`) from `ISP_BB` to `ISP_HUE`; the IDs are the `isp_basic.h:11` enum.
Parsing the day and night blocks with that schema and diffing per module:

| id | module | offset | len | bytes differing |
|---|---|---|---|---|
| 11 | SHARP | `0x03fba` | 11096 | **5102** |
| 10 | WDR | `0x018cc` | 9966 | **2625** |
| 7 | GAMMA (rgb gamma) | `0x0146e` | 876 | **498** |
| 3 | NR | `0x006e6` | 2268 | 269 |
| 4 | 3DNR | `0x00fc2` | 1088 | 196 |
| 19 | WB (white balance) | `0x07dc2` | 528 | 173 |
| 0 | BB (black balance) | `0x00200` | 186 | 65 |
| 8 | CCM (colour correction) | `0x017da` | 136 | 63 |
| 12 | SATURATION | `0x06b12` | 206 | 30 |
| 20 | EXP (exposure) | `0x07fd2` | 240 | 26 |
| 13 | CONTRAST | `0x06be0` | 64 | 16 |
| 16 | DPC | `0x06c34` | 4188 | 12 |
| 5 | GB (green balance) | `0x01402` | 86 | 10 |
| 21 | MISC | `0x080c2` | 30 | 7 |
| 15 | YUVEFFECT | `0x06c26` | 14 | 4 |
| 2 | RAW_LUT (raw gamma) | `0x0037a` | 876 | 1 |
| 17 | WEIGHT (zone weight) | `0x07c90` | 260 | 1 |
| — | LSC, DEMOSAIC, FCS, RGB2YUV, AF, Y_GAMMA, HUE, **SENSOR** | | | **0** |

Two things fall out of this table:

- **The `SENSOR` module is byte-identical.** The sensor register table does not change; the
  10 fps comes out of `ISP_EXP` via `init_fps_info()` / `set_exptimemax_by_curfps()` at the
  tail of `isp_switch_mode`.
- **`ISP_MISC` is deliberately skipped on a switch** — `isp_switch_mode` has an explicit
  `if (ISP_MISC != i)` guard, "only set misc once when init". So its 7 differing bytes are
  loaded from the file and never applied. Anything tuned there is day-only, permanently.

The three logged numbers live at block offsets `0x7fe2`, `0x7fee`, `0x7ff6` — inside
`ISP_EXP`, as expected.

The practical consequence: **switching day/night swaps the entire image pipeline tuning,
not an exposure setting.** Sharpening, WDR, the RGB gamma curve, both denoise stages, white
balance and the colour matrix all change together. Trying to "fix" the dark night image by
adjusting gain alone is fighting one of several dozen coupled parameters.

### User imaging settings survive a switch — the SDK re-applies them

Worth knowing before anyone adds re-apply logic on our side: the tail of `isp_switch_mode`
saves the user effect offsets, zeroes them, loads the module data, and then puts them back:

```c
isp_set_effect(VPSS_EFFECT_HUE,        effect.hue);
isp_set_effect(VPSS_EFFECT_BRIGHTNESS, effect.brightness);
isp_set_effect(VPSS_EFFECT_SATURATION, effect.saturation);
isp_set_effect(VPSS_EFFECT_CONTRAST,   effect.contrast);
isp_set_effect(VPSS_EFFECT_SHARP,      effect.sharp);
```

So ONVIF brightness / contrast / saturation / sharpness set through
`ak_vpss_effect_set` are preserved across every day/night transition, and
`AnykaImagingControl`'s cached `settings` stays truthful. The monochrome night image comes
from the profile's own `SATURATION` / `YUVEFFECT` modules, not from a user setting being
lost — which is why settled night frames measure `SATAVG = 0` while the configured
saturation is still 50.

Also worth knowing: `ak_vi_open` → `AK_ISP_sdk_init` loads the **day** block with no
`isp_switch` log line at all. Today only onvif-rust startup opens VI and the AUTO
reconcile re-drives night immediately after, so it is harmless — but the ISP's mode is not
ours to assume, and there is no read-back. `isp_get_day_night_mode()` exists in the SDK
and is not exposed; IPC commands stop at `CMD_ISP_GET_AE_LUMA = 106`.

## 8. Measured fleet state, 2026-08-13 ~22:20 UTC

Mean frame luma and saturation from one FLV frame, `ffmpeg signalstats`, 0–255:

| camera | IR_LED | AE luma | YAVG | SATAVG | mode reported |
|---|---|---|---|---|---|
| `.146` | 1 | 2–3 | 3.04 | 0 | Night — switched 18:48 UTC from luma 38→13→4 |
| `.198` | 1 | — | 3.20 | 0 | Night |
| `.127` | 0 | **50, constant** | 127.4 | 12.4 | Day |
| `.121` | — | ~40, pinned | — | — | see the dusk-lag analysis |

`.127` is not a failure: its room lights are on, the frame is correctly exposed and in
colour, and AE luma of 50 is above `ae_day_threshold = 28`, so Day is the right answer.

`.146` and `.198` are behaving identically — both switched to night, both monochrome, both
at the noise floor because the scene is genuinely dark and the IR illuminator is weak.
On `.146`, settled A/B/A/B measurement gives IR off = 0.73–0.90, IR on = 2.86–3.04, a
reproducible ~3.5x. `WHITE_LED` on the same scene gives **110** — the only illuminator that
actually lights a room. Neither vendor app uses `WHITE_LED` as a lamp; `anyka_ipc` drives
it only as a status indicator via `led_status.sh` (off / blink 200 / blink 2000).

> **Measurement caveat:** AE re-ramps for tens of seconds after any mode or lamp change.
> Frames grabbed ~5 s after a change read low and can invert the comparison. Allow 30 s.

## 9. Live ISP AE state, measured on `.121` 2026-08-14 01:5x

First capture through the new `CMD_ISP_GET_AE_ATTR`, both modes, 30 s settle:

| field | night | day | night conf | day conf |
|---|---|---|---|---|
| `exp_time_max` | **2250** | **1500** | 2250 | 1500 |
| `a_gain_max` | 16384 | 16384 | — | — |
| `target_lumiance` | 55 | 35 | — | — |
| `ae_luma` (achieved) | 3 | 2 | | |

**The day/night switch is complete and correct.** `exp_time_max` tracks the sensor conf
exactly in both directions. This is outcome 1 of the gate: the profile really is applied.

Three findings that change the picture:

1. **`a_gain_max` does not change between profiles**, and at 16384 it is not a restrictive
   ceiling. The conf's `low_light_gain` (24 day / 10 night) is therefore **a different
   parameter** — an AE algorithm envelope, not the hard gain limit. The working hypothesis
   that the night profile starves the image of gain is **disproved.**
2. **`target_lumiance` rises 35 → 55 at night.** The night profile asks the AE for a
   *brighter* image than the day profile does, the opposite of a deliberate darkening.
3. **AE is railed in both modes** — achieved luma 3 against a target of 55 at night, 2
   against 35 by day. With that much gain headroom unused, the constraint is not the ISP
   configuration at all: not enough light is reaching the sensor.

Consequence: overriding `a_gain_max` (the planned Task 5) is **not justified** — it is not
the binding constraint. The next question is why AE cannot converge with 16384 of gain
available, which points at exposure time or the sensor's own limits rather than the profile.

Note the vendor's `calc_cur_lumi` divides by a hardcoded `40`, while this camera's actual
`target_lumiance` is 55 at night and 35 by day. The constant is not the AE target.

## 9. What to change

1. **Expose the vendor's signal.** `handle_isp_get_ae_luma` already calls
   `ak_vpss_isp_get_ae_run_info()` and fills a struct containing `current_darked_flag`,
   `current_a_gain`, `current_exp_time`, then returns **one byte**. Widening that response
   is nearly free and gives both the diagnostic and the ISP-profile read-back.
2. ~~**Better: wrap `ak_vpss_isp_get_auto_day_night_level(pre_ir_level)`**~~ **Not
   actionable — see §11.** The function is declared in the headers we ship but is not
   exported by any library we have, and the camera rootfs has no `libplat_vpss.so` at all.
   The vendor's soft-IR algorithm is linked statically inside `anyka_ipc` and cannot be
   called from our processes. Reimplementing it on `isp_get_statinfo` is the only route.
3. **Consider `WHITE_LED` as an opt-in night illuminator.** `Node::WhiteLed`, the caps
   probe and `set_white_light` already exist; only `plan()` never writes it. Opt-in, since
   on an outdoor camera a floodlight is not a sane default.

## 10. The missing AWB gate is a live defect, not a gap — `.127`, 2026-08-14

Observed on `.127` as a black picture that repaired itself minutes later. It is not an
intermittent failure to switch: it is a **30-minute oscillation**, half of it spent in Day
mode at night, i.e. filter in, lamp off, black frames.

Four consecutive cycles out of `/mnt/logs/onvif-debug.log.2026-08-14`:

```text
12:08:49  sample raw=2  ae → Night     applied Day→Night
12:08:59  sample raw=60 ae → Day       (10 s later, same scene)
12:23:59                               applied Night→Day
12:24:09  sample raw=1  ae → Night
12:39:09                               applied Day→Night
12:39:19  sample raw=52 ae → Day
12:54:19                               applied Night→Day
13:09:29                               applied Day→Night
13:19:39  sample raw=49 ae → Day
```

Every apply is exactly `lock_time_ms` (900 s) after the decision that caused it. The lock is
not hysteresis here — it is the oscillation period. Confirmed live at 13:20 UTC:
`IR_LED=1`, `ircut_a=0` (in Night), AE reading 49, i.e. already committed to the next flip.

**Mechanism — the camera meters its own illuminator.** `current_calc_avg_lumi` is a
regulated servo output. In Night the filter is out and `IR_LED` is on, so the AE reaches its
setpoint and reads ~50, which is `>= ae_day_threshold` (28) → "day" → switch to Day → filter
in, lamp off → reads 1–2, `<= ae_night_threshold` (8) → "night" → switch back. The actuator
feeds the sensor.

Three consequences worth keeping:

1. **Threshold tuning cannot fix this class of bug.** AE sits near its setpoint in daylight
   *and* in IR-lit night. No pair of thresholds separates them, because the signal is
   regulated. The same argument kills `lum_factor`: the lamp lowers exposure effort and
   raises achieved luma, so the ratio is contaminated from both ends.
2. **Multi-sample voting (Task 8) does not touch it.** All N samples agree on the same lie.
3. **The AWB gate (Task 7) is the defence**, and now has a hardware defect to justify it
   rather than a symmetry argument: chroma collapses under IR, so the colour bins withhold
   consent no matter how bright the frame looks. That is precisely why the stock firmware
   never flips on luminance alone.

`.121` hides this bug: its IR illuminator emits nothing, so night AE rails at 2–3, no day
classification is ever produced, and the loop never closes. Any board whose lamp works is
exposed.

**Open risk, unmeasured:** if the night ISP profile leaves AWB idle, the bins never rise and
the gate can never permit night→day — stuck in night instead of flapping. The bins must be
logged across one night before the gate is enabled.

## 11. What is actually callable — symbol availability, 2026-08-14

Checked with the vendored ARM `nm` against the libraries we ship and the camera's own
rootfs. The headers in `anyka_reference` describe a superset of what exists here.

| symbol | where | usable |
|---|---|---|
| `isp_get_statinfo` | `libplat_vi.so` (shipped) | **yes** — the planned AWB route |
| `isp_get_cur_lum_factor` | `libplat_vi.so` (shipped) | yes, already used by `CMD_ISP_GET_LUM_FACTOR` |
| `Ak_ISP_get_awb_stat_info` | `libakispsdk.so` (shipped, byte-identical to the camera's `/usr/lib` copy) | yes — alternative route, one library further down |
| `ak_vpss_isp_get_ae_attr` / `_run_info` | `libplat_vpss.so` (shipped) | yes, already used |
| `ak_vpss_isp_get_awb_stat_info` | declared in `ak_vpss.h`, **not exported** by our `libplat_vpss.so` (29 exported functions, none of them soft-IR) | no |
| `ak_vpss_isp_get_auto_day_night_level` | same — declared, not exported | no |
| `ak_vpss_isp_set_auto_day_night_param` | same — declared, not exported | no |

`/usr/lib/libplat_vpss.so` **does not exist on `.127`**: the stock `anyka_ipc` links libplat
statically, so the vendor's soft-IR algorithm is inside that binary and is not reachable as
a library from any process of ours. Adopting the vendor's loop wholesale (§9 item 2) is
therefore off the table; reimplementing it on `isp_get_statinfo` is the route that exists.

## 12. The AWB gate is viable — measured on `.198`, 2026-08-14

First hardware readings of the colour bins, from `CMD_ISP_GET_AWB_STAT` (Task 6) logged on
the `night sample` line (Task 6b). This is the gate Task 7 was blocked on.

```text
19:26  raw=1071    Day    awb=[192414, 1152, 151, 0, 0, 1650, 269, 0, 0, 0]   lit room, lamp off
19:27  raw=327680  Night  awb=[10620, 10695, 11449, 3645, 4302, 10666, 2056, 2562, 0, 0]
19:31  night mode applied Day->Night isp=0                                    IR lamp on
19:37  raw=218453  Night  awb=[7, 2, 1, 3, 1, 115, 115, 115, 111, 115]
19:47  raw=218453  Night  awb=[5, 0, 0, 0, 0, 93, 93, 93, 93, 93]
19:57  raw=163840  Night  awb=[2, 0, 0, 0, 0, 113, 113, 113, 113, 113]
20:07  raw=218453  Night  awb=[12, 22, 16, 21, 4, 159, 159, 159, 117, 159]
```

**AWB is not idle under the night profile, and chroma collapses under IR by three to four
orders of magnitude.** The vendor's own `[autoir]` thresholds work unmodified:

| test | threshold | measured | outcome |
|---|---|---|---|
| night->day, IR-lit (`night_cnt`) | any bin > 1200 | max **159** | refuses to leave night — 7.5x margin |
| night->day, real light | any bin > 1200 | **192414** | permits day — 160x margin |
| day->night (`day_cnt`) | any bin > 600000 | 192414 → "not day" | permits the night switch |

Three findings to carry into Task 7:

1. **Near-zero bins mean *IR-lit*, not merely *dark*.** The 19:27 sample is dark with the
   lamp not yet on (the transition applied at 19:31) and reads 3,000–11,000 — above the
   1200 threshold. Darkness alone produces noise-driven chroma; it was the lamp that pushed
   the bins to ~100. So a dark **unilluminated** scene classifies as "day" to this gate.
   That combination only arises in day-mode-at-night, where the night→day question is never
   asked, so it does not break the design — but any future use of these bins outside that
   context must account for it.

   **Corroborated on `.146` the same evening**, which sits in exactly that state all night:
   its IR does not effectively illuminate (the AE rails at luma 2–3 against a ~40 setpoint),
   so a settled night frame reads
   `awb=[5754, 7804, 12105, 4589, 6285, 7317, 1439, 3214, 0, 0]` — the same 3,000–12,000
   noise-chroma band, an order of magnitude above `night_cnt`. The gate is still safe there
   because luminance is primary and AWB only vetoes: at `lum=327160` the night→day test never
   reaches the colour check. The veto only bites on a camera whose lamp is strong enough to
   fool the luminance meter, i.e. `.127`.
2. **Bins 5–9 return suspiciously uniform values** under IR (`115,115,115,111,115`, then
   `93x5`, `113x5`). Identical counts across five colour-temperature buckets look like a
   fixed pattern rather than a measurement. Gate on the max across bins, as the vendor does;
   do not attribute meaning to an individual bin.
3. **`.198` cannot reproduce the `.127` flap**, so this measurement does not exercise the
   failure the gate exists to prevent. Its IR illuminator is the weak one (110 vs 3 YAVG
   against `WHITE_LED` indoors), so the lum factor stays at ~218453 — correctly "dark" —
   with the lamp on, and the camera never oscillates. The strong-lamp case is `.127`, which
   went off-network the same evening. The chroma-collapse result stands on its own; what is
   still unmeasured is the bin behaviour under an illuminator strong enough to fool the
   luminance meter.
