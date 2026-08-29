# Event Audio WebUI Trigger — Design

Date: 2026-08-26
Status: approved (design)
Branch: `feat/event-audio-playback`
Parent: `docs/plans/2026-08-26-event-audio-playback-design.md` (Phases 1–2 shipped)

## Problem

Phases 1–2 play clips on lifecycle events (`boot_ready`, `network_lost` /
`network_up`, `upgrade_result`) through `SoundPlayer` → `CMD_AUDIO_PLAY`.
Operators still cannot trigger a chime from the WebUI to verify the speaker or
audition a mapped event without restarting or flapping the link.

## Goals

1. Administrator can list configured sound events and play one from Diagnostics.
2. Playback uses the same `SoundPlayer` policy path as lifecycle events
   (debounce, volume, clip map, busy-drop) — no parallel play API.
3. UI reflects live `[sound]` config (`enabled` + event→clip map).

## Non-goals

- Clip upload, per-event volume, editing the event→clip map in the UI
- Two-way audio / talkback
- User-level (non-admin) access
- ONVIF SOAP verb for play

## Decisions (brainstorming)

| Topic | Choice |
| --- | --- |
| What to play | Event picker (configured events only) |
| Where | Diagnostics page |
| Who | Administrator only |
| Event list source | `GET` from device (`[sound.events]`) |
| Transport | Thin REST (`/api/sound`), not SOAP or diagnostics-actions bag |

## Architecture

```text
WebUI Diagnostics card
  │  GET  /api/sound
  │  POST /api/sound/play { event }
  ▼
onvif-rust diagnostics HTTP (Admin auth)
  ▼
SharedSoundPlayer::play(event)   ← same path as boot/network/upgrade
  ▼
CMD_AUDIO_PLAY → vendor-daemon AO worker
```

At Anyka platform bring-up (where `build_shared_player` already runs), keep an
`Arc` of `SharedSoundPlayer` reachable by the HTTP layer via the same
`Extension` pattern used for `/api/network` and `/api/update`.

### API

**`GET /api/sound`** (Administrator)

```json
{
  "enabled": true,
  "events": [
    { "id": "boot_ready", "clip": "boot.raw" },
    { "id": "network_lost", "clip": "alert.raw" }
  ]
}
```

- `events` sorted by `id`
- Empty array when nothing is mapped

**`POST /api/sound/play`** (Administrator)

Request: `{ "event": "boot_ready" }`

| Status | Meaning |
| --- | --- |
| `200` | Accepted (async play started) |
| `409` | Busy (another clip playing; dropped) |
| `404` | Unknown event, or `[sound] enabled=false` |
| `503` | IPC / hardware failure |
| `401`/`403` | Auth failure / insufficient level |

No debounce bypass. No raw path parameter.

## WebUI

- New card on Diagnostics, below firmware update, above system logs.
- Event `Select` + **Play** button + short helper text.
- If `enabled === false` or `events.length === 0`: card visible, controls
  disabled, muted explanation.
- `useMutation` + sonner toasts (success / busy / error).
- `data-testid`: `sound-card`, `sound-event-select`, `sound-play-button`.
- `soundService.ts` via `authorizedFetch` (same as network/snmp).

## Testing

- **Rust:** handler unit tests with fake sink / injectable player; auth level
  checks (User denied, Admin allowed).
- **WebUI:** mock `soundService`; render, disabled state, success/busy toasts;
  `data-testid` selectors only.
- **Device (optional smoke on `.198`):** Admin → Diagnostics → `boot_ready` →
  Play → audible chime + `event=sound_played` in vendor-daemon log.

## Risks

1. **Player lifetime** — player is created inside Anyka platform init; must be
   shared into the router without creating a second IPC client or skipping
   `clip_dir` resolution.
2. **Stub / non-Anyka builds** — host and stub platforms have no AO; GET can
   return `enabled: false, events: []`, POST returns `404`/`503` without panic.
3. **Debounce** — rapid Play clicks within `debounce_secs` are silent no-ops at
   the player (treat as success with no second chime, or surface a soft toast;
   implementation plan picks one and tests it).
