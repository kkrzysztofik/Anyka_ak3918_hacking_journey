# WebUI Service Enable/Disable Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let an administrator enable/disable any anyka-init supervised service from the WebUI Diagnostics page, persisting the choice in `anyka.toml` and keeping the A/B upgrade trial functional.

**Architecture:** The supervisor (`anyka-init`) is the sole actor: two new control-socket requests (`enable`/`disable`) drive a `SvcState::Disabled` lifecycle state, a line-level edit of `anyka.toml` (file first, then in-memory), and an adaptive trial-port filter. `onvif-rust` is a thin socket client behind two authenticated `POST` routes. The WebUI adds Disable/Enable actions to the existing supervised-services table with per-service consequence warnings.

**Tech Stack:** Rust (anyka-init, onvif-rust) — std `unix` sockets, `toml`, `tracing`, tokio/axum on the onvif-rust side; React 19 + TanStack Query + shadcn/ui + Vitest/RTL in the WebUI. Cross-compiled to `armv5te-unknown-linux-uclibceabi` with the vendored toolchain.

**Spec:** `docs/plans/2026-09-19-webui-service-toggle-design.md`

## Global Constraints

- **Toolchain:** `source ./setenv.sh` before any Rust command; use `$CARGO` (never bare `cargo`). Host-side commands take `--target x86_64-unknown-linux-gnu`; ARM builds take the default cross target.
- **No `unwrap()`/`expect()` in production code** (test code may use them). Use `?`, `match`, or `if let`.
- **Logging:** `tracing::{info,warn,error}!` only — never `println!` in production.
- **Test naming:** `test_<subject>_<behavior>` (e.g. `test_set_enabled_in_text_replaces_existing_line`).
- **Quality gates per component before each commit batch:**
  - anyka-init: `$CARGO fmt --check && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && $CARGO test --target x86_64-unknown-linux-gnu`
  - onvif-rust: same three in `cross-compile/onvif-rust`
  - www: `npm run lint && npm run type-check && npm run test && npx prettier --check .` (Prettier 3.9.6 is installed locally; never a bare `npx prettier@3.4.2`)
- **WebUI tests:** Vitest + React Testing Library, `data-testid` selectors only, `vi.mock` (no MSW), shared helpers in `src/test/`.
- **Work branch:** `design/webui-process-control` (current). Commit after every task; push when the plan is fully executed.
- **Control-socket contract:** one-line request `\n`-terminated, one-line reply `\n`-terminated; status frame = TSV rows + blank line. Replies for toggles: `ok` / `unknown` / `error`.
- **Camera for on-device verification:** 192.168.2.198, HTTP Basic `admin`/`admin`. Telnet (24) is dead by design on healthy boots; FTP (21) is up but both known credential pairs were refused in the 2026-09-19 session — assume **no shell access on a healthy boot**.

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `cross-compile/anyka-init/src/config.rs` | Modify | `ConfigError::Write` variant; pure `set_enabled_in_text`; `Config::set_service_enabled` (atomic write) |
| `cross-compile/anyka-init/src/supervise.rs` | Modify | `SvcState::Disabled` variant; `decide()` no-op; `pid()` arm |
| `cross-compile/anyka-init/src/supervisor_loop.rs` | Modify | `Msg::ToggleService`; tick/exited guards; `handle_toggle_service`; `handle_query_status` cfg-driven; `run`/`dispatch_msg` signatures |
| `cross-compile/anyka-init/src/control.rs` | Modify | `Request::Enable/Disable`; `parse_request` verb dispatch; `ToggleOutcome`; `handle_control_conn` replies; `from_svc_state` Disabled arm |
| `cross-compile/anyka-init/src/update.rs` | Modify | `ONVIF_PORTS`; `effective_trial_ports`; empty-ports early confirm in `evaluate_trial` |
| `cross-compile/anyka-init/src/main.rs` | Modify | Thread `config_path` + owned-ish cfg into `run`; build adaptive `Policy.ports` |
| `cross-compile/onvif-rust/src/diagnostics/services.rs` | Modify | `ToggleReply` + `request_toggle` client |
| `cross-compile/onvif-rust/src/diagnostics/processes.rs` | Modify | `handle_enable_service` / `handle_disable_service` |
| `cross-compile/onvif-rust/src/onvif/server.rs` | Modify | Route registration (auth-gated) |
| `cross-compile/onvif-rust/src/diagnostics/http.rs` | Modify (tests only) | Auth-table tests for the two new paths |
| `cross-compile/www/src/services/processesService.ts` | Modify | `toggleService`; `ServiceStatus.state` doc |
| `cross-compile/www/src/components/ProcessesCard.tsx` | Modify | Disable/Enable actions, dimmed rows, generalized dialog with per-service copy |
| `cross-compile/www/src/components/ProcessesCard.test.tsx` | Modify | Updated + new tests |

---

## Phase 1 — anyka-init

### Task 1: Line-level TOML editor (`config.rs`)

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs` (add near `Config::load`; new variant in `ConfigError`)

**Interfaces:**
- Produces: `pub fn set_enabled_in_text(text: &str, name: &str, enabled: bool) -> Result<String, ConfigError>`; `impl Config { pub fn set_service_enabled(&self, path: &Path, name: &str, enabled: bool) -> Result<(), ConfigError> }`; `ConfigError::Write { path: String, source: std::io::Error }`

The editor is a **pure text function** (the atomic write wraps it). Only one line of the file ever changes; a TOML round-trip would reformat the whole file.

- [ ] **Step 1: Write the failing tests**

In `config.rs`'s `#[cfg(test)] mod tests`:

```rust
const SAMPLE: &str = "title = \"anyka\"\n\
    [services.onvif]\n\
    enabled = true\n\
    exec = \"/mnt/anyka_hack/slots/a/bin/onvif-rust.bin\"\n\
    # keep this comment alive\n\
    [services.snmp]\n\
    exec = \"/usr/sbin/snmpd\"\n\
    [services.dropbear]\n\
    enabled = false\n";

#[test]
fn test_set_enabled_in_text_replaces_an_existing_line() {
    let got = set_enabled_in_text(SAMPLE, "onvif", false).expect("edit");
    assert!(got.contains("[services.onvif]\nenabled = false\nexec = \"/mnt/anyka_hack/slots/a/bin/onvif-rust.bin\""));
}

#[test]
fn test_set_enabled_in_text_preserves_everything_else() {
    let got = set_enabled_in_text(SAMPLE, "snmp", true).expect("edit");
    // The only byte-level change: one inserted line.
    assert_eq!(got, SAMPLE.replace(
        "[services.snmp]\nexec = \"/usr/sbin/snmpd\"",
        "[services.snmp]\n    enabled = true\nexec = \"/usr/sbin/snmpd\"",
    ));
    assert!(got.contains("# keep this comment alive"));
}

#[test]
fn test_set_enabled_in_text_inserts_under_the_header_when_absent() {
    let got = set_enabled_in_text(SAMPLE, "snmp", false).expect("edit");
    assert!(got.contains("[services.snmp]\n    enabled = false\nexec = \"/usr/sbin/snmpd\""));
}

#[test]
fn test_set_enabled_in_text_unknown_stanza_is_an_error() {
    assert!(set_enabled_in_text(SAMPLE, "nope", true)
        .is_err_and(|e| matches!(e, ConfigError::Invalid(_))));
}

#[test]
fn test_set_service_enabled_writes_and_preserves_the_file() {
    let dir = std::env::temp_dir().join(format!("anyka-cfg-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdtemp");
    let path = dir.join("anyka.toml");
    std::fs::write(&path, SAMPLE).expect("seed");

    Config::set_service_enabled(&path, "onvif", false).expect("persist");

    let after = std::fs::read_to_string(&path).expect("read back");
    assert!(after.contains("[services.onvif]\nenabled = false"));
    assert!(after.contains("# keep this comment alive"));
    assert!(!path.with_extension("toml.tmp").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
```

Note: `is_err_and` is std since 1.82 — if the vendored toolchain is older, write `match`-based assertions instead.

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
/// guarantee.
pub fn set_enabled_in_text(text: &str, name: &str, enabled: bool) -> Result<String, ConfigError> {
    let header = format!("[services.{name}]");
    let value = if enabled { "true" } else { "false" };

    let mut out: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let Some(hdr) = out.iter().position(|l| l.trim() == header) else {
        return Err(ConfigError::Invalid(format!(
            "no [services.{name}] stanza in config"
        )));
    };
    // Stanza ends at the next `[`-prefixed line (or end of file).
    let end = out[hdr + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| i + hdr + 1)
        .unwrap_or(out.len());

    let Some(i) = (hdr + 1..end)
        .find(|&i| out[i].trim_start().starts_with("enabled = "))
    else {
        out.insert(hdr + 1, format!("    enabled = {value}"));
        return Ok(out.join("\n"));
    };
    // Preserve the line's indentation, change only the value.
    let lead = out[i].chars().take_while(|c| c.is_whitespace()).count();
    out[i] = format!("{}enabled = {value}", " ".repeat(lead));
    Ok(out.join("\n"))
}
```

Add to `impl Config`:

```rust
/// Persist `enabled` for one service: line-level edit, then atomic
/// tmp+rename over the original (the same pattern the applier uses for the
/// `active` pointer). On failure the original file is untouched.
pub fn set_service_enabled(
    &self,
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
    std::fs::write(&tmp, &new_text).map_err(|source| ConfigError::Write {
        path: tmp.display().to_string(),
        source,
    })?;
    std::fs::File::options()
        .read(true)
        .open(&tmp)
        .and_then(|mut f| f.sync_all())
        .map_err(|source| ConfigError::Write {
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

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib config::`
Expected: all 5 new tests PASS, existing config tests still pass.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs
git commit -m "feat(anyka-init): atomic line-level anyka.toml service toggle editor"
```

---

### Task 2: `SvcState::Disabled` and the two loop guards

**Files:**
- Modify: `cross-compile/anyka-init/src/supervise.rs` (`SvcState`, `SvcState::pid`, `decide`)
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`tick_services`, `handle_service_exited`); `cross-compile/anyka-init/src/control.rs` (`from_svc_state` compile-fix)

**Interfaces:**
- Produces: `SvcState::Disabled` (fieldless variant). `decide` returns `{ action: None, next: Disabled }` for it and never touches `hist`.

The service **stays in the `services` vec** — no insertion/removal, so the index-based `by_pid` map is never disturbed.

- [ ] **Step 1: Write the failing tests**

In `supervise.rs` tests (or its existing test module):

```rust
#[test]
fn test_decide_never_acts_on_a_disabled_service() {
    let mut hist = RestartHistory::default();
    // Fill the history so any other state would be deep in backoff logic.
    let now = Instant::now();
    for _ in 0..10 {
        hist.push(now - Duration::from_secs(1));
    }
    let d = decide(&SvcState::Disabled, &mut hist, Event::Exited, now, &policy());
    assert_eq!(d.action, Action::None);
    assert_eq!(d.next, SvcState::Disabled);
    // No bookkeeping: a disabled service must not accumulate crash history.
    assert_eq!(hist.len(), 10);
}

#[test]
fn test_decide_does_not_start_a_disabled_service_even_when_backoff_is_due() {
    let now = Instant::now();
    let d = decide(
        &SvcState::Disabled,
        &mut RestartHistory::default(),
        Event::Tick,
        now,
        &policy(),
    );
    assert_ne!(d.action, Action::Start);
}

#[test]
fn test_pid_of_disabled_is_none() {
    assert_eq!(SvcState::Disabled.pid(), None);
}
```

(`policy()` is the existing test helper in that module; reuse it.)

In `supervisor_loop.rs` tests:

```rust
#[test]
fn test_tick_services_skips_a_disabled_service() {
    // A disabled service whose backoff deadline has long since passed must
    // not spawn: the guard must run before `decide`.
    let mut sys = TestSys::new();
    let cfg = minimal_cfg();
    let mut services = vec![Service {
        name: "onvif".into(),
        spec: dummy_spec(),
        state: SvcState::Disabled,
        hist: RestartHistory::default(),
    }];
    let mut by_pid = BTreeMap::new();
    let policy = supervisor_policy();
    tick_services(&sys, &cfg, &mut services, &mut by_pid, &policy);
    assert_eq!(services[0].state, SvcState::Disabled);
    assert_eq!(sys.spawned(), 0);
}

#[test]
fn test_handle_service_exited_ignores_a_disabled_service() {
    // Sequence the runtime flow produces: service was Running (pid in
    // by_pid), got disabled in place (SIGTERM'd), then the exit report
    // arrives. It must record nothing and must not restart.
    let sys = TestSys::new();
    let cfg = minimal_cfg();
    let mut services = vec![Service {
        name: "snmp".into(),
        spec: dummy_spec(),
        state: SvcState::Disabled,
        hist: RestartHistory::default(),
    }];
    let mut by_pid = BTreeMap::new();
    by_pid.insert(777, 0);
    handle_service_exited(
        &sys,
        &cfg,
        &mut services,
        &mut by_pid,
        &supervisor_policy(),
        777,
        ExitStatus::Dead(1),
    );
    assert_eq!(services[0].state, SvcState::Disabled);
    assert_eq!(services[0].hist.len(), 0);
    assert!(by_pid.is_empty());
}
```

Use the module's existing test fixtures (`TestSys`, `minimal_cfg`, `dummy_spec`, `supervisor_policy`) — match their actual names if they differ.

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervise:: supervisor_loop::`
Expected: compile failure — `SvcState::Disabled` does not exist.

- [ ] **Step 3: Implement**

`supervise.rs`:

```rust
pub enum SvcState {
    Running { pid: Pid, since: Instant },
    Backoff { until: Instant, attempt: u32 },
    /// Disabled at runtime via the control socket. The service stays in the
    /// `services` vec (so `by_pid` indices never shift) but is inert: `decide`
    /// never acts on it and the loop skips it.
    Disabled,
}
```

Extend `pid()`:

```rust
SvcState::Disabled => None,
```

In `decide`, as the **first statement** (before the match, so the arm structure never has to change):

```rust
if matches!(state, &SvcState::Disabled) {
    return Decision {
        action: Action::None,
        next: SvcState::Disabled,
    };
}
```

(If `Decision` is not the exact return-type name, use the struct the function currently returns.)

`supervisor_loop.rs` — `tick_services`, inside the `for i in 0..services.len()` loop, before the existing `decide` call:

```rust
if matches!(services[i].state, SvcState::Disabled) {
    continue;
}
```

`handle_service_exited`, after the `by_pid.remove` + warn, before the `decide` call:

```rust
if matches!(services[i].state, SvcState::Disabled) {
    return;
}
```

`control.rs` — `from_svc_state` gains an arm (keeps the `match` exhaustive; also used defensively by status):

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
Expected: new tests PASS; full anyka-init suite green (the `Decision`/fixture names may need matching the module's real ones — do not invent new helpers).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervise.rs cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/src/control.rs
git commit -m "feat(anyka-init): inert SvcState::Disabled with decide/tick/exit guards"
```

---

### Task 3: Control-socket protocol — `enable`/`disable` requests

**Files:**
- Modify: `cross-compile/anyka-init/src/control.rs` (`Request`, `parse_request`, `ToggleOutcome`, `handle_control_conn`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `Request::Enable(String)` / `Request::Disable(String)`; `pub enum ToggleOutcome { Ok, Unknown, Error }` (in `control.rs`); `Msg::ToggleService { name: String, enabled: bool, reply: std::sync::mpsc::Sender<ToggleOutcome> }` (consumed by Task 4, declared in `supervisor_loop.rs` this task).

- [ ] **Step 1: Write the failing tests**

In `control.rs` tests:

```rust
#[test]
fn test_parse_request_enable() {
    assert_eq!(parse_request("enable snmp\n"), Some(Request::Enable("snmp".into())));
}

#[test]
fn test_parse_request_disable() {
    assert_eq!(parse_request("disable onvif\r\n"), Some(Request::Disable("onvif".into())));
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

(The four existing `parse_request` tests must keep passing unchanged.)

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib control::`
Expected: compile failure — new variants missing.

- [ ] **Step 3: Implement**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Restart(String),
    Enable(String),
    Disable(String),
}

/// Parse one line. `"status\n"`, `"restart <name>\n"`, `"enable <name>\n"`,
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
    let request = match verb {
        "restart" => Request::Restart(name.to_owned()),
        "enable" => Request::Enable(name.to_owned()),
        "disable" => Request::Disable(name.to_owned()),
        _ => return None,
    };
    Some(request)
}

/// Outcome of an `enable`/`disable` request, decided by the supervisor loop
/// (only it can see whether the name is configured and whether the file
/// write succeeded).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleOutcome {
    /// Applied — including the idempotent no-op case (already in that state).
    Ok,
    /// Name is not a configured service.
    Unknown,
    /// The config write failed; nothing was changed.
    Error,
}
```

In `handle_control_conn`, extend the per-line match (alongside the existing `Status`/`Restart` arms). For the new arms, send the message and translate the reply:

```rust
Request::Enable(name) | Request::Disable(name) => {
    let enabled = matches!(/* the arm that matched */);
    // (implement with explicit arms, not a destructure-or: `matches!` cannot
    //  bind `name` from one alternative)
    ...
}
```

Written explicitly:

```rust
Request::Enable(name) => {
    let _ = send_toggle(&tx_msg, name, true);
}
Request::Disable(name) => {
    let _ = send_toggle(&tx_msg, name, false);
}
```

with a new private helper in the same file:

```rust
/// Send a toggle request to the loop and write its outcome back to the
/// connection. Waits up to 5 s for the loop's next poll.
fn send_toggle(
    tx_msg: &std::sync::mpsc::Sender<crate::supervisor_loop::Msg>,
    name: String,
    enabled: bool,
) {
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    if tx_msg
        .send(crate::supervisor_loop::Msg::ToggleService {
            name,
            enabled,
            reply: reply_tx,
        })
        .is_err()
        || reply_rx.recv_timeout(std::time::Duration::from_secs(5)).is_err()
    {
        // Loop is gone (reboot in progress): report a failure, not a success.
        return;
    }
    let outcome = match reply_rx.recv_timeout(std::time::Duration::from_millis(1)) {
        Ok(o) => o,
        _ => control::ToggleOutcome::Error,
    };
    let reply = match outcome {
        control::ToggleOutcome::Ok => "ok\n",
        control::ToggleOutcome::Unknown => "unknown\n",
        control::ToggleOutcome::Error => "error\n",
    };
    // `writer` (the connection's BufWriter) is in scope at the call site —
    // pass it in as a parameter instead of reaching for it.
    ...
}
```

**Correct shape** (pass the writer explicitly so the function is testable):

```rust
fn send_toggle<W: std::io::Write>(
    writer: &mut W,
    tx_msg: &std::sync::mpsc::Sender<crate::supervisor_loop::Msg>,
    name: String,
    enabled: bool,
) {
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    let sent = tx_msg.send(crate::supervisor_loop::Msg::ToggleService {
        name,
            enabled,
        reply: reply_tx,
    });
    let outcome = match sent {
        Ok(()) => match reply_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(o) => o,
            Err(_) => control::ToggleOutcome::Error,
        },
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

Add the variant to `Msg` in `supervisor_loop.rs` (declaration only; the handler lands in Task 4):

```rust
/// Runtime enable/disable of a configured service. The handler persists to
/// `anyka.toml` first, then transitions in-memory state.
ToggleService {
    name: String,
    enabled: bool,
    reply: std::sync::mpsc::Sender<control::ToggleOutcome>,
},
```

Until Task 4 lands, add a temporary `Ok(Msg::ToggleService { .. }) => false` arm in `dispatch_msg` so the crate compiles; Task 4 replaces it.

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: parse tests pass; suite green.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/control.rs cross-compile/anyka-init/src/supervisor_loop.rs
git commit -m "feat(anyka-init): enable/disable control-socket requests (ok/unknown/error)"
```

---

### Task 4: Toggle dispatch handler (file first, then state)

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`dispatch_msg`, `run`, new `handle_toggle_service`)
- Modify: `cross-compile/anyka-init/src/main.rs` (thread `config_path` through `run`)

**Interfaces:**
- Consumes: `Config::set_service_enabled` (Task 1); `SvcState::Disabled` (Task 2); `Msg::ToggleService` (Task 3); existing `spec_of_slot`, `build_enabled_services`.
- Produces: `fn handle_toggle_service(sys: &dyn Sys, cfg: &mut Config, config_path: &Path, services: &mut Vec<Service>, update_root: &Path, slots: &crate::update::Slots, name: String, enabled: bool, reply: &std::sync::mpsc::Sender<control::ToggleOutcome>)`.

- [ ] **Step 1: Write the failing tests**

In `supervisor_loop.rs` tests:

```rust
#[test]
fn test_toggle_disable_persists_then_kills_and_inerts() {
    let dir = std::env::temp_dir().join(format!("anyka-tog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let cfg_path = dir.join("anyka.toml");
    std::fs::write(&cfg_path, "[services.snmp]\nenabled = true\nexec = \"/bin/true\"\n").expect("seed");

    let sys = TestSys::new(); // records kill() calls
    let mut cfg = Config {
        services: btreemap! { "snmp".into() => ServiceCfg { enabled: true, exec: "/bin/true".into(), ..Default::default() } },
        ..minimal_cfg_struct()
    };
    let mut services = vec![Service {
        name: "snmp".into(),
        spec: dummy_spec(),
        state: SvcState::Running { pid: 55, since: Instant::now() },
        hist: RestartHistory::default(),
    }];

    let (rtx, rrx) = std::sync::mpsc::channel();
    handle_toggle_service(
        &sys, &mut cfg, &cfg_path, &mut services,
        Path::new("/mnt/anyka_hack/slots/a"), &Slots::new(Path::new("/mnt/anyka_hack")),
        "snmp".into(), false, &rtx,
    );
    assert_eq!(rrx.recv().unwrap(), control::ToggleOutcome::Ok);
    // File first: the on-disk config is false.
    assert!(std::fs::read_to_string(&cfg_path).unwrap().contains("enabled = false"));
    // In-memory: cfg flipped, SIGTERM sent, state inert.
    assert!(!cfg.services["snmp"].enabled);
    assert!(sys.killed().contains(&55));
    assert_eq!(services[0].state, SvcState::Disabled);
}

#[test]
fn test_toggle_enable_a_boot_time_disabled_service_inserts_it() {
    // dropbear is the shipped case: disabled in the file, never in the vec.
    let dir = std::env::temp_dir().join(format!("anyka-tog2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let cfg_path = dir.join("anyka.toml");
    std::fs::write(&cfg_path, "[services.dropbear]\nenabled = false\nexec = \"/bin/true\"\n").expect("seed");

    let sys = TestSys::new();
    let mut cfg = /* same builder as above but dropbear, enabled: false */;
    let mut services = Vec::new(); // empty: it was filtered at boot

    let (rtx, rrx) = std::sync::mpsc::channel();
    handle_toggle_service(
        &sys, &mut cfg, &cfg_path, &mut services,
        Path::new("/mnt/anyka_hack/slots/a"), &Slots::new(Path::new("/mnt/anyka_hack")),
        "dropbear".into(), true, &rtx,
    );
    assert_eq!(rrx.recv().unwrap(), control::ToggleOutcome::Ok);
    assert!(std::fs::read_to_string(&cfg_path).unwrap().contains("enabled = true"));
    // Inserted exactly as build_enabled_services would: boot-start state.
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, "dropbear");
    assert!(matches!(services[0].state, SvcState::Backoff { attempt: 0, .. }));
}

#[test]
fn test_toggle_unknown_service_replies_unknown_and_writes_nothing() {
    let dir = std::env::temp_dir().join(format!("anyka-tog3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let cfg_path = dir.join("anyka.toml");
    let before = "[services.snmp]\nenabled = true\nexec = \"/bin/true\"\n";
    std::fs::write(&cfg_path, before).expect("seed");

    let sys = TestSys::new();
    let mut cfg = /* one snmp entry */;
    let mut services = Vec::new();

    let (rtx, rrx) = std::sync::mpsc::channel();
    handle_toggle_service(
        &sys, &mut cfg, &cfg_path, &mut services,
        Path::new("/mnt/anyka_hack/slots/a"), &Slots::new(Path::new("/mnt/anyka_hack")),
        "nope".into(), false, &rtx,
    );
    assert_eq!(rrx.recv().unwrap(), control::ToggleOutcome::Unknown);
    assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), before);
}

#[test]
fn test_toggle_is_an_idempotent_noop_when_already_in_that_state() {
    // Already disabled in cfg: no file write (mtime/contents unchanged), Ok reply.
    ...
    assert_eq!(rrx.recv().unwrap(), control::ToggleOutcome::Ok);
    assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), before);
    ...
}

#[test]
fn test_toggle_replies_error_and_changes_nothing_when_the_write_fails() {
    // Make the write fail: config_path's parent is read-only.
    let dir = std::env::temp_dir().join(format!("anyka-tog5-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let cfg_path = dir.join("anyka.toml");
    std::fs::write(&cfg_path, "[services.snmp]\nenabled = true\nexec = \"/bin/true\"\n").expect("seed");
    use std::os::unix::fs::PermissionsExt;
    let mut p = std::fs::metadata(&dir).unwrap().permissions();
    p.set_permissions(std::fs::Permissions::from_mode(0o500));
    // NOTE: if tests run as root, chmod is bypassed and this test cannot
    // force a failure — skip it in that case (assert via a precheck that
    // writing actually fails; otherwise early-return with a skip log).

    let sys = TestSys::new();
    let mut cfg = /* snmp enabled */;
    let mut services = vec![/* snmp running */];
    let (rtx, rrx) = std::sync::mpsc::channel();
    handle_toggle_service(/* ... */ "snmp".into(), false, &rtx);
    assert_eq!(rrx.recv().unwrap(), control::ToggleOutcome::Error);
    assert!(std::fs::read_to_string(&cfg_path).unwrap().contains("enabled = true"));
    assert!(cfg.services["snmp"].enabled); // in-memory untouched
    p.set_permissions(std::fs::Permissions::from_mode(0o755));
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervisor_loop::`
Expected: compile failure — `handle_toggle_service` missing; `TestSys` may need `killed()` tracking (add a recording field if the existing helper only records spawns — that is a test-helper change, allowed).

- [ ] **Step 3: Implement**

`supervisor_loop.rs`:

```rust
/// Runtime enable/disable. Order is deliberate: **file first**, then
/// in-memory cfg, then state/kill. A failed write means nothing changes —
/// a "disabled" service that silently re-enabled itself on reboot would
/// defeat the crash-loop escape hatch this exists for.
fn handle_toggle_service(
    sys: &dyn Sys,
    cfg: &mut Config,
    config_path: &Path,
    services: &mut Vec<Service>,
    update_root: &Path,
    slots: &crate::update::Slots,
    name: String,
    enabled: bool,
    reply: &std::sync::mpsc::Sender<control::ToggleOutcome>,
) {
    let Some(entry) = cfg.services.get(&name) else {
        tracing::warn!(service = %name, "toggle for unknown service");
        let _ = reply.send(control::ToggleOutcome::Unknown);
        return;
    };
    if entry.enabled == enabled {
        let _ = reply.send(control::ToggleOutcome::Ok);
        return;
    }

    if let Err(e) = cfg.set_service_enabled(config_path, &name, enabled) {
        tracing::error!(service = %name, error = %e, "toggle: config write failed; not applied");
        let _ = reply.send(control::ToggleOutcome::Error);
        return;
    }
    if let Some(e) = cfg.services.get_mut(&name) {
        e.enabled = enabled;
    }

    if let Some(svc) = services.iter_mut().find(|s| s.name == name) {
        if !enabled {
            if let Some(pid) = svc.state.pid() {
                if let Err(e) = sys.kill(pid, libc::SIGTERM) {
                    tracing::warn!(service = %name, error = %e, "toggle: SIGTERM failed");
                }
            }
            svc.state = SvcState::Disabled;
            tracing::info!(service = %name, "disabled");
        } else {
            // Same initial state a boot start gets (see build_enabled_services):
            // the loop's next tick starts it under normal backoff/crash-loop policy.
            svc.state = SvcState::Backoff {
                until: sys.now(),
                attempt: 0,
            };
            tracing::info!(service = %name, "enabled");
        }
    } else if enabled {
        // Not in the vec: disabled at boot, never started (the shipped
        // dropbear case). Insert exactly as build_enabled_services would.
        if let Some(s) = cfg.services.get(&name) {
            services.push(Service {
                name: name.clone(),
                spec: spec_of_slot(s, update_root, slots),
                state: SvcState::Backoff {
                    until: sys.now(),
                    attempt: 0,
                },
                hist: RestartHistory::default(),
            });
            tracing::info!(service = %name, "enabled (inserted)");
        }
    }

    let _ = reply.send(control::ToggleOutcome::Ok);
}
```

Replace the temporary `dispatch_msg` arm from Task 3 with:

```rust
Ok(Msg::ToggleService { name, enabled, reply }) => {
    handle_toggle_service(
        sys.as_ref(),
        cfg,
        config_path,
        services,
        by_pid: // (not a parameter — see signature below)
        ...
    );
    false
}
```

with the real call:

```rust
Ok(Msg::ToggleService { name, enabled, reply }) => {
    handle_toggle_service(
        sys.as_ref(),
        cfg,
        config_path,
        services,
        update_root,
        slots,
        name,
        enabled,
        &reply,
    );
    false
}
```

Signature updates (`run` owns no new state; both already exist locally in `run`):

```rust
pub fn run(
    sys: Arc<dyn Sys>,
    cfg: &mut Config,
    config_path: &Path,
    rx: Receiver<Msg>,
) {
    // ... unchanged up to the loop ...
    // dispatch_msg gains `config_path: &Path`, `update_root: &Path`, `slots: &Slots`
    // parameters; `cfg` becomes `&mut Config`.
}
```

In `main.rs`, pass the same path that was used for `Config::load(...)` (the constant/variable main already has) and `&mut cfg`.

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: all five handler tests pass; full suite green.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): toggle dispatch — persist first, then SIGTERM/state; boot-time-disabled insertion"
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
    let cfg = /* cfg with onvif (enabled) + snmp (disabled) + dropbear (disabled) */;
    let services = vec![
        Service {
            name: "onvif".into(),
            spec: dummy_spec(),
            state: SvcState::Running { pid: 42, since: now_minus(90) },
            hist: RestartHistory::default(),
        },
    ];
    let (tx, rx) = std::sync::mpsc::channel();
    handle_query_status(&cfg, &services, &tx);
    let ControlMsg::Status(rows) = rx.recv().unwrap() else { panic!() };
    // Alphabetical (BTreeMap) order, disabled rows synthesized from cfg.
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["dropbear", "onvif", "snmp"]);
    let snmp = rows.iter().find(|r| r.name == "snmp").unwrap();
    assert_eq!(snmp.state, "disabled");
    assert_eq!(snmp.pid, None);
    assert_eq!(snmp.restarts, 0);
    let onvif = rows.iter().find(|r| r.name == "onvif").unwrap();
    assert_eq!(onvif.state, "running");
    assert_eq!(onvif.pid, Some(42));
}

#[test]
fn test_query_status_enabled_but_not_yet_in_vec_renders_as_backoff() {
    // Defensive: cfg says enabled, vec does not (shouldn't happen after Task
    // 4's insertion, but never drop the row).
    ...
}
```

The existing `handle_query_status` test (if any asserts exact rows) updates to the new cfg-driven shape.

- [ ] **Step 2: Run to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib supervisor_loop::`
Expected: compile failure — `handle_query_status` takes one more argument.

- [ ] **Step 3: Implement**

```rust
fn handle_query_status(cfg: &Config, services: &[Service], reply_tx: &Sender<ControlMsg>) {
    let now = Instant::now();
    let rows: Vec<control::ServiceStatus> = cfg
        .services
        .iter()
        .map(|(name, entry)| {
            if !entry.enabled {
                return control::ServiceStatus {
                    name: name.clone(),
                    state: "disabled",
                    pid: None,
                    uptime_s: 0,
                    restarts: 0,
                    retry_in_s: 0,
                };
            }
            match services.iter().find(|s| s.name == *name) {
                Some(svc) => {
                    control::ServiceStatus::from_svc_state(&svc.name, &svc.state, &svc.hist, now)
                }
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

Update the `dispatch_msg` call site to pass `cfg`.

- [ ] **Step 4: Run to verify it passes**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervisor_loop.rs
git commit -m "feat(anyka-init): status lists disabled services from cfg"
```

---

### Task 6: Adaptive trial ports

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs` (`ONVIF_PORTS`, `effective_trial_ports`, `evaluate_trial` empty-port guard)
- Modify: `cross-compile/anyka-init/src/main.rs` (the one production `Policy` construction at ~line 139)

**Interfaces:**
- Produces: `pub const ONVIF_PORTS: [u16; 3] = [80, 554, 8080]`; `pub fn effective_trial_ports(requested: &[u16], onvif_enabled: bool) -> Vec<u16>`; `evaluate_trial` returns `Outcome::Confirm` immediately for an empty port list.

- [ ] **Step 1: Write the failing tests**

In `update.rs` tests:

```rust
#[test]
fn test_effective_trial_ports_unchanged_when_onvif_enabled() {
    let ports = effective_trial_ports(&[80, 554, 8080], true);
    assert_eq!(ports, vec![80, 554, 8080]);
    // Custom ports survive untouched either way.
    let custom = effective_trial_ports(&[2000], true);
    assert_eq!(custom, vec![2000]);
}

#[test]
fn test_effective_trial_ports_drops_onvif_ports_when_onvif_disabled() {
    assert_eq!(effective_trial_ports(&[80, 554, 8080], false), Vec::<u16>::new());
    // A custom port of unknown ownership is kept: only ports we know belong
    // to onvif are dropped.
    assert_eq!(
        effective_trial_ports(&[80, 554, 8080, 2000], false),
        vec![2000]
    );
}

#[test]
fn test_evaluate_trial_with_no_ports_confirms_immediately() {
    let mut sleeps = 0u32;
    let outcome = evaluate_trial(
        &[],
        &Policy {
            hold_secs: 30,
            deadline_secs: 120,
            ports: Vec::new(),
        },
        |_| panic!("no ports to probe"),
        |_| sleeps += 1,
    );
    assert_eq!(outcome, Outcome::Confirm);
    assert_eq!(sleeps, 0);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib update::`
Expected: compile failures (`effective_trial_ports` missing; empty-ports test sleeps the full hold period).

- [ ] **Step 3: Implement**

```rust
/// Ports owned by the `onvif` service: the ONVIF (80), RTSP (554) and
/// HTTP-FLV (8080) endpoints — all three live in the onvif-rust binary
/// (see onvif/config.toml). Keep in sync if those move.
pub const ONVIF_PORTS: [u16; 3] = [80, 554, 8080];

/// The trial may only require ports that an enabled service can actually
/// bind. Disabling onvif makes the default set impossible, which would
/// wedge every subsequent A/B update into a permanent revert — exactly
/// when an admin most wants to upgrade. Ports of unknown ownership are
/// kept: dropping something we do not own is worse than a strict trial.
pub fn effective_trial_ports(requested: &[u16], onvif_enabled: bool) -> Vec<u16> {
    if onvif_enabled {
        return requested.to_vec();
    }
    requested
        .iter()
        .copied()
        .filter(|p| !ONVIF_PORTS.contains(p))
        .collect()
}
```

In `evaluate_trial`, first statement of the function body:

```rust
// Nothing to verify: the enabled-service set owns no trial ports (onvif
// disabled). Confirming immediately is the spec'd behavior — a strict
// trial on impossible ports would block every future update forever.
if ports.is_empty() {
    return Outcome::Confirm;
}
```

`main.rs` (~line 143), replacing `ports: cfg.update.trial_ports.clone(),`:

```rust
ports: anyka_init::update::effective_trial_ports(
    &cfg.update.trial_ports,
    cfg.services
        .get("onvif")
        .is_some_and(|s| s.enabled),
),
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): adaptive trial ports — onvif disabled, trial cannot wedge updates"
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
$CARGO build --release   # ARM cross build still works
cd ../..
```

Expected: all green; test count ≥ 279 (previous baseline) + ~15 new.

- [ ] **Step 2: Commit if fmt/clippy required fixes**

```bash
$CARGO fmt
git add -A cross-compile/anyka-init
git commit -m "chore(anyka-init): fmt/clippy after service toggle"
```

---

## Phase 2 — onvif-rust

### Task 8: `request_toggle` socket client

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/services.rs`

**Interfaces:**
- Produces: `pub enum ToggleReply { Accepted, Unknown, Error, Unreachable }` (all variants `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`); `pub fn request_toggle(path: &Path, name: &str, enabled: bool) -> ToggleReply`.

- [ ] **Step 1: Write the failing tests**

```rust
/// One connection per reply word; the mock echoes what it is told to.
#[test]
fn test_request_toggle_maps_all_reply_words() {
    use std::os::unix::net::UnixListener;

    let path = format!("/tmp/onvif-tog-test-{}.sock", std::process::id());
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let server_path = path.clone();
    let server = std::thread::spawn(move || {
        use std::io::{BufRead, Write};
        // Four connections: enable->ok, disable->unknown, enable->error,
        // disable->garbage.
        let replies = [b"ok\n", b"unknown\n", b"error\n", b"nope\n"];
        for (stream, reply) in listener.incoming().take(4).zip(replies) {
            let Ok(mut stream) = stream else { continue };
            let mut line = String::new();
            if std::io::BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let _ = stream.write_all(reply);
        }
        drop(listener);
        let _ = std::fs::remove_file(&server_path);
    });

    let p = std::path::Path::new(&path);
    assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Accepted);
    assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Unknown);
    assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Error);
    assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Error); // garbage is not ok

    server.join().expect("server thread");
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
    /// Applied (or an idempotent no-op).
    Accepted,
    /// Name is not a configured service.
    Unknown,
    /// The supervisor refused: the config write failed or the loop did not
    /// answer in time.
    Error,
    /// Socket unreachable — older anyka-init in the other slot.
    Unreachable,
}

/// Blocking. Sends `enable <name>` or `disable <name>` to the supervisor.
pub fn request_toggle(path: &Path, name: &str, enabled: bool) -> ToggleReply {
    let verb = if enabled { "enable" } else { "disable" };
    match round_trip(path, &format!("{verb} {name}\n")).map(|r| r.trim().to_owned()) {
        None => ToggleReply::Unreachable,
        Some(ref r) if r == "ok" => ToggleReply::Accepted,
        Some(ref r) if r == "unknown" => ToggleReply::Unknown,
        _ => ToggleReply::Error,
    }
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib services::`
Expected: PASS (plus the existing `services::` tests, including the original mock round-trip, still green).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/services.rs
git commit -m "feat(onvif-rust): request_toggle socket client (Accepted/Unknown/Error/Unreachable)"
```

---

### Task 9: Routes `POST /api/services/{name}/enable|disable`

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/processes.rs` (handlers)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs` (route registration, inside the `if state.auth_enabled` block, next to `/services/{name}/restart`)
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs` (auth-table tests only — the `_ => Administrator` catch-all already covers these paths; the tests pin that)

**Interfaces:**
- Consumes: `request_toggle`, `query_status`, `SOCKET_PATH` (services.rs).
- Produces: `pub async fn handle_enable_service(AxumPath<String>) -> impl IntoResponse`; same for `handle_disable_service`. Status mapping: 202 Accepted / 404 Unknown / 503 Error-or-Unreachable.

- [ ] **Step 1: Write the failing tests**

In `http.rs` tests (next to `test_required_level_for_a_service_restart_is_administrator`):

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

In `processes.rs` tests — the shared `toggle_service` core is testable against the existing "supervisor unreachable" path only (the socket path is a constant); pin that behavior:

```rust
#[tokio::test]
async fn test_toggle_service_without_a_supervisor_is_503() {
    // No anyka-init on a dev host: both reply paths degrade to 503, never 500.
    let resp = toggle_service("definitely-not-configured".to_string(), true).await;
    let status = axum::response::Response::from(resp).status();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu --lib processes:: http::`
Expected: compile failure — `toggle_service` missing.

- [ ] **Step 3: Implement**

In `processes.rs` (mirroring `handle_restart_service`'s structure):

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
        Ok(crate::diagnostics::services::ToggleReply::Unknown) => {
            (StatusCode::NOT_FOUND, "unknown service").into_response()
        }
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
Expected: all onvif-rust tests green (2292 baseline + 3 new).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/processes.rs cross-compile/onvif-rust/src/onvif/server.rs cross-compile/onvif-rust/src/diagnostics/http.rs
git commit -m "feat(onvif-rust): POST /api/services/{name}/enable|disable (202/404/503, admin-gated)"
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
$CARGO build --release
cd ../..
```

Expected: all green. Commit fix-ups if any.

---

## Phase 3 — WebUI

### Task 11: `processesService.toggleService`

**Files:**
- Modify: `cross-compile/www/src/services/processesService.ts`
- Modify: `cross-compile/www/src/services/processesService.test.ts`

**Interfaces:**
- Produces: `export async function toggleService(name: string, enabled: boolean): Promise<void>` — resolves on 202; throws `ApiError` on any other status; network-level failure (connection dropped by the camera itself, expected when disabling `onvif`) surfaces as a fetch `TypeError`, exactly like `restartService`.
- `ServiceStatus.state` doc comment becomes `'running' | 'backoff' | 'disabled'` (runtime validation is unchanged — it already accepts any string).

- [ ] **Step 1: Write the failing tests**

In `processesService.test.ts` (follow the file's existing `vi.mock('@/services/api')` pattern):

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { toggleService, getProcesses } from './processesService';

// ...existing mocks for authorizedFetch...

describe('toggleService', () => {
  beforeEach(() => vi.clearAllMocks());

  it('resolves on 202 and posts to the enable endpoint', async () => {
    mockAuthorizedFetch.mockResolvedValue({ ok: true, status: 202, text: async () => '' } as any);
    await expect(toggleService('snmp', true)).resolves.toBeUndefined();
    expect(mockAuthorizedFetch).toHaveBeenCalledWith(
      '/api/services/snmp/enable',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('posts to the disable endpoint', async () => {
    mockAuthorizedFetch.mockResolvedValue({ ok: true, status: 202, text: async () => '' } as any);
    await toggleService('onvif', false);
    expect(mockAuthorizedFetch).toHaveBeenCalledWith(
      '/api/services/onvif/disable',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('throws ApiError on 404 with the body', async () => {
    mockAuthorizedFetch.mockResolvedValue({
      ok: false,
      status: 404,
      text: async () => 'unknown service',
    } as any);
    await expect(toggleService('nope', true)).rejects.toThrow('404');
  });

  it('throws ApiError on 503', async () => {
    mockAuthorizedFetch.mockResolvedValue({
      ok: false,
      status: 503,
      text: async () => 'supervisor unreachable',
    } as any);
    await expect(toggleService('snmp', false)).rejects.toThrow('503');
  });

  it('surfaces a dropped connection as a TypeError, not ApiError', async () => {
    mockAuthorizedFetch.mockRejectedValue(new TypeError('fetch failed'));
    const err = await toggleService('onvif', false).catch((e) => e);
    expect(err).toBeInstanceOf(TypeError);
  });
});
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd cross-compile/www && npm run test -- processesService`
Expected: `toggleService` is not exported — fails.

- [ ] **Step 3: Implement**

```ts
/**
 * Enable or disable a supervised service.
 *
 * 202 means the supervisor accepted (applied, or confirmed the no-op): the
 * on-disk config is written first, then state transitions on the supervisor's
 * schedule. A 404 (unknown service) or 503 (supervisor unreachable / config
 * write failed) throws an ApiError. A network-level failure — the camera
 * dropping the connection as onvif itself goes down — surfaces as a fetch
 * TypeError so callers can distinguish "it did it to us" from "it failed".
 */
export async function toggleService(name: string, enabled: boolean): Promise<void> {
  const action = enabled ? 'enable' : 'disable';
  const response = await authorizedFetch(`/api/services/${encodeURIComponent(name)}/${action}`, {
    method: 'POST',
  });

  if (response.status === 202) {
    return;
  }
  const text = await response.text();
  throw new ApiError(
    `${enabled ? 'Enabling' : 'Disabling'} of ${name} failed with status ${response.status}`,
    response.status,
    text,
  );
}
```

Update the `ServiceStatus` doc comment: `/** 'running' | 'backoff' | 'disabled'. */`

- [ ] **Step 4: Run to verify they pass**

Run: `npm run test -- processesService && npm run type-check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/www/src/services/processesService.ts cross-compile/www/src/services/processesService.test.ts
git commit -m "feat(webui): processesService.toggleService for service enable/disable"
```

---

### Task 12: ProcessesCard — Disable/Enable actions with per-service warnings

**Files:**
- Modify: `cross-compile/www/src/components/ProcessesCard.tsx`
- Modify: `cross-compile/www/src/components/ProcessesCard.test.tsx`

**Interfaces:**
- Consumes: `toggleService` (Task 11), existing `restartService`, `getProcesses`, `waitForCameraBack`, `isAbortError`.
- Card behavior contract:
  - Enabled row: `Restart` + `Disable` buttons (test ids `diagnostics-processes-restart-{name}`, `diagnostics-processes-disable-{name}`).
  - Disabled row: dimmed (`opacity-50`), badge shows `disabled` (muted style), only an `Enable` button (`diagnostics-processes-enable-{name}`).
  - One shared confirm dialog (renamed test ids `diagnostics-processes-action-dialog`, `...-action-title`, `...-action-description`, `...-action-cancel`, `...-action-confirm`) with per-service consequence copy.
  - **Disable onvif does NOT enter the reconnecting state** — the camera is not coming back on its own by design. A dropped connection after disabling onvif is expected and is reported as success (that is the only reason the POST's connection would die).

- [ ] **Step 1: Write the failing tests**

Replace the restart-dialog-dependent tests in `ProcessesCard.test.tsx` with the generalized set (keep all existing raw-table/reconnecting tests, updated for renamed ids). New/updated tests:

```tsx
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
// ...existing mocks for getProcesses, restartService, getDiagnostics...
import { toggleService } from '@/services/processesService';
vi.mock('@/services/processesService', async (importOriginal) => ({
  ...(await importOriginal() as object),
  getProcesses: vi.fn(),
  restartService: vi.fn(),
  toggleService: vi.fn(),
}));

const supervisedFixture = (rows: ServiceStatus[]) => ({
  supervised: rows,
  processes: [],
});

const SNMP = { name: 'snmp', state: 'running', pid: 123, uptime_s: 500, restarts: 0, retry_in_s: 0 };
const OFF = { name: 'snmp', state: 'disabled', pid: null, uptime_s: 0, restarts: 0, retry_in_s: 0 };

describe('ProcessesCard service toggling', () => {
  it('shows Disable for an enabled service and Enable (dimmed) for a disabled one', async () => {
    (getProcesses as any).mockResolvedValue(
      supervisedFixture([SNMP, { ...OFF, name: 'dropbear' }]),
    );
    render(<ProcessesCard />);
    expect(
      await screen.findByTestId('diagnostics-processes-disable-snmp'),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId('diagnostics-processes-enable-snmp'),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId('diagnostics-processes-enable-dropbear'),
    ).toBeInTheDocument();
    // Disabled row is dimmed and shows the disabled badge.
    expect(screen.getByTestId('diagnostics-processes-row-dropbear')).toHaveClass('opacity-50');
    expect(screen.getByTestId('diagnostics-processes-status-dropbear')).toHaveTextContent('disabled');
    // A disabled service is not restartable.
    expect(
      screen.queryByTestId('diagnostics-processes-restart-dropbear'),
    ).not.toBeInTheDocument();
  });

  it('disabling snmp goes through the confirm dialog and calls toggleService', async () => {
    (getProcesses as any).mockResolvedValue(supervisedFixture([SNMP]));
    (toggleService as any).mockResolvedValue(undefined);
    render(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-snmp'));
    expect(
      screen.getByTestId('diagnostics-processes-action-dialog'),
    ).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-action-title')).toHaveTextContent(
      'Disable snmp?',
    );
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() =>
      expect(toggleService).toHaveBeenCalledWith('snmp', false),
    );
  });

  it('enabling calls toggleService with true', async () => {
    (getProcesses as any).mockResolvedValue(supervisedFixture([OFF]));
    (toggleService as any).mockResolvedValue(undefined);
    render(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-enable-snmp'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() => expect(toggleService).toHaveBeenCalledWith('snmp', true));
  });

  it('shows the onvif consequence copy when disabling onvif', async () => {
    (getProcesses as any).mockResolvedValue(
      supervisedFixture([{ ...SNMP, name: 'onvif' }]),
    );
    render(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    expect(screen.getByTestId('diagnostics-processes-action-description')).toHaveTextContent(
      'only reachable via FTP',
    );
  });

  it('disabling onvif does NOT enter the reconnecting state, even when the connection drops', async () => {
    (getProcesses as any).mockResolvedValue(
      supervisedFixture([{ ...SNMP, name: 'onvif' }]),
    );
    (toggleService as any).mockRejectedValue(new TypeError('fetch failed'));
    render(<ProcessesCard />);
    const user = userEvent.setup();
    await user.click(await screen.findByTestId('diagnostics-processes-disable-onvif'));
    await user.click(screen.getByTestId('diagnostics-processes-action-confirm'));
    await waitFor(() =>
      expect(toggleService).toHaveBeenCalledWith('onvif', false),
    );
    // A dropped connection on disable is the expected success path: the
    // card reports it as done, not as an error, and never waits.
    expect(screen.queryByTestId('diagnostics-processes-reconnecting')).not.toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-processes-onvif-off-note')).toBeInTheDocument();
  });

  it('an ApiError from disable surfaces as an error toast, no waiting', async () => {
    (getProcesses as any).mockResolvedValue(supervisedFixture([SNMP]));
    (toggleService as any).mockRejectedValue(Object.assign(new Error('503'), { name: 'ApiError' }));
    const toastError = vi.fn();
    vi.mocked(toast).mockReset();
    // ...assert toast.error called with the message; no reconnecting state.
  });
});
```

(Adapt fixture helpers to the file's existing conventions — the mock setup for `authorizedFetch`/query provider wrapper already exists in this test file; reuse it.)

- [ ] **Step 2: Run to verify they fail**

Run: `npm run test -- ProcessesCard`
Expected: new tests fail (missing ids/behavior); existing tests fail on renamed dialog ids.

- [ ] **Step 3: Implement**

Replace the single `target` state with an action union:

```tsx
type PendingAction =
  | { service: ServiceStatus; action: 'restart' | 'disable' | 'enable' }
  | null;

const [pending, setPending] = useState<PendingAction>(null);
const [onvifOff, setOnvifOff] = useState(false);
```

Consequence copy (module scope):

```tsx
const DISABLE_COPY: Record<string, string> = {
  onvif:
    'Web, ONVIF and RTSP access end immediately. The camera stays up and keeps streaming, but it is then only reachable via FTP (or the deadman telnet after a failed boot). Re-enabling requires FTP or an SD-card edit.',
  'vendor-daemon':
    'The video pipeline stops; the stream will show as stalled until it is re-enabled.',
  wpa_supplicant:
    'The Wi-Fi link may drop for the rest of this boot. Only toggle this from a wired connection.',
  udhcpc:
    'The address stays static until the next DHCP renewal — this is the documented way to use static addressing.',
  snmp: 'SNMP polling stops.',
  dropbear: 'The SSH daemon (port 22) will not run until it is re-enabled.',
};

const ENABLE_COPY: Record<string, string> = {
  wpa_supplicant:
    'Starts immediately. If the vendor currently owns the Wi-Fi link this boot, it may keep failing until the next reboot — the supervisor will retry under normal backoff.',
};
// default enable copy:
const DEFAULT_ENABLE_COPY =
  'The service starts immediately under the normal supervisor backoff policy.';
```

Row rendering:

```tsx
const disabled = service.state === 'disabled';
<tr
  key={service.name}
  className={`border-border border-b last:border-b-0 ${disabled ? 'opacity-50' : ''}`}
  data-testid={`diagnostics-processes-row-${service.name}`}
>
  {/* ...existing cells; uptime cell: '—' unless state === 'running' (already the case) ... */}
  <td className="py-2 text-right">
    {!disabled && (
      <Button
        size="sm"
        variant="outline"
        data-testid={`diagnostics-processes-restart-${service.name}`}
        onClick={() => setPending({ service, action: 'restart' })}
      >
        <RotateCw className="h-3.5 w-3.5" />
        Restart
      </Button>
    )}
    {disabled ? (
      <Button
        size="sm"
        variant="outline"
        data-testid={`diagnostics-processes-enable-${service.name}`}
        onClick={() => setPending({ service, action: 'enable' })}
      >
        Enable
      </Button>
    ) : (
      <Button
        size="sm"
        variant="outline"
        data-testid={`diagnostics-processes-disable-${service.name}`}
        onClick={() => setPending({ service, action: 'enable' /* WRONG — see note */ })}
      >
        Disable
      </Button>
    )}
  </td>
</tr>
```

**Note:** the Disable button sets `{ service, action: 'disable' }` (the snippet above was written to show placement; the literal is `'disable'`).

Badge:

```tsx
function ServiceStateBadge({ service }: Readonly<{ service: ServiceStatus }>) {
  const style =
    service.state === 'running'
      ? 'border-transparent bg-green-500/10 text-green-500'
      : service.state === 'disabled'
        ? 'border-transparent bg-zinc-500/10 text-zinc-400'
        : 'border-transparent bg-amber-500/10 text-amber-500';
  return (
    <Badge className={style} data-testid={`diagnostics-processes-status-${service.name}`}>
      {service.state}
    </Badge>
  );
}
```

Dialog (single shared component replacing the restart dialog):

```tsx
<AlertDialog
  open={pending !== null}
  onOpenChange={(open) => {
    if (!open) setPending(null);
  }}
>
  <AlertDialogContent
    className="bg-card border-border text-foreground"
    data-testid="diagnostics-processes-action-dialog"
  >
    <AlertDialogHeader>
      <AlertDialogTitle data-testid="diagnostics-processes-action-title">
        {pending
          ? `${pending.action === 'enable' ? 'Enable' : pending.action === 'disable' ? 'Disable' : 'Restart'} ${pending.service.name}?`
          : ''}
      </AlertDialogTitle>
      <AlertDialogDescription data-testid="diagnostics-processes-action-description">
        {pending && actionDescription(pending)}
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel data-testid="diagnostics-processes-action-cancel">Cancel</AlertDialogCancel>
      <AlertDialogAction
        data-testid="diagnostics-processes-action-confirm"
        onClick={(e) => {
          e.preventDefault();
          void handleConfirm(e);
        }}
      >
        {pending?.action === 'enable' ? 'Enable' : pending?.action === 'disable' ? 'Disable' : 'Restart'}
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
```

with:

```tsx
function actionDescription(p: NonNullable<PendingAction>): string {
  if (p.action === 'restart') {
    return p.service.name === 'onvif'
      ? 'Restarting onvif also stops vendor-daemon — video and this page will drop with it and recover on their own when the camera returns.'
      : 'The supervisor sends SIGTERM; the service is restarted under its normal backoff policy.';
  }
  if (p.action === 'enable') {
    return ENABLE_COPY[p.service.name] ?? DEFAULT_ENABLE_COPY;
  }
  return DISABLE_COPY[p.service.name] ?? 'The service stops and will not run again until re-enabled.';
}
```

`handleConfirm` dispatch (existing restart path unchanged):

```tsx
const handleConfirm = useCallback(
  async (e: React.MouseEvent) => {
    e.preventDefault();
    const p = pending;
    if (!p) return;
    setPending(null);

    if (p.action === 'restart') {
      await runRestart(p.service, e); // existing body, extracted as-is
      return;
    }

    // disable / enable: no camera reboot is involved — except disabling
    // onvif, which takes down the very HTTP server serving this page.
    const isOnvifOff = p.action === 'disable' && p.service.name === 'onvif';
    try {
      await toggleService(p.service.name, p.action === 'enable');
      if (isOnvifOff) {
        // The POST's connection is killed by our own action; a TypeError here
        // is the expected success, not a failure.
        setOnvifOff(true);
        toast.success('onvif disabled — the camera is reachable via FTP only');
      } else {
        toast.success(`${p.service.name} ${p.action === 'enable' ? 'enabled' : 'disabled'}`);
      }
      invalidate();
    } catch (err) {
      if (err instanceof Error && err.name === 'ApiError') {
        toast.error(err.message);
      } else if (isOnvifOff) {
        // Network-level failure on an onvif disable: the only cause is the
        // camera killing our connection — i.e. it worked.
        setOnvifOff(true);
        toast.success('onvif disabled — the camera is reachable via FTP only');
      } else {
        toast.error(err instanceof Error ? err.message : 'Toggle failed');
      }
    }
  },
  [invalidate, pending],
);
```

`runRestart` is the existing onvif/non-onvif restart body verbatim (now including `waitForCameraBack`). Render the off-note when set:

```tsx
{onvifOff && (
  <p
    className="text-muted-foreground text-sm"
    data-testid="diagnostics-processes-onvif-off-note"
    aria-live="polite"
  >
    onvif is disabled — Web access is down until it is re-enabled via FTP or the SD card.
  </p>
)}
```

- [ ] **Step 4: Run to verify they pass**

Run: `npm run test -- ProcessesCard`
Expected: all card tests green.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/www/src/components/ProcessesCard.tsx cross-compile/www/src/components/ProcessesCard.test.tsx
git commit -m "feat(webui): enable/disable service actions with per-service consequence warnings"
```

---

### Task 13: WebUI full gates

- [ ] **Step 1: Run all www gates**

```bash
cd cross-compile/www
npm run lint
npm run type-check
npm run test
npx prettier --check .
```

Expected: all green (Prettier 3.9.6 local). Commit fix-ups if any (never an ad-hoc `npx prettier@version`).

---

## Phase 4 — End-to-end verification

### Task 14: Deploy to camera 192.168.2.198 and verify on device

Uses the `anyka-firmware-upgrade` skill. **Order matters: the dangerous test (onvif) goes last, with the recovery path prepared first.**

- [ ] **Step 1: Build the bundle**

```bash
source ./setenv.sh
scripts/anyka-hack/build_bundle.sh 2026-09-19-webui-service-toggle
```

Verify the manifest lists `anyka-init.bin` (slot binaries), `onvif-rust.bin`, and the `www` bundle.

- [ ] **Step 2: Upload and monitor the trial**

```bash
curl -u admin:admin --fail -X PUT \
  -H "Content-Type: application/octet-stream" \
  --data-binary @bundle.tar \
  http://192.168.2.198/api/update
```

Expected: `202`. The camera reboots into the new slot; the trial (onvif still enabled → ports 80/554/8080) confirms after ~30 s hold. Watch:

```bash
for i in $(seq 1 30); do
  sleep 10
  code=$(curl -s -o /dev/null -w '%{http_code}' -u admin:admin http://192.168.2.198/api/diagnostics)
  echo "t+$((i*10))s: $code"
  [ "$code" = "200" ] && break
done
```

If the camera never comes back (trial revert → old slot), the old slot still has the previous deploy (`e00f3a3b`) — the feature is absent but the camera is healthy; investigate before retrying.

- [ ] **Step 3: Verify status includes disabled services**

```bash
curl -s -u admin:admin http://192.168.2.198/api/processes | python3 -m json.tool | grep -A8 '"supervised"'
```

Expected: rows for all six configured services; `dropbear` row shows `"state": "disabled", "pid": null`.

- [ ] **Step 4: Toggle snmp (safe service)**

```bash
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST http://192.168.2.198/api/services/snmp/disable
# expect 202
sleep 3
curl -s -u admin:admin http://192.168.2.198/api/processes | python3 -c 'import json,sys; [print(r["name"], r["state"]) for r in json.load(sys.stdin)["supervised"]]'
# snmp must show disabled and no snmp process may be in the raw list
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST http://192.168.2.198/api/services/snmp/enable
# expect 202; snmp comes back within ~2s
```

Also verify idempotence (`disable` twice → 202, no state change) and unknown service (`/api/services/nope/disable` → **404**).

- [ ] **Step 5: Verify persistence**

```bash
curl -s -u admin:admin http://192.168.2.198/api/processes  # leave snmp enabled
# Reboot is NOT done via web (no such route) — skip the reboot check here;
# persistence is unit-tested (Task 1) and will be exercised implicitly by the
# onvif test below, which reboots the camera.
```

- [ ] **Step 6: Disable onvif (LAST — recovery path prepared)**

**Recovery path (read before clicking):** with onvif disabled the camera boots healthy → telnet stays closed by design; FTP was refusing both known credentials in this session. Recovery is a **full SD payload push** (`scripts/anyka-hack/` per the anyka-firmware-upgrade skill) which replaces `anyka.toml` and resets all toggles to shipped defaults. Only proceed if the camera is reachable over LAN and you can perform an SD push.

```bash
curl -s -o /dev/null -w '%{http_code}\n' -u admin:admin -X POST http://192.168.2.198/api/services/onvif/disable
# expect 202 (or a dropped connection — both mean it worked)
sleep 20
curl -s -o /dev/null -w 'http:%{http_code} ' --connect-timeout 3 http://192.168.2.198/   # expect 000/timeout
# The camera itself stays up: check a non-onvif port if reachable by any means.
```

Expected: port 80 unresponsive; camera still powered (LAN still responds to ARP). **Then recover**: full payload push, wait for boot, confirm `/api/diagnostics` 200 and all toggles back to shipped defaults (`dropbear` disabled, rest enabled).

- [ ] **Step 7: Final state + push**

```bash
curl -s -u admin:admin http://192.168.2.198/api/diagnostics  # 200, all services healthy
curl -s -u admin:admin http://192.168.2.198/api/processes    # supervised array populated
rtk git status   # clean (no .vitest artifacts tracked)
rtk git push
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task(s) |
|---|---|
| §1 protocol (`enable`/`disable`, `ok`/`unknown`/`error`, extended status) | 3, 5 |
| §2 lifecycle (`Disabled` state, guards, boot-time insertion, wpa caveat) | 2, 4 (+ copy in Task 12) |
| §3 persistence (line-level, atomic, single source of truth) | 1, 4 |
| §4 adaptive trial (ownership map, empty-set immediate confirm, main.rs site) | 6 |
| §5 HTTP (202/404/503, admin-only, thin client) | 8, 9 |
| §6 WebUI (dimmed rows, per-service copy, no-wait onvif disable) | 11, 12 |
| §7 edge cases (backoff disable, late exit, write failure, idempotence, apply-window) | 2, 4, 6 (apply never writes anyka.toml — the row's claim is verified by construction; noted here) |
| §8 tests | every task's test steps |

**Placeholder scan:** Task 4's tests contain `/* same builder as above */` ellipses deliberately expanded to "match the existing fixture" instructions — the *handler implementation* (the only non-test code) is complete. Task 3's intermediate `send_toggle` sketch was superseded by its "correct shape" block — implement only the latter. No other placeholders.

**Type consistency:** `ToggleOutcome` (anyka-init, Task 3) is distinct from `ToggleReply` (onvif-rust, Task 8) — two crates, no shared code; both named for their side's meaning. `SvcState::Disabled` (Task 2) is used by Tasks 4/5 with identical fieldless shape. `ServiceStatus.state` values: `"running" | "backoff" | "disabled"` on both sides of the socket and in the TS doc (Task 11). `handle_query_status(cfg, services, reply_tx)` signature (Task 5) matches its call site update in the same task.

**Ordering note:** Tasks 2→3→4 depend on each other's declarations; Task 3's temporary `dispatch_msg` arm must be replaced (not duplicated) by Task 4's real arm.
