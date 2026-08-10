# Image Flip (180° Rotate) — Design

Date: 2026-08-07
Status: implemented 2026-08-08 (on-device smoke test still pending)

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

3. **Platform layer — live apply, mirroring `ImagingControl`.** The codebase
   already has a proven shape for "ONVIF-exposed setting reaches hardware
   immediately": `Platform::imaging_control() -> Option<Arc<dyn
   ImagingControl>>`, whose `set_settings()` applies to hardware first, then
   the caller persists (`app.rs:708-734`, `onvif/imaging/store.rs:302-337`).
   Reusing that shape is less new code than a bespoke boot-only path, since
   the boot-only version has no existing plumbing to build on.
   - New trait `VideoControl` (`platform/common/traits.rs`, next to
     `ImagingControl`): `async fn set_flip_mirror(&self, rotated: bool) ->
     PlatformResult<()>`. Added to `Platform` as `fn video_control(&self) ->
     Option<Arc<dyn VideoControl>>`, with a `None` default in the same macro
     that already defaults `imaging_control`.
   - `AnykaVideoInput` (`platform/anyka/video_input.rs`) implements
     `VideoControl` directly — no separate wrapper struct needed, unlike
     `AnykaImagingControl`, because the VI handle it already owns is exactly
     what flip/mirror needs. `set_flip_mirror()`: stores `rotated` in a new
     `AtomicBool` field, and if the VI is currently open
     (`self.opened.load()`), also calls the new IPC wrapper immediately.
     Storing the flag even when closed is what makes reapply-on-reattach
     (below) work without a second code path.
   - `AnykaPlatform::video_control()` returns
     `Some(self.video_input.clone() as Arc<dyn VideoControl>)`.
   - `init_video_input()` (`platform/anyka/mod.rs:307-357`) — the sequence
     the supervisor runs on *every* attach, cold boot or post-crash
     reattach alike — gets a new step right after `capture_on()` succeeds
     (matching the vendor demo's ordering): call
     `self.video_input.set_flip_mirror(self.video_input.rotated())` using
     the stored flag. This is what makes a vendor-daemon crash-and-respawn
     reapply the setting without any extra wiring — the existing retry path
     already re-runs this function.

4. **Persistence and boot seed — single source of truth.** `profiles.toml`
   is the *only* place `rotated` is stored; there is no separate
   `config.toml` seed (see "Rejected alternatives" for why a second layer
   was rejected).
   - `StoredVideoSourceConfig` in `config/profiles/mod.rs` gets a `rotated:
     bool` field (`#[serde(default)]`), wired through
     `video_source_config_to_stored` / `stored_to_video_source_config` in
     `profile_manager.rs`.
   - `Application::run()` (`app.rs`) currently calls `wire_profile_persistence`
     *after* `init_platform` (line 1011 vs 982); this ordering flips so the
     profile store is loaded first. The boot value is read straight out of
     it: `profile_storage.snapshot().video_source_configs.first().map(|c|
     c.rotated).unwrap_or(false)`, passed as a new parameter through
     `init_platform` into `AnykaPlatform::with_isp_config(...)`, which seeds
     `AnykaVideoInput`'s `AtomicBool`. `init_platform` has no dependency on
     `wire_profile_persistence` or vice versa, so the reorder is safe.

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

Setting the toggle (WebUI or ONVIF client) applies to the running video
pipeline immediately (if the VI is open) and persists to `profiles.toml` —
same order as `ImagingControl::set_settings`: platform first, then cache/
persist. No restart needed. The same stored flag is also what gets re-applied
every time the supervisor's attach sequence runs, which covers a
vendor-daemon crash-and-respawn without extra code (see Architecture, step 3).

## Error handling

- vendor-daemon: unknown/stale VI handle → `VD_STATUS_STALE_EPOCH`, the
  existing convention for every other VI command.
- onvif-rust: an invalid `Mode` in `SetVideoSourceConfiguration` is a SOAP
  fault (`InvalidArgVal`), consistent with other Set* validation in this
  service. A live-apply failure from `VideoControl::set_flip_mirror` also
  propagates as a fault, matching `ImagingControl::set_settings`'s `?` —
  intentionally not soft, for consistency with the pattern being mirrored.
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

- **Restart-to-apply instead of live apply.** The original ponytail pass cut
  live-apply as a new abstraction built for one caller. Reversed after
  tracing the actual boot sequence: the codebase already has this exact
  abstraction (`ImagingControl`/`ImagingSettingsStore`), so reusing it is
  *less* code than the bespoke restart-only path, not more — the restart-only
  version would have needed its own boot-time config-threading mechanism
  that doesn't exist anywhere else for this kind of setting.
- **Seeding the boot value from `config.toml` (mirroring `imaging_cfg`
  exactly).** `ImagingControl`'s actual pattern has two persistence layers:
  `config.toml`'s `[imaging]` section seeds the platform at construction,
  while `imaging.toml` (`ImagingSettingsStore`) holds live ONVIF-set values
  in a separate file that is never pushed back to hardware on load. Copying
  that split for Rotate would mean `GetVideoSourceConfiguration` could report
  a `rotated` value that doesn't match actual hardware state after a plain
  `onvif-rust` restart (profiles.toml says one thing, config.toml — which
  boot actually reads — says another). Since Rotate has no hardware readback
  (see next bullet), nothing would catch that drift. Fixed by making
  `profiles.toml` the only source of truth: `app.rs` loads the profile store
  before constructing the platform and reads the boot value straight from
  it, instead of from `config.toml`. No new config section.
- **`CMD_VI_GET_FLIP_MIRROR`.** Originally proposed so `GetVideoSourceConfiguration`
  could confirm hardware state. Cut: nothing else in that response reads
  live hardware — it's all persisted profile state — so Rotate doesn't need
  to be the exception.
- **`RotateMode::Auto`.** The ONVIF schema allows it, but the hardware can
  never honor or report it. Modeling it would mean a permanent dead enum
  arm that some downstream match always has to reject anyway; rejecting
  `Auto` at the request-validation boundary does the same job with no
  enum variant.
