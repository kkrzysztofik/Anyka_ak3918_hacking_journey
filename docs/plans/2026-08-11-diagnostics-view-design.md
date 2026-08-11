# Diagnostics View — Design

Date: 2026-08-11
Status: Approved, pending implementation plan

## Problem

`cross-compile/www/src/pages/DiagnosticsPage.tsx` (632 lines) is fully designed and
entirely fake. Every value is a literal or `Math.random()`. Several of those values
describe hardware that does not exist on an AK3918.

Meanwhile `App::health()` (`onvif-rust/src/app.rs:1662`) already computes real health —
uptime, per-component state, stream liveness — and is never exposed over HTTP. It is
called only from tests.

## Goals

Serve two audiences from one page:

1. **Operator** — "is my camera OK right now?"
2. **Developer** — "why did it break?"

Hard constraint: the AK3918 is a single-core ARM926EJ-S at 199 BogoMIPS with 35 MB of
RAM. Diagnostics must not compete with the encoder.

## What the hardware can actually provide

Probed on `.198`, 2026-08-11:

| Mock card showed | Reality |
| --- | --- |
| Temperature 64°C | **No sensor.** `/sys/class/thermal` does not exist. |
| Memory 69%, 1.4 GB / 2.0 GB | 35 MB total, ~11 MB used excluding cache |
| CPU 51% | Computable from `/proc/stat` deltas only |
| Network up/download | `/proc/net/dev` wlan0 counters |
| Storage 85% (42.5/50 GB) | SD at `/mnt`, 29.7 G, 3% used |
| Uptime 12d 5h | `/proc/uptime`, currently 4.9 days |
| System logs | Real files in `/mnt/logs/` |

Two kernel-age gotchas:

- **No `MemAvailable`** (added in Linux 3.14). The honest used figure must be assembled
  as `MemTotal - MemFree - Buffers - Cached`. A naive `MemFree/MemTotal` gauge reads 92%
  full at idle and would cry wolf permanently.
- `/proc/stat` counters are cumulative, so they yield the since-boot average (~17.5%),
  not current load. Live CPU% requires two samples and a delta, which means holding
  state between polls.

## Transport: JSON, sampled on demand

Rejected protobuf. At ~350 bytes every 5 s the wire format is not the bottleneck:

| Step | Rough cost |
| --- | --- |
| Read 4 `/proc` files | hundreds of µs (the kernel generates these on read) |
| Serialize ~15 scalars to JSON | tens of µs |
| One `.await` yield | **~12 ms** |

Serialization is ~0.4% of a single scheduler quantum. Protobuf might halve that while
adding a 40–100 KB JS runtime that this camera has to serve over wifi on every page
load — spending device bandwidth to save device CPU that was never scarce. It also
costs `curl`-ability, which on this project is the primary diagnostic path.

Rejected SOAP vendor ops: XML build/parse on every poll, plus types, ops, dispatch
entries and client `soapBodies` — the most code for the least benefit.

**The real lever for not overloading the core is not sampling when nobody is watching.**
TanStack Query pauses `refetchInterval` on a hidden tab by default, so a backgrounded
tab costs the camera exactly zero. No server-side background sampler.

## Backend

Two routes on the existing axum router, ahead of the static fallback:

```
GET /api/diagnostics
GET /api/logs?source=<enum>&level=<lvl>&lines=<n>
```

Auth reuses `verify_basic_auth_self` (`onvif/dispatcher/auth.rs:23`), widened from
`pub(super)` to `pub(crate)`, behind a thin middleware. Reuse rather than
re-implementation: a second credential path is how auth bypasses are born.

The global rate-limit and memory-check layers already wrap the whole router, so `/api`
inherits both. Budget: `rate_limit_per_minute = 300`; this page uses ~24/min at a 5 s
poll.

### Sampler — `src/diagnostics/mod.rs`

One `sample()` reading `/proc/uptime`, `/proc/stat`, `/proc/meminfo`, `/proc/net/dev`,
and `statvfs("/mnt")`.

Delta state is a single `Arc<Mutex<Option<Sample>>>` on `OnvifServerState` holding the
previous raw sample and its `Instant`. CPU% and throughput are computed against it. The
first poll after idle returns `null` for those two; the UI shows `—` until the second.
`std::sync::Mutex`, never held across an `.await`.

Composed with data that already exists:

- `App::health()` — status, components, degraded list, process uptime
- `stream_frame_age_ms()` (`platform/anyka/video_encoder.rs:1283`) — stall detector
- `StreamState { frame_count, total_bytes, iframe_count }`
  (`platform/anyka/video_encoder.rs:295`) — already incremented per frame for main and
  sub. FPS and bitrate come from deltas of these, so the encoder callback is untouched.
  Reading an existing `AtomicU64` is free; adding accounting to a path running 25×/sec
  would not be.

### Payload

```json
{
  "status": "healthy",
  "uptime": { "process_s": 12345, "system_s": 421240 },
  "cpu_percent": 17.4,
  "memory": { "total_kb": 36540, "used_kb": 11200, "process_rss_kb": 4200 },
  "storage": { "total_kb": 31138000, "used_kb": 936000 },
  "network": { "rx_bps": 12000, "tx_bps": 480000 },
  "streams": [{ "name": "main", "fps": 25.0, "bitrate_bps": 2100000, "frame_age_ms": 38 }],
  "components": [{ "name": "Stream Health", "status": "healthy", "message": null }]
}
```

### Log endpoint safety

- `source` is an **enum mapped to fixed paths**, never a user-supplied path. No
  traversal surface. Sources: `onvif_rust`, `vendor_daemon`, `anyka_init`,
  `wpa_supplicant`.
- Tail by seeking to end-minus-64 KB. `onvif_rust.log` is already 510 KB; it is never
  read whole.
- Level filtering server-side, to keep the payload small.
- Logs require Administrator (they carry IPs and usernames). Metrics require User.

## Frontend

- `services/diagnosticsService.ts` — plain `fetch`, not `soapRequest`, reusing the auth
  header getter from `api.ts`.
- `hooks/useDiagnostics.ts` — `useQuery` with `refetchInterval: 5000`.
  `refetchIntervalInBackground` stays at its default `false`.

`DiagnosticsPage.tsx` rewrite:

- Delete `generateData` / `generateNetworkData`.
- **Temperature card → Storage card.** No sensor exists; that card can never be honest.
- Stat cards: Status / CPU / Memory / Storage.
- Uptime shows process *and* system, flagging "restarted 2h ago" when they diverge.
  This is the highest-value debug field on the page and costs one subtraction: when
  process uptime is far below system uptime, the supervisor restarted onvif-rust. That
  is exactly the signature of the .121 dusk crash and the vendor-daemon restart pairing,
  and it turns "the camera feels flaky" into "it restarted 4 times today".
- Charts keep the existing `Sparkline`, fed by a client-side ring buffer capped at ~60
  samples (5 min). Empty on open, fills as you watch.
- "System Metrics" fiction → real per-stream FPS / bitrate / frame age.
- Device Info → existing `deviceService.getDeviceInformation()`. Already implemented,
  not duplicated into the JSON payload.
- Log table → source dropdown; the level filter buttons and Export button become real
  (Export = `Blob` download of the loaded lines).

## Testing

- Rust: sampling parsers take `&str`, not paths, so `/proc` formats are testable
  off-device with fixture strings. Plus delta math and an allowlist-rejection test.
- TS: existing `renderWithProviders` + mocked service, per `anyka-webui-testing`.

## Deliberately not building

- Server-side history ring buffer — client accumulation is enough and costs the device
  nothing.
- Temperature — no sensor.
- Dropped-frame counters — would need new hot-path accounting.
- Log streaming / SSE — polling is sufficient.
