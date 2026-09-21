# Profile management stops at "Coming Soon"

Date: 2026-09-21
Status: design approved, implementation not started

Branch: `design/profile-management`, worktree `.worktrees/profile-management`,
based on `origin/main` at 32b5cea4.

## Problem

`www/src/pages/settings/ProfilesPage.tsx` renders a complete-looking Profiles
tab: a card per media profile, six configuration tiles per card, Create and
Delete. Most of it cannot be acted on.

Every tile that is not the video encoder renders a disabled button labelled
"Add (Coming Soon)". The Create Profile button works, and produces a profile
that cannot stream.

Three independent defects hide under one symptom.

## What is actually wired

Worth stating, because as with the Time tab the backend is in better shape
than the page suggests.

- `onvif-rust` dispatches `AddVideoSourceConfiguration`,
  `AddVideoEncoderConfiguration`, `AddAudioSourceConfiguration`,
  `AddAudioEncoderConfiguration`, all four `Remove*` twins, and all four
  `GetCompatible*` queries.
- `ProfileManager` has the matching eight `add_*`/`remove_*` methods at
  `profile_manager.rs:837-978` and the `get_compatible_*` queries at `:1480`.
- `config/profiles/mod.rs` persists profiles, both source kinds, both encoder
  kinds and their attachments to `profiles.toml`.
- The PTZ service already answers `GetConfigurations` and
  `GetCompatibleConfigurations`, which is what a PTZ picker needs.

## The gaps

| # | Gap | Location |
|---|-----|----------|
| 1 | Six tiles pass no `onEdit`; every button renders disabled as "Add (Coming Soon)" | `ProfilesPage.tsx:493` |
| 2 | No `add*`/`remove*`/`getCompatible*` calls exist in the service layer at all | `profileService.ts` |
| 3 | No audio encoder editor, though `GetAudioEncoderConfigurationOptions` is dispatched | `ProfilesPage.tsx` |
| 4 | `AddPTZConfiguration`/`RemovePTZConfiguration` are serde types that nothing dispatches — dead code | `types/media.rs:1181-1208` |
| 5 | `GetMetadataConfigurations` returns `vec![]` | `media/service.rs:668` |
| 6 | `SetMetadataConfiguration` unconditionally faults `ter:NoConfig` | `media/service.rs:679` |
| 7 | `metadata_configuration` hardcoded `None` on every profile built | `profile_manager.rs:525,1459` |
| 8 | `StoredProfile` has `ptz_config` but no `metadata_config`, so an attachment cannot persist | `config/profiles/mod.rs:61` |
| 9 | RTSP channel derived by substring-matching the **profile token**; anything else yields the dead path `/stream` | `ops/streaming.rs:64` |

Gap 9 is the one that makes the already-shipped Create Profile button a trap:
it produces profiles whose `GetStreamUri` hands out a URI no RTSP path serves.
The code carries a TODO saying exactly this.

## Constraints discovered

- `MAX_PROFILES = 16` (`media/types.rs:37`), but ONVIF Media1 has no
  `CreateVideoEncoderConfiguration` — the encoder set is device-fixed at two.
  The real model is *16 profiles, each attaching one of 2 encoders*, which is
  still useful: a main-stream profile with audio and one without.
- Neither `StoredProfile` nor `ProfilesFile` uses `deny_unknown_fields`, so
  adding `metadata_config` is rollback-safe against the older binary in the
  other A/B slot. The attachment is lost on rollback; the camera still boots.
  Contrast `anyka.toml`, where a new section is a hard parse error.
- There is exactly one PTZ configuration
  (`create_default_ptz_configuration`), gated on `ptz.enabled`.

## Scope

In: the six tiles, an audio encoder editor, PTZ and Metadata as first-class
profile configurations, and the routing fix.

Out, decided explicitly:

- **VideoAnalytics.** Not a stub on this hardware — the SDK ships a full
  motion-detection API (`ak_md_init`, `ak_md_enable`,
  `ak_md_set_area_sensitivity`, `ak_md_get_result`), the stock
  `orig/usr/bin/anyka_ipc` links it statically, and the static lib is vendored
  at `anyka_reference/IOT-ANYKA-PTZdaemon/libs/libmpi_md.a`. A real
  `tt:CellMotionEngine` is reachable. It is deferred because it needs a live
  MD pipeline in the vendor daemon plus metadata RTP plus event wiring, and
  because a vendored SDK artifact is not the shipped lib — the same assumption
  cost us the encoder on `.121` and the audio output lib on Cloud39EV2. Treat
  "`libmpi_md.a` links and returns real results on our board" as a hardware
  spike, not a given. `analytics/service.rs:46` keeps honestly advertising
  `analytics_module_support: false`.
- **AudioOutput / AudioDecoder.** The speaker is confirmed working, but there
  is no ONVIF backchannel HAL; a third distinct subsystem.
- **New encoder configurations.** No such ONVIF operation exists.

## Architecture

### Backend

Everything follows the shape already established for the four working
families, so there is no new pattern to invent.

- `ProfileManager` gains `add_ptz_configuration`, `remove_ptz_configuration`,
  `get_compatible_ptz_configurations` and the same trio for metadata,
  alongside the existing eight.
- `ops/profiles.rs` gains six thin handlers; `media/service.rs` gains six
  dispatch arms plus `GetMetadataConfiguration` (singular),
  `GetMetadataConfigurationOptions`, and a `SetMetadataConfiguration` that
  stops faulting.
- PTZ needs no candidate store. `get_compatible_ptz_configurations` returns a
  one-element vec when `ptz.enabled` is true and an empty one when it is
  false — which is also the correct answer for the picker.
- Metadata needs a real store: a default `MetadataConfiguration` (PTZStatus,
  Analytics flag, SessionTimeout), a `metadata_configs: Vec<StoredMetadataConfig>`
  in `ProfilesFile`, and `metadata_config: Option<String>` on `StoredProfile`.

### Routing

`get_stream_path` takes the profile's attached `VideoEncoderConfiguration`
token instead of the profile token, maps that to a channel, and returns an
`OnvifError` when the profile has no encoder or the token is unknown.
`get_stream_uri` already loads the profile at `ops/streaming.rs:40` and
discards it, so the encoder token is free.

Faulting is deliberate: a client learns the profile is unstreamable instead of
receiving a URI that quietly serves nothing.

### Frontend

`src/pages/settings/profiles/` — `ProfilesPage.tsx`, `ConfigSection.tsx`,
`ConfigPickerDialog.tsx`, `VideoEncoderDialog.tsx`, `AudioEncoderDialog.tsx`.
The current single file is 780 lines and six pickers plus a dialog would
roughly double it.

`ConfigPickerDialog` is one generic component parameterised by a fetch
function and an attach function. All six pickers are the same radio list over
`{token, name}`; six near-identical dialogs would be the thing worth deleting
later.

`profileService.ts` gains twelve `add*`/`remove*` calls, six `getCompatible*`
calls, and the audio-encoder get/set/options trio.

## Data flow

Attach is a single round trip plus an invalidation, matching how
`setVideoEncoderConfiguration` already behaves:

```
tile [Add] → ConfigPickerDialog opens
           → getCompatibleX(profileToken)    → radio list
  [Attach] → addXConfiguration(profile, cfg) → invalidate ['profiles']
           → getProfiles() re-renders the card
```

No optimistic updates and no client-side diff. The device is the source of
truth, a refetch is one small SOAP call, and partial-failure handling across
N calls is the complexity the per-tile design buys us out of.

## Error handling

Device faults surface as they already do: `soapRequest` throws, the mutation's
`onError` raises a `toast.error` carrying the fault text. Three cases get
explicit treatment.

- **PTZ disabled in config.** `getCompatiblePTZConfigurations` returns empty;
  the picker shows "No compatible configurations" rather than an empty radio
  group above a live Attach button.
- **Removing the video encoder from a streaming profile.** The device accepts
  it, as ONVIF requires, and the profile then correctly faults on
  `GetStreamUri`. The Remove confirmation says so.
- **Fixed profiles.** `fixed: true` already hides Delete. Per ONVIF, fixed
  profiles still accept Add/Remove of configurations, so the tiles stay live.

## Testing

Rust: a unit test per new `ProfileManager` method, following the existing test
module in `ops/profiles.rs`, plus a round-trip test that a `metadata_config`
written to `profiles.toml` reloads.

For routing the test that matters is that a profile with no encoder faults and
one holding the sub encoder returns `/sub`. That is the regression that would
let dead `/stream` URIs back in.

WebUI: Vitest and React Testing Library via the project's `renderWithProviders`
helper. One test per picker asserting the right service function fires with the
right tokens, plus mutation tests for the audio encoder dialog. The pickers use
radio inputs rather than Radix Select specifically so jsdom can read the
selection back — a Radix Select value set after mount reads as `undefined`
under jsdom and fails zod validation in vitest only.

## Known ceiling

An attached metadata configuration round-trips and satisfies conformance
checks and NVR discovery, but **no metadata RTP track is emitted**. An NVR that
attaches it and waits for PTZ-status metadata gets silence. This will carry a
`ponytail:` comment naming the ceiling and the upgrade path.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
