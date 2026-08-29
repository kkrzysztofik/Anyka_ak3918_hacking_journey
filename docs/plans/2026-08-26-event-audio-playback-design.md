# Event Audio Playback — Design

Date: 2026-08-26
Status: approved (design)
Branch: `feat/event-audio-playback` (from `main`)

## Problem

The cameras have a working built-in speaker that nothing in our stack drives. We
want selected events to make a sound: an audible confirmation that the camera
came back after a power cut, that the link dropped, or that an A/B upgrade
committed — on a headless device where the alternative is opening a laptop.

Audio *capture* landed in PR #97 (`AI → AENC → AAC` → RTSP/HTTP-FLV). Audio
*output* does not exist anywhere in our code: only ONVIF type definitions
(`AudioOutputConfiguration`) and zero `ak_ao` / `ak_adec` usage.

## Hardware verification (192.168.2.198, 2026-08-26)

Everything below was measured, not inferred from headers.

| Question | Result |
| --- | --- |
| Is there a physical speaker? | **Yes.** Stock `/usr/bin/ak_adec_demo` played an MP3 to completion, exit 0 |
| Does raw PCM play? | **Yes.** Our own generated 8 kHz mono s16le file, `ak_adec_demo 8000 1 pcm`, exit 0 |
| Does playback disturb streaming? | **No.** onvif-rust (509) and vendor-daemon (514) PIDs unchanged across both tests |
| Does capture contend with playback? | **No.** The daemon holds `/dev/akpcm_cdev1` read-only *permanently*; playback is a separate fd |
| Are `libplat_ao.so` / `libmpi_adec.so` available? | **Yes**, already in the repo, md5-identical to the camera's stock vendor `onvifd` lib set |

Two traps worth recording:

- Boot dmesg reads `adc_ready=1, dac_ready=0`. That is **not** missing hardware —
  the DAC is simply unopened until something plays. A driver state flag is not a
  capability flag. Same shape as `ae-luma-cannot-see-dusk`.
- The community README claims "volume control fails when running". It does not.
  `ak_adec_demo` hardcodes `ak_ao_set_dac_volume(h, 6)` (DAC max) plus
  `ak_ao_set_aslc_volume(h, 2)` and exposes no argument. The API is fine; the
  demo is the problem.

## Goals

1. Play a short sound on selected events, without ever affecting streaming.
2. Triggers: **WebUI-initiated** plus **system/lifecycle** events — boot ready,
   network state change, config-applied / upgrade result.
3. A **small fixed clip set** shipped in the bundle.
4. A **real volume setting**, since the deafening-playback defect is ours to fix.

## Non-goals (v1)

- Motion detection (no `ak_md` integration exists; not a chosen trigger)
- ONVIF Events service build-out (it is a scaffold returning `ActionNotSupported`;
  the chosen triggers do not need it)
- Clip upload UI / user-supplied audio
- Two-way audio / ONVIF backchannel
- Per-event volume, playlists, looping

## Decisions

| Topic | Choice |
| --- | --- |
| Where | New IPC verb in vendor-daemon `handlers_audio.c` |
| Format | Raw PCM s16le mono (`.raw`) |
| Libs | `libplat_ao.so` only |
| Volume | `ak_ao_set_dac_volume`, configurable 0–6 |
| Queueing | None — drop if busy |
| AO handle | Opened per play, not persistent |

### Why in the daemon, not by spawning the demo

Shelling out to `ak_adec_demo` was the first proposal and was rejected:

- It is a **stock-rootfs demo we do not control**, verified on `.198` only. The
  fleet includes the zt9101 board on `.127` with a different rootfs. Depending on
  vendor demo binaries makes the feature hostage to per-camera rootfs contents.
- It hardcodes near-max volume and offers no stop, no status, no queue.
- It would make audio playback the one subsystem not following the pattern that
  video, OSD, ISP, PTZ and audio *capture* all follow: the daemon owns SDK handles.
- The stated benefits did not survive scrutiny. "Crash isolation" is weak when the
  daemon already links 23 vendor libs including `libmpi_venc` and `libplat_ai`.
  "No new libs" is weak when it is 61 KB into a `lib/` dir that already has 23 files.
  "Zero new code" buys a permanently crippled feature and a `fork()` on a box with
  3 MB free.

The demo's real job was proving the hardware. That job is done.

### Why PCM, not MP3

This is what makes the daemon implementation *smaller* than the shell-out:

- **PCM needs only `libplat_ao.so`**: `ak_ao_open` → `ak_ao_set_dac_volume` →
  `ak_ao_send_frame` loop → `ak_ao_notice_frame_end` → `ak_ao_close`. ~50 lines.
- **MP3 drags in the whole ADEC subsystem**: a second library, a decode thread,
  `ak_adec_request_stream` / `cancel_stream` lifecycle, and decode-buffer reallocs
  we watched happen during the probe — on a device with 3 MB free.

MP3's only advantage is file size, which is worth nothing at chime length
(16 kHz mono s16le ≈ 32 KB/s; a handful of clips stays under 300 KB).

## Architecture

Mechanism in C, policy in Rust.

```text
supervisor / network / applier ─┐
                                ├─► SoundPlayer (Rust)  ── IPC ──►  handlers_audio.c
WebUI ──► ONVIF/HTTP ───────────┘   debounce, event→clip,           play worker thread:
                                    drop-if-busy, config            ao_open → set_volume →
                                                                    send_frame loop →
                                                                    notice_frame_end → close
```

The C side stays deliberately dumb: *"play this file at this volume; return BUSY
if already playing."* One worker thread, one AO handle opened per play — less
retained state than the vendor's persistent-handle approach in `ak_voice_tips.c`,
and plays are rare.

All policy worth testing lives in Rust, where it is unit-testable without
hardware: debounce windows, event→clip mapping, drop-when-busy.

Playback is serialized by construction — one DAC, one worker. A second sound
arriving mid-play is **dropped, not queued**: a chime backlog is worse than a
missed chime. Network events in particular must not machine-gun the speaker when
a link flaps, hence the debounce.

## Components

| Component | Change |
| --- | --- |
| `cross-compile/vendor-daemon/src/handlers_audio.c` | New play handler + worker thread |
| `cross-compile/vendor-daemon/src/protocol.h` | New IPC verb |
| `cross-compile/vendor-daemon/lib/libplat_ao.so` | Added to the shipped lib set |
| `cross-compile/vendor-daemon/include/ak_ao.h` | Added |
| `onvif-rust` `platform/anyka/sound.rs` | New `SoundPlayer` |
| `onvif-rust` config | New `[sound]` section |
| Event call sites | supervisor (boot ready), network, upgrade applier |
| Bundle | `sounds/*.raw` clip set |

## Configuration

Lives in **`config.toml`** — onvif-rust's own config, the same file that already
carries `[osd]`, `[imaging]` and `[ptz]`. Not `anyka.toml`, which belongs to the
anyka-init supervisor. `[osd]` took exactly this path in PR #95, so it is proven.

```toml
[sound]
enabled = true
clip_dir = "sounds"        # relative to slot, like static_root
volume = 3                 # 0-6 DAC; the demo's hardcoded 6 is what made it deafening
debounce_secs = 30

[sound.events]
boot_ready     = "boot.raw"
network_lost   = "alert.raw"
network_up     = "ok.raw"
upgrade_result = "ok.raw"
```

An unmapped event is silent. That is the per-event off switch — no extra flags.

Clips are s16le mono. **16 kHz proposed; 8 kHz is already proven on hardware** and
is the fallback if 16 kHz misbehaves.

## Error handling

Playback is best-effort and must never affect streaming.

- Missing clip dir or `enabled = false` → log **once** at startup, not per event.
- `ao_open` failure or BUSY → warn, drop the sound, continue.
- **Worker watchdog**: a wedged play must not hold the DAC forever and block every
  later sound. This is the one case that genuinely needs a timeout.
- No caller ever blocks or awaits on audio — a yielding await costs ~12 ms here.

## Testing

- **Host (Rust)**: debounce windows, drop-when-busy, unmapped-event-is-silent,
  config parsing. IPC boundary mocked.
- **Host (C)**: the frame-send loop against a fake AO — the only real logic in C.
- **Device**: trigger all four events; assert daemon PIDs unchanged after each.
  That is the same check that validated both hardware probes.

## Risks

1. **The device's `config.toml` diverges from the repo copy.** The device file is
   the "DEBUG variant" (~11.7 KB vs the repo's ~10.8 KB) with different logging and
   stream profiles, so deploying the repo copy wholesale silently overwrites live
   settings. Add the `[sound]` stanza **in place** with `sed -i` (busybox sed
   supports it), back up first, and validate. This is the `[osd]` rollout problem,
   not the `anyka.toml` one.
2. **Only `.198` is verified.** Confirm the DAC on `.127` (zt9101, different
   rootfs) before fleet rollout. This risk is *smaller* than the shell-out
   approach, since we ship our own lib rather than depending on a vendor demo.
3. **16 kHz unverified.** 8 kHz is the proven fallback.
4. Speaker is loud and the casing resonates. Default `volume = 3`, not 6.

## Phasing

1. C playback verb + `libplat_ao.so` into the daemon lib set, one hardcoded clip,
   verified on `.198`.
2. Rust `SoundPlayer`, config, debounce, the four events.
3. WebUI trigger + test button.

## References

- Vendor blueprint: `cross-compile/anyka_reference/akipc/misc/ak_voice_tips.c`
- Demo source (volume hardcode): `cross-compile/anyka_reference/platform/libmpi/demo/adec_demo/ak_adec_demo.c:244`
- AO API + volume ranges: `cross-compile/anyka_reference/venc_demo/include/ak_ao.h`
- Libs: `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/{libplat_ao.so,libmpi_adec.so}`
- On-camera probe material: `/mnt/anyka_hack/ak_adec_demo/` (README, sample MP3)
