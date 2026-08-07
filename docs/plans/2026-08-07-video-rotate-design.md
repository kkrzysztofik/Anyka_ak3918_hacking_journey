# Image Flip (180° Rotate) — Design

Date: 2026-08-07
Status: proposed

## Problem

The camera image is upside-down and there is no way to correct it. The
vendor SDK already supports this at the VI layer
(`ak_vi_set_flip_mirror(handle, flip_enable, mirror_enable)`,
`cross-compile/vendor-daemon/include/ak_vi.h`), and the reference demo calls
it with both flags set right after `ak_vi_capture_on()`
(`cross-compile/anyka_reference/venc_demo/ak_venc_demo.c:485`) — nothing in
this repo wires it up.

## Goal

Expose a 180°-rotate control through both the ONVIF Media service
(`VideoSourceConfiguration.Extension.Rotate`, per the ONVIF `tt:Rotate`
schema) and a WebUI toggle, backed by the same persisted setting.

## Non-goals

- Independent horizontal-mirror control. ONVIF's Rotate extension has no
  mirror concept at all, and the immediate need is fixing an upside-down
  mount, not arbitrary mirroring.
- 90°/270° rotation. The vendor VI API only exposes flip/mirror, not a real
  transpose; this hardware cannot do it.
- Live apply. See "Rejected alternatives" — the setting takes effect on the
  next `onvif-rust` restart, not mid-stream.
- Reading rotate state back from hardware. `GetVideoSourceConfiguration`
  already reads every other field (bounds, name, use_count) from persisted
  profile state, not from a live device query; Rotate follows the same
  convention.

## Architecture

Bottom to top:

1. **vendor-daemon (C, IPC layer).** One new command,
   `CMD_VI_SET_FLIP_MIRROR`, added to `protocol.h`. Handler in
   `handlers_vi.c` follows the existing `handle_vi_capture_on` shape:
   resolve the VI handle from the token, call
   `ak_vi_set_flip_mirror(handle, flip, mirror)`. Wire format:
   `[u64 handle][u8 flip][u8 mirror]` request, `STATUS_OK`/error response.
   No get-side command — see non-goals.

2. **onvif-rust HAL (Rust, IPC client).** `video_input_set_flip_mirror()` in
   `hal/common/video.rs`, mirroring `video_input_set_channel_attr`. IPC send
   in `hal/anyka/ipc/video.rs`. No-op stub in `hal/stub/video.rs` for host
   tests.

3. **Platform layer.** `AnykaVideoInput::set_flip_mirror(&self, rotated:
   bool)` in `platform/anyka/video_input.rs`, called once during boot right
   after `capture_on()` succeeds (same order the vendor demo uses), reading
   the persisted setting. No trait method on `Platform`/`VideoInput` — this
   is boot-sequence-only, not reachable from the Media service.

4. **Persistence.** `StoredVideoSourceConfig` in
   `onvif/media/profile_manager.rs` gets a `rotated: bool` field (default
   `false`), saved to `profiles.toml` through the existing persistence path
   — same mechanism bounds/resolution already use.

5. **ONVIF Media service.** `VideoSourceConfiguration`'s currently-untyped
   `extension: Option<Extension>` (`onvif/types/common.rs:496`) becomes a
   typed `VideoSourceConfigurationExtension { rotate: Option<Rotate> }`,
   with `Rotate { mode: RotateMode, degree: Option<i32> }` and `RotateMode`
   restricted to `Off`/`On` (no `Auto` variant — this hardware can never
   report or honor it; a request specifying `Auto` is rejected as invalid
   rather than modeled).
   - `get_video_source_configuration(_options)`: populate `Extension.Rotate`
     from `StoredVideoSourceConfig.rotated`. Options advertise
     `Mode: [OFF, ON]`, no degree list (per spec, omitting `Degree` on `On`
     means 180°, which is the only degree this hardware supports).
   - `set_video_source_configuration`: validate `Mode ∈ {Off, On}`, persist
     `rotated` via `profile_manager` — same as today's bounds/resolution
     handling, nothing more.

6. **WebUI.** A "Flip image 180°" `Switch` on `ImagingPage.tsx`, same
   mutation/toast pattern as the existing IR/WDR controls, calling
   `SetVideoSourceConfiguration` under the hood.

## Applying a change

Setting the toggle (WebUI or ONVIF client) persists to `profiles.toml`
immediately but does not touch the running video pipeline. It takes effect
on the next `onvif-rust` restart, which the `anyka-init` supervisor already
performs automatically (`killall onvif-rust.bin vendor-daemon.bin` via
telnet, as used for the last binary deploy — respawn is near-instant, no
camera reboot required).

## Error handling

- vendor-daemon: unknown/stale VI handle → `VD_STATUS_STALE_EPOCH`, the
  existing convention for every other VI command.
- onvif-rust: an invalid `Mode` in `SetVideoSourceConfiguration` is a SOAP
  fault (`InvalidArgVal`), consistent with other Set* validation in this
  service.
- WebUI: standard toast-on-error via the existing mutation pattern in
  `ImagingPage.tsx`.

## Testing

- onvif-rust: host-side unit tests for the new HAL wrapper (mocked
  `VideoHalTrait`), the persistence path in `set_video_source_configuration`,
  and `Rotate`/`RotateMode` (de)serialization against the WSDL-shaped XML.
- vendor-daemon: no C-level test exists for any `handlers_vi.c` command
  today (`vendor-daemon/tests/` has only `test_ring_epoch.c`); this follows
  the same manual on-device verification as the rest of that file.
- WebUI: Vitest component test for the new switch, per
  `anyka-webui-testing` conventions.

## Rejected alternatives

- **Live apply.** Originally proposed calling into the already-open VI
  handle from `SetVideoSourceConfiguration` via a new `Platform`/`VideoInput`
  trait method, so the image would flip without a restart. Cut: it's a new
  abstraction built for exactly one caller, and `SetVideoSourceConfiguration`
  already only persists for every other field it handles (bounds,
  resolution) — restart-to-apply is free here because the supervisor already
  respawns `onvif-rust.bin` on kill. Revisit if manual restarts prove
  annoying in practice; it's a one-method addition, not a redesign.
- **`CMD_VI_GET_FLIP_MIRROR`.** Originally proposed so `GetVideoSourceConfiguration`
  could confirm hardware state. Cut: nothing else in that response reads
  live hardware — it's all persisted profile state — so Rotate doesn't need
  to be the exception.
- **`RotateMode::Auto`.** The ONVIF schema allows it, but the hardware can
  never honor or report it. Modeling it would mean a permanent dead enum
  arm that some downstream match always has to reject anyway; rejecting
  `Auto` at the request-validation boundary does the same job with no
  enum variant.
