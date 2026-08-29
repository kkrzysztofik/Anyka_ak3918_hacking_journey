# Event Audio Playback — Revision Design

Date: 2026-08-28
Status: approved (design)
Branch: `feat/event-audio-playback`
Supersedes decisions in: `2026-08-26-event-audio-playback-design.md`

## Why a revision

The branch shipped an in-daemon `libplat_ao` worker, then abandoned it for
`system("/usr/bin/ak_adec_demo ...")` after TTS clips played silently while
short tones worked. This document reverts that workaround and fixes the defects
found in the worker.

**The silence had a different cause than this document originally concluded.**
It was `SPK_PA` polarity — an active-high amplifier *shutdown* pin that we were
driving high before every clip. See the 2026-08-29 addendum. The send-loop,
stereo and drain defects recorded below are real and independently confirmed
against the vendored SDK, and the worker could not have played anything with
them, but they are not what the original symptom was about. Sections written
before that addendum are left as they stood, so the reasoning stays auditable.

## Root cause

`ak_ao_demo.c:174-188` — the raw-PCM AO reference, which the original design
never consulted (it cited only the two ADEC-based examples) — shows the DA
accepts **stereo only**:

```c
case AUDIO_CHANNEL_MONO:
    copy_for_dual_channel(data, read_len, full_buf);  /* duplicate each sample L+R */
    send_len = (read_len << 1);                       /* and double the length */
case AUDIO_CHANNEL_STEREO:
    memcpy(full_buf, data, read_len);
    send_len = read_len;
ak_ao_send_frame(ao_handle, full_buf, send_len, 0);
```

Our worker fed mono straight in. The DA read that stream as interleaved
stereo, so each channel received every *other* sample: a 2:1 decimation at
double pitch, with L and R carrying two different half-rate versions.

### The evidence that pinned it

`vendor_daemon.log` on `.198`, from the `ak_ao` builds:

| clip | logged `bytes=` (the `sent` accumulator) | real file size | ratio |
| --- | --- | --- | --- |
| `boot.raw` | 59600 | 29800 | **2.00×** |
| `alert.raw` | 95980 | 47990 | **2.00×** |
| `ok.raw` | 106436 | 53218 | **2.00×** |

`ak_ao_send_frame` returned exactly twice the bytes it was handed, on every
clip — AO accounting DA-side stereo bytes for mono input. The current
shell-out build logs `st.st_size` instead, which is why the 2× disappears
from later log lines; that is a changed log statement, not a fix.

### Why tones survived and speech did not

A tone at double pitch is still a tone: 660 Hz becomes 1320 Hz and nobody
notices a chime is off-key. Speech decimated 2:1 is aliased mush. Worse, on a
single speaker summing L+R, adjacent samples that are nearly equal (a low
tone) reinforce, while adjacent samples that differ (speech HF) partially
cancel. Hence "loud beep, nothing at all for speech".

Two hypotheses were tested and **refuted** on the way, recorded so they are
not re-run:

- **Level.** Measured RMS: tones 0.144–0.149 FS, TTS 0.108 FS. A 2.5 dB
  deficit cannot turn audible into inaudible. (An a-priori crest-factor
  estimate of 12–15 dB was wrong; espeak peaks at 0.962 FS.)
- **Missing clip file.** All four clips are present on `.198` in the flat
  path and both slots.

## Decisions

| Topic | Choice | Change from 2026-08-26 |
| --- | --- | --- |
| Where | In-daemon `libplat_ao` | restored (workaround dropped) |
| Reference | `platform/libplat/demo/ao_demo/ak_ao_demo.c` | **new** — raw-PCM path, not ADEC |
| Channel | Duplicate mono → stereo before send | **new** — the bug |
| Drain | Poll `ak_ao_get_play_status()` to `FINISHED` | **new** |
| Volume | `ak_ao_set_dac_volume`, range 0–6 | unchanged |
| Capture during play | Do not stop it | was permanent `push_stop_audio()` |
| Format / queueing / handle | unchanged (s16le mono 8 kHz, drop-if-busy, per-play open) | — |

## Scope

### 1. Restore the `ak_ao` worker, aligned to `ak_ao_demo.c`

`git checkout HEAD -- sound_worker.c` recovers the watchdog, configurable
volume and chunked send loop. Then add what the reference does and we did not:

- `copy_for_dual_channel()` + `send_len = read_len << 1` — the root-cause fix
- `ak_ao_enable_speaker(h, ENABLE)` before first send, `DISABLE` on teardown
- `ak_ao_set_resample(h, DISABLE)` before first send
- `ak_ao_clear_frame_buffer(h)` after open
- drain to `AO_PLAY_STATUS_FINISHED` before `ak_ao_close()`

The header marks the middle three mandatory. The reference's
`wait_play_finished()` accumulates `total_time` but **never checks it** — an
unbounded poll. Ours must be bounded by the existing watchdog.

**CORRECTED 2026-08-29 — this said "retain `spk_pa_set()`; the `SPK_PA`
amplifier GPIO is a real discovery". That was wrong, and it was the actual
cause of the silence.** `SPK_PA` is **active-high shutdown**, not an enable.
Measured at the 8002D: `1` → outputs 0 V (shutdown), `0` → outputs VDD/2
(enabled). The pin is low at boot and `ak_ao_enable_speaker()` never touches
it, so the stock state is already enabled; the workaround's `spk_pa_set(1)`
switched the amplifier **off** before every clip. Use `spk_amp_enable()`, which
only ever drives it low. See the 2026-08-29 addendum.

Buffer note: the send buffer must be `SOUND_CHUNK_BYTES * 2` to hold the
duplicated frame, and the read chunk stays `SOUND_CHUNK_BYTES`.

### 2. Drop the shell-out and its dependencies

Deletes `system()`, the documented command-injection path, the hardcoded
`/mnt/anyka_hack/slots/b/...` `LD_LIBRARY_PATH` (silently breaks on slot `a`),
and the dependency on `/usr/bin/ak_adec_demo` existing in every camera's
rootfs — the `.127` zt9101 risk the original design named.

Also removes `include/ak_adec.h`, `lib/libmpi_adec.so`, `lib/libakaudiocodec.so`
and the 0-byte `lib/libakaudiocodec.so.stock`; restores `-lplat_ao`. The ADEC
route is a dead end here, recorded in the log as `[sound] ak_adec_open PCM failed`.

### 3. Stop killing live audio capture

`sound_worker.c:45` calls `push_stop_audio()`, which tears down the AI → AENC
chain. `start_audio_push` is called exactly once, from
`Application::start_streaming`, and `hal/anyka/ipc/mod.rs:1637-1651` documents
that nothing restarts it. **One chime silences RTSP and HTTP-FLV audio until
onvif-rust restarts.**

The shell-out calls it too, so quiescing capture may be a genuine constraint —
but the original contention evidence was gathered against the corrupted send
loop and is suspect. Remove the call and verify audio capture survives
playback. If real contention appears, the fix is stop-and-restart under the
supervisor, never stop-forever.

### 4. Volume and clip set

- Restoring `ak_ao` makes `[sound] volume` live again; it is currently parsed,
  clamped and shipped over IPC while the demo hardcodes DAC 6 + ASLC 2.
- `ak_ao_demo.c:153` documents the DAC range as 0–12, not the 0–6 the config
  clamps to. Widen the clamp; keep the default well below max.
- **The repo clip set does not match the camera.** Repo holds the old tones
  (3.8–7.0 KB); `.198` holds TTS (29.8–57.6 KB). `make_speech.py` was run and
  deployed but only `upgrade.raw` was written back. Regenerate all four and
  commit, so the bundle matches what is tested.

## Error handling

Unchanged in shape from the original design, plus:

- The drain poll is watchdog-bounded; a wedged DAC must not hold the worker.
- `SPK_PA` is driven low (amp enabled) before playback and **never restored**.
  Restoring it would shut the amp off and make the next play silent.

## Testing

- **Host (C)**: `copy_for_dual_channel` output — the only new logic worth a
  test. Assert each mono sample appears twice and length doubles.
- **Host (Rust)**: unchanged; debounce, drop-when-busy, unmapped-event,
  config parsing already covered.
- **Device, headless**: log `sent`, wall-clock elapsed, and
  `ak_ao_get_params()` rate readback. Assert `sent == 2 × filesize` (correct,
  post-duplication) and elapsed ≈ clip duration. **`sent == 2 × filesize` with
  a non-duplicated buffer is the exact signature of this bug** — the
  instrumentation that would have caught it in one run.
- **Device, audible**: one listen for intelligibility on a TTS clip.
- **Device, regression**: RTSP/FLV audio still flowing after a play; daemon and
  onvif-rust PIDs unchanged.

## Addendum 2026-08-28: we ship a foreign `libplat_ao`

Investigated after `## ERROR: CHIP(14) unsupported` appeared in every playback
log. Recorded so it is not re-derived.

`libplat_ao` hardcodes `s_ininfo.chip = AUDIOLIB_CHIP_AK39XXEV3` for every
filter open (`libplat/include_inner/pcm.h:168,196`). That enum member is **14**
(`medialib_global.h:133-152`, counting from `AUDIOLIB_CHIP_UNKNOW = 0`).
`libakaudiofilter`'s `ak_aslc_init` accepts only 0, 1 or 2 — disassembly of the
shipped lib: `if ((unsigned)(v-1) <= 1) ok; if (v != 0) { printf("## ERROR:
CHIP(%d) unsupported"); return -1; }`. So ASLC can never open.

**Why:** the versions do not match, because our `libplat_ao.so` is not this
camera's.

| version | location | form |
| --- | --- | --- |
| V2.4.03 | `/usr/bin/anyka_ipc` (vendor main app) | statically linked |
| V2.4.02 | `/usr/bin/ak_adec_demo` | statically linked |
| **V1.2.02** | **ours**, from `anyka_reference/IOT-ANYKA-PTZdaemon/libs/` | the only `.so` |

The camera's rootfs contains **no `libplat_ao.so` at all** — nothing outside our
SD payload exports `ak_ao_open`. The native V2.4.x exists only as static code
inside stock binaries, so it cannot be linked against, and the original design's
claim that our libs are "md5-identical to the camera's stock vendor lib set"
does not hold for this one: there is nothing to be identical to.

Consequences:

- The abandoned `system("ak_adec_demo")` workaround was, accidentally, the only
  path using the **correct** `libplat_ao` for this SoC. That is a stronger
  argument for it than the one the original design rejected.
- Unknown residual risk: whether V1.2.02 differs from V2.4.x in more than the
  chip constant — DAC register layout or ioctl numbering would produce
  silence while every status call still reports success.

ASLC itself is **harmless to lose**: the write path skips a NULL aslc
(`ak_ao.c:566`) and the source notes it applies only to volume 7-12, above our
0-6 DAC range.

### The board is EV2 and our library targets EV3

`/proc/cpuinfo` and dmesg both report `Cloud39EV2_AK3918E80PIN_MNBD`,
`ANYKA CPU AK3918 (ID 0x20150200)` — an AK39XX **EV2**. `AUDIOLIB_CHIP_AK39XXEV2`
is 13; our library hardcodes 14 (`EV3`). So V1.2.02 is not merely an older
version, it asserts a **different silicon revision** than the camera has. For a
library that drives DAC registers that is a real latent risk, though it is *not*
the cause of the 2026-08-28 silence — the native V2.4.02 path is equally silent.

### A newer SDK does not exist publicly (searched 2026-08-28)

Do not repeat this search. Every known `libplat_ao`:

| version | source | form |
| --- | --- | --- |
| V2.4.03 | this camera's `/usr/bin/anyka_ipc` | static only |
| V2.4.02 | this camera's `/usr/bin/ak_adec_demo` | static only |
| V1.12.03 | `medevil84/ipcd` (AK3918EN080 v330) | Ghidra decompilation, no binary |
| V1.2.02 | `kuhnchris/IOT-ANYKA-PTZdaemon` — **ours** | `.so` + `.a` |

`biappi/anyka-fw` is an extracted rootfs from the **identical board string**. Its
`/lib` ships `libakaudiocodec.so`, `libakaudiofilter.so`, `libakispsdk.so`,
`libakstreamenc.so` and **no `libplat_*` at all** — independently confirming what
this camera shows. A GitHub-wide filename search for `libplat_ao.so` returns
exactly one hit, the decompilation above.

**Conclusion: Cloud39EV2 firmware does not ship `libplat_ao` as a shared object.**
The vendor statically links it into `anyka_ipc`. There is nothing to download.

### Options, none taken

1. Keep V1.2.02 and accept the EV2/EV3 mismatch and dead ASLC.
2. `fork`+`execv` a stock binary to reach the native V2.4.02 — safe if done with
   an argv array (no shell) and paths resolved from `/proc/self/exe`. Costs a
   hard dependency on `/usr/bin/ak_adec_demo`, i.e. the `.127` zt9101 risk.
3. Drop the vendor library entirely: `ak_ao` is a thin wrapper over
   `open("/dev/akpcm_cdev0")` + ioctls + `write()`. The V1.12.03 decompilation
   plus `OpenIPC/ak3918ev200`'s RE'd ioctl tables (which document
   `akpcm_cdev0/1` and the codec) make a ~200-line direct driver plausible.
4. Patch the chip immediate (`e3a0300e` → `e3a03002`) and accept wrong-silicon
   ASLC tuning.

Decide only after the output stage is repaired, using one A/B on the same clip:
our worker (V1.2.02) versus `ak_adec_demo` (V2.4.02). If both play, option 1 wins
for free. For loudness prefer raising clip level in `make_speech.py` over any of
these — clips currently sit at RMS 0.105 FS while peaking at 1.0, so limiting
buys ~7 dB *and* removes the existing clipping.

## Addendum 2026-08-29: root cause was `SPK_PA` polarity — RESOLVED

**Verified working on `.198`.** Our worker plays intelligible speech.

`SPK_PA` is the 8002D's **shutdown** pin and it is **active high**. Measured
with a meter at the amplifier:

| `SPK_PA` | amp outputs (pins 5/8 to ground) | state |
| --- | --- | --- |
| 1 | 0 V | shutdown |
| 0 | 2.5 V (VDD/2 on a 5 V rail) | enabled |

The pin is low at boot and `ak_ao_enable_speaker()` never writes it — the stock
state is enabled. The abandoned workaround added `spk_pa_set(1)` on the
assumption that the name meant "enable"; nothing ever verified it. That single
line switched the amplifier off before every clip.

**Why it presented as dead hardware.** Every layer above the amp kept reporting
success: `sent == 2 × filesize`, drain reaching `AO_PLAY_STATUS_FINISHED`, SDK
logging `set volume 6` and `dac ioctl set sample: 8000`, dmesg logging
`dac start` / `set_channel_source: s_dac=1` on every play. The amplifier was
simply switched off downstream of all of it. Teardown measurements settled it:
speaker 7.2 Ω (healthy), amp VDD→GND 15 kΩ (not shorted), VDD 5 V (rail good),
outputs 0 V (shutdown).

**How the diagnosis went wrong.** The "tones work, TTS does not" split that
motivated this whole design was a **timeline artifact**: the original `ak_ao`
worker never touched `SPK_PA` and played tones audibly; every build after the
workaround forced it high and was silent. It looked content-dependent because
the content changed at the same commit as the GPIO write. Worse, every
"known-good control" run during debugging set `SPK_PA=1` first, so the control
could not produce a positive result.

The three defects found along the way (mono→stereo, send-loop retry, unguarded
drain) are real and confirmed against `ak_ao.c`, and the worker could not have
worked with them — but they were not what the original symptom was about.

## Risks

1. **Only `.198` is verified.** `.127` (zt9101) unconfirmed, and its amplifier
   may differ in part or in polarity — measure before assuming this fix
   transfers. Dropping the rootfs binary dependency *reduces* the older form of
   this risk, but the `SPK_PA` finding adds a new one.
2. **Clip levels.** Resolved for the shipped set: compression plus EBU R128 puts
   peaks at 0.78–0.80 against a −2 dBFS ceiling. Re-check after any regeneration,
   since the peak ceiling binds before the loudness target on speech.
3. **The SD card's FAT was corrupting.** `.198` remounted read-only twice during
   this work (`clusters badly computed`), which blocks the supervisor from
   starting services. Repaired 2026-08-29; unrelated to audio.
4. **Capture contention was never demonstrated.** `push_stop_audio()` is not
   called from the sound path, and playback measurably does not disturb capture
   (`[audio] push stopped` stayed at 0, `frames=300 drops=0` after a play).
5. **The speaker resonates the casing at high volume.** `volume = 6` is DAC max;
   lower it if the housing rattles.

## References

- **Primary**: `cross-compile/anyka_reference/platform/libplat/demo/ao_demo/ak_ao_demo.c`
  (`copy_for_dual_channel` :66, `wait_play_finished` :80, setup :149-159)
- AO API and ranges: `cross-compile/anyka_reference/platform/libplat/include/ak_ao.h`
- Superseded design: `docs/plans/2026-08-26-event-audio-playback-design.md`
