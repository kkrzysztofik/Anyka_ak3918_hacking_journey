# HTTP-FLV onMetaData tag (framerate for all clients)

Date: 2026-08-13  
Status: draft

## Problem

mpegts.js reports `23.976 fps` on the camera's HTTP-FLV stream. That value is a
hardcoded fallback (`flv-demuxer.js:91-96`), not measured data: the demuxer only
overrides it from `onMetaData.framerate` (`flv-demuxer.js:438-448`), and our
HTTP-FLV server never emits an `onMetaData` script tag. ffprobe already derives
the true ~15 fps from timestamps, but clients that rely on FLV metadata do not.

Verified on-device (192.168.2.198): the camera SPS has
`vui_parameters_present_flag=1` + `timing_info_present_flag=1` but
`fixed_frame_rate_flag=0`, so mpegts.js ignores the SPS VUI fps and falls back
to `_referenceFrameRate`. Emitting `onMetaData.framerate` therefore fixes
mpegts.js; it also fixes VLC/ffprobe-class clients that read FLV metadata.

## Decision

Emit a single `onMetaData` FLV script tag (tag type 18) as the first tag after
the FLV header, carrying the configured `video_framerate` (15) plus
`videocodecid`, `hasVideo`, `hasAudio` for tool friendliness. No width/height:
every consumer (mpegts.js, ffprobe, VLC) already reads those from SPS.

## Design

### streaming-lib `HttpFlv` (`protocol/httpflv/httpflv.rs`)

Two small edits to make the connection metadata-capable:

1. `process_header_phase`: cache `FrameData::MetaData` alongside audio/video
   frames (currently dropped via `_ => {}`), so it lands right after the FLV
   header.
2. `extract_flv_tag_data`: map `FrameData::MetaData` → tag type 18
   (`SCRIPT_DATA_AMF`), replacing the current `Err(UnexpectedFrameData)`.

Update `test_write_flv_tag_metadata_rejected` (now written as script tag) and
add a header-phase caching test.

### onvif-rust `ValidationHttpFlvRemuxer` (`validation/httpflv_remux.rs`)

1. Add `video_framerate: u32` to the struct + `new()`.
2. Add `on_metadata_tag(timestamp) -> FrameData::MetaData` building the AMF0
   body: `"onMetaData"` string + ECMA array `{videocodecid: 7, hasVideo: true,
   hasAudio: <from audio_config>, framerate: <configured>}` — matching the
   byte format ffmpeg/mpegts.js parse (`08` ECMA marker, `00` number type,
   `00 00 09` object end).
3. ~20-line AMF0 encoder helper (string, number, bool, ECMA array).

No SPS parsing, no width/height.

### Wiring (`streaming/helpers.rs`, `service.rs`, `main.rs`)

- `send_httpflv_prior_frames` sends the metadata tag **first**, before the
  video sequence header (FLV spec ordering).
- Thread `video_framerate` into the remuxer from `LiveStreamHandler` (already
  holds it), the validation handler, and `FanoutTask`.

The value advertised is the configured `video_framerate` (15) — the same value
SDP `a=framerate` already advertises.

## Non-goals

- Width/height in onMetaData (SPS covers them; add only if a client requires it)
- Measured-frame-rate advertising (frontend already shows real fps via
  `decodedFrames` deltas from the prior fix)
- Rewriting SPS VUI timing

## Testing

- streaming-lib: MetaData frame → tag type 18 written; header-phase cache keeps
  it first after the FLV header
- onvif-rust: `on_metadata_tag` bytes decode to `onMetaData` + ECMA array with
  the configured framerate (assert against the ffmpeg/mpegts.js byte layout)
- On-device: mpegts.js Live View shows ~15 fps (frontend measured) and
  ffprobe/ffplay read correct metadata
