# WebUI Process Control — Design

Date: 2026-09-19
Status: approved, not yet implemented

## Goal

Extend the WebUI Diagnostics page to show what is running on the camera and let
an administrator restart a service from the browser instead of over telnet.

Two lists, one page:

- **Supervised services** — the `[services.*]` entries `anyka-init` actually
  manages, with state, pid, uptime and restart count. This is the actionable set.
- **All processes** — a raw `/proc` table for debugging (a wedged D-state task,
  an RSS leak), collapsed by default.

Every supervised service is restartable, including `onvif`.

## What already exists

Establishing this first, because most of the feature turns out to be plumbing
rather than new logic.

`anyka-init` already owns the restart machinery:

- `supervisor_loop::Msg::RestartService(String)` SIGTERMs a service; the normal
  `handle_service_exited` path then restarts it under the existing backoff and
  storm-guard policy. `Msg::KillService(String)` escalates to SIGKILL.
- Both are sent today only by the watchdog in `monitor.rs`.
- `supervise.rs::decide` is pure and already exhaustively tested.

`onvif-rust` already owns the diagnostics surface:

- `diagnostics/proc.rs` parses `/proc/stat`, `/proc/meminfo`, `/proc/net/dev`,
  `/proc/uptime`.
- `diagnostics/state.rs:302` collects blocking reads off the async executor.
- `GET /api/diagnostics` and `GET /api/logs` are mounted in
  `onvif/server.rs:594` (`build_router`), behind auth and rate-limit layers.

The gap is that **there is no external control channel into `anyka-init`**. The
only cross-process IPC today is the update spool (`spool/bundle.tar` +
`spool/bundle.trigger`), polled on its own thread from `main.rs:158`.

### The constraint that picked the design

The supervisor loop is event-driven, not polling:

```rust
loop {
    let next_deadline = tick_services(...);
    let timeout = next_deadline...unwrap_or(Duration::from_secs(3600));
    dispatch_msg(..., rx.recv_timeout(timeout));
}
```

With nothing scheduled it blocks for up to an hour. So a file-drop IPC cannot
rely on the loop noticing a dropped file — it needs its own poller thread.

That removes the file-drop's apparent laziness: you pay for a thread either
way. And any thread holding a `Sender<Msg>` clone gets instant wake-up for
free, because sending on the channel *is* the wake mechanism.

### Rejected alternatives

- **Kill-by-pid, zero `anyka-init` changes.** `onvif-rust` lists `/proc` and
  SIGTERMs the pid; the supervisor's existing exit handler restarts it. Shortest
  possible diff, but it loses all supervisor state (restart counts, backoff,
  "enabled but currently stopped") and forces `onvif-rust` to parse `anyka.toml`
  to map pids to service names. The supervised card is the point of the feature
  and this cannot render it honestly.
- **Status file + trigger file**, mirroring the update spool. Familiar and
  `cat`/`touch`-debuggable over telnet, but needs the same new thread as the
  socket, plus poll latency, minus acknowledgements. Same cost, less capability.

## Design

### 1. `anyka-init` control channel

New `anyka-init/src/control.rs`: one thread spawned from `main.rs` alongside the
existing optional threads, holding a `Sender<Msg>` clone. Blocking accept on a
`UnixListener`, one request per connection, line protocol:

```
"status\n"          -> one TSV line per service, then a blank line:
                       <name>\t<state>\t<pid>\t<uptime_s>\t<restarts>\t<retry_in_s>\n
"restart <name>\n"  -> "ok\n" | "unknown\n"
```

**Not JSON.** `anyka-init` has no `serde_json` and this does not justify adding
one: the payload is a flat list of six scalars per row between two processes we
own on both ends. TSV needs no escaping decision, is ~20 lines to write, and is
readable straight off `nc`/`socat` over telnet. `onvif-rust` converts to JSON
for the browser, where `serde_json` already exists.

Changes to `supervisor_loop.rs`:

- New `Msg::QueryStatus(Sender<Vec<ServiceStatus>>)` — request/reply over the
  existing channel. `dispatch_msg` builds a snapshot from `services` and replies
  on the caller's one-shot. The caller blocks with a bounded `recv_timeout`, so a
  wedged loop surfaces as an error rather than hanging an HTTP handler.
- `Msg::RestartService(String)` is reused **unchanged**.

`ServiceStatus` is a plain owned struct derived from `SvcState`:

- `Running { pid, since }` → `{state:"running", pid, uptime_s}`
- `Backoff { until, attempt }` → `{state:"backoff", retry_in_s, attempt}`
- `restarts` from `hist.len()`

`Instant` is converted to seconds at snapshot time so nothing non-serializable
crosses the socket.

Socket path: **hardcoded** `/tmp/anyka-init.sock`, mode `0600`, unlinked before
bind so a stale socket from an unclean exit does not block bind permanently.

No config key, deliberately. `Config` carries `#[serde(deny_unknown_fields)]`
(`config.rs:29`), so an `anyka.toml` containing a `[control]` stanza is a hard
parse error for any older `anyka-init` binary — which is exactly what sits in
the other A/B slot after a rollback. A cosmetic tunable is not worth converting
a rollback into a config failure, and bundles do not carry `anyka.toml` anyway,
so the key would have to be appended by hand on every camera. If the path ever
needs to move, add the key then, behind a `schema` bump.

Notes:

- `QueryStatus` carrying its own reply `Sender` is the std-only equivalent of a
  oneshot. There is no `tokio` in `anyka-init`, and the pattern fits because the
  loop is already a message pump. A shared `Arc<Mutex<Vec<ServiceStatus>>>` would
  force the loop to write on every tick even when nobody is reading.
- The reply timeout matters: `tick_services` can block on spawn syscalls, and the
  handler runs inside an HTTP request on a single-core 36 MB box. A bounded wait
  turns "supervisor is wedged" into a visible 503 instead of a hung page — which
  is precisely the failure this feature exists to diagnose.
- Bundles do not carry `anyka.toml`, so a new `[control]` stanza will not exist
  on already-deployed cameras. The default must be the working value, never a
  required key.

### 2. `onvif-rust` endpoints

Two new modules under `onvif-rust/src/diagnostics/`:

- `control.rs` — socket client. Connects, writes one line, reads the reply, with
  connect and read timeouts. Returns `None` on a refused or missing socket
  rather than an error.
- `processes.rs` — one walk of `/proc/[pid]/stat` and `/proc/[pid]/status`,
  yielding `{pid, ppid, comm, state, rss_kb, cpu_time_s}`.

Two routes, registered in the existing `if let Some(diagnostics)` block:

| Route | Auth | Responses |
|---|---|---|
| `GET /api/processes` | authenticated | `{ supervised: ServiceStatus[] \| null, processes: Process[] }` |
| `POST /api/services/{name}/restart` | **Administrator** | `202`, `404` unknown, `503` supervisor unreachable |

Admin-gating restart but not the listing matches the existing split: reading
diagnostics is any authenticated user, changing device state (`PUT /api/update`)
is Administrator. A restart is a state change.

`supervised: null` covers an older `anyka-init` in the other A/B slot, or a
control thread that failed to bind. The raw table still renders and the frontend
degrades to a note. This mirrors `ptz` and `wifi`, already optional in the
`Diagnostics` payload.

Decisions:

- **No per-process CPU percentage.** That needs two samples, per-pid delta
  bookkeeping, pid-reuse handling, and a decision about the first request.
  Cumulative `utime+stime` needs one sample and answers the real question ("has
  this been burning CPU since boot?").
- **One `spawn_blocking` for the whole walk**, not one per file. Each yielding
  `await` costs roughly a scheduler quantum (~12 ms) on this camera; per-file
  awaits across ~50 processes would turn a few-millisecond walk into most of a
  second.
- **Not folded into `/api/diagnostics`.** That payload is polled continuously,
  and ~50 processes × 2 reads would add ~100 file reads to every poll on a
  single-core box that is also encoding video.

Known race: for the `onvif` target the handler returns `202` and *then* takes the
SIGTERM, so the response may not flush. The frontend treats a dropped connection
on that request as success-in-progress.

### 3. Frontend

New files: `src/services/processesService.ts` (+ test) and
`src/components/ProcessesCard.tsx` (+ test). `DiagnosticsPage.tsx` gains one line.

`processesService.ts` follows `diagnosticsService.ts` exactly — `authorizedFetch`,
`ApiError`, hand-written type guards (this codebase validates runtime shapes by
hand; no schema library) — plus `restartService(name)`.

`ProcessesCard.tsx` renders both lists from one
`useQuery(['processes'], …, { refetchInterval: 10_000 })`:

- **Supervised services**: name, state badge (running green / backoff yellow),
  pid, uptime, restart count, Restart button. When `supervised` is `null` the
  rows become a muted "supervisor control unavailable" note and the buttons go.
- **All processes**: a `Collapsible`, collapsed by default, sorted by RSS
  descending. Fixed sort, no sortable columns.

Every restart goes through one `AlertDialog` naming the service. The `onvif`
entry carries extra warning text: restarting it also stops `vendor-daemon`, so
video and the page itself drop with it.

The `onvif` path: POST, treat a dropped connection as expected, switch the card
to "reconnecting…", await the shared wait helper, then invalidate `['processes']`.

Two reuse extractions instead of new code:

- `pollUntilBack` (`FirmwareUpgradeDialog.tsx:131`) is already a down→up edge
  detector for exactly this problem. Extract the loop to
  `src/lib/waitForCameraBack.ts`, leaving the firmware-version comparison in the
  dialog. Both callers share it rather than there being two polling loops.
- `formatDuration` moves from `DiagnosticsPage.tsx` to `src/lib/utils.ts`.

One endpoint feeding both tables keeps polling at one request per 10 s; splitting
the queries would double requests to save nothing, since the backend walks
`/proc` either way.

Sorting by RSS rather than offering sortable columns is a deliberate ceiling: on
a 36 MB device memory is the axis that catches problems, and a sortable table is
a component you then own forever.

## Testing

- `anyka-init`: `ServiceStatus` conversion from `SvcState` (pure, table-driven);
  `dispatch_msg` handling of `QueryStatus` including the unknown-service and
  disconnected-reply cases; control-thread line-protocol parsing.
- `onvif-rust`: `/proc` text parsing against fixtures; the socket client's
  missing-socket path returning `None`; route auth (non-admin restart → 403,
  unauthenticated → 401), matching the existing `/api/update` tests.
- WebUI: `processesService` type guards and error paths; `ProcessesCard`
  rendering for running / backoff / `supervised: null`; the confirm dialog; the
  `onvif` reconnect flow. Per `anyka-webui-testing` — `data-testid`,
  `renderWithProviders`, `vi.mock` of the service module.

## Out of scope

- Start/stop as separate actions. Restart is what the failure modes call for; a
  stopped service the supervisor immediately restarts is a confusing button.
- Editing `[services.*]` from the WebUI.
- Per-process CPU percentage (see above).
- Killing arbitrary non-supervised pids from the raw table. Listing is for
  diagnosis; the box runs vendor processes that must not be killed casually.
