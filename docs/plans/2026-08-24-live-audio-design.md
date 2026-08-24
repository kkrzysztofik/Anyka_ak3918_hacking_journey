# Live AAC audio (RTSP + HTTP-FLV)

Date: 2026-08-24
Status: draft

## Problem

The camera has a working microphone and none of it reaches a client. ONVIF
already *claims* audio — `platform/anyka/audio_encoder.rs` answers
`GetAudioEncoderConfiguration` from a `RwLock<Vec<AudioEncoderConfig>>` that
hardware never touches, and its `Arc<dyn AudioHalTrait>` field is literally
marked `#[allow(dead_code)]`. So the device advertises an audio encoder and
delivers silence.

Scope for this design is **live audio in the streams only**: a second track in
RTSP (SDP `m=audio` + RTP) and in HTTP-FLV, so VLC, an NVR and the WebUI live
preview all get sound. No files are written; ONVIF Recording/Replay is out of
scope.

## What already exists

Audio was scaffolded across four layers and the wire between them was never
connected. Verified by reading the code, not assumed:

| Layer | State |
|---|---|
| `vendor-daemon/include/vd_ring_buffer.h:89` | `VD_STREAM_AUDIO 2` already defined; `vd_frame_notify` already carries `stream_id` |
| `vendor-daemon/src/handlers_audio.c` | `AI_OPEN/CLOSE`, `AENC_OPEN/CLOSE/SET_ATTR` (cmds 50–56), object-table registered |
| `onvif-rust/src/hal/common/audio.rs` | RAII handles, `AudioHalTrait`, mockall'd, ~10 unit tests |
| `onvif-rust/src/hal/anyka/ipc/audio.rs:14` | full `AudioHalTrait` impl over IPC |
| `onvif-rust/src/streaming/bridge.rs:452` | `StreamId::Audio` → `FrameData::Audio`, fanned to **both** main and sub queues |
| `onvif-rust/src/streaming/service.rs:301` | SDP emits the audio track whenever `bridge.audio_config` is `Some` |
| `onvif-rust/src/streaming/helpers.rs:192` | AAC `AudioSpecificConfig` → SDP `config=` hex; FLV audio sequence header |
| `streaming-lib` | `rtp_aac.rs` packer complete, FLV soundformat 10, hub `FrameData::Audio` |
| `onvif-rust/src/config/types.rs:946` | `audio_enabled` / `audio_encoding` / `audio_bitrate` / `audio_sample_rate` per profile |
| `vendor-daemon/Makefile` LDFLAGS | already links `-lplat_ai -lmpi_aenc -lakaudiocodec -lakaudiofilter` |

Only two things are missing:

1. **`push.c` contains zero audio.** `push_slot_index()` returns `-1` for
   anything but MAIN/SUB, so no slot is ever written with
   `stream_id = VD_STREAM_AUDIO`.
2. **Nothing ever sets `bridge.audio_config`.** It is `RwLock::new(None)` at
   construction (`bridge.rs:406`) with no setter, so `generate_av_sdp` silently
   omits the audio m-line and no client negotiates the track.

## Hardware verification (192.168.2.198, 2026-08-24)

- `/dev/akpcm_cdev0`, `/dev/akpcm_cdev1` present.
- `libplat_ai.so`, `libmpi_aenc.so`, `libakaudiocodec.so`, `libakaudiofilter.so`
  all present in the **active slot** `slots/b/vendor-daemon/lib/`. Unlike the
  OSD case, the audio libs ship.
- `ak_ai_open` succeeds while `vendor-daemon` is running — the PCM device is not
  held by the vendor stack.

Codec cost, measured with the on-device `aenc_demo`, 10 s mic capture, only the
codec argument changed:

| Codec | user | sys | total CPU | ≈ % of one core | 10 s output | bitrate |
|---|---|---|---|---|---|---|
| G.711 A-law | 0.17 s | 0.56 s | 0.73 s | 6.8 % | 75,200 B | 60 kbps |
| AAC | 0.77 s | 0.42 s | 1.19 s | 11.6 % | 19,769 B | 15.8 kbps |

Camera baseline at the time: **76.9 % idle**, 36.5 MB RAM with 15 MB reclaimable
cache.

## Decisions

**Codec: AAC.** G.711 is 4.5× cheaper in user time, but the delta is 0.46
CPU-seconds per 10 s of audio — ~5 % of one core against 77 % idle. Encoding
happens once and is fanned out by the hub; transmission happens *per client*, so
G.711's extra 44 kbps is paid per viewer while AAC's 5 % is paid once. At two
concurrent clients AAC is already cheaper system-wide. AAC is also the option
whose RTP packer, SDP path and FLV path are already written and tested here —
`RtspCodecId::G711A` is a `// TODO` stub in both `create_packer` and
`create_unpacker` (`rtsp_channel.rs:166,193`).

Accepted risk: an NVR that negotiates only G.711 gets no audio. The capture path
is codec-agnostic, so adding G.711 later is one packer, not a redesign.

**Transport: reuse the existing ring.** `VD_STREAM_AUDIO` already exists, the
notify struct already carries `stream_id`, and `bridge.rs` already routes
`StreamId::Audio`. Zero new transport code on either side. The cost is that a
~200-byte AAC frame occupies a whole 128 KB slot (0.15 % utilization) and adds
~10 writes/s on top of main+sub's ~50/s — ~20 % more slot pressure and
`g_ring_write_lock` contention for 0.4 % more bytes. Rejected: a second
right-sized shm ring (duplicates create/attach/epoch/notify logic in C and Rust,
plus a new epoch-mismatch failure class) and socket-carried frames (new protocol
message type and a new Rust read path, since the frame client assumes
notify-then-read-slot).

**Lifecycle: always-on when `audio_enabled`.** Matches video, which is already
always-on from encoder startup (`video_encoder.rs:817`) rather than
subscriber-driven. Audio is not per-stream — `bridge.rs:457` pushes each frame
into *both* stream queues, so there is exactly one mic, one encoder and one push
thread regardless of profile count.

Rejected: on-demand capture on first subscriber. RTSP `DESCRIBE` calls
`send_information()` → `generate_av_sdp(audio_config)`. If capture starts only
when a subscriber attaches, the first client's DESCRIBE arrives before
`audio_config` is populated, gets a video-only SDP, and never negotiates the
audio track even though audio starts flowing moments later — presenting as
"audio works on reconnect but not first connect".

**Default stays enabled.** `config/types.rs:966` has
`stream_profile_1.audio_enabled = true` today and this design keeps it. Stated
plainly because it has a consequence: deploying this will start capturing room
audio on `.198`, `.121`, `.146` and `.127` with no further action. The
alternative (default `false`, enabling as an explicit act) was offered and not
taken.

**Sample rate stays 8000 Hz**, matching the existing config default and the
measurement above. 16 kHz is a config-value change with no code change — the ASC
table below already covers it.

## Architecture

One microphone → one encoder → one push thread → existing ring → existing bridge
fan-out.

```
ak_aenc_request_stream(ai, aenc)   [binds input to encoder, once]
        │
        ▼
ak_aenc_get_stream(stream_handle) ──▶ vd_ring_write(VD_STREAM_AUDIO)
   (list of aenc_entry, raw AAC)        │
   ak_aenc_release_stream per entry     ▼
                                    send_frame_notification
                                                │
                                                ▼
                              bridge.route_owned_frame(StreamId::Audio)
                                                │
                                    ┌───────────┴───────────┐
                                    ▼                       ▼
                            main_stream.queue        sub_stream.queue
                                    │                       │
                              RTSP (rtp_aac) ─── HTTP-FLV (soundformat 10)
```

## Components

**C — `vendor-daemon`** (~120 lines; no Makefile change, the libs are already
linked)

- `protocol.h`: `CMD_AUDIO_START_PUSH = 57`, `CMD_AUDIO_STOP_PUSH = 58`
  (57–58 are free; the audio block ends at 56 and ISP starts at 100).
- `globals.h`: `PUSH_STREAM_SLOT_COUNT` 2 → 3.
- `push.c`: add `VD_STREAM_AUDIO` arms to `push_slot_index()` (→ index 2) and
  `push_stream_id_to_ring_stream()`; add `audio_push_thread()`.
- `handlers_audio.c` + `dispatcher.c`: the two new handlers.

**The start-push handler owns the whole SDK chain.** Unlike video — where Rust
opens the encoder and passes a handle — `ak_aenc_request_stream(ai_handle,
aenc_handle)` is the step that binds input to encoder, and the resulting stream
handle must stay alive exactly as long as the push thread. Marshalling three
handles across IPC to achieve that would couple token lifetimes to thread
lifetime for no gain. So `CMD_AUDIO_START_PUSH` carries only
`{sample_rate, channel_num, frame_interval_ms}` and the daemon does:

```
ak_ai_open(&pcm_param{sample_bits=16, ...})
ak_ai_set_aec / set_nr_agc / set_resample   (all DISABLE)
ak_ai_set_source(AI_SOURCE_MIC)
ak_ai_clear_frame_buffer
ak_ai_set_frame_interval(interval_ms)        <-- before start_capture
ak_ai_start_capture
ak_aenc_open(&audio_param{type=AK_AUDIO_TYPE_AAC, ...})
ak_aenc_set_attr({aac_head = AENC_AAC_CUT_FRAME_HEAD})
ak_aenc_request_stream(ai_handle, aenc_handle) -> stream_handle
```

then spawns the thread, which loops `ak_aenc_get_stream(stream_handle,
&stream_head)`, walks the returned `aenc_entry` list writing each
`audio_stream` to the ring, and calls `ak_aenc_release_stream(entry)` per entry.
Teardown reverses it: `ak_aenc_cancel_stream`, `ak_aenc_close`,
`ak_ai_stop_capture`, `ak_ai_close`.

The existing cmds 50–56 stay for ONVIF-side volume and config; they are not on
the streaming path.

Frame interval is derived from the codec, not configured: AAC is 1024
samples/frame, so `interval_ms = 1024 * 1000 / sample_rate` — 128 ms at 8 kHz,
64 ms at 16 kHz (`ak_aenc_demo.c:216`).

**Rust — HAL** (~40 lines): `start_audio_push(sample_rate, channels)` /
`stop_audio_push()` in `hal/anyka/ipc/mod.rs`, mirroring `start_push()`
including stale-epoch handling.

**Rust — platform** (~60 lines): `AnykaAudioEncoder::start()` calls
`start_audio_push` and publishes the ASC via `bridge.set_audio_config`. Invoked
from where `video_encoder.rs:817` starts video push.

`AENC_AAC_CUT_FRAME_HEAD` (value 2 in `aenc_aac_attr`) is load-bearing: it emits
raw AAC frames. The alternative `AENC_AAC_SAVE_FRAME_HEAD` prepends ADTS
headers, which is right for files (and is what `aenc_demo` used) but wrong for
both RTP AAC-hbr and FLV. Getting this backwards yields a negotiated track that
decodes to garbage.

**Rust — bridge** (~5 lines): `set_audio_config(Vec<u8>)`. This single setter
turns on the SDP audio m-line and the FLV audio sequence header simultaneously.

**Config**: flip the `audio_encoding` default from `"G711"` to `"AAC"` so ONVIF
stops misreporting. `parse_audio_encoding` already maps `"aac"` →
`AudioEncoding::AAC`.

## Data: AudioSpecificConfig

Two bytes derived from sample rate and channel count — 5 bits object type
(2 = AAC-LC), 4 bits sampling-frequency index, 4 bits channel configuration:

| Rate | Freq index | ASC |
|---|---|---|
| 8000 Hz mono | 11 | `15 88` |
| 16000 Hz mono | 8 | `14 08` |

These two bytes appear in three places: the SDP `config=` fmtp, the FLV
`AudioSpecificConfig` sequence header, and the RTP AAC-hbr sizing.

## Error handling

- `ai_open` / `aenc_open` failure → log, leave `audio_config` as `None`, and
  leave video completely unaffected. Audio is strictly additive; its failure must
  never take down video.
- Audio frames are written as `VD_FRAME_TYPE_P`, deliberately. The ring evicts
  P-frames to make room for I-frames, so under pressure audio sheds *before*
  video keyframes. Video is the primary product on a security camera. Marked
  with a `ponytail:` comment naming the trade-off.
- Daemon restart re-establishes audio push through the existing epoch/stale-token
  path. No new mechanism.

## Testing

**Host**
- ASC bit-packing for both 8 kHz and 16 kHz (real bit manipulation, so it gets a
  test).
- IPC command encoding for 57/58, including the stale-epoch path.
- `set_audio_config` → `generate_av_sdp` output contains `m=audio`.
- `push_slot_index(VD_STREAM_AUDIO) == 2`.

**Device**
- `ffprobe` on the RTSP URL asserts two streams present.
- Decode 10 s and assert the audio track is non-silent — guards against a
  wired-up-but-mute microphone, which the track-exists check alone would miss.

## Risks

**A/V sync (main risk).** Audio RTP runs on an 8000 Hz clock and video on
90000 Hz. Both SDK timestamps are in milliseconds, but they must share a time
origin or lip-sync drifts. Video normalizes by subtracting its *own* first
timestamp (`push.c:218`), and main and sub each anchor independently — harmless
today because they are separate sessions, but audio is fanned into *both*
queues, so it must anchor consistently. Plan: anchor the audio thread to
`g_push_streams[0].first_timestamp_ms` once video has initialized rather than
letting audio pick its own anchor. Verify on hardware early; this is why the
device test decodes rather than only checking that the track exists.

**Growing `PUSH_STREAM_SLOT_COUNT` to 3 breaks two hardcoded spots.**
`handle_venc_stop_push` with an empty payload stops slots 0 and 1 by literal
index (`push.c:707-708`), and the ring-reset-on-first-activation check uses
`other_idx = (idx == 0) ? 1 : 0` (`push.c:634`). Both must become loops over
`PUSH_STREAM_SLOT_COUNT` or audio will be left running by "stop all" and the
ring will be reset out from under an active stream.

**Frame interval is out of documented range at 8 kHz.** AAC's interval is
`1024*1000/sample_rate` = 128 ms at 8 kHz, but `ak_ai.h` documents
`ak_ai_set_frame_interval` as accepting [10, 125] ms. `aenc_demo` computes 128
regardless and produced a valid 19,769-byte AAC file in the measurement above,
so the SDK tolerates or clamps it. Noted rather than acted on: 16 kHz yields
64 ms, inside the documented range, and is a config-value change with no code
change if 8 kHz misbehaves. Check the return code of
`ak_ai_set_frame_interval` and log it rather than ignoring it.
