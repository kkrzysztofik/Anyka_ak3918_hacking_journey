# WebUI Process Control Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show running processes on the WebUI Diagnostics page and let an administrator restart any supervised service from the browser.

**Architecture:** `anyka-init` gains a Unix-socket control thread that holds a `Sender<Msg>` clone, answering `status` (TSV snapshot) and `restart <name>` (reuses the existing `Msg::RestartService`). `onvif-rust` gains a socket client, a `/proc` walker, and two routes (`GET /api/processes`, `POST /api/services/{name}/restart`). The WebUI gains one card rendering both a supervised-service table and a collapsible raw process table.

**Tech Stack:** Rust (std only in `anyka-init`; axum + serde_json in `onvif-rust`), React 19 + TanStack Query + shadcn/ui + Vitest in the WebUI.

**Design doc:** `docs/plans/2026-09-19-webui-process-control-design.md` — read it first.

---

## Before You Start

**Toolchain.** This repo uses a vendored cross-toolchain. Every `cargo` invocation below assumes:

```bash
export CARGO=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
export PATH=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH
```

The `PATH` prefix is not optional — `cargo clippy` fails with E0514 without it.

**Host tests run from `cross-compile/`** and always pass `--target x86_64-unknown-linux-gnu`. Without that flag cargo silently uses the ARM target.

**Read these skills before the phase they cover:**
- Phase 1 & 2 (Rust): @anyka-rust-testing
- Phase 3 (WebUI): @camera-webui-components, @anyka-webui-testing

**Branch:** work continues on `design/webui-process-control`.

**Do not deploy to a camera as part of this plan.** Phase 4 is a manual on-hardware verification checklist for a human to run.

---

## Phase 1 — `anyka-init` control channel

### Task 1: `ServiceStatus` snapshot type

A plain owned struct with no `Instant` in it, so it can cross a thread boundary and be formatted without borrowing the supervisor's state.

**Files:**
- Create: `cross-compile/anyka-init/src/control.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs` (add `pub mod control;`)

**Step 1: Write the failing test**

Add to `cross-compile/anyka-init/src/control.rs`:

```rust
//! Unix-socket control channel. Lets another process on the camera read
//! supervisor state and request a service restart.
//!
//! The wire format is TSV, not JSON: `anyka-init` has no `serde_json`, the
//! payload is six scalars per row between two processes we own on both ends,
//! and TSV is readable straight off `nc` during a telnet debug session.

use crate::supervise::SvcState;
use crate::sys::Pid;
use std::time::{Duration, Instant};

/// A supervisor state snapshot with every `Instant` already resolved to
/// seconds, so it can be sent across a channel and formatted freely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    /// `"running"` or `"backoff"`.
    pub state: &'static str,
    /// Zero when not running — TSV carries no null.
    pub pid: i32,
    pub uptime_s: u64,
    pub restarts: usize,
    /// Seconds until the next start attempt; zero when running.
    pub retry_in_s: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_from_running_reports_pid_and_uptime() {
        let now = Instant::now();
        let since = now - Duration::from_secs(90);
        let got = ServiceStatus::new("onvif", SvcState::Running { pid: Pid(42), since }, 3, now);
        assert_eq!(got.name, "onvif");
        assert_eq!(got.state, "running");
        assert_eq!(got.pid, 42);
        assert_eq!(got.uptime_s, 90);
        assert_eq!(got.restarts, 3);
        assert_eq!(got.retry_in_s, 0);
    }

    #[test]
    fn test_service_status_from_backoff_reports_retry_and_no_pid() {
        let now = Instant::now();
        let until = now + Duration::from_secs(12);
        let got = ServiceStatus::new(
            "vendor-daemon",
            SvcState::Backoff { until, attempt: 2 },
            7,
            now,
        );
        assert_eq!(got.state, "backoff");
        assert_eq!(got.pid, 0);
        assert_eq!(got.uptime_s, 0);
        assert_eq!(got.retry_in_s, 12);
    }

    /// A deadline already in the past must clamp to zero, not underflow.
    #[test]
    fn test_service_status_backoff_deadline_in_the_past_clamps_to_zero() {
        let now = Instant::now();
        let until = now - Duration::from_secs(5);
        let got = ServiceStatus::new("snmp", SvcState::Backoff { until, attempt: 1 }, 0, now);
        assert_eq!(got.retry_in_s, 0);
    }
}
```

**Step 2: Run the test to verify it fails**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init control:: -- --nocapture
```

Expected: FAIL — `no function or associated item named 'new' found for struct 'ServiceStatus'`.

**Step 3: Write the minimal implementation**

Add above the `#[cfg(test)]` block:

```rust
impl ServiceStatus {
    /// `now` is passed in rather than read here so the conversion stays pure
    /// and the clamping cases are testable without sleeping.
    pub fn new(name: &str, state: SvcState, restarts: usize, now: Instant) -> Self {
        match state {
            SvcState::Running { pid, since } => Self {
                name: name.to_owned(),
                state: "running",
                pid: pid.0,
                uptime_s: now.saturating_duration_since(since).as_secs(),
                restarts,
                retry_in_s: 0,
            },
            SvcState::Backoff { until, .. } => Self {
                name: name.to_owned(),
                state: "backoff",
                pid: 0,
                uptime_s: 0,
                restarts,
                retry_in_s: until.saturating_duration_since(now).as_secs(),
            },
        }
    }
}
```

If `Pid` is not a tuple struct with a public `.0`, use its existing accessor instead — check `cross-compile/anyka-init/src/sys.rs` and adapt.

Add to `cross-compile/anyka-init/src/lib.rs`, keeping the module list alphabetical:

```rust
pub mod control;
```

**Step 4: Run the test to verify it passes**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init control::
```

Expected: PASS, 3 tests.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/control.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): add ServiceStatus supervisor snapshot type"
```

---

### Task 2: TSV encoding and request parsing

Both pure functions, so the whole wire format is testable without a socket.

**Files:**
- Modify: `cross-compile/anyka-init/src/control.rs`

**Step 1: Write the failing tests**

Add to the `tests` module:

```rust
#[test]
fn test_encode_status_writes_one_tab_separated_line_per_service() {
    let rows = vec![
        ServiceStatus {
            name: "onvif".into(),
            state: "running",
            pid: 42,
            uptime_s: 90,
            restarts: 3,
            retry_in_s: 0,
        },
        ServiceStatus {
            name: "snmp".into(),
            state: "backoff",
            pid: 0,
            uptime_s: 0,
            restarts: 7,
            retry_in_s: 12,
        },
    ];
    assert_eq!(
        encode_status(&rows),
        "onvif\trunning\t42\t90\t3\t0\nsnmp\tbackoff\t0\t0\t7\t12\n\n"
    );
}

/// The trailing blank line is the frame terminator; an empty service list
/// must still send it or the reader blocks forever.
#[test]
fn test_encode_status_with_no_services_is_just_the_terminator() {
    assert_eq!(encode_status(&[]), "\n");
}

#[test]
fn test_parse_request_recognises_status() {
    assert_eq!(parse_request("status\n"), Some(Request::Status));
}

#[test]
fn test_parse_request_recognises_restart_with_a_name() {
    assert_eq!(
        parse_request("restart vendor-daemon\n"),
        Some(Request::Restart("vendor-daemon".into()))
    );
}

#[test]
fn test_parse_request_rejects_restart_without_a_name() {
    assert_eq!(parse_request("restart\n"), None);
    assert_eq!(parse_request("restart   \n"), None);
}

#[test]
fn test_parse_request_rejects_unknown_verbs() {
    assert_eq!(parse_request("shutdown\n"), None);
    assert_eq!(parse_request("\n"), None);
}

/// A name with a tab would corrupt the status frame, and a service name can
/// never legitimately contain one — it is a TOML table key.
#[test]
fn test_parse_request_rejects_a_name_containing_a_tab() {
    assert_eq!(parse_request("restart a\tb\n"), None);
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init control::
```

Expected: FAIL — `cannot find function 'encode_status'`, `cannot find type 'Request'`.

**Step 3: Write the implementation**

```rust
/// A parsed control request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Restart(String),
}

/// Parse one request line. `None` means "reject" — the caller answers `err`
/// and closes, rather than guessing at intent.
pub fn parse_request(line: &str) -> Option<Request> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "status" {
        return Some(Request::Status);
    }
    let name = line.strip_prefix("restart ")?.trim();
    if name.is_empty() || name.contains('\t') {
        return None;
    }
    Some(Request::Restart(name.to_owned()))
}

/// Encode a status snapshot. One TSV row per service, then a blank line as the
/// frame terminator so the reader knows the list ended without closing.
pub fn encode_status(rows: &[ServiceStatus]) -> String {
    let mut out = String::new();
    for r in rows {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            r.name, r.state, r.pid, r.uptime_s, r.restarts, r.retry_in_s
        ));
    }
    out.push('\n');
    out
}
```

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init control::
```

Expected: PASS, 10 tests.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/control.rs
git commit -m "feat(anyka-init): add control-channel TSV encoding and request parsing"
```

---

### Task 3: `Msg::QueryStatus` and its dispatch arm

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (`Msg` enum ~line 32, `dispatch_msg` ~line 317)

**Step 1: Write the failing test**

Add to the `tests` module at the bottom of `supervisor_loop.rs`. Model it on the existing `test_run_restart_message_for_unknown_service_is_ignored` (~line 665) — copy that test's setup helpers rather than inventing new ones.

```rust
/// A QueryStatus request must be answered on the caller's reply channel with
/// one row per configured service, so the HTTP handler never blocks forever.
#[test]
fn test_run_query_status_replies_with_one_row_per_service() {
    // Reuse whatever harness the neighbouring `test_run_*` tests use to build
    // a Config with a single service and drive `run` on a MockSys.
    let (tx, rx) = make_channel();
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();

    tx.send(Msg::QueryStatus(reply_tx)).unwrap();
    tx.send(Msg::Shutdown).unwrap();

    // ... drive `run(sys, &cfg, rx)` exactly as the neighbouring tests do ...

    let rows = reply_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("QueryStatus must be answered");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "svc");
}

/// A caller that gave up and dropped its receiver must not take the supervisor
/// down with it — the send error is expected and ignored.
#[test]
fn test_run_query_status_with_a_dropped_receiver_does_not_stop_the_loop() {
    let (tx, rx) = make_channel();
    let (reply_tx, reply_rx) = std::sync::mpsc::channel::<Vec<crate::control::ServiceStatus>>();
    drop(reply_rx);

    tx.send(Msg::QueryStatus(reply_tx)).unwrap();
    tx.send(Msg::Shutdown).unwrap();

    // ... drive `run` ...
    // Reaching here without a panic is the assertion: the loop processed the
    // dead reply channel and went on to handle Shutdown.
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init supervisor_loop::
```

Expected: FAIL — `no variant named 'QueryStatus'`.

**Step 3: Write the implementation**

Add to the `Msg` enum:

```rust
    /// Snapshot request from the control thread. The reply travels on the
    /// caller's own channel — the std-only equivalent of a oneshot, which
    /// fits because this loop is already a message pump. A shared
    /// `Arc<Mutex<Vec<ServiceStatus>>>` would instead force a write on every
    /// tick even when nobody is reading.
    QueryStatus(Sender<Vec<crate::control::ServiceStatus>>),
```

Add the arm to `dispatch_msg`, before the `Shutdown` arm:

```rust
        Ok(Msg::QueryStatus(reply)) => {
            let now = sys.now();
            let rows: Vec<crate::control::ServiceStatus> = services
                .iter()
                .map(|s| {
                    crate::control::ServiceStatus::new(&s.name, s.state, s.hist.len(), now)
                })
                .collect();
            // The caller may have timed out and gone; that is not our problem.
            let _ = reply.send(rows);
            false
        }
```

`SvcState` is `Copy`, so `s.state` needs no clone.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
```

Expected: PASS, all `anyka-init` tests green.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/supervisor_loop.rs
git commit -m "feat(anyka-init): answer QueryStatus with a supervisor snapshot"
```

---

### Task 4: The control thread

**Files:**
- Modify: `cross-compile/anyka-init/src/control.rs`
- Modify: `cross-compile/anyka-init/src/main.rs` (spawn next to the update-poll thread, ~line 158)

**Step 1: Write the failing test**

An end-to-end test over a real socket in a temp dir. Add to `control.rs` tests:

```rust
#[test]
fn test_serve_answers_status_over_a_real_socket() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let dir = std::env::temp_dir().join(format!("anyka-ctl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.sock");

    let (tx, rx) = std::sync::mpsc::channel();
    let listener = bind(&path).expect("bind must succeed on a fresh path");

    // Stand in for the supervisor loop: answer one QueryStatus.
    let responder = std::thread::spawn(move || {
        if let Ok(crate::supervisor_loop::Msg::QueryStatus(reply)) = rx.recv() {
            let _ = reply.send(vec![ServiceStatus {
                name: "onvif".into(),
                state: "running",
                pid: 42,
                uptime_s: 90,
                restarts: 0,
                retry_in_s: 0,
            }]);
        }
    });

    let server = std::thread::spawn(move || serve_one(&listener, &tx));

    let mut client = UnixStream::connect(&path).unwrap();
    client.write_all(b"status\n").unwrap();
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert_eq!(line, "onvif\trunning\t42\t90\t0\t0\n");

    server.join().unwrap();
    responder.join().unwrap();
    let _ = std::fs::remove_file(&path);
}

/// A stale socket file from an unclean exit must not make bind fail forever.
#[test]
fn test_bind_replaces_a_stale_socket_file() {
    let dir = std::env::temp_dir().join(format!("anyka-stale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stale.sock");
    std::fs::write(&path, b"not really a socket").unwrap();

    assert!(bind(&path).is_ok(), "bind must unlink a stale path first");
    let _ = std::fs::remove_file(&path);
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init control::
```

Expected: FAIL — `cannot find function 'bind'`, `cannot find function 'serve_one'`.

**Step 3: Write the implementation**

```rust
use crate::supervisor_loop::Msg;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::Sender;

/// Where the control socket lives. Hardcoded on purpose: `Config` carries
/// `deny_unknown_fields`, so a `[control]` stanza in `anyka.toml` would be a
/// hard parse error for the older binary in the other A/B slot after a
/// rollback. A cosmetic tunable is not worth that.
pub const SOCKET_PATH: &str = "/tmp/anyka-init.sock";

/// How long to wait for the supervisor loop to answer a snapshot request.
/// Bounded so a wedged loop surfaces as an error instead of hanging the
/// caller's HTTP request — which is precisely the failure this feature exists
/// to diagnose.
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// Bind the listener, clearing any stale socket file first.
pub fn bind(path: &Path) -> std::io::Result<UnixListener> {
    // An abandoned socket file from an unclean exit makes bind fail with
    // EADDRINUSE forever, so unlink unconditionally. Errors are ignored
    // because "it was not there" is the normal case.
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    // 0600: this restarts services, so it is root-only.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Accept and serve exactly one connection. Split out from [`serve`] so the
/// protocol is testable without an unstoppable loop.
pub fn serve_one(listener: &UnixListener, tx: &Sender<Msg>) -> std::io::Result<()> {
    let (stream, _) = listener.accept()?;
    handle_conn(stream, tx)
}

fn handle_conn(stream: UnixStream, tx: &Sender<Msg>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(REPLY_TIMEOUT))?;
    stream.set_write_timeout(Some(REPLY_TIMEOUT))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let mut out = stream;
    match parse_request(&line) {
        Some(Request::Status) => {
            let (reply_tx, reply_rx) = std::sync::mpsc::channel();
            if tx.send(Msg::QueryStatus(reply_tx)).is_err() {
                return out.write_all(b"err\n");
            }
            match reply_rx.recv_timeout(REPLY_TIMEOUT) {
                Ok(rows) => out.write_all(encode_status(&rows).as_bytes()),
                Err(_) => {
                    tracing::warn!("supervisor did not answer a status request in time");
                    out.write_all(b"err\n")
                }
            }
        }
        Some(Request::Restart(name)) => {
            // Fire-and-forget by design: RestartService SIGTERMs, and the
            // normal exit path restarts under the existing backoff and
            // storm-guard policy. "ok" means accepted, not completed.
            tracing::info!(service = %name, "restart requested over the control socket");
            match tx.send(Msg::RestartService(name)) {
                Ok(()) => out.write_all(b"ok\n"),
                Err(_) => out.write_all(b"err\n"),
            }
        }
        None => out.write_all(b"err\n"),
    }
}

/// Accept forever. One request per connection.
pub fn serve(listener: UnixListener, tx: Sender<Msg>) {
    loop {
        if let Err(e) = serve_one(&listener, &tx) {
            tracing::debug!(error = %e, "control connection failed");
        }
    }
}
```

Note: `handle_conn` answers `ok` for *any* service name — the supervisor ignores unknown names (see `handle_restart_service`). Distinguishing unknown names would require a second round-trip; Task 6 covers this by having `onvif-rust` validate the name against the status snapshot before sending, which costs nothing extra because the frontend already has that list.

Wire it up in `main.rs`, next to the update-poll thread:

```rust
    // Control socket. One thread, holding a Sender clone — sending on the
    // channel is itself the supervisor's wake mechanism, so a restart request
    // takes effect immediately rather than waiting for the next tick.
    {
        let tx = tx.clone();
        match anyka_init::control::bind(std::path::Path::new(
            anyka_init::control::SOCKET_PATH,
        )) {
            Ok(listener) => {
                let _ = std::thread::Builder::new()
                    .name("control".into())
                    .stack_size(supervisor_loop::thread_stack())
                    .spawn(move || anyka_init::control::serve(listener, tx));
            }
            Err(e) => {
                // Not fatal: the camera supervises fine without a control
                // socket, and the WebUI degrades to "control unavailable".
                tracing::error!(error = %e, "control socket unavailable");
            }
        }
    }
```

Check the surrounding code for the actual name of the `Sender` in scope (it is `tx` where `spawn_optional_threads` is called) and match it.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -p anyka-init -- -D warnings
```

Expected: PASS, no clippy warnings.

**Step 5: Verify it cross-compiles for the camera**

```bash
cd cross-compile/anyka-init && $CARGO build --release
```

Must be run from the crate directory, not the workspace root, or cargo links against the host toolchain.

**Step 6: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/control.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): serve supervisor status and restarts on a control socket"
```

---

## Phase 2 — `onvif-rust` endpoints

### Task 5: `/proc` process walker

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/processes.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs` (add `pub mod processes;`, alphabetical)

**Step 1: Write the failing tests**

Parsing is separated from I/O so it is testable against fixture strings.

```rust
//! `/proc` walk backing the raw process table on the diagnostics page.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Process {
    pub pid: i32,
    pub ppid: i32,
    pub comm: String,
    /// Single-letter state from `/proc/[pid]/stat`: R, S, D, Z, T.
    pub state: String,
    pub rss_kb: u64,
    /// Cumulative utime+stime in seconds.
    ///
    /// Deliberately not a percentage: that needs two samples, per-pid delta
    /// bookkeeping and pid-reuse handling, and answers no question this one
    /// does not — "has this been burning CPU since boot?".
    pub cpu_time_s: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real line. Note `comm` is parenthesised and may itself contain
    /// spaces and parens, so fields must be counted from the LAST ')'.
    const STAT: &str = "1234 (onvif-rust.bin) S 1 1234 1234 0 -1 4194560 812 0 0 0 150 30 0 0 20 0 9 0 5678 24576000 1500 4294967295 1 2 3 4 5 6 7";

    #[test]
    fn test_parse_stat_extracts_pid_comm_state_and_ppid() {
        let got = parse_stat(STAT, 100).expect("a well-formed stat line must parse");
        assert_eq!(got.pid, 1234);
        assert_eq!(got.comm, "onvif-rust.bin");
        assert_eq!(got.state, "S");
        assert_eq!(got.ppid, 1);
    }

    /// utime=150 + stime=30 jiffies at 100 Hz = 1 second.
    #[test]
    fn test_parse_stat_converts_jiffies_to_seconds() {
        let got = parse_stat(STAT, 100).unwrap();
        assert_eq!(got.cpu_time_s, 1);
    }

    /// A kernel thread's comm contains spaces and parens. Counting fields from
    /// the left would shift every subsequent field and report a bogus state.
    #[test]
    fn test_parse_stat_handles_a_comm_containing_spaces_and_parens() {
        let line = "7 (kworker/0:1 (x)) D 2 0 0 0 -1 0 0 0 0 0 5 5 0 0 20 0 1 0 9 0 0";
        let got = parse_stat(line, 100).expect("comm with parens must still parse");
        assert_eq!(got.comm, "kworker/0:1 (x)");
        assert_eq!(got.state, "D");
        assert_eq!(got.ppid, 2);
    }

    #[test]
    fn test_parse_stat_rejects_a_truncated_line() {
        assert!(parse_stat("1234 (x) S", 100).is_none());
        assert!(parse_stat("garbage", 100).is_none());
    }

    #[test]
    fn test_parse_status_rss_reads_vmrss_in_kb() {
        let status = "Name:\tonvif\nState:\tS (sleeping)\nVmRSS:\t  4096 kB\nThreads:\t9\n";
        assert_eq!(parse_status_rss(status), 4096);
    }

    /// Kernel threads have no VmRSS line at all; that is zero, not an error.
    #[test]
    fn test_parse_status_rss_without_a_vmrss_line_is_zero() {
        assert_eq!(parse_status_rss("Name:\tkworker\nState:\tS\n"), 0);
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib processes::
```

Expected: FAIL — `cannot find function 'parse_stat'`.

**Step 3: Write the implementation**

```rust
/// Parse one `/proc/[pid]/stat` line. `hz` is `sysconf(_SC_CLK_TCK)`.
///
/// Fields are counted from the last `)`, never from the left: `comm` is
/// parenthesised and may contain spaces and parens of its own (`kworker/0:1
/// (x)`), so a naive split shifts every field after it.
pub fn parse_stat(line: &str, hz: u64) -> Option<Process> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    if close < open {
        return None;
    }
    let pid: i32 = line.get(..open)?.trim().parse().ok()?;
    let comm = line.get(open + 1..close)?.to_owned();

    // Fields after comm, 0-indexed: 0=state, 1=ppid, 11=utime, 12=stime.
    let rest: Vec<&str> = line.get(close + 1..)?.split_whitespace().collect();
    let state = (*rest.first()?).to_owned();
    let ppid: i32 = rest.get(1)?.parse().ok()?;
    let utime: u64 = rest.get(11)?.parse().ok()?;
    let stime: u64 = rest.get(12)?.parse().ok()?;

    Some(Process {
        pid,
        ppid,
        comm,
        state,
        rss_kb: 0,
        cpu_time_s: if hz == 0 { 0 } else { (utime + stime) / hz },
    })
}

/// Pull `VmRSS` out of `/proc/[pid]/status`. Kernel threads have no such line;
/// that is legitimately zero, not a failure.
pub fn parse_status_rss(text: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Walk `/proc`, newest RSS-heaviest first.
///
/// Blocking. Callers must wrap the WHOLE walk in a single `spawn_blocking` —
/// never one per file. Each yielding await costs roughly a scheduler quantum
/// (~12 ms) on this camera, which across ~50 processes turns a
/// few-millisecond walk into most of a second.
pub fn collect() -> Vec<Process> {
    // SAFETY: sysconf with a constant name has no preconditions and no
    // side effects; it only reads a kernel-provided constant.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out: Vec<Process> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            // Only numeric entries are processes.
            name.parse::<i32>().ok()?;
            let stat = std::fs::read_to_string(e.path().join("stat")).ok()?;
            let mut p = parse_stat(&stat, hz)?;
            // A process that exits mid-walk is normal; treat it as 0 RSS
            // rather than dropping the row we already parsed.
            p.rss_kb = std::fs::read_to_string(e.path().join("status"))
                .map(|s| parse_status_rss(&s))
                .unwrap_or(0);
            Some(p)
        })
        .collect();
    out.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    out
}
```

Confirm `libc` is already a dependency of `onvif-rust` (`grep libc cross-compile/onvif-rust/Cargo.toml`). If it is not, read `_SC_CLK_TCK` as the constant `100` with a comment — it is 100 on every Linux ARM kernel this ships to, and that is cheaper than a new dependency.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib processes::
```

Expected: PASS, 6 tests.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/diagnostics/processes.rs cross-compile/onvif-rust/src/diagnostics/mod.rs
git commit -m "feat(onvif-rust): add a /proc process walker for diagnostics"
```

---

### Task 6: Control-socket client

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/services.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs`

**Step 1: Write the failing tests**

```rust
//! Client for the `anyka-init` control socket.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    /// `None` when the service is not running.
    pub pid: Option<i32>,
    pub uptime_s: u64,
    pub restarts: u64,
    pub retry_in_s: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_status_parses_tab_separated_rows() {
        let got = decode_status("onvif\trunning\t42\t90\t3\t0\nsnmp\tbackoff\t0\t0\t7\t12\n\n");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "onvif");
        assert_eq!(got[0].pid, Some(42));
        assert_eq!(got[0].uptime_s, 90);
        assert_eq!(got[1].state, "backoff");
        assert_eq!(got[1].retry_in_s, 12);
    }

    /// pid 0 on the wire means "not running" — JSON gets a real null so the
    /// UI never renders a process that does not exist.
    #[test]
    fn test_decode_status_maps_pid_zero_to_none() {
        let got = decode_status("snmp\tbackoff\t0\t0\t7\t12\n\n");
        assert_eq!(got[0].pid, None);
    }

    #[test]
    fn test_decode_status_of_an_empty_frame_is_empty() {
        assert!(decode_status("\n").is_empty());
    }

    /// A malformed row must be dropped, not panic and not poison the rest.
    #[test]
    fn test_decode_status_skips_rows_with_too_few_fields() {
        let got = decode_status("broken\trunning\nonvif\trunning\t42\t90\t3\t0\n\n");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "onvif");
    }

    /// The socket is absent on an older anyka-init in the other A/B slot.
    /// That must read as "control unavailable", never as a 500.
    #[test]
    fn test_query_status_on_a_missing_socket_returns_none() {
        let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
        assert!(query_status(missing).is_none());
    }

    #[test]
    fn test_request_restart_on_a_missing_socket_is_false() {
        let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
        assert!(!request_restart(missing, "onvif"));
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib services::
```

Expected: FAIL — `cannot find function 'decode_status'`.

**Step 3: Write the implementation**

```rust
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Must match `anyka_init::control::SOCKET_PATH`.
pub const SOCKET_PATH: &str = "/tmp/anyka-init.sock";

const TIMEOUT: Duration = Duration::from_secs(2);

/// Decode the TSV status frame. Malformed rows are dropped rather than
/// failing the whole snapshot: one bad row must not blank the page.
pub fn decode_status(text: &str) -> Vec<ServiceStatus> {
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 6 {
                return None;
            }
            let pid: i32 = f[2].parse().ok()?;
            Some(ServiceStatus {
                name: f[0].to_owned(),
                state: f[1].to_owned(),
                pid: if pid > 0 { Some(pid) } else { None },
                uptime_s: f[3].parse().ok()?,
                restarts: f[4].parse().ok()?,
                retry_in_s: f[5].parse().ok()?,
            })
        })
        .collect()
}

fn round_trip(path: &Path, request: &str) -> Option<String> {
    let mut stream = UnixStream::connect(path).ok()?;
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    stream.write_all(request.as_bytes()).ok()?;
    let mut out = String::new();
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        // EOF or the blank terminator line both end the frame.
        if reader.read_line(&mut line).ok()? == 0 || line == "\n" {
            break;
        }
        out.push_str(&line);
    }
    Some(out)
}

/// Blocking. `None` means the supervisor is unreachable — an older
/// `anyka-init` in the other A/B slot, or a control thread that failed to
/// bind. Callers render the raw process table anyway.
pub fn query_status(path: &Path) -> Option<Vec<ServiceStatus>> {
    round_trip(path, "status\n").map(|t| decode_status(&t))
}

/// Blocking. `true` means the restart was accepted, not that it completed.
pub fn request_restart(path: &Path, name: &str) -> bool {
    round_trip(path, &format!("restart {name}\n"))
        .is_some_and(|r| r.trim() == "ok")
}
```

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib services::
```

Expected: PASS, 6 tests.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/diagnostics/services.rs cross-compile/onvif-rust/src/diagnostics/mod.rs
git commit -m "feat(onvif-rust): add an anyka-init control socket client"
```

---

### Task 7: HTTP handlers, routes and auth

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/processes.rs` (add handlers)
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs:96-107` (`required_level_for_path`)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs:650-676` (route registration)

**Step 1: Write the failing tests**

Auth first, in `http.rs` tests:

```rust
/// Reading the process list is a diagnostic, like /diagnostics: User is enough.
#[test]
fn test_required_level_for_processes_is_user() {
    assert_eq!(required_level_for_path("/processes"), AuthLevel::User);
    assert_eq!(required_level_for_path("/processes/"), AuthLevel::User);
}

/// Restarting a service is a state change, like /update. It is NOT listed in
/// the table on purpose — it must inherit the fail-closed default. This test
/// pins that, so opening the default later cannot silently expose it.
#[test]
fn test_required_level_for_a_service_restart_is_administrator() {
    assert_eq!(
        required_level_for_path("/services/onvif/restart"),
        AuthLevel::Administrator
    );
}
```

Then a route test in `server.rs`, modelled on `test_update_route_rejects_non_admin_credentials_with_403` (~line 1869):

```rust
/// A restart is a state change; Operator-level credentials must not carry it.
#[tokio::test]
async fn test_service_restart_route_rejects_non_admin_credentials_with_403() {
    // ... copy the harness from test_update_route_rejects_non_admin_credentials_with_403,
    // POST to /api/services/onvif/restart, assert StatusCode::FORBIDDEN ...
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib required_level_for_processes
```

Expected: FAIL — asserts `Administrator == User` (the fail-closed default catches `/processes` today).

**Step 3: Write the implementation**

In `http.rs::required_level_for_path`, add one arm alongside `/diagnostics`:

```rust
        "/processes" | "/processes/" => AuthLevel::User,
```

Add nothing for the restart route — the existing `_ => AuthLevel::Administrator` fail-closed arm already covers it, and the test above pins that.

Add handlers to `processes.rs`:

```rust
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessesResponse {
    /// `None` when the supervisor control socket is unreachable. The raw
    /// table still renders; the UI degrades to a note.
    pub supervised: Option<Vec<crate::diagnostics::services::ServiceStatus>>,
    pub processes: Vec<Process>,
}

/// GET /api/processes
pub async fn handle_processes() -> impl IntoResponse {
    // One spawn_blocking for BOTH the socket round-trip and the whole /proc
    // walk — see the note on `collect`.
    let result = tokio::task::spawn_blocking(|| {
        let supervised = crate::diagnostics::services::query_status(std::path::Path::new(
            crate::diagnostics::services::SOCKET_PATH,
        ));
        ProcessesResponse {
            supervised,
            processes: collect(),
        }
    })
    .await;

    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "process listing task failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /api/services/{name}/restart
///
/// 202, not 200: the supervisor SIGTERMs and the normal exit path restarts
/// under backoff. "Accepted" is the honest status — nothing here waits for the
/// service to come back.
pub async fn handle_restart_service(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    let sock = std::path::Path::new(crate::diagnostics::services::SOCKET_PATH);

    let known = tokio::task::spawn_blocking(move || {
        // Validate the name against the live snapshot so an unknown service is
        // a 404 rather than a silently-ignored "ok". Costs one extra
        // round-trip on a path a human clicks, which is free.
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

    let accepted =
        tokio::task::spawn_blocking(move || crate::diagnostics::services::request_restart(sock, &name))
            .await;

    match accepted {
        Ok(true) => StatusCode::ACCEPTED.into_response(),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "restart not accepted").into_response(),
    }
}
```

Register the routes in `server.rs`. `/processes` goes with the unauthenticated-tier GETs next to `/logs`; the restart goes **inside** the `if state.auth_enabled` block, exactly like `/update`, so it cannot exist at all when auth is off:

```rust
                .route(
                    "/processes",
                    get(crate::diagnostics::processes::handle_processes).layer(timeout()),
                );
```

and inside `if state.auth_enabled`:

```rust
                api = api.route(
                    "/services/{name}/restart",
                    post(crate::diagnostics::processes::handle_restart_service).layer(timeout()),
                );
```

Check the axum version's path-parameter syntax: axum 0.8 uses `{name}`, axum 0.7 uses `:name`. Grep an existing parameterised route, or `grep '^axum' cross-compile/onvif-rust/Cargo.toml`.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
cd cross-compile && $CARGO fmt --check
```

Expected: all green.

**Step 5: Verify the ARM build**

```bash
cd cross-compile/onvif-rust && $CARGO build --release
```

From the crate directory, not the workspace root.

**Step 6: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/diagnostics/processes.rs \
        cross-compile/onvif-rust/src/diagnostics/http.rs \
        cross-compile/onvif-rust/src/onvif/server.rs
git commit -m "feat(onvif-rust): add /api/processes and service restart routes"
```

---

## Phase 3 — WebUI

All commands in this phase run from `cross-compile/www`.

### Task 8: Extract the two shared helpers

Pure refactor, no behaviour change. Doing it first keeps Task 10 from duplicating a polling loop.

**Files:**
- Create: `cross-compile/www/src/lib/waitForCameraBack.ts`
- Create: `cross-compile/www/src/lib/waitForCameraBack.test.ts`
- Modify: `cross-compile/www/src/components/FirmwareUpgradeDialog.tsx:131-165`
- Modify: `cross-compile/www/src/lib/utils.ts` (add `formatDuration`)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:62-71` (remove the local copy, import instead)

**Step 1: Write the failing test**

`src/lib/waitForCameraBack.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest';

import { waitForCameraBack } from '@/lib/waitForCameraBack';

describe('waitForCameraBack', () => {
  it('resolves saw-down true once the probe fails and then succeeds again', async () => {
    const probe = vi
      .fn()
      .mockResolvedValueOnce(undefined)      // still up (pre-reboot)
      .mockRejectedValueOnce(new Error('down'))
      .mockResolvedValueOnce(undefined);     // back
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 5000 });
    expect(result).toBe('back');
  });

  it('reports a timeout without ever seeing the camera go down', async () => {
    const probe = vi.fn().mockResolvedValue(undefined);
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 0 });
    expect(result).toBe('never-went-down');
  });

  it('reports still-unreachable when it went down and never came back', async () => {
    const probe = vi.fn().mockRejectedValue(new Error('down'));
    const result = await waitForCameraBack(probe, { intervalMs: 0, timeoutMs: 10 });
    expect(result).toBe('still-down');
  });
});
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile/www && npm run test -- waitForCameraBack
```

Expected: FAIL — cannot resolve `@/lib/waitForCameraBack`.

**Step 3: Write the implementation**

Lift the loop from `FirmwareUpgradeDialog.tsx:131-165`, dropping the firmware-version comparison (that stays in the dialog, keyed off the returned outcome):

```ts
export type WaitOutcome = 'back' | 'still-down' | 'never-went-down';

/**
 * Wait for the camera to disappear and come back.
 *
 * ponytail: the down→up edge approximates a reconnect. Upgrade path if false
 * positives appear: a trial-status API that reports the reboot explicitly.
 */
export async function waitForCameraBack(
  probe: (signal?: AbortSignal) => Promise<unknown>,
  opts: { intervalMs: number; timeoutMs: number; signal?: AbortSignal },
): Promise<WaitOutcome> {
  // ... moved loop: sawDown flag, deadline, probe/catch, sleep ...
}
```

Then rewrite `pollUntilBack` in the dialog to call it and branch on the outcome for its version-comparison messages. Its existing tests must keep passing untouched — that is the check that the refactor preserved behaviour.

Move `formatDuration` from `DiagnosticsPage.tsx:62` verbatim into `src/lib/utils.ts` and import it in `DiagnosticsPage.tsx`.

**Step 4: Run to verify everything passes**

```bash
cd cross-compile/www && npm run test
```

Expected: PASS, including the untouched `FirmwareUpgradeDialog.test.tsx`.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/www/src/lib cross-compile/www/src/components/FirmwareUpgradeDialog.tsx \
        cross-compile/www/src/pages/DiagnosticsPage.tsx
git commit -m "refactor(webui): extract waitForCameraBack and formatDuration for reuse"
```

---

### Task 9: `processesService.ts`

**Files:**
- Create: `cross-compile/www/src/services/processesService.ts`
- Create: `cross-compile/www/src/services/processesService.test.ts`

Follow `diagnosticsService.ts` exactly: `authorizedFetch`, `ApiError`, hand-written type guards. **Do not add a schema library** — this codebase validates runtime shapes by hand.

**Step 1: Write the failing tests**

Cover: a well-formed response parses; `supervised: null` parses (not an error); a malformed shape throws `ApiError`; `restartService` resolves on 202; 404 throws; 503 throws.

**Step 2: Run to verify they fail**

```bash
cd cross-compile/www && npm run test -- processesService
```

**Step 3: Write the implementation**

```ts
export interface Process {
  pid: number;
  ppid: number;
  comm: string;
  state: string;
  rss_kb: number;
  cpu_time_s: number;
}

export interface ServiceStatus {
  name: string;
  state: string;
  pid: number | null;
  uptime_s: number;
  restarts: number;
  retry_in_s: number;
}

export interface ProcessesResponse {
  /** Null when the anyka-init control socket is unreachable. */
  supervised: ServiceStatus[] | null;
  processes: Process[];
}

export async function getProcesses(signal?: AbortSignal): Promise<ProcessesResponse> { /* ... */ }

export async function restartService(name: string): Promise<void> { /* POST, 202 = ok */ }
```

**Step 4: Run to verify they pass**

```bash
cd cross-compile/www && npm run test -- processesService
```

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/www/src/services/processesService.ts cross-compile/www/src/services/processesService.test.ts
git commit -m "feat(webui): add processesService for the process and restart API"
```

---

### Task 10: `ProcessesCard`

**Files:**
- Create: `cross-compile/www/src/components/ProcessesCard.tsx`
- Create: `cross-compile/www/src/components/ProcessesCard.test.tsx`

Use the existing primitives — `Card`, `Badge`, `Collapsible`, `AlertDialog`, `Button` all already live in `src/components/ui/`. Match the visual language of `VisionCard`/`PtzCard` in `DiagnosticsPage.tsx` (icon tile in the header, `dl`/rows, `data-testid` on everything).

**Step 1: Write the failing tests**

Per @anyka-webui-testing: `renderWithProviders`, `vi.mock('@/services/processesService')`, `data-testid` selectors.

Cover:
- running service renders pid and uptime
- backoff service renders the retry countdown and no pid
- `supervised: null` renders the unavailable note and no restart buttons
- the raw table is collapsed by default and expands on click
- clicking Restart opens the confirm dialog naming the service
- confirming calls `restartService` with that name
- the `onvif` row's dialog contains the vendor-daemon warning
- confirming `onvif` enters the reconnecting state

**Step 2: Run to verify they fail**

```bash
cd cross-compile/www && npm run test -- ProcessesCard
```

**Step 3: Write the implementation**

Structure:

```tsx
export function ProcessesCard() {
  const { data } = useQuery({
    queryKey: ['processes'],
    queryFn: ({ signal }) => getProcesses(signal),
    refetchInterval: 10_000,
  });
  // one AlertDialog, target held in state
  // onvif path: restartService -> tolerate a dropped connection ->
  //   waitForCameraBack(getDiagnostics, ...) -> invalidate ['processes']
  // other services: restartService -> toast -> invalidate ['processes']
}
```

Points to get right:
- Sort is fixed (RSS descending, already done by the backend). Do **not** add sortable columns.
- The `onvif` warning text must state plainly that `vendor-daemon` goes down with it and the page will drop.
- A dropped connection on the `onvif` restart POST is expected, not an error — do not surface it as a failed toast.

**Step 4: Run to verify they pass**

```bash
cd cross-compile/www && npm run test -- ProcessesCard
```

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/www/src/components/ProcessesCard.tsx cross-compile/www/src/components/ProcessesCard.test.tsx
git commit -m "feat(webui): add ProcessesCard with supervised services and restart"
```

---

### Task 11: Mount the card and run the full gate

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx` (render after `FirmwareUpdateCard`, before `SoundTestCard`)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Step 1: Write the failing test**

Assert `diagnostics-processes-card` is present on the rendered page.

**Step 2: Run to verify it fails**

```bash
cd cross-compile/www && npm run test -- DiagnosticsPage
```

**Step 3: Add one line**

```tsx
      <ProcessesCard />
```

**Step 4: Run the full gate**

```bash
cd cross-compile/www && npm run test
cd cross-compile/www && npm run verify
```

`npm run verify` covers type-check (both tsc versions), lint and format.

**Do not trust `rtk prettier --check`** — it has reported "All files formatted correctly" on a real exit-1 with drifted files. Run the raw binary and read `$?`:

```bash
cd cross-compile/www && npx prettier --check . ; echo "exit=$?"
```

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/www/src/pages/DiagnosticsPage.tsx cross-compile/www/src/pages/DiagnosticsPage.test.tsx
git commit -m "feat(webui): mount ProcessesCard on the diagnostics page"
```

---

## Phase 4 — Verification (manual, on hardware)

Not part of the automated plan. A human runs this after building and deploying a bundle with @anyka-firmware-upgrade.

1. **Socket exists and is root-only.** Over telnet: `ls -l /tmp/anyka-init.sock` → `srw-------`.
2. **Protocol answers by hand.** `echo status | nc -U /tmp/anyka-init.sock` → one TSV row per service. This is the payoff for choosing TSV over JSON.
3. **Page renders.** Load Diagnostics; the supervised table lists the same services as `anyka.toml`, with plausible uptimes.
4. **Restart a safe service.** Restart `snmp`; its uptime resets and `restarts` increments. Confirm from `rtk git`-independent evidence — the camera log, not just the UI.
5. **Restart `onvif`.** Confirm the dialog warns about `vendor-daemon`, the page enters "reconnecting", and it recovers on its own. Expect video to drop and return.
6. **Degraded path.** Confirm the UI shows "supervisor control unavailable" rather than erroring when the socket is absent (test by renaming it over telnet, then restoring).
7. **Check the log level.** A missing `[logging]` section defaults to `error`, so `grep` for a `warn!` returning zero proves nothing. Verify the level before drawing conclusions from log counts.

---

## Out of Scope

Listed so nobody helpfully adds them:

- Start/stop as separate actions from restart.
- Editing `[services.*]` from the WebUI.
- Per-process CPU percentage.
- Killing arbitrary non-supervised pids from the raw table.
- A `[control]` config key for the socket path (see the design doc for why it is a rollback hazard).
