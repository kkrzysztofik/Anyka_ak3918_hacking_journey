# go2rtc SDP Direction — Design

Date: 2026-08-05
Status: approved; implementation plan at `docs/plans/2026-08-05-go2rtc-sdp-direction.md`
Branch context: `feat/ir-led-support` (unrelated to IR; camera `.198` feeds Frigate on `.6`)

## Problem

go2rtc 1.9.10 inside Frigate on `192.168.2.6` cannot consume either camera stream
from `192.168.2.198`. Frigate logs, once per retry:

```
WRN [rtsp] error="streams: codecs not matched:  => video:ANY, audio:ANY" stream=salon-detect
ERROR ffmpeg.salon.detect : method DESCRIBE failed: 404 Not Found
```

The empty left side of `=>` is the producer's media list: go2rtc kept zero tracks.

Root cause: our DESCRIBE response ends each media section with `a=sendonly`.
go2rtc normalises direction to `recvonly` **only when the SDP omits it**
(`pkg/rtsp/helpers.go:94-97`; the sole exception is a hardcoded
`o=CV-RTSPHandler` camera workaround). An explicit `sendonly` survives, and
`internal/streams/play.go:138` then skips every producer media whose direction is
not `recvonly` — go2rtc reads it as a backchannel, not a video source. The `404`
ffmpeg sees on go2rtc's own restream is downstream of the same cause:
`pkg/rtsp/server.go:112-117` answers 404 when it has no senders to offer.

`a=sendonly` is *correct* per RFC 4566 §6, where direction is stated from the SDP
author's viewpoint. go2rtc stores it from its own viewpoint instead. Being
standards-compliant is what breaks us; most cameras omit the attribute entirely
and land in go2rtc's happy path.

### Evidence

tcpdump on `.6`, go2rtc → camera: auth succeeds, the 336-byte SDP arrives intact,
and go2rtc sends `TEARDOWN` without ever issuing `SETUP`.

Single-variable control, a throwaway RTSP responder serving the byte-identical SDP:

| SDP | go2rtc |
|---|---|
| with `a=sendonly` | `DESCRIBE` → `TEARDOWN`, no SETUP |
| without `a=sendonly` | `DESCRIBE` → `SETUP trackID=0` → `PLAY` |

ffprobe parses both `/main` and `/sub` fine — ffmpeg and VLC ignore the attribute,
so this is a go2rtc-convention problem, not a malformed SDP.

## Decisions

| # | Choice |
|---|---|
| D1 | **Omit** the direction attribute; do not emit `recvonly` instead |
| D2 | Fix **both** SDP generators, not just the one the report named |
| D3 | One regression test per generator, asserting *no* direction attribute |
| D4 | Deploy to `.198` and verify on the wire, not by inspection |

Rejected: an `sdp_direction` config knob (config for a value that never changes);
emitting `a=recvonly` (same diff size, semantically backwards, works only against
go2rtc's convention); bypassing go2rtc in Frigate's config (a workaround that
leaves the camera wrong for every future consumer, including WebRTC live view).

Ponytail cuts: no compatibility shim, no direction enum/constant, no dual-format
emission, no touching the cosmetic `s=H264 Validation Stream` /
`a=tool:onvif-validation` leftovers.

## Architecture

```
onvif-rust generate_av_sdp()            → live /main, /sub   ─┐
streaming-lib generate_sdp_from_sps_pps() → MockVideoPublisher ┘
       │  (drop "a=sendonly")
       ▼
DESCRIBE 200 + SDP with no direction attribute
       │
       ▼
go2rtc: media.Direction == "" → DirectionRecvonly → SETUP → PLAY
```

RFC 4566 §6 defaults an absent direction to `sendrecv`, which every RTSP client
already treats as "server sends media". ffmpeg, VLC and ONVIF clients do not read
direction off a DESCRIBE response at all.

## Components

| Piece | Change |
|---|---|
| `cross-compile/onvif-rust/src/streaming/helpers.rs:189` | drop `a=sendonly` (video media) |
| `cross-compile/onvif-rust/src/streaming/helpers.rs:205` | drop `a=sendonly` (audio media) |
| `cross-compile/streaming-lib/src/hub/mock_publisher.rs:431` | drop `a=sendonly` |
| streaming-lib SDP **parser** tests | unchanged — parsing what other servers send is still correct |
| Config, HAL, vendor-daemon, WebUI | no change |

Both generators ship in the camera binary: `generate_av_sdp` serves the live
streams (`streaming/service.rs:213-225`, branching on `is_main`), and
`generate_sdp_from_sps_pps` serves `MockVideoPublisher`, which `main.rs:578`
instantiates for H.264-file playback. Patching only the first would leave the
sibling broken.

## Error handling

None to add — the change deletes lines from a string builder. The existing
"Cannot generate SDP: SPS/PPS missing" warn path (`service.rs:238`) is untouched,
and no new failure mode is introduced.

## Testing

**Host:** `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, all
`--target x86_64-unknown-linux-gnu` via the vendored toolchain with its bin dir
prefixed onto `PATH` (clippy fails with E0514 otherwise).

New checks:

- `helpers.rs` — call `generate_av_sdp` **with** audio config so both media
  sections are covered; assert none of `sendonly`/`recvonly`/`sendrecv`/`inactive`
  appear. Comment names go2rtc's `pkg/rtsp/helpers.go` rule so the line is not
  helpfully restored later.
- `mock_publisher.rs:1730` — `test_generate_sdp_contains_sendonly` currently
  asserts the bug; invert and rename it.

**On `.198`:** checksum the on-camera binary against the tree copy, back it up to
`.bak` over telnet, deploy via `scripts/deploy_onvif.sh 192.168.2.198 admin admin`
(FTP to `/mnt/anyka_hack/onvif/`, written as both `onvif-rust` and
`onvif-rust.bin`), then `killall onvif-rust.bin` — the anyka-init supervisor
respawns it. Rollback is one `mv` of the `.bak`.

**Success:** tcpdump on `.6` shows `DESCRIBE → SETUP trackID=0 → PLAY` instead of
`DESCRIBE → TEARDOWN`; go2rtc `/api/streams` lists medias under both `salon` and
`salon-detect`; Frigate's log goes quiet on both `codecs not matched` and
`404 Not Found`.

## Notes

`SD_card_contents/anyka_hack/onvif/onvif-rust.bin` is already dirty in the working
tree (8524924 → 8557760 bytes, uncommitted IR-LED work), so the rebuild carries
that work along with this fix. The checksum step confirms that is what `.198` is
already running before anything is overwritten.

## Out of scope

- The cosmetic `s=H264 Validation Stream` / `a=tool:onvif-validation` session
  naming on the live path
- Camera `.121` (blocked on the libakstreamenc version mismatch)
- Frigate-side config changes on `.6`
- WebRTC / go2rtc live view beyond confirming the producer connects
