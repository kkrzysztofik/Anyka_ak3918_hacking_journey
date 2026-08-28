# Event Audio Playback — Revision Design

Date: 2026-08-28
Status: approved (design)
Branch: `feat/event-audio-playback`
Supersedes decisions in: `2026-08-26-event-audio-playback-design.md`

## Why a revision

The branch shipped an in-daemon `libplat_ao` worker, then abandoned it for
`system("/usr/bin/ak_adec_demo ...")` after TTS clips played silently while
short tones worked. This document records the root cause of that silence,
which is a four-line bug in our own send loop, and reverts the workaround.

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
| Volume | `ak_ao_set_dac_volume`, range 0–12 | was 0–6 |
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

Retain `spk_pa_set()` around playback. The `SPK_PA` amplifier GPIO is a real
discovery from the workaround and is orthogonal to the buffer bug.

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
- `SPK_PA` is released on every exit path, including the watchdog abort.

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
0-6 DAC range. Options, none taken: keep V1.2.02; shell out to reach V2.4.02;
source a V2.4.x `.so` from a newer SDK; or patch the chip immediate
(`e3a0300e` → `e3a03002`) and accept wrong-silicon ASLC tuning. Prefer raising
clip loudness in `make_speech.py` over any of these — the clips currently sit at
RMS 0.105 FS while peaking at 1.0, so limiting buys ~7 dB *and* removes existing
clipping.

## Risks

1. **Not yet verified on hardware, and currently unverifiable.** As of
   2026-08-28 `.198` produces only a click per play from *any* source, including
   the stock `ak_adec_demo` that worked on 2026-08-27. Survives a power cycle.
   See [[198-speaker-output-dead-since-tts-work]]. The fixes below are grounded
   in the vendored SDK source, not in a passing listen test.
1. **The symptom that motivated this design was misread.** "Tones work, TTS
   does not" was a **timeline artifact** — the tones were tested before the
   output stage failed and the speech after. The three defects found are real
   and independently confirmed against `ak_ao.c`, but they are not established
   as the cause of the original silence.
2. **Capture contention may be real.** Removing `push_stop_audio()` is the
   lazy correct default; the test in §4 decides.
3. **Only `.198` is verified.** `.127` (zt9101) still unconfirmed — but this
   revision *reduces* that risk by dropping the rootfs binary dependency.
4. Speaker resonates the casing at high volume. Keep the default modest.

## References

- **Primary**: `cross-compile/anyka_reference/platform/libplat/demo/ao_demo/ak_ao_demo.c`
  (`copy_for_dual_channel` :66, `wait_play_finished` :80, setup :149-159)
- AO API and ranges: `cross-compile/anyka_reference/platform/libplat/include/ak_ao.h`
- Superseded design: `docs/plans/2026-08-26-event-audio-playback-design.md`
