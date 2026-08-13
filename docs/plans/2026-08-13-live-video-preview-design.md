# Live Video Preview — Design

Date: 2026-08-13
Scope: `cross-compile/www` only. No Rust changes.

## Problem

`LiveViewPage.tsx` presents a complete-looking live view that shows no video.
The PTZ d-pad, presets, and main/sub toggle are wired to real ONVIF calls, but
the video area (`LiveViewPage.tsx:245`) is a placeholder graphic, and the
surrounding chrome is hardcoded mock data:

| Line | Element | Current value |
| --- | --- | --- |
| `241` | "Connected" badge | hardcoded string |
| `266` | preview subtitle | `1920×1080 @ 30fps` |
| `275` | LIVE indicator | always rendered |
| `291` | Stream URL | `rtsp://192.168.1.100:554/main` |
| `293` | Copy button | no `onClick` |
| `320`–`336` | Stream Info card | all four rows hardcoded |
| `357`–`370` | Network Stats card | all four rows hardcoded |

The camera already streams. Nothing is missing server-side; the picture simply
never reaches the browser.

## What already exists

| Service | Port | Endpoint |
| --- | --- | --- |
| WebUI + ONVIF SOAP | 80 | `/`, `/onvif/*` |
| HTTP-FLV | 8080 | `/live/main.flv`, `/live/sub.flv` |
| RTSP | 554 | `/main`, `/sub` |

Three properties of the existing HTTP-FLV server make this a frontend-only task:

1. `streaming-lib/src/protocol/httpflv/server.rs:87` already sends
   `Access-Control-Allow-Origin: *`, so a page on :80 may fetch from :8080.
2. `streaming-lib/src/protocol/httpflv/httpflv.rs:120` (`process_header_phase`)
   withholds the FLV header until both audio and video are seen, or a frame
   counter expires. A late-joining client therefore still receives metadata and
   the AVC sequence header — the usual reason browser FLV playback fails.
3. `server.rs:51` makes Basic auth conditional, so the player works whether or
   not auth is enabled on the media port.

## Decisions

**Browser target: desktop Chrome/Firefox/Edge.** iOS Safari is out of scope; its
MSE support (ManagedMediaSource, iOS 17.1+) is unreliable for live FLV, and
supporting it would mean camera-side fMP4/HLS muxing.

**Transport: `mpegts.js` → Media Source Extensions.** No browser plays FLV
natively. `mpegts.js@1.8.1` (maintained successor to `flv.js`) demuxes FLV in JS
and remuxes to fMP4 for MSE. ~180 KB minified, ~45 KB brotli, landing in the
already-lazy `LiveViewPage` chunk.

Rejected alternatives:

- *Hand-rolled FLV parse → WebCodecs → canvas.* No dependency, but we would own
  frame pacing, buffer management, stall detection, loss recovery, and the
  `AVCDecoderConfigurationRecord` parse. That is not a few lines, and the
  failure modes are ones this project has already paid to diagnose. A dependency
  that replaces genuinely hard code is the correct trade.
- *Camera-side fMP4/HLS.* Would allow a plain `<video src>` and work on Safari,
  but costs new Rust and CPU on a 36 MB device, and adds a failure surface to a
  streaming stack that is currently broken on three of four cameras.

**Development target: 192.168.2.198.** Per `main-stream-dies-on-60d460dd`, it is
the only camera where FLV main survived that upgrade. `vite.config.ts` already
proxies to it.

## Architecture

```text
onvif-rust :8080  ──HTTP-FLV──▶  mpegts.js  ──fMP4──▶  MSE ──▶ <video>
  /live/main.flv                (demux+remux)                     │
  /live/sub.flv                       │                           │
                                      ├── MEDIA_INFO ──────┐      │
                                      ├── STATISTICS_INFO ─┼──▶ React state
                                      └── ERROR ───────────┘      │
                                                            Stream Info +
                                                            Network Stats cards
```

### URL construction

The page is served from :80 and the stream from :8080, so a relative path will
not do in production:

- Production: `http://${location.hostname}:8080/live/${streamType}.flv`
- Development: a fourth entry in `vite.config.ts` proxying `/live` to port 8080
  of `VITE_API_TARGET`'s host, matching the three proxy entries already there.

### Authentication

The app stores credentials under `sessionStorage['onvif_camera_auth']`, but the
password is **AES-GCM encrypted** (`hooks/useAuth.tsx:97`), so that key must
never be read directly. The correct accessor is `useAuth().getBasicAuthHeader()`
(`hooks/useAuth.tsx:132`), which decrypts on demand and returns
`"Basic <base64>"` or `null`.

It is `async`, which means the player's setup effect is asynchronous: await the
header and the dynamic `import('mpegts.js')` together, then create the player.
Pass the result through mpegts.js's `headers` config. No second login, no new
credential storage.

### Components

One new file, `components/common/LiveVideoPlayer.tsx`, plus edits to
`LiveViewPage.tsx`. The player owns the `<video>` ref and the mpegts.js
lifecycle (`createPlayer` → `attachMediaElement` → `load`, and `destroy()` on
unmount and on stream switch), reporting upward through a single `onStatus`
callback. `mpegts.js` is pulled in with a dynamic `import()` so it stays in the
lazy chunk.

### Player configuration

```js
{
  enableStashBuffer: false,        // do not hold frames before handing to MSE
  liveBufferLatencyChasing: false, // deliberately off — see below
}
```

`liveBufferLatencyChasing` accelerates playback to burn off accumulated buffer.
That is the same class of mechanism as the `push.c` stall-catchup ratchet that
caused the VLC late-pictures bug and was removed on 2026-08-06. Enabling it
invites a client-side rerun of an already-diagnosed bug. It stays off by
default, available as a knob.

## Replacing the mock chrome

The stats UI already exists; it is fed fabricated values. This work replaces the
data source, not the components.

| Element | Becomes |
| --- | --- |
| preview subtitle (`:266`) | `MEDIA_INFO.width/height/fps` |
| Stream URL (`:291`) | the URL actually in use, with a working Copy button |
| Status badges (`:241`, Network Stats) | real player state |
| Stream Info — Resolution, Frame Rate, Codec | `MEDIA_INFO` |
| Stream Info — Bitrate | `STATISTICS_INFO.speed × 8` |
| LIVE indicator (`:275`) | rendered only while playing |
| Network Stats — Packet Loss, Latency | **removed**, replaced by `Dropped frames` and `Buffer ahead` |

MSE exposes neither packet loss nor RTT. Rather than keep two convincing
fabrications on screen, the rows are swapped for two values that are real and
that actually diagnose stalls.

## Error handling

State machine: `idle → connecting → playing → (stalled | error)`, driven by
mpegts.js `ERROR` events plus the `<video>` element's `waiting` and `playing`
events. The error state renders a human-readable reason and a Retry button. A
401 is surfaced distinctly as rejected credentials, since that is the
`users.toml` failure mode recorded in `missing-users-toml-kills-streaming`.

## Testing

Follows the existing `anyka-webui-testing` conventions: `vi.mock('mpegts.js')`
with a fake player exposing the event emitter, `data-testid` on new elements,
`renderWithProviders`. Cases:

- URL is built correctly for main and for sub
- auth header is passed to the player
- `destroy()` runs on unmount and on stream switch (leak check)
- `MEDIA_INFO` populates the Stream Info card
- `ERROR` renders the retry affordance
- 401 renders the credentials-specific message

## Risks

1. **Slow start from the header phase.** If this camera emits no audio,
   `process_header_phase` waits out its frame counter before releasing the FLV
   header, and every viewer pays that delay. Combined with the fixed ~2 s RTSP
   startup already recorded in `ak3918-stream-timing-measurement`, first frame
   could land 3–4 s after page open. Measure on .198 before changing anything;
   the fix, if warranted, is one constant in Rust rather than player config.
2. **H.264 profile.** MSE must accept the codec string. 720p main on .198 is
   very likely Baseline or Main, but verify from `MEDIA_INFO`.
3. **Fleet health.** This demos only on .198 until `main-stream-dies-on-60d460dd`
   is resolved.

### Hardware check on 192.168.2.198 (2026-08-13)

Rebuilt WebUI deployed to the active slot (`/mnt/anyka_hack/slots/b/onvif/www`).
Port 80 serves the new `index-O9x_J6Y9.js`; Live View dynamically imports
`mpegts-DT5H0-tC.js` (not in the eager modulepreload list). The production URL
in that chunk is `http://${hostname}:8080/live/{main,sub}.flv`.

| Stream | Resolution | Codec | Header on the wire |
| --- | --- | --- | --- |
| `/live/main.flv` | 1280x720 | H.264 Main ~15 fps | 0.51–0.58 s |
| `/live/sub.flv` | 640x360 | H.264 Main ~15 fps | 0.54 s |

CORS is `Access-Control-Allow-Origin: *`. Header delay is well under the 3 s
`process_header_phase` threshold, so no Rust change is warranted from this check.
A full browser sit (moving picture, 5-minute dropped-frame watch) still needs a
human at `http://192.168.2.198/`.

## Out of scope

Fullscreen, snapshot-to-PNG, and audio playback. Also out of scope, but noted as
a separate defect: `onvif/media/ops/capabilities.rs:32` advertises
`snapshot_uri: Some(true)` and `onvif/media/ops/streaming.rs:86` returns a
`/snapshot` URI, but no HTTP handler serves that path. ONVIF clients that follow
the advertisement receive a 404.
