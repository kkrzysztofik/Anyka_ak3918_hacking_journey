# WebUI Service Enable/Disable Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let an administrator enable/disable an anyka-init supervised service from the WebUI Diagnostics page, persisting the choice in `anyka.toml` and keeping the A/B upgrade trial functional.

**Architecture:** The supervisor (`anyka-init`) is the sole actor: two new control-socket requests (`enable`/`disable`) drive a `SvcState::Disabled` lifecycle state, a line-level edit of `anyka.toml` (file first, then in-memory), and an adaptive trial-port filter. `onvif-rust` is a thin socket client behind two authenticated `POST` routes. The WebUI adds Disable/Enable actions to the existing supervised-services table with per-service consequence warnings.

**Tech Stack:** Rust (anyka-init, onvif-rust) — std `unix` sockets, `toml`, `tracing`, tokio/axum on the onvif-rust side; React 19 + TanStack Query + shadcn/ui + Vitest/RTL in the WebUI. Cross-compiled to `armv5te-unknown-linux-uclibceabi` with the vendored toolchain.

**Spec:** `docs/plans/2026-09-19-webui-service-toggle-design.md` (revised 2026-09-19)

## Global Constraints

- **Toolchain:** `source ./setenv.sh` before any Rust command; use `$CARGO` (never bare `cargo`). Host-side commands take `--target x86_64-unknown-linux-gnu`; ARM builds take the default cross target and must run from the crate directory, not the workspace root.
- **No `unwrap()`/`expect()` in production code** (test code may use them). Use `?`, `match`, or `if let`.
- **Logging:** `tracing::{info,warn,error}!` only — never `println!` in production.
- **Test naming:** `test_<subject>_<behavior>` (e.g. `test_set_enabled_in_text_replaces_existing_line`).
- **Quality gates per component before each commit batch:**
  - anyka-init: `$CARGO fmt --check && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && $CARGO test --target x86_64-unknown-linux-gnu`
  - onvif-rust: same three in `cross-compile/onvif-rust`
  - www: `npm run lint && npm run type-check && npm run test && npx prettier --check .` (Prettier 3.9.6 is installed locally; never a bare `npx prettier@3.4.2`). Read `$?` — do not trust an RTK-filtered prettier verdict.
- **Clippy budget:** `too_many_arguments` fires above **7**. `dispatch_msg` is already at exactly 7 — Task 4 introduces `LoopCtx` before adding anything, not after.
- **WebUI tests:** Vitest + React Testing Library, `data-testid` selectors only, `vi.mock` (no MSW), shared helpers in `src/test/`.
- **Work branch:** `design/webui-process-control` (current). Commit after every task with an **explicit pathspec** — this repo's index is usually fully staged, so a bare `git commit` sweeps in unrelated rework. Push when the plan is fully executed.
- **Control-socket contract:** one-line request `\n`-terminated, one-line reply `\n`-terminated; status frame = TSV rows + blank line. Replies for toggles: `ok` / `unknown` / `error`.
- **Camera for on-device verification:** 192.168.2.198, HTTP Basic `admin`/`admin`. Telnet (24) is dead by design on healthy boots; FTP (21) is up but both known credential pairs were refused in the 2026-09-19 session — assume **no shell access on a healthy boot**.

---

## Ground truth the plan depends on

Verified against the code on 2026-09-19. If any of these has drifted, stop and re-check before writing the task that relies on it.

| Fact | Location |
|---|---|
| `handle_control_conn` lives in `supervisor_loop.rs`, not `control.rs` | `supervisor_loop.rs:333` |
| The onvif-rust client's socket timeout is **2 s** | `diagnostics/services.rs:26` |
| `dispatch_msg` already takes 7 arguments; `run` takes `cfg: &Config` | `supervisor_loop.rs:385`, `:424` |
| anyka-init's `supervisor_loop` tests use mockall `MockSys` + a `test_config(BTreeMap<String, ServiceCfg>)` helper — there is **no** `TestSys`/`dummy_spec`/`supervisor_policy` fixture | `supervisor_loop.rs:683-799` |
| The three existing `run_tests` wrap config in `Arc<Config>` and move it into a thread | `supervisor_loop.rs:735,760,788` |
| `ExitStatus` is `Code(i32) \| Signal(i32)` (no `Dead`) | `sys.rs:33` |
| `SpawnSpec` has no `Default` | `sys.rs:19` |
| `ServiceStatus.state` is `&'static str` | `control.rs:15` |
| The monitor reboots on a stalled video heartbeat (5 ticks) and on an unhealthy link (cap 3), calling `sys.reboot()` itself | `monitor.rs:150`, `:80` |
| `read_heartbeat` returning `None` resets the stall counter | `monitor.rs:139,158` |
| All six shipped `[services.*]` stanzas carry an explicit `enabled` line | `SD_card_contents/anyka_hack/anyka.toml:89-124` |
| Nothing in the codebase writes `anyka.toml` today, and `netoverlay.rs` says so | `netoverlay.rs:3-7` |
| The network overlay merges `[wifi]` only — it cannot conflict with `[services]` | `config.rs:513-544` |

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `cross-compile/anyka-init/src/config.rs` | Modify | `ConfigError::Write` variant; pure `set_enabled_in_text`; `Config::set_service_enabled` (atomic write) |
| `cross-compile/anyka-init/src/supervise.rs` | Modify | `SvcState::Disabled` variant; `decide()` guard; `pid()` arm |
| `cross-compile/anyka-init/src/supervisor_loop.rs` | Modify | `LoopCtx`; `Msg::ToggleService`; `handle_toggle_service`; `handle_query_status` cfg-driven; `send_toggle`; `run`/`dispatch_msg` signatures |
| `cross-compile/anyka-init/src/control.rs` | Modify | `Request::Enable/Disable`; `parse_request` verb dispatch; `ToggleOutcome`; `from_svc_state` Disabled arm |
| `cross-compile/anyka-init/src/update.rs` | Modify | `effective_trial_ports` |
| `cross-compile/anyka-init/src/main.rs` | Modify | `&mut cfg` + config path into `run`; adaptive `Policy.ports` |
| `cross-compile/anyka-init/src/netoverlay.rs` | Modify (comment only) | Correct the "nothing writes anyka.toml" invariant |
| `cross-compile/onvif-rust/src/diagnostics/services.rs` | Modify | `ToggleReply` + `request_toggle` client |
| `cross-compile/onvif-rust/src/diagnostics/processes.rs` | Modify | `toggle_service` core + two handlers; 409 guard on restart |
| `cross-compile/onvif-rust/src/onvif/server.rs` | Modify | Route registration (auth-gated) |
| `cross-compile/onvif-rust/src/diagnostics/http.rs` | Modify (tests only) | Auth-table tests for the two new paths |
| `cross-compile/www/src/services/processesService.ts` | Modify | `serviceAction`; `restartService` becomes a wrapper; `ServiceStatus.state` doc |
| `cross-compile/www/src/components/ProcessesCard.tsx` | Modify | Disable/Enable actions, dimmed rows, generalized dialog with per-service copy |
| `cross-compile/www/src/components/ProcessesCard.test.tsx` | Modify | Updated + new tests |

---

## Phase 1 — anyka-init

### Task 1: Line-level TOML editor (`config.rs`)

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs` (add near `Config::load`; new variant in `ConfigError`)
- Modify: `cross-compile/anyka-init/src/netoverlay.rs` (module doc comment)

**Interfaces:**
- Produces: `pub fn set_enabled_in_text(text: &str, name: &str, enabled: bool) -> Result<String, ConfigError>`; `Config::set_service_enabled(path: &Path, name: &str, enabled: bool) -> Result<(), ConfigError>` (**associated function, no `self`** — it edits the file, not the in-memory struct); `ConfigError::Write { path: String, source: std::io::Error }`

The editor is a **pure text function** (the atomic write wraps it). Only one line of the file ever changes; a TOML round-trip would reformat the whole operator-owned file.

- [ ] **Step 1: Write the failing tests**

In `config.rs`'s `#[cfg(test)] mod tests`:

```rust
const SAMPLE: &str = concat!(
    "title = \"anyka\"\n",
    "[services.onvif]\n",
    "enabled = true\n",
    "exec = \"/mnt/anyka_hack/slots/a/bin/onvif-rust.bin\"\n",
    "# keep this comment alive\n",
    "[services.snmp]\n",
    "exec = \"/usr/sbin/snmpd\"\n",
    "[services.dropbear]\n",
    "enabled = false\n",
);

#[test]
fn test_set_enabled_in_text_replaces_an_existing_line() {
    let got = set_enabled_in_text(SAMPLE, "onvif", false).expect("edit");
    assert!(got.contains("[services.onvif]\nenabled = false\nexec ="));
}

#[test]
fn test_set_enabled_in_text_preserves_everything_else() {
    let got = set_enabled_in_text(SAMPLE, "snmp", true).expect("edit");
    // The only byte-level change: one inserted line.
    assert_eq!(
        got,
        SAMPLE.replace(
            "[services.snmp]\nexec =",
            "[services.snmp]\nenabled = true\nexec =",
        )
    );
    assert!(got.contains("# keep this comment alive"));
}

#[test]
fn test_set_enabled_in_text_inserts_under_the_header_when_absent() {
    // A hand-edited config may omit `enabled` entirely (it defaults true),
    // so "disable" has to be able to create the line.
    let got = set_enabled_in_text(SAMPLE, "snmp", false).expect("edit");
    assert!(got.contains("[services.snmp]\nenabled = false\nexec ="));
}

#[test]
fn test_set_enabled_in_text_does_not_escape_the_stanza() {
    // dropbear's line must be untouched when onvif is edited.
    let got = set_enabled_in_text(SAMPLE, "onvif", false).expect("edit");
    assert!(got.contains("[services.dropbear]\nenabled = false\n"));
}

#[test]
fn test_set_enabled_in_text_unknown_stanza_is_an_error() {
    match set_enabled_in_text(SAMPLE, "nope", true) {
        Err(ConfigError::Invalid(_)) => {}
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn test_set_service_enabled_writes_atomically_and_preserves_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("anyka.toml");
    std::fs::write(&path, SAMPLE).expect("seed");

    Config::set_service_enabled(&path, "onvif", false).expect("persist");

    let after = std::fs::read_to_string(&path).expect("read back");
    assert!(after.contains("[services.onvif]\nenabled = false"));
    assert!(after.contains("# keep this comment alive"));
    // No temp file left behind.
    assert!(!path.with_extension("toml.tmp").exists());
    // Still parses.
    Config::load_without_overlay(path.to_str().expect("utf8")).ok();
}
```

`tempfile` is already a dev-dependency of this crate (`supervisor_loop.rs` tests use it) — use it rather than hand-rolling a temp dir.

- [ ] **Step 2: Run to verify they fail**

Run: `cd cross-compile/anyka-init && $CARGO test --target x86_64-unknown-linux-gnu --lib config::`
Expected: compile errors — `set_enabled_in_text` / `ConfigError::Write` do not exist.

- [ ] **Step 3: Implement**

Add to the `ConfigError` enum:

```rust
#[error("failed to write {path}: {source}")]
Write {
    path: String,
    #[source]
    source: std::io::Error,
},
```

Add the pure editor (free function, module scope):

```rust
/// Line-level edit of `enabled =` under `[services.<name>]`.
///
/// Only the one boolean line changes — comments, ordering and formatting
/// everywhere else survive byte-for-byte, which a TOML round-trip cannot
/// guarantee. That matters because this file is the operator's: hand-edited,
/// comment-rich, and holding the Wi-Fi credentials.
pub fn set_enabled_in_text(text: &str, name: &str, enabled: bool) -> Result<String, ConfigError> {
    let header = format!("[services.{name}]");
    let value = if enabled { "true" } else { "false" };

    let mut out: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let Some(hdr) = out.iter().position(|l| l.trim() == header) else {
        return Err(ConfigError::Invalid(format!(
            "no [services.{name}] stanza in config"
        )));
    };
    // The stanza ends at the next `[`-prefixed line, or at end of file.
    let end = out[hdr + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| i + hdr + 1)
        .unwrap_or(out.len());

    let Some(i) = (hdr + 1..end).find(|&i| out[i].trim_start().starts_with("enabled")) else {
        out.insert(hdr + 1, format!("enabled = {value}"));
        return Ok(out.join("\n"));
    };
    // Preserve the line's indentation, change only the value.
    let lead: String = out[i].chars().take_while(|c| c.is_whitespace()).collect();
    out[i] = format!("{lead}enabled = {value}");
    Ok(out.join("\n"))
}
```

Add to `impl Config`:

```rust
/// Persist `enabled` for one service: line-level edit, then atomic
/// tmp+rename over the original (the same pattern `update.rs` uses for the
/// `active` pointer on this filesystem). On failure the original is
/// untouched.
///
/// Associated, not a method: this writes the file, and the in-memory
/// `Config` is updated separately by the caller so the two steps stay
/// visibly ordered.
pub fn set_service_enabled(
    path: &std::path::Path,
    name: &str,
    enabled: bool,
) -> Result<(), ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;
    let new_text = set_enabled_in_text(&text, name, enabled)?;

    let tmp = path.with_extension("toml.tmp");
    let write = |p: &std::path::Path| -> Result<(), std::io::Error> {
        let mut f = std::fs::File::create(p)?;
        std::io::Write::write_all(&mut f, new_text.as_bytes())?;
        f.sync_all()
    };
    write(&tmp).map_err(|source| ConfigError::Write {
        path: tmp.display().to_string(),
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| ConfigError::Write {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}
```

Then correct the now-false invariant in `netoverlay.rs:3-7`:

```rust
//! `anyka.toml` is the operator's file: hand-edited, comment-rich, and holding
//! the Wi-Fi credentials. The only writer in this codebase is
//! `Config::set_service_enabled`, which rewrites a single `enabled =` line in
//! place. Runtime *network* changes made from the WebUI land here instead, in
//! a file that has no comments to lose and no operator intent to clobber, and
//! that a support engineer can neutralise with a single `rm`.
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib config::`
Expected: the 6 new tests PASS, existing config tests still pass (including `test_shipped_anyka_toml_loads_cleanly`).

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/anyka-init/src/config.rs cross-compile/anyka-init/src/netoverlay.rs
rtk git commit -m "feat(anyka-init): atomic line-level anyka.toml service toggle editor"
```

---

### Task 2: `SvcState::Disabled`

**Files:**
- Modify: `cross-compile/anyka-init/src/supervise.rs` (`SvcState`, `SvcState::pid`, `decide`)
- Modify: `cross-compile/anyka-init/src/control.rs` (`from_svc_state` arm — required for exhaustiveness)

**Interfaces:**
- Produces: `SvcState::Disabled` (fieldless variant). `decide` returns `{ action: None, next: Disabled }` for it and never touches `hist`.

The service **stays in the `services` vec** — no insertion/removal, so the index-based `by_pid` map is never disturbed.

**One guard, not three.** With `decide` returning `Action::None`, the per-tick stepper does nothing (it acts only on `Action::Start`) and the exit path records nothing beyond the `by_pid` removal and one `warn!` it would do anyway. Extra `continue`/`return` guards in `tick_services` / `handle_service_exited` would be unreachable by effect — do not add them.

- [ ] **Step 1: Write the failing tests**

In `supervise.rs`'s existing test module (reuse its existing policy/history helpers; do not invent new ones):

```rust
#[test]
fn test_decide_never_acts_on_a_disabled_service() {
    let now = Instant::now();
    let mut hist = RestartHistory::default();
    // Pre-load history so any other state would be deep in crash-loop logic.
    for _ in 0..10 {
        hist.push(now - Duration::from_secs(1));
    }
    let before = hist.len();

    let d = decide(&SvcState::Disabled, &mut hist, Event::Exited, now, &policy());

    assert_eq!(d.action, Action::None);
    assert_eq!(d.next, SvcState::Disabled);
    // A disabled service must not accumulate crash history: re-enabling it
    // later must not inherit a crash-loop it never had.
    assert_eq!(hist.len(), before);
}

#[test]
fn test_decide_does_not_start_a_disabled_service_on_a_tick() {
    let now = Instant::now();
    let d = decide(
        &SvcState::Disabled,
        &mut RestartHistory::default(),
        Event::Tick,
        now,
        &policy(),
    );
    assert_eq!(d.action, Action::None);
}

#[test]
fn test_pid_of_disabled_is_none() {
    assert_eq!(SvcState::Disabled.pid(), None);
}
```

In `control.rs` tests:

```rust
#[test]
fn test_from_svc_state_disabled() {
    let got = ServiceStatus::from_svc_state(
        "dropbear",
        &SvcState::Disabled,
        &RestartHistory::default(),
        Instant::now(),
    );
    assert_eq!(got.state, "disabled");
    assert_eq!(got.pid, None);
    assert_eq!(got.restarts, 0);
    assert_eq!(got.retry_in_s, 0);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervise:: control::`
Expected: compile failure — `SvcState::Disabled` does not exist.

- [ ] **Step 3: Implement**

`supervise.rs`:

```rust
pub enum SvcState {
    Running { pid: Pid, since: Instant },
    Backoff { until: Instant, attempt: u32 },
    /// Disabled at runtime via the control socket. The service stays in the
    /// `services` vec (so `by_pid` indices never shift) but is inert: `decide`
    /// never acts on it and never records history for it.
    Disabled,
}
```

Extend `pid()` with `SvcState::Disabled => None`.

In `decide`, as the **first statement** of the body, so no existing arm has to change and no `hist` path can be reached:

```rust
if matches!(state, SvcState::Disabled) {
    return Decision {
        action: Action::None,
        next: SvcState::Disabled,
    };
}
```

(Use whatever the function's real return struct is named; do not introduce a new one.)

`control.rs` — `from_svc_state` gains the arm that keeps the `match` exhaustive:

```rust
SvcState::Disabled => Self {
    name: name.to_owned(),
    state: "disabled",
    pid: None,
    uptime_s: 0,
    restarts: 0,
    retry_in_s: 0,
},
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: new tests PASS; full anyka-init suite green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/anyka-init/src/supervise.rs cross-compile/anyka-init/src/control.rs
rtk git commit -m "feat(anyka-init): inert SvcState::Disabled"
```

---

### Task 3: Control-socket protocol — `enable`/`disable` requests

**Files:**
- Modify: `cross-compile/anyka-init/src/control.rs` (`Request`, `parse_request`, `ToggleOutcome`)
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`Msg::ToggleService` declaration, `send_toggle`, `handle_control_conn` arms)

**Interfaces:**
- Produces: `Request::Enable(String)` / `Request::Disable(String)`; `pub enum ToggleOutcome { Ok, Unknown, Error }` (in `control.rs`); `Msg::ToggleService { name, enabled, reply }` (declared here, handled in Task 4).

**Reply timeout: 1 s.** The client's socket timeout is 2 s (`diagnostics/services.rs:26`). A server that waits longer than the client hands the user a 503 for a toggle that was applied and persisted — the worst possible outcome for this feature. A loop that cannot answer in 1 s is wedged, and `error` is then honest.

- [ ] **Step 1: Write the failing tests**

In `control.rs` tests:

```rust
#[test]
fn test_parse_request_enable() {
    assert_eq!(
        parse_request("enable snmp\n"),
        Some(Request::Enable("snmp".into()))
    );
}

#[test]
fn test_parse_request_disable() {
    assert_eq!(
        parse_request("disable onvif\r\n"),
        Some(Request::Disable("onvif".into()))
    );
}

#[test]
fn test_parse_request_toggle_rejects_blank_or_tabbed_names() {
    assert_eq!(parse_request("disable \n"), None);
    assert_eq!(parse_request("enable a\tb\n"), None);
}

#[test]
fn test_parse_request_still_rejects_unknown_verbs() {
    assert_eq!(parse_request("destroy onvif\n"), None);
    assert_eq!(parse_request("status\n"), Some(Request::Status));
}
```

The four existing `parse_request` tests must keep passing unchanged — the rewrite below is behaviour-preserving for `status` and `restart`.

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib control::`
Expected: compile failure — new variants missing.

- [ ] **Step 3: Implement**

`control.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Restart(String),
    Enable(String),
    Disable(String),
}

/// Parse one line: `"status\n"`, `"restart <name>\n"`, `"enable <name>\n"`,
/// `"disable <name>\n"`. Blank or tab-bearing names are rejected; unknown
/// verbs are `None`.
pub fn parse_request(line: &str) -> Option<Request> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "status" {
        return Some(Request::Status);
    }
    let (verb, name) = line.split_once(' ')?;
    let name = name.trim();
    if name.is_empty() || name.contains('\t') {
        return None;
    }
    match verb {
        "restart" => Some(Request::Restart(name.to_owned())),
        "enable" => Some(Request::Enable(name.to_owned())),
        "disable" => Some(Request::Disable(name.to_owned())),
        _ => None,
    }
}

/// Outcome of an `enable`/`disable` request, decided by the supervisor loop —
/// only it can see whether the name is configured and toggleable, and whether
/// the config write succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleOutcome {
    /// Applied — including the idempotent no-op case (already in that state).
    Ok,
    /// Not a configured service, or not toggleable.
    Unknown,
    /// The config write failed, or the loop did not answer. Nothing changed.
    Error,
}
```

`supervisor_loop.rs` — add the `Msg` variant (declaration only; Task 4 handles it):

```rust
/// Runtime enable/disable of a configured service. The handler persists to
/// `anyka.toml` first, then transitions in-memory state.
ToggleService {
    name: String,
    enabled: bool,
    reply: Sender<control::ToggleOutcome>,
},
```

Add the helper, next to `handle_control_conn`:

```rust
/// Ask the loop to toggle a service and write its verdict back to the
/// connection.
///
/// The 1 s budget is deliberately below the client's 2 s socket timeout
/// (`onvif-rust/src/diagnostics/services.rs`): if we answered later than the
/// client waits, an applied-and-persisted toggle would surface as a 503.
fn send_toggle<W: std::io::Write>(
    writer: &mut W,
    tx: &Sender<Msg>,
    name: String,
    enabled: bool,
) {
    let (reply_tx, reply_rx) = channel();
    let sent = tx.send(Msg::ToggleService {
        name,
        enabled,
        reply: reply_tx,
    });
    let outcome = match sent {
        Ok(()) => reply_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or(control::ToggleOutcome::Error),
        Err(_) => control::ToggleOutcome::Error,
    };
    let reply = match outcome {
        control::ToggleOutcome::Ok => "ok\n",
        control::ToggleOutcome::Unknown => "unknown\n",
        control::ToggleOutcome::Error => "error\n",
    };
    let _ = writer.write_all(reply.as_bytes());
}
```

And two arms in `handle_control_conn`, alongside the existing `Status`/`Restart` ones:

```rust
Some(control::Request::Enable(name)) => send_toggle(&mut stream, tx, name, true),
Some(control::Request::Disable(name)) => send_toggle(&mut stream, tx, name, false),
```

Until Task 4 lands, add a temporary arm in `dispatch_msg` so the crate compiles — Task 4 **replaces** it:

```rust
// TEMPORARY (Task 3 → replaced in Task 4)
Ok(Msg::ToggleService { reply, .. }) => {
    let _ = reply.send(control::ToggleOutcome::Error);
    false
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: parse tests pass; suite green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/anyka-init/src/control.rs cross-compile/anyka-init/src/supervisor_loop.rs
rtk git commit -m "feat(anyka-init): enable/disable control-socket requests (ok/unknown/error)"
```

---

### Task 4: `LoopCtx` + the toggle dispatch handler

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`LoopCtx`, `dispatch_msg`, `run`, `handle_toggle_service`, `run_tests` migration)
- Modify: `cross-compile/anyka-init/src/main.rs` (pass `&mut cfg` and the config path)

**Interfaces:**
- Consumes: `Config::set_service_enabled` (Task 1); `SvcState::Disabled` (Task 2); `Msg::ToggleService` (Task 3); existing `spec_of_slot`, `build_enabled_services`.
- Produces: `struct LoopCtx<'a>`; `fn handle_toggle_service(ctx: &mut LoopCtx<'_>, services: &mut Vec<Service>, name: String, enabled: bool, reply: &Sender<control::ToggleOutcome>)`.

**Do `LoopCtx` first.** `dispatch_msg` is already at 7 arguments and the toggle handler needs three more (config path, update root, slots) plus a `&mut Config`. Adding them as parameters fails `clippy -- -D warnings` at Task 7, so the grouping is not a style preference — it is the only way this compiles under the project's own gate.

- [ ] **Step 1: Refactor to `LoopCtx` (no behaviour change)**

```rust
/// Borrowed state the message handlers need. Exists because `dispatch_msg`
/// was already at clippy's seven-argument limit before the toggle handler
/// added a config path, an update root and the slot pointer.
struct LoopCtx<'a> {
    sys: &'a dyn Sys,
    cfg: &'a mut Config,
    config_path: &'a Path,
    update_root: &'a Path,
    slots: &'a crate::update::Slots,
    policy: &'a Policy,
}
```

`dispatch_msg(ctx: &mut LoopCtx<'_>, services: &mut Vec<Service>, by_pid: &mut BTreeMap<Pid, usize>, rx: &Receiver<Msg>, msg: Result<Msg, RecvTimeoutError>) -> bool` — 5 arguments. Existing arms become `handle_service_exited(ctx.sys, ctx.cfg, services, by_pid, ctx.policy, pid, st)` and so on; the disjoint field borrows are fine.

`run` becomes:

```rust
pub fn run(sys: Arc<dyn Sys>, cfg: &mut Config, config_path: &Path, rx: Receiver<Msg>) {
    let policy = Policy { /* unchanged */ };

    // Owned, not borrowed from `cfg`: `LoopCtx` holds `&mut Config`, so a
    // live immutable borrow of `cfg.update.root` would conflict. Neither
    // value changes at runtime.
    let slots = crate::update::Slots::new(cfg.update.root.clone());
    let update_root = std::path::PathBuf::from(&cfg.update.root);

    let mut services = build_enabled_services(sys.as_ref(), cfg, &update_root, &slots);
    let mut by_pid: BTreeMap<Pid, usize> = BTreeMap::new();

    loop {
        let next_deadline = tick_services(sys.as_ref(), cfg, &mut services, &mut by_pid, &policy);
        let timeout = /* unchanged */;

        let mut ctx = LoopCtx {
            sys: sys.as_ref(),
            cfg,
            config_path,
            update_root: &update_root,
            slots: &slots,
            policy: &policy,
        };
        if dispatch_msg(&mut ctx, &mut services, &mut by_pid, &rx, rx.recv_timeout(timeout)) {
            return;
        }
    }
}
```

The three existing `run_tests` (`supervisor_loop.rs:735,760,788`) pass `Arc<Config>`; they become an owned `Config` moved into the closure:

```rust
let mut cfg = test_config(BTreeMap::new());
let (tx, rx) = make_channel();
let sys: Arc<dyn Sys> = Arc::new(sys);
let handle = std::thread::spawn(move || {
    run(sys, &mut cfg, Path::new("/nonexistent/anyka.toml"), rx)
});
```

`main.rs`: `supervisor_loop::run(sysimpl, &mut cfg, Path::new(CONFIG_PATH), rx);`. Hoist the literal `"/mnt/anyka_hack/anyka.toml"` (used at `main.rs:18` and `:66`) into one `const CONFIG_PATH: &str` so the loader and the writer cannot drift apart.

Run `$CARGO test --target x86_64-unknown-linux-gnu --lib` and `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings` here, before Step 2 — this step must be green on its own.

- [ ] **Step 2: Write the failing tests**

Add to the `run_tests` module (it already has `test_config`; add the two small helpers below next to it):

```rust
fn svc_cfg(exec: &str, enabled: bool) -> ServiceCfg {
    ServiceCfg {
        enabled,
        exec: exec.into(),
        args: Vec::new(),
        env: BTreeMap::new(),
        log: "/nonexistent/svc.log".into(),
        core_dump: false,
    }
}

fn dummy_spec() -> SpawnSpec {
    SpawnSpec {
        exec: "/bin/true".into(),
        args: Vec::new(),
        env: BTreeMap::new(),
        tz: None,
        log: "/nonexistent/svc.log".into(),
        core_dump: false,
    }
}

/// A `LoopCtx` over test-owned parts. Returned by value so each test can keep
/// its tempdir alive.
fn ctx<'a>(
    sys: &'a dyn Sys,
    cfg: &'a mut Config,
    config_path: &'a Path,
    update_root: &'a Path,
    slots: &'a crate::update::Slots,
    policy: &'a Policy,
) -> LoopCtx<'a> {
    LoopCtx { sys, cfg, config_path, update_root, slots, policy }
}
```

Tests:

```rust
#[test]
fn test_toggle_disable_persists_then_kills_and_inerts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("anyka.toml");
    std::fs::write(
        &cfg_path,
        "[services.snmp]\nenabled = true\nexec = \"/bin/true\"\n",
    )
    .expect("seed");

    let mut sys = MockSys::new();
    sys.expect_now().returning(Instant::now);
    sys.expect_kill()
        .withf(|pid, sig| *pid == 55 && *sig == libc::SIGTERM)
        .times(1)
        .returning(|_, _| Ok(()));

    let mut services = BTreeMap::new();
    services.insert("snmp".to_string(), svc_cfg("/bin/true", true));
    let mut cfg = test_config(services);
    let slots = crate::update::Slots::new(dir.path());
    let policy = test_policy();
    let mut svcs = vec![Service {
        name: "snmp".into(),
        spec: dummy_spec(),
        state: SvcState::Running { pid: 55, since: Instant::now() },
        hist: RestartHistory::default(),
    }];

    let (rtx, rrx) = channel();
    let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
    handle_toggle_service(&mut c, &mut svcs, "snmp".into(), false, &rtx);

    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
    // File first.
    assert!(
        std::fs::read_to_string(&cfg_path)
            .expect("read")
            .contains("enabled = false")
    );
    assert!(!cfg.services["snmp"].enabled);
    assert_eq!(svcs[0].state, SvcState::Disabled);
}

#[test]
fn test_toggle_enable_a_boot_time_disabled_service_inserts_it() {
    // dropbear is the shipped case: disabled in the file, never in the vec.
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("anyka.toml");
    std::fs::write(
        &cfg_path,
        "[services.dropbear]\nenabled = false\nexec = \"/bin/true\"\n",
    )
    .expect("seed");

    let mut sys = MockSys::new();
    sys.expect_now().returning(Instant::now);

    let mut services = BTreeMap::new();
    services.insert("dropbear".to_string(), svc_cfg("/bin/true", false));
    let mut cfg = test_config(services);
    let slots = crate::update::Slots::new(dir.path());
    let policy = test_policy();
    let mut svcs: Vec<Service> = Vec::new();

    let (rtx, rrx) = channel();
    let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
    handle_toggle_service(&mut c, &mut svcs, "dropbear".into(), true, &rtx);

    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
    assert!(
        std::fs::read_to_string(&cfg_path)
            .expect("read")
            .contains("enabled = true")
    );
    // Inserted exactly as build_enabled_services would: boot-start state.
    assert_eq!(svcs.len(), 1);
    assert_eq!(svcs[0].name, "dropbear");
    assert!(matches!(svcs[0].state, SvcState::Backoff { attempt: 0, .. }));
}

#[test]
fn test_toggle_unknown_service_replies_unknown_and_writes_nothing() {
    // ... seed as above with one snmp entry; call with "nope" ...
    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Unknown);
    assert_eq!(std::fs::read_to_string(&cfg_path).expect("read"), before);
}

#[test]
fn test_toggle_of_wpa_supplicant_is_refused() {
    // Configured and enabled, but not toggleable: disabling it would let the
    // monitor's wifi ladder reboot the camera, and there is no way back in.
    // ... seed with a wpa_supplicant entry ...
    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Unknown);
    assert_eq!(std::fs::read_to_string(&cfg_path).expect("read"), before);
    assert!(cfg.services["wpa_supplicant"].enabled);
}

#[test]
fn test_toggle_is_an_idempotent_noop_when_already_in_that_state() {
    // cfg says disabled, request says disable: Ok, and the file is byte-equal.
    // MockSys with no kill expectation — calling it would fail the test.
    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
    assert_eq!(std::fs::read_to_string(&cfg_path).expect("read"), before);
}

#[test]
fn test_toggle_replies_error_and_changes_nothing_when_the_write_fails() {
    // Force the failure with a config_path inside a directory that does not
    // exist, so the read fails before anything is touched. (chmod is not a
    // reliable lever: CI may run as root.)
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("no-such-dir").join("anyka.toml");
    // ... cfg has snmp enabled, svcs has snmp Running ...
    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Error);
    assert!(cfg.services["snmp"].enabled); // in-memory untouched
    assert!(matches!(svcs[0].state, SvcState::Running { .. }));
}

#[test]
fn test_disabling_vendor_daemon_removes_the_video_heartbeat() {
    // Without this the monitor keeps reading a stale counter and reboots the
    // camera five ticks later — the exact thing disabling is meant to stop.
    let dir = tempfile::tempdir().expect("tempdir");
    let hb = dir.path().join("video.heartbeat");
    std::fs::write(&hb, "12345\n").expect("seed heartbeat");
    // ... cfg.monitor.video_heartbeat_path = hb, vendor-daemon enabled+Running,
    //     MockSys expects one kill ...
    handle_toggle_service(&mut c, &mut svcs, "vendor-daemon".into(), false, &rtx);
    assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
    assert!(!hb.exists());
}
```

Add a `test_policy()` helper mirroring the `Policy` that `run` builds from `test_config`.

- [ ] **Step 3: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervisor_loop::`
Expected: compile failure — `handle_toggle_service` missing.

- [ ] **Step 4: Implement**

```rust
/// Services that may not be toggled at runtime.
///
/// `wpa_supplicant`: the monitor's wifi ladder reboots the camera on an
/// unhealthy link (monitor.rs:80) and is driven by link health, not by a file
/// we can stand down. Disabling the supplicant on a Wi-Fi camera also loses
/// the device outright. See §2.1 of the design.
const NON_TOGGLEABLE: [&str; 1] = ["wpa_supplicant"];

/// Runtime enable/disable. Order is deliberate: **file first**, then
/// in-memory cfg, then state/kill. A failed write means nothing changes — a
/// "disabled" service that silently re-enabled itself on reboot would defeat
/// the crash-loop escape hatch this exists for.
fn handle_toggle_service(
    ctx: &mut LoopCtx<'_>,
    services: &mut Vec<Service>,
    name: String,
    enabled: bool,
    reply: &Sender<control::ToggleOutcome>,
) {
    if NON_TOGGLEABLE.contains(&name.as_str()) {
        tracing::warn!(service = %name, "toggle refused: service is not toggleable");
        let _ = reply.send(control::ToggleOutcome::Unknown);
        return;
    }
    let Some(entry) = ctx.cfg.services.get(&name) else {
        tracing::warn!(service = %name, "toggle for unknown service");
        let _ = reply.send(control::ToggleOutcome::Unknown);
        return;
    };
    if entry.enabled == enabled {
        let _ = reply.send(control::ToggleOutcome::Ok);
        return;
    }

    if let Err(e) = Config::set_service_enabled(ctx.config_path, &name, enabled) {
        tracing::error!(service = %name, error = %e, "toggle: config write failed; not applied");
        let _ = reply.send(control::ToggleOutcome::Error);
        return;
    }
    if let Some(e) = ctx.cfg.services.get_mut(&name) {
        e.enabled = enabled;
    }

    match services.iter_mut().find(|s| s.name == name) {
        Some(svc) if !enabled => {
            if let Some(pid) = svc.state.pid()
                && let Err(e) = ctx.sys.kill(pid, libc::SIGTERM)
            {
                tracing::warn!(service = %name, error = %e, "toggle: SIGTERM failed");
            }
            svc.state = SvcState::Disabled;
            tracing::info!(service = %name, "disabled");
        }
        Some(svc) => {
            // The same initial state a boot start gets (build_enabled_services):
            // the next tick starts it under normal backoff/crash-loop policy.
            svc.state = SvcState::Backoff {
                until: ctx.sys.now(),
                attempt: 0,
            };
            tracing::info!(service = %name, "enabled");
        }
        None if enabled => {
            // Not in the vec: disabled at boot, never started (the shipped
            // dropbear case). Insert exactly as build_enabled_services would.
            if let Some(s) = ctx.cfg.services.get(&name) {
                services.push(Service {
                    name: name.clone(),
                    spec: spec_of_slot(s, ctx.update_root, ctx.slots),
                    state: SvcState::Backoff {
                        until: ctx.sys.now(),
                        attempt: 0,
                    },
                    hist: RestartHistory::default(),
                });
                tracing::info!(service = %name, "enabled (inserted)");
            }
        }
        None => {}
    }

    // Stand the video watchdog down with the daemon it watches. The heartbeat
    // file survives in /tmp holding its last counter value, and the monitor
    // reads a stalled counter as a crash: restart, kill, then reboot five
    // ticks later (monitor.rs:150). An absent file reads as "no signal yet",
    // which it already treats as not-a-stall.
    if !enabled && name == "vendor-daemon" {
        let hb = Path::new(&ctx.cfg.monitor.video_heartbeat_path);
        if let Err(e) = std::fs::remove_file(hb)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(error = %e, "failed to clear the video heartbeat");
        }
    }

    let _ = reply.send(control::ToggleOutcome::Ok);
}
```

Replace Task 3's temporary `dispatch_msg` arm:

```rust
Ok(Msg::ToggleService { name, enabled, reply }) => {
    handle_toggle_service(ctx, services, name, enabled, &reply);
    false
}
```

`services` must be `&mut Vec<Service>` (not `&mut [Service]`) through `dispatch_msg` — the enable path pushes.

- [ ] **Step 5: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: all seven handler tests pass; full suite green.

- [ ] **Step 6: Commit**

```bash
rtk git add cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/src/main.rs
rtk git commit -m "feat(anyka-init): toggle dispatch — persist first, then SIGTERM/state; stand down the video watchdog"
```

---

### Task 5: `status` lists disabled services

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`handle_query_status`)

**Interfaces:**
- Consumes: `cfg.services` (single source of truth); `control::ServiceStatus::from_svc_state`.
- Produces: one TSV row per **configured** service; disabled rows are `name disabled -1 0 0 0`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn test_query_status_lists_enabled_and_disabled_services() {
    let mut services = BTreeMap::new();
    services.insert("onvif".to_string(), svc_cfg("/bin/true", true));
    services.insert("snmp".to_string(), svc_cfg("/bin/true", false));
    services.insert("dropbear".to_string(), svc_cfg("/bin/true", false));
    let cfg = test_config(services);

    let svcs = vec![Service {
        name: "onvif".into(),
        spec: dummy_spec(),
        state: SvcState::Running {
            pid: 42,
            since: Instant::now() - Duration::from_secs(90),
        },
        hist: RestartHistory::default(),
    }];

    let (tx, rx) = channel();
    handle_query_status(&cfg, &svcs, &tx);
    let ControlMsg::Status(rows) = rx.recv().expect("status");

    // BTreeMap order; disabled rows synthesized from cfg.
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["dropbear", "onvif", "snmp"]);
    let snmp = rows.iter().find(|r| r.name == "snmp").expect("snmp row");
    assert_eq!(snmp.state, "disabled");
    assert_eq!(snmp.pid, None);
    let onvif = rows.iter().find(|r| r.name == "onvif").expect("onvif row");
    assert_eq!(onvif.state, "running");
    assert_eq!(onvif.pid, Some(42));
}

#[test]
fn test_query_status_never_drops_a_configured_service() {
    // Defensive: cfg says enabled, the vec does not have it (should not happen
    // after Task 4's insertion). The row must still appear, never vanish.
    let mut services = BTreeMap::new();
    services.insert("snmp".to_string(), svc_cfg("/bin/true", true));
    let cfg = test_config(services);

    let (tx, rx) = channel();
    handle_query_status(&cfg, &[], &tx);
    let ControlMsg::Status(rows) = rx.recv().expect("status");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, "backoff");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervisor_loop::`
Expected: compile failure — `handle_query_status` takes one more argument.

- [ ] **Step 3: Implement**

```rust
/// One row per **configured** service, not per running one: a disabled service
/// has to be listable or the UI has no way to offer re-enabling it. `cfg` is
/// the single source of truth — both the boot path and the toggle handler
/// write it before anything else.
fn handle_query_status(cfg: &Config, services: &[Service], reply_tx: &Sender<ControlMsg>) {
    let now = Instant::now();
    let rows: Vec<control::ServiceStatus> = cfg
        .services
        .iter()
        .map(|(name, entry)| {
            match services.iter().find(|s| s.name == *name) {
                _ if !entry.enabled => control::ServiceStatus::from_svc_state(
                    name,
                    &SvcState::Disabled,
                    &RestartHistory::default(),
                    now,
                ),
                Some(svc) => {
                    control::ServiceStatus::from_svc_state(&svc.name, &svc.state, &svc.hist, now)
                }
                // Enabled but not yet in the vec: render as pending, never drop.
                None => control::ServiceStatus {
                    name: name.clone(),
                    state: "backoff",
                    pid: None,
                    uptime_s: 0,
                    restarts: 0,
                    retry_in_s: 0,
                },
            }
        })
        .collect();
    let _ = reply_tx.send(ControlMsg::Status(rows));
}
```

Update the `dispatch_msg` arm to pass `ctx.cfg`.

- [ ] **Step 4: Run to verify it passes**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/anyka-init/src/supervisor_loop.rs
rtk git commit -m "feat(anyka-init): status lists disabled services from cfg"
```

---

### Task 6: Adaptive trial ports

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs` (`effective_trial_ports`)
- Modify: `cross-compile/anyka-init/src/main.rs` (the one production `Policy` construction, `main.rs:139`)

**Interfaces:**
- Produces: `pub fn effective_trial_ports(requested: &[u16], onvif_enabled: bool) -> Vec<u16>`.

**`evaluate_trial` is not touched.** On an empty port list `ports.iter().all(..)` is already vacuously true, so it confirms after the normal hold without probing anything. An early return would only skip the hold — no behaviour worth a code change, and the existing function stays untested-by-nobody. The port filter is the whole feature.

`TRIAL_PORTS` is reused rather than duplicated into an `ONVIF_PORTS`: it already *is* the onvif-owned set, cited to the same `onvif/config.toml` lines.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn test_effective_trial_ports_unchanged_when_onvif_enabled() {
    assert_eq!(effective_trial_ports(&TRIAL_PORTS, true), TRIAL_PORTS.to_vec());
    // A camera with a custom trial_ports override is unaffected either way.
    assert_eq!(effective_trial_ports(&[2000], true), vec![2000]);
}

#[test]
fn test_effective_trial_ports_drops_onvif_ports_when_onvif_disabled() {
    assert_eq!(effective_trial_ports(&TRIAL_PORTS, false), Vec::<u16>::new());
}

#[test]
fn test_effective_trial_ports_keeps_ports_of_unknown_ownership() {
    // Only ports we know belong to onvif are dropped; a custom port could
    // belong to anything, and a too-lax trial is worse than a strict one.
    assert_eq!(effective_trial_ports(&[80, 554, 8080, 2000], false), vec![2000]);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib update::`
Expected: compile failure — `effective_trial_ports` missing.

- [ ] **Step 3: Implement**

```rust
/// The trial may only require ports an enabled service can actually bind.
///
/// All of `TRIAL_PORTS` is owned by the `onvif` service — ONVIF, RTSP and
/// HTTP-FLV all live in the onvif-rust binary. With onvif disabled the default
/// set is unbindable, so every subsequent A/B update would fail its trial and
/// revert: exactly when an admin most needs to ship a fix. Ports of unknown
/// ownership are kept; dropping something we do not own is the worse error.
pub fn effective_trial_ports(requested: &[u16], onvif_enabled: bool) -> Vec<u16> {
    if onvif_enabled {
        return requested.to_vec();
    }
    requested
        .iter()
        .copied()
        .filter(|p| !TRIAL_PORTS.contains(p))
        .collect()
}
```

`main.rs:142`, replacing `ports: cfg.update.trial_ports.clone(),`:

```rust
ports: anyka_init::update::effective_trial_ports(
    &cfg.update.trial_ports,
    cfg.services.get("onvif").is_some_and(|s| s.enabled),
),
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/anyka-init/src/update.rs cross-compile/anyka-init/src/main.rs
rtk git commit -m "feat(anyka-init): adaptive trial ports — onvif disabled cannot wedge updates"
```

---

### Task 7: anyka-init full gates

- [ ] **Step 1: Run all anyka-init gates**

```bash
source ./setenv.sh
cd cross-compile/anyka-init
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO build --release   # ARM cross build, from this directory
cd ../..
```

Expected: all green; test count ≥ 279 (previous baseline) + ~19 new. If `too_many_arguments` fires, Task 4's `LoopCtx` step was skipped or partially applied — fix there, not with an `#[allow]`.

Note: `cargo clippy` needs the vendored toolchain's bin directory first on `PATH` or it dies with E0514. `setenv.sh` handles this; if it does not, prefix the command.

- [ ] **Step 2: Commit if fmt/clippy required fixes**

```bash
$CARGO fmt
rtk git add cross-compile/anyka-init
rtk git commit -m "chore(anyka-init): fmt/clippy after service toggle"
```

---

## Phase 2 — onvif-rust

### Task 8: `request_toggle` socket client

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/services.rs`

**Interfaces:**
- Produces: `pub enum ToggleReply { Accepted, Unknown, Error, Unreachable }` (`#[derive(Debug, Clone, Copy, PartialEq, Eq)]`); `pub fn request_toggle(path: &Path, name: &str, enabled: bool) -> ToggleReply`.

- [ ] **Step 1: Write the failing tests**

Follow the existing `test_round_trip_against_a_mock_control_server` harness in this file.

```rust
#[test]
fn test_request_toggle_maps_all_reply_words() {
    use std::os::unix::net::UnixListener;

    let path = format!("/tmp/onvif-tog-test-{}.sock", std::process::id());
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let server = std::thread::spawn(move || {
        use std::io::{BufRead, Write};
        // Four connections: ok, unknown, error, garbage.
        let replies: [&[u8]; 4] = [b"ok\n", b"unknown\n", b"error\n", b"nope\n"];
        for (stream, reply) in listener.incoming().take(4).zip(replies) {
            let Ok(mut stream) = stream else { continue };
            let mut line = String::new();
            if std::io::BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let _ = stream.write_all(reply);
        }
    });

    let p = std::path::Path::new(&path);
    assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Accepted);
    assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Unknown);
    assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Error);
    // Anything that is not one of the three words is a failure, not a success.
    assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Error);

    server.join().expect("server thread");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_request_toggle_sends_the_right_verb() {
    // The verb is the only thing distinguishing the two calls; pin it.
    use std::os::unix::net::UnixListener;
    let path = format!("/tmp/onvif-tog-verb-{}.sock", std::process::id());
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let server = std::thread::spawn(move || {
        use std::io::{BufRead, Write};
        let mut seen = Vec::new();
        for stream in listener.incoming().take(2) {
            let Ok(mut stream) = stream else { continue };
            let mut line = String::new();
            let _ = std::io::BufReader::new(&stream).read_line(&mut line);
            seen.push(line);
            let _ = stream.write_all(b"ok\n");
        }
        seen
    });

    let p = std::path::Path::new(&path);
    request_toggle(p, "snmp", true);
    request_toggle(p, "onvif", false);
    let seen = server.join().expect("server thread");
    assert_eq!(seen, vec!["enable snmp\n", "disable onvif\n"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_request_toggle_on_a_missing_socket_is_unreachable() {
    let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
    assert_eq!(request_toggle(missing, "snmp", true), ToggleReply::Unreachable);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd cross-compile/onvif-rust && $CARGO test --target x86_64-unknown-linux-gnu --lib services::`
Expected: compile failure — `request_toggle`/`ToggleReply` missing.

- [ ] **Step 3: Implement**

```rust
/// The supervisor's answer to an `enable`/`disable` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleReply {
    /// Applied, or an idempotent no-op.
    Accepted,
    /// Not a configured service, or not toggleable.
    Unknown,
    /// The supervisor refused: the config write failed, or its loop did not
    /// answer within its own 1 s budget.
    Error,
    /// Socket unreachable — an older anyka-init in the other A/B slot.
    Unreachable,
}

/// Blocking. Sends `enable <name>` or `disable <name>` to the supervisor.
pub fn request_toggle(path: &Path, name: &str, enabled: bool) -> ToggleReply {
    let verb = if enabled { "enable" } else { "disable" };
    match round_trip(path, &format!("{verb} {name}\n")) {
        None => ToggleReply::Unreachable,
        Some(r) => match r.trim() {
            "ok" => ToggleReply::Accepted,
            "unknown" => ToggleReply::Unknown,
            _ => ToggleReply::Error,
        },
    }
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib services::`
Expected: PASS, with the existing `services::` tests still green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/diagnostics/services.rs
rtk git commit -m "feat(onvif-rust): request_toggle socket client (Accepted/Unknown/Error/Unreachable)"
```

---

### Task 9: Routes `POST /api/services/{name}/enable|disable`, plus a 409 on the restart route

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/processes.rs` (handlers + restart guard)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs` (route registration, inside the `if state.auth_enabled` block, next to `/services/{name}/restart` at `server.rs:662`)
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs` (auth-table tests only — the `_ => Administrator` catch-all at `http.rs:106` already covers these paths; the tests pin that)

**Interfaces:**
- Consumes: `request_toggle`, `query_status`, `SOCKET_PATH`.
- Produces: `handle_enable_service` / `handle_disable_service`. Mapping: 202 Accepted / 404 Unknown / 503 Error-or-Unreachable.

**Restart gets a 409.** Task 5 made `status` list disabled services, so the restart route's "is this a known name" guard now passes for them — it would return 202 while the supervisor logs "not running" and does nothing. It already holds the status rows, so checking the matched row's state costs nothing.

- [ ] **Step 1: Write the failing tests**

In `http.rs` tests, next to `test_required_level_for_a_service_restart_is_administrator`:

```rust
#[test]
fn test_required_level_for_a_service_enable_is_administrator() {
    assert_eq!(
        required_level_for_path("/services/onvif/enable"),
        AuthLevel::Administrator
    );
}

#[test]
fn test_required_level_for_a_service_disable_is_administrator() {
    assert_eq!(
        required_level_for_path("/services/snmp/disable"),
        AuthLevel::Administrator
    );
}
```

In `processes.rs` tests — the socket path is a constant, so the testable axis on a dev host is the no-supervisor path. Pin that it degrades to 503 and never 500:

```rust
#[tokio::test]
async fn test_toggle_service_without_a_supervisor_is_503() {
    let resp = toggle_service("snmp".to_string(), true).await.into_response();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

(`.into_response()` — `toggle_service` returns `impl IntoResponse`, which is not convertible with `Response::from`.)

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib processes:: http::`
Expected: compile failure — `toggle_service` missing.

- [ ] **Step 3: Implement**

In `processes.rs`, mirroring `handle_restart_service`:

```rust
/// Shared core for the enable/disable routes.
///
/// 202, not 200: accepted, not confirmed — the supervisor SIGTERMs (disable)
/// or starts under backoff (enable) on its own schedule.
async fn toggle_service(name: String, enabled: bool) -> impl IntoResponse {
    let sock = std::path::Path::new(crate::diagnostics::services::SOCKET_PATH);

    // Validate against the live snapshot (which now includes disabled rows)
    // so an unknown name is a 404, not a silently-accepted "ok".
    let known = tokio::task::spawn_blocking(move || {
        crate::diagnostics::services::query_status(sock)
            .map(|rows| rows.iter().any(|r| r.name == name))
            .map(|found| (found, name))
    })
    .await;

    let Ok(Some((found, name))) = known else {
        return (StatusCode::SERVICE_UNAVAILABLE, "supervisor unreachable").into_response();
    };
    if !found {
        return (StatusCode::NOT_FOUND, "unknown service").into_response();
    }

    let reply = tokio::task::spawn_blocking(move || {
        crate::diagnostics::services::request_toggle(sock, &name, enabled)
    })
    .await;

    match reply {
        Ok(crate::diagnostics::services::ToggleReply::Accepted) => {
            StatusCode::ACCEPTED.into_response()
        }
        Ok(crate::diagnostics::services::ToggleReply::Unknown) => (
            StatusCode::NOT_FOUND,
            "unknown or non-toggleable service",
        )
            .into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "toggle not accepted by supervisor",
        )
            .into_response(),
    }
}

/// POST /api/services/{name}/enable
pub async fn handle_enable_service(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    toggle_service(name, true).await
}

/// POST /api/services/{name}/disable
pub async fn handle_disable_service(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    toggle_service(name, false).await
}
```

In `handle_restart_service`, widen the existing snapshot lookup from "is the name present" to "what state is it in", and refuse a disabled service:

```rust
crate::diagnostics::services::query_status(sock)
    .map(|rows| rows.iter().find(|r| r.name == name).map(|r| r.state.clone()))
    .map(|state| (state, name))
```

```rust
let Ok(Some((state, name))) = known else { /* 503 as today */ };
let Some(state) = state else {
    return (StatusCode::NOT_FOUND, "unknown service").into_response();
};
if state == "disabled" {
    // The supervisor would accept this and do nothing: there is no process to
    // signal. Enable it instead.
    return (StatusCode::CONFLICT, "service is disabled").into_response();
}
```

In `server.rs`, inside the `if state.auth_enabled { ... }` block, after the restart route:

```rust
api = api.route(
    "/services/{name}/enable",
    post(crate::diagnostics::processes::handle_enable_service).layer(timeout()),
);
api = api.route(
    "/services/{name}/disable",
    post(crate::diagnostics::processes::handle_disable_service).layer(timeout()),
);
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu`
Expected: all onvif-rust tests green (2292 baseline + ~5 new). Any existing restart-route test asserting a 202 for a name-only match may need the richer fixture.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/diagnostics/processes.rs cross-compile/onvif-rust/src/onvif/server.rs cross-compile/onvif-rust/src/diagnostics/http.rs
rtk git commit -m "feat(onvif-rust): POST /api/services/{name}/enable|disable, 409 on restarting a disabled service"
```

---

### Task 10: onvif-rust full gates

- [ ] **Step 1: Run all onvif-rust gates**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO build --release   # from this directory, not the workspace root
cd ../..
```

Expected: all green. Commit fix-ups if any.

---

## Phase 3 — WebUI

### Task 11: `processesService.serviceAction`

**Files:**
- Modify: `cross-compile/www/src/services/processesService.ts`
- Modify: `cross-compile/www/src/services/processesService.test.ts`

**Interfaces:**
- Produces: `export async function serviceAction(name: string, action: 'restart' | 'enable' | 'disable'): Promise<void>`; `restartService` becomes a one-line wrapper (its call sites and tests are untouched).
- `ServiceStatus.state` doc comment becomes `'running' | 'backoff' | 'disabled'` (runtime validation is unchanged — `isServiceStatus` already accepts any string).

One function, not two: a `toggleService` would be a character-level copy of `restartService` — same fetch, same 202 check, same `ApiError` shape — differing only in the last path segment.

- [ ] **Step 1: Write the failing tests**

In `processesService.test.ts`, following the file's existing `authorizedFetch` mock pattern:

```ts
describe('serviceAction', () => {
  beforeEach(() => vi.clearAllMocks());

  it.each([
    ['enable', '/api/services/snmp/enable'],
    ['disable', '/api/services/snmp/disable'],
    ['restart', '/api/services/snmp/restart'],
  ] as const)('posts to the %s endpoint', async (action, url) => {
    mockAuthorizedFetch.mockResolvedValue({ ok: true, status: 202, text: async () => '' });
    await expect(serviceAction('snmp', action)).resolves.toBeUndefined();
    expect(mockAuthorizedFetch).toHaveBeenCalledWith(url, expect.objectContaining({ method: 'POST' }));
  });

  it('encodes the service name', async () => {
    mockAuthorizedFetch.mockResolvedValue({ ok: true, status: 202, text: async () => '' });
    await serviceAction('a b', 'disable');
    expect(mockAuthorizedFetch).toHaveBeenCalledWith(
      '/api/services/a%20b/disable',
      expect.anything(),
    );
  });

  it('throws ApiError on 404 with the body', async () => {
    mockAuthorizedFetch.mockResolvedValue({ ok: false, status: 404, text: async () => 'unknown service' });
    await expect(serviceAction('nope', 'enable')).rejects.toThrow('404');
  });

  it('throws ApiError on 503', async () => {
    mockAuthorizedFetch.mockResolvedValue({ ok: false, status: 503, text: async () => 'supervisor unreachable' });
    await expect(serviceAction('snmp', 'disable')).rejects.toThrow('503');
  });

  it('surfaces a dropped connection as a TypeError, not ApiError', async () => {
    mockAuthorizedFetch.mockRejectedValue(new TypeError('fetch failed'));
    const err = await serviceAction('onvif', 'disable').catch((e) => e);
    expect(err).toBeInstanceOf(TypeError);
  });
});
```

The existing `restartService` tests stay as they are — they are the wrapper's regression net.

- [ ] **Step 2: Run to verify they fail**

Run: `cd cross-compile/www && npm run test -- processesService`
Expected: `serviceAction` is not exported — fails.

- [ ] **Step 3: Implement**

```ts
export type ServiceAction = 'restart' | 'enable' | 'disable';

/**
 * Restart, enable or disable a supervised service.
 *
 * 202 means the supervisor accepted: for a toggle the on-disk config is
 * already written, and state transitions on the supervisor's schedule. A 404
 * (unknown or non-toggleable service), 409 (restart of a disabled service) or
 * 503 (supervisor unreachable / config write failed) throws an ApiError. A
 * network-level failure — the camera dropping the connection as onvif itself
 * goes down — surfaces as a fetch TypeError, so callers can distinguish "it
 * did this to us" from "it failed".
 */
export async function serviceAction(name: string, action: ServiceAction): Promise<void> {
  const response = await authorizedFetch(`/api/services/${encodeURIComponent(name)}/${action}`, {
    method: 'POST',
  });

  if (response.status === 202) {
    return;
  }
  const text = await response.text();
  throw new ApiError(`${action} of ${name} failed with status ${response.status}`, response.status, text);
}

export async function restartService(name: string): Promise<void> {
  return serviceAction(name, 'restart');
}
```

Update the `ServiceStatus.state` doc comment to `/** 'running' | 'backoff' | 'disabled'. */` and the module header to mention the two new endpoints.

- [ ] **Step 4: Run to verify they pass**

Run: `npm run test -- processesService && npm run type-check`
Expected: PASS, including the pre-existing `restartService` tests.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/www/src/services/processesService.ts cross-compile/www/src/services/processesService.test.ts
rtk git commit -m "feat(webui): serviceAction covers restart/enable/disable"
```

---

### Task 12: ProcessesCard — Disable/Enable actions with per-service warnings

**Files:**
- Modify: `cross-compile/www/src/components/ProcessesCard.tsx`
- Modify: `cross-compile/www/src/components/ProcessesCard.test.tsx`

**Card behaviour contract:**
- Enabled row: `Restart` + `Disable` (`diagnostics-processes-restart-{name}`, `diagnostics-processes-disable-{name}`).
- Disabled row: dimmed (`opacity-50`), `disabled` badge in muted style, only `Enable` (`diagnostics-processes-enable-{name}`) — no Restart.
- `wpa_supplicant` row: **no toggle at all**, Restart only. The supervisor refuses it (Task 4) and offering a button that always 404s is worse than offering none.
- One shared confirm dialog (`diagnostics-processes-action-dialog`, `...-action-title`, `...-action-description`, `...-action-cancel`, `...-action-confirm`) with per-service consequence copy.
- **Disabling onvif does NOT enter the reconnecting state.** The camera is not coming back on its own, by design. A dropped connection after disabling onvif is the expected success path — it is the only reason that POST's connection would die.

- [ ] **Step 1: Write the failing tests**

Keep every existing raw-table and reconnecting test, updated for the renamed dialog ids. New tests:

```tsx
const SNMP: ServiceStatus = { name: 'snmp', state: 'running', pid: 123, uptime_s: 500, restarts: 0, retry_in_s: 0 };
const OFF: ServiceStatus = { name: 'snmp', state: 'disabled', pid: null, uptime_s: 0, restarts: 0, retry_in_s: 0 };
const supervisedFixture = (rows: ServiceStatus[]) => ({ supervised: rows, processes: [] });

describe('ProcessesCard service toggling', () => {
  it('shows Disable for an enabled service and Enable (dimmed) for a disabled one', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP, { ...OFF, name: 'dropbear' }]));
    renderWithProviders(<ProcessesCard />);
    expect(await screen.findByTestId('diagnostics-processes-disable-snmp')).toBeInTheDocument();
    expect(screen.queryByTestId('diagnostics-processes-enable-snmp')).not.toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-enable-dropbear')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-row-dropbear')).toHaveClass('opacity-50');
    expect(screen.getByTestId('diagnostics-processes-status-dropbear')).toHaveTextContent('disabled');
    // A disabled service is not restartable.
    expect(screen.queryByTestId('diagnostics-processes-restart-dropbear')).not.toBeInTheDocument();
  });

  it('offers no toggle for wpa_supplicant', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'wpa_supplicant' }]));
    renderWithProviders(<ProcessesCard />);
    expect(await screen.findByTestId('diagnostics-processes-restart-wpa_supplicant')).toBeInTheDocument();
    expect(screen.queryByTestId('diagnostics-processes-disable-wpa_supplicant')).not.toBeInTheDocument();
  });

  it('disabling snmp goes through the confirm dialog', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-snmp'));
    expect(screen.getByTestId('diagnostics-processes-action-title')).toHaveTextContent('Disable snmp?');
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('snmp', 'disable'));
  });

  it('enabling calls serviceAction with enable', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([OFF]));
    vi.mocked(serviceAction).mockResolvedValue(undefined);
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-enable-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('snmp', 'enable'));
  });

  it('warns that disabling vendor-daemon stops video', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'vendor-daemon' }]));
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-vendor-daemon'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent('streams go dead');
  });

  it('shows the onvif consequence copy when disabling onvif', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'onvif' }]));
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent('only reachable via FTP');
  });

  it('disabling onvif does NOT enter the reconnecting state, even when the connection drops', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([{ ...SNMP, name: 'onvif' }]));
    vi.mocked(serviceAction).mockRejectedValue(new TypeError('fetch failed'));
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(serviceAction).toHaveBeenCalledWith('onvif', 'disable'));
    // A dropped connection on disable is the expected success path: the card
    // reports it as done, not as an error, and never waits for a comeback.
    expect(screen.queryByTestId('diagnostics-processes-reconnecting')).not.toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-onvif-off-note')).toBeInTheDocument();
  });

  it('an ApiError from disable is an error toast with no waiting', async () => {
    vi.mocked(getProcesses).mockResolvedValue(supervisedFixture([SNMP]));
    vi.mocked(serviceAction).mockRejectedValue(
      Object.assign(new Error('disable of snmp failed with status 503'), { name: 'ApiError' }),
    );
    renderWithProviders(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith(expect.stringContaining('503')));
    expect(screen.queryByTestId('diagnostics-processes-reconnecting')).not.toBeInTheDocument();
  });
});
```

Reuse this file's existing mock setup and query-provider wrapper rather than adding new ones.

- [ ] **Step 2: Run to verify they fail**

Run: `npm run test -- ProcessesCard`
Expected: new tests fail on missing ids/behaviour; existing tests fail on the renamed dialog ids.

- [ ] **Step 3: Implement**

State becomes an action union, and `handleConfirm` splits into the existing restart path (extracted verbatim, `waitForCameraBack` and all) plus a toggle path:

```tsx
type PendingAction = { service: ServiceStatus; action: 'restart' | 'disable' | 'enable' } | null;

const [pending, setPending] = useState<PendingAction>(null);
const [onvifOff, setOnvifOff] = useState(false);
```

Module scope:

```tsx
/// Services the supervisor refuses to toggle (anyka-init: NON_TOGGLEABLE).
/// Rendering a button that always 404s is worse than rendering none.
const NON_TOGGLEABLE = new Set(['wpa_supplicant']);

const DISABLE_COPY: Record<string, string> = {
  onvif:
    'Web, ONVIF and RTSP access end immediately. The camera stays up, but it is then only reachable via FTP (or the deadman telnet after a failed boot). Re-enabling requires FTP or an SD-card edit.',
  'vendor-daemon':
    'Video capture and encoding stop; streams go dead until it is re-enabled. The camera does not reboot — the video watchdog is stood down with it.',
  udhcpc:
    'Stops DHCP renewals, so a configured static address is no longer overwritten on renewal. The link watchdog still runs a one-shot udhcpc if the default route disappears.',
  snmp: 'SNMP polling stops.',
  dropbear: 'The SSH daemon will not run until it is re-enabled.',
};

const DEFAULT_ENABLE_COPY = 'The service starts immediately under the normal supervisor backoff policy.';
```

Row actions — Restart only when not disabled, then the toggle when the service is toggleable:

```tsx
const isDisabled = service.state === 'disabled';
const toggleable = !NON_TOGGLEABLE.has(service.name);
```

`<tr className={`border-border border-b last:border-b-0 ${isDisabled ? 'opacity-50' : ''}`}>`, and in the action cell: `{!isDisabled && <Button … action 'restart' …>}` then, when `toggleable`, either an Enable button (`{ service, action: 'enable' }`) or a Disable button (`{ service, action: 'disable' }`).

Badge: add the `disabled` case, `border-transparent bg-zinc-500/10 text-zinc-400`, keeping the existing running/backoff styles.

Dialog: one `AlertDialog` driven by `pending`, title `` `${verb} ${pending.service.name}?` ``, description from:

```tsx
function actionDescription(p: NonNullable<PendingAction>): string {
  if (p.action === 'restart') {
    return p.service.name === 'onvif'
      ? 'Restarting onvif also stops vendor-daemon — video and this page will drop with it and recover on their own when the camera returns.'
      : 'The supervisor sends SIGTERM; the service is restarted under its normal backoff policy.';
  }
  if (p.action === 'enable') {
    return DEFAULT_ENABLE_COPY;
  }
  return DISABLE_COPY[p.service.name] ?? 'The service stops and will not run again until re-enabled.';
}
```

Confirm handler:

```tsx
const p = pending;
if (!p) return;
setPending(null);

if (p.action === 'restart') {
  await runRestart(p.service, e); // existing body, extracted unchanged
  return;
}

// Disable/enable involves no reboot — except disabling onvif, which takes
// down the very HTTP server serving this page.
const isOnvifOff = p.action === 'disable' && p.service.name === 'onvif';
const reportOnvifOff = () => {
  setOnvifOff(true);
  toast.success('onvif disabled — the camera is reachable via FTP only');
};
try {
  await serviceAction(p.service.name, p.action);
  if (isOnvifOff) reportOnvifOff();
  else toast.success(`${p.service.name} ${p.action === 'enable' ? 'enabled' : 'disabled'}`);
  invalidate();
} catch (err) {
  if (err instanceof Error && err.name === 'ApiError') {
    toast.error(err.message);
  } else if (isOnvifOff) {
    // Network-level failure on an onvif disable: the only cause is the camera
    // killing our own connection — i.e. it worked.
    reportOnvifOff();
  } else {
    toast.error(err instanceof Error ? err.message : 'Toggle failed');
  }
}
```

And the terminal note, rendered beside the `reconnecting` note:

```tsx
{onvifOff && (
  <p className="text-muted-foreground text-sm" data-testid="diagnostics-processes-onvif-off-note" aria-live="polite">
    onvif is disabled — Web access is down until it is re-enabled via FTP or the SD card.
  </p>
)}
```

- [ ] **Step 4: Run to verify they pass**

Run: `npm run test -- ProcessesCard`
Expected: all card tests green.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/www/src/components/ProcessesCard.tsx cross-compile/www/src/components/ProcessesCard.test.tsx
rtk git commit -m "feat(webui): enable/disable service actions with per-service consequence warnings"
```

---

### Task 13: WebUI full gates

- [ ] **Step 1: Run all www gates**

```bash
cd cross-compile/www
npm run lint
npm run type-check
npm run test
npx prettier --check . ; echo "prettier exit: $?"
```

Expected: all green, and the prettier exit code is `0` — read the code, not the summary line. Commit fix-ups if any (never an ad-hoc `npx prettier@version`).

---

## Phase 4 — End-to-end verification

### Task 14: Deploy to camera 192.168.2.198 and verify on device

Uses the `anyka-firmware-upgrade` skill.

**Order matters, and Step 6 is destructive.** The safe toggles come first and prove the mechanism; the onvif disable proves the escape hatch but ends with the camera unreachable over HTTP. Read Step 6's preconditions before starting Step 1.

- [ ] **Step 1: Build the bundle**

```bash
source ./setenv.sh
scripts/anyka-hack/build_bundle.sh 2026-09-19-webui-service-toggle
```

Verify the manifest lists `anyka-init.bin`, `onvif-rust.bin`, and the `www` bundle. This feature adds no new `anyka.toml` key, so no config stanza needs appending and there is no rollback hazard for the other slot's older supervisor.

- [ ] **Step 2: Upload and monitor the trial**

```bash
curl -u admin:admin --fail -X PUT \
  -H "Content-Type: application/octet-stream" \
  --data-binary @bundle.tar \
  http://192.168.2.198/api/update
```

Expected: `202`. The camera reboots into the new slot; the trial (onvif still enabled → ports 80/554/8080) confirms after the ~30 s hold.

```bash
for i in $(seq 1 30); do
  sleep 10
  code=$(curl -s -o /dev/null -w '%{http_code}' -u admin:admin http://192.168.2.198/api/diagnostics)
  echo "t+$((i*10))s: $code"
  [ "$code" = "200" ] && break
done
```

If the camera never returns (trial revert → old slot), the old slot still has `e00f3a3b` — the feature is absent but the camera is healthy. Investigate before retrying.

- [ ] **Step 3: Verify status includes disabled services**

```bash
curl -s -u admin:admin http://192.168.2.198/api/processes \
  | python3 -c 'import json,sys; [print(r["name"], r["state"], r["pid"]) for r in json.load(sys.stdin)["supervised"]]'
```

Expected: rows for all six configured services; `dropbear` shows `disabled` with a null pid. Before this change only the five started services appeared.

- [ ] **Step 4: Toggle snmp, and check the error paths**

```bash
S=http://192.168.2.198/api/services
st() { curl -s -u admin:admin http://192.168.2.198/api/processes \
  | python3 -c 'import json,sys; print({r["name"]: r["state"] for r in json.load(sys.stdin)["supervised"]})'; }

curl -s -o /dev/null -w 'disable: %{http_code}\n' -u admin:admin -X POST $S/snmp/disable   # 202
sleep 3; st                                                                                # snmp: disabled
curl -s -o /dev/null -w 'idempotent: %{http_code}\n' -u admin:admin -X POST $S/snmp/disable # 202, no change
curl -s -o /dev/null -w 'restart-disabled: %{http_code}\n' -u admin:admin -X POST $S/snmp/restart # 409
curl -s -o /dev/null -w 'unknown: %{http_code}\n' -u admin:admin -X POST $S/nope/disable   # 404
curl -s -o /dev/null -w 'wpa: %{http_code}\n' -u admin:admin -X POST $S/wpa_supplicant/disable # 404
curl -s -o /dev/null -w 'enable: %{http_code}\n' -u admin:admin -X POST $S/snmp/enable     # 202
sleep 3; st                                                                                # snmp: running
```

Also confirm no `snmpd` appears in the raw process list while snmp is disabled, and that the toggle survives: the next step reboots the camera.

- [ ] **Step 5: Verify persistence across a reboot**

```bash
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST $S/snmp/disable   # 202
# Reboot: there is no reboot route, so re-upload the same bundle — the A/B
# apply reboots the camera, which is what we need.
curl -u admin:admin --fail -X PUT -H "Content-Type: application/octet-stream" \
  --data-binary @bundle.tar http://192.168.2.198/api/update
# wait for the camera as in Step 2, then:
st   # snmp must still read "disabled" — proving the anyka.toml edit survived
     # both the reboot and the slot flip
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST $S/snmp/enable   # restore
```

This is the only end-to-end check of §3's central claim (both slots read the same file). Do not skip it.

- [ ] **Step 6: Disable vendor-daemon and confirm the camera does NOT reboot**

This is the §2.1 regression check — the one that fails loudly if the heartbeat removal is missing.

```bash
UP() { curl -s -u admin:admin http://192.168.2.198/api/diagnostics \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["uptime_s"])'; }
UP
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST $S/vendor-daemon/disable  # 202
sleep 600   # well past 5 monitor ticks
UP          # must be MONOTONICALLY LARGER — a smaller value means it rebooted
st          # vendor-daemon: disabled
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST $S/vendor-daemon/enable
sleep 20; st  # vendor-daemon: running, video back
```

A drop in uptime here means the heartbeat file was not cleared — go back to Task 4, do not work around it in the monitor.

- [ ] **Step 7: Disable onvif — LAST, and only with a recovery path in hand**

**Preconditions. Read before running.** With onvif disabled: port 80 is gone, so `PUT /api/update` is gone with it. Telnet 24 is closed on a healthy boot. FTP refused both known credential pairs in the 2026-09-19 session. That leaves the `spool/` poll (a bundle dropped on the SD card, picked up within 60 s) — which needs FTP, or the card in your hand.

**Do this on a bench camera, or with physical access to the SD card. Do not run it on a remote `.198` you cannot walk to.**

```bash
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST $S/onvif/disable
# 202, or a dropped connection — both mean it worked
sleep 20
curl -s -o /dev/null -w 'http:%{http_code}\n' --connect-timeout 3 http://192.168.2.198/   # expect 000
ping -c 3 192.168.2.198   # the camera itself is still up
```

Then recover: drop a bundle into `spool/` (FTP or SD card), or push a full SD payload — the latter replaces `anyka.toml` and resets every toggle to shipped defaults. Confirm `/api/diagnostics` returns 200 and `st` shows shipped defaults (`dropbear` disabled, the rest enabled).

- [ ] **Step 8: Final state + push**

```bash
curl -s -u admin:admin http://192.168.2.198/api/diagnostics   # 200, services healthy
st                                                            # shipped defaults
rtk git status                                                # clean; no .vitest artifacts, no bundle.tar
rtk git push
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task(s) |
|---|---|
| §1 protocol (`enable`/`disable`, `ok`/`unknown`/`error`, 1 s < client 2 s, extended status) | 3, 5 |
| §2 lifecycle (`Disabled` state, single `decide` guard, boot-time insertion) | 2, 4 |
| §2.1 monitor (heartbeat removal, `wpa_supplicant` not toggleable) | 4, 12; verified in 14 Step 6 |
| §3 persistence (line-level, atomic, single source of truth, netoverlay comment) | 1; verified in 14 Step 5 |
| §4 adaptive trial (filter only; `evaluate_trial` untouched) | 6 |
| §5 HTTP (202/404/503, 409 on restarting a disabled service, admin-only) | 8, 9 |
| §6 WebUI (dimmed rows, per-service copy, no wpa toggle, no-wait onvif disable) | 11, 12 |
| §7 edge cases | 2, 4, 9; the apply-window row is true by construction — nothing but `set_service_enabled` writes `anyka.toml`, and the applier writes only slot files |
| §8 tests | every task's test steps |

**Placeholders:** Tasks 4 and 5 abbreviate repeated test *setup* with `// ...` where it is a verbatim repeat of the fully-written test above it. Every line of non-test code in this plan is complete. There are no "wrong version shown first" blocks — the earlier draft's two (Task 3's `send_toggle`, Task 12's row snippet) are gone.

**Type consistency:** `ToggleOutcome` (anyka-init) and `ToggleReply` (onvif-rust) are deliberately distinct — two crates, no shared code, each named for its own side. `SvcState::Disabled` is fieldless and used identically in Tasks 2/4/5. State strings are `"running" | "backoff" | "disabled"` on both sides of the socket and in the TS doc. `handle_query_status(cfg, services, reply_tx)` matches its call-site update in Task 5. `serviceAction`'s three action values match the three route segments exactly.

**Ordering:** 2 → 3 → 4 depend on each other's declarations. Task 3's temporary `dispatch_msg` arm must be **replaced** by Task 4's, not duplicated. Task 4's `LoopCtx` refactor is its own green checkpoint before the handler lands — if the argument-count gate is going to fail, it fails there, cheaply.
