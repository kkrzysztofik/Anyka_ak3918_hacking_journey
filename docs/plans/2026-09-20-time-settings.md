# Time Settings Tab Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every control on the WebUI Time tab reflect and change the camera's real clock, NTP servers and sync state.

**Architecture:** One owner per item. `anyka-init` owns `anyka.toml [time].servers` and writes `state/ntp.status`; `onvif-rust` never opens `anyka.toml` and instead sends `set-ntp` over the existing supervisor control socket; the WebUI reads status from the existing `/api/diagnostics` snapshot and writes settings over ONVIF SOAP. No new HTTP routes, no new threads, no new dependencies.

**Tech Stack:** Rust 2024 (vendored ARM toolchain), libc, `toml`, axum; React 19 + TypeScript, TanStack Query, react-hook-form + zod, Vitest + React Testing Library.

Design: `docs/plans/2026-09-20-time-settings-design.md`.

---

## Before You Start

Every Rust command in this plan assumes these two exports. The `PATH` prefix is
not optional — `cargo clippy` dies with `E0514` without it.

```bash
export PATH=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH
export CARGO=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
```

Host-side Rust always runs with `--target x86_64-unknown-linux-gnu`. Nothing in
this plan needs an ARM build until Task 16.

**Committing:** this repo's index is usually fully staged — `M` appears in
`git status`' *first* column. Always `git add` an explicit pathspec and never
run a bare `git commit -a`, or you will sweep unrelated in-flight work into
your commit.

Relevant skills: @anyka-rust-testing for Rust test conventions,
@anyka-webui-testing for the WebUI helpers, @camera-webui-components for the
design-system tokens, @anyka-embedded-build for the toolchain.

---

## Phase A — anyka-init: own the servers, publish the status

### Task 1: Generalize the TOML line editor to any value

`set_bool_in_text` writes only `true`/`false`. Task 2 needs to write a string
array. Generalizing is fewer lines than a parallel array writer and keeps one
code path under `verified()`.

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs:398-432`
- Test: same file, `mod tests`

**Step 1: Write the failing test**

Add to `cross-compile/anyka-init/src/config.rs` in `mod tests`:

```rust
#[test]
fn test_set_value_in_text_writes_an_array_and_keeps_comments() {
    let src = "\
# the operator's note, must survive
[time]
# IP first, deliberately
servers = [\"192.168.2.1\"]
timezone = \"UTC0\"
";
    let out = set_value_in_text(src, "[time]", "servers", "[\"a.example\", \"b.example\"]")
        .expect("edit must succeed");
    assert!(out.contains("servers = [\"a.example\", \"b.example\"]"));
    assert!(out.contains("# the operator's note, must survive"));
    assert!(out.contains("# IP first, deliberately"));
    assert!(out.contains("timezone = \"UTC0\""));
}

#[test]
fn test_set_value_in_text_preserves_crlf() {
    let src = "[time]\r\nservers = [\"old\"]\r\n";
    let out = set_value_in_text(src, "[time]", "servers", "[\"new\"]").unwrap();
    assert!(out.contains("servers = [\"new\"]\r\n"));
    assert!(!out.contains("servers = [\"new\"]\n\r"));
}

#[test]
fn test_set_value_in_text_rejects_an_edit_that_breaks_toml() {
    // `verified()` must catch a raw value that is not valid TOML.
    let src = "[time]\nservers = [\"old\"]\n";
    let err = set_value_in_text(src, "[time]", "servers", "[\"unterminated]");
    assert!(err.is_err(), "a malformed raw value must be refused, not written");
}

#[test]
fn test_set_bool_in_text_still_works_after_generalization() {
    let src = "[services.snmp]\nenabled = false\n";
    let out = set_bool_in_text(src, "[services.snmp]", "enabled", true).unwrap();
    assert!(out.contains("enabled = true"));
}
```

**Step 2: Run to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init set_value_in_text
```

Expected: FAIL — `cannot find function 'set_value_in_text'`.

**Step 3: Implement**

In `cross-compile/anyka-init/src/config.rs`, rename the existing
`set_bool_in_text` body to `set_value_in_text`, taking a pre-encoded raw value,
and reduce `set_bool_in_text` to a wrapper. Keep the existing doc comment on
`set_value_in_text` and adjust its first line.

```rust
/// Line-level edit of `key = <raw>` under `section`, where `raw` is already
/// encoded TOML (`true`, `"text"`, `["a", "b"]`).
///
/// Only the one line changes — comments, ordering and formatting everywhere
/// else survive byte-for-byte, which a TOML round-trip cannot guarantee. That
/// matters because this file is the operator's: hand-edited, comment-rich, and
/// holding the Wi-Fi credentials.
pub fn set_value_in_text(
    text: &str,
    section: &str,
    key: &str,
    raw: &str,
) -> Result<String, ConfigError> {
    // Preserve the file's line ending. Splitting on '\n' alone would leave a
    // '\r' on every existing line while the rewritten one has none, producing
    // a mixed-ending file out of a CRLF original.
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = text.split(nl).map(str::to_owned).collect();

    let Some(hdr) = out.iter().position(|l| strip_comment(l) == section) else {
        return Err(ConfigError::Invalid(format!(
            "no {section} stanza in config"
        )));
    };
    // The stanza ends at the next `[`-prefixed line, or at end of file.
    let end = out[hdr + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| i + hdr + 1)
        .unwrap_or(out.len());

    let Some(i) = (hdr + 1..end).find(|&i| line_key(&out[i]).is_some_and(|k| k == key)) else {
        out.insert(hdr + 1, format!("{key} = {raw}"));
        return verified(out.join(nl), section, key);
    };
    // Preserve the line's indentation, change only the value.
    let lead: String = out[i].chars().take_while(|c| c.is_whitespace()).collect();
    out[i] = format!("{lead}{key} = {raw}");
    verified(out.join(nl), section, key)
}

/// Line-level edit of a boolean, e.g. `enabled =` under `[services.<name>]`.
pub fn set_bool_in_text(
    text: &str,
    section: &str,
    key: &str,
    enabled: bool,
) -> Result<String, ConfigError> {
    set_value_in_text(text, section, key, if enabled { "true" } else { "false" })
}
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
```

Expected: PASS, including the pre-existing `set_bool_in_text` and
`set_service_enabled` tests — they must not change.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs
git commit -m "refactor(anyka-init): generalize set_bool_in_text to any TOML value

Task 2 needs to write a string array into [time].servers. One generalized
editor keeps a single code path under verified(), which is what turns a
mis-scanned edit into a refused write rather than a bricked camera.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Persist NTP servers into `anyka.toml`

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs` (near `set_service_enabled`, ~line 610)
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_set_time_servers_rewrites_the_array() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anyka.toml");
    std::fs::write(
        &path,
        "[time]\n# keep me\nservers = [\"old.example\"]\ntimezone = \"UTC0\"\n",
    )
    .unwrap();

    Config::set_time_servers(&path, &["a.example".into(), "192.168.2.1".into()]).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("servers = [\"a.example\", \"192.168.2.1\"]"));
    assert!(text.contains("# keep me"));
    // The written file must still parse as the real config.
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(parsed["time"]["servers"][1].as_str(), Some("192.168.2.1"));
}

#[test]
fn test_set_time_servers_rejects_quotes_and_whitespace() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anyka.toml");
    let original = "[time]\nservers = [\"old.example\"]\n";
    std::fs::write(&path, original).unwrap();

    for bad in ["a\"b", "a b", "a\\b", "", "a\nb"] {
        assert!(
            Config::set_time_servers(&path, &[bad.to_string()]).is_err(),
            "{bad:?} must be refused"
        );
    }
    // Nothing was written on any of those.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn test_set_time_servers_rejects_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anyka.toml");
    std::fs::write(&path, "[time]\nservers = [\"old.example\"]\n").unwrap();
    assert!(Config::set_time_servers(&path, &[]).is_err());
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init set_time_servers
```

Expected: FAIL — no function `set_time_servers`.

**Step 3: Implement**

Add next to `set_service_enabled` in `cross-compile/anyka-init/src/config.rs`:

```rust
/// A server string safe to embed in a TOML basic string without escaping.
///
/// Validated here as well as at the ONVIF boundary: this function writes into
/// the operator's config, and a stray quote or newline would either corrupt
/// the file or smuggle in a second key. `verified()` would catch the corrupt
/// case, but refusing the input is the clearer failure.
fn valid_ntp_server(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !s.chars().any(|c| {
            c.is_whitespace() || c.is_control() || c == '"' || c == '\\' || c == '#'
        })
}

/// Persist `[time].servers`. Same file-first atomic-write discipline as
/// `set_service_enabled`; updating the in-memory `Config` is the caller's job,
/// in that order.
pub fn set_time_servers(
    path: &std::path::Path,
    servers: &[String],
) -> Result<(), ConfigError> {
    if servers.is_empty() {
        return Err(ConfigError::Invalid(
            "at least one NTP server is required".to_string(),
        ));
    }
    if let Some(bad) = servers.iter().find(|s| !valid_ntp_server(s)) {
        return Err(ConfigError::Invalid(format!(
            "rejected NTP server {bad:?}: must be non-empty and free of whitespace, quotes, backslashes and '#'"
        )));
    }
    let raw = format!(
        "[{}]",
        servers
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let text = read_config_text(path)?;
    let new_text = set_value_in_text(&text, "[time]", "servers", &raw)?;
    persist_text(path, &new_text)
}
```

Place `set_time_servers` inside the same `impl Config` block as
`set_service_enabled`; `valid_ntp_server` goes at module level next to
`line_key`.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs
git commit -m "feat(anyka-init): persist [time].servers with input validation

Server strings land in a TOML array inside the operator's hand-edited file,
so quotes, whitespace, backslashes and '#' are refused before the write
rather than caught afterwards by verified().

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: Add the `set-ntp` control verb

**Files:**
- Modify: `cross-compile/anyka-init/src/control.rs:78-104`
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_parse_request_set_ntp_takes_one_or_more_servers() {
    assert_eq!(
        parse_request("set-ntp a.example\n"),
        Some(Request::SetNtp(vec!["a.example".into()]))
    );
    assert_eq!(
        parse_request("set-ntp a.example 192.168.2.1\n"),
        Some(Request::SetNtp(vec!["a.example".into(), "192.168.2.1".into()]))
    );
}

#[test]
fn test_parse_request_set_ntp_rejects_empty_and_tabs() {
    assert_eq!(parse_request("set-ntp \n"), None);
    assert_eq!(parse_request("set-ntp a\tb\n"), None);
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init parse_request_set_ntp
```

Expected: FAIL — no variant `SetNtp`.

**Step 3: Implement**

In `cross-compile/anyka-init/src/control.rs`, extend the enum and the parser.
The existing `name.is_empty() || name.contains('\t')` guard already runs before
the match, so tabs and blanks are handled for free — and keeping tabs out is
what makes the TSV status frame in Task 5 safe.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Restart(String),
    Enable(String),
    Disable(String),
    /// Replace `[time].servers`. One or more whitespace-separated hosts.
    SetNtp(Vec<String>),
}
```

In `parse_request`, update the doc comment to mention
`"set-ntp <s1> [s2…]\n"` and add the arm:

```rust
        "set-ntp" => {
            let servers: Vec<String> =
                name.split_whitespace().map(str::to_owned).collect();
            if servers.is_empty() {
                return None;
            }
            Some(Request::SetNtp(servers))
        }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/control.rs
git commit -m "feat(anyka-init): add the set-ntp control verb

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: Wire `set-ntp` through the supervisor loop

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` — `Msg` (~line 40), `handle_control_conn` (~line 544), `dispatch_msg` (~line 689), new `handle_set_ntp`
- Test: same file

**Step 1: Write the failing test**

Model it on the existing toggle tests. Add to `mod tests` in
`supervisor_loop.rs`:

```rust
#[test]
fn test_handle_set_ntp_writes_the_file_then_memory() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("anyka.toml");
    std::fs::write(&cfg_path, "[time]\nservers = [\"old.example\"]\n").unwrap();

    let mut cfg = Config::default();
    cfg.time.servers = vec!["old.example".into()];
    let (reply_tx, reply_rx) = channel();

    handle_set_ntp(
        &mut cfg,
        &cfg_path,
        vec!["new.example".into()],
        &reply_tx,
    );

    assert_eq!(reply_rx.try_recv(), Ok(control::ToggleOutcome::Ok));
    assert_eq!(cfg.time.servers, vec!["new.example".to_string()]);
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(text.contains("servers = [\"new.example\"]"));
}

#[test]
fn test_handle_set_ntp_leaves_memory_alone_when_the_write_fails() {
    let mut cfg = Config::default();
    cfg.time.servers = vec!["old.example".into()];
    let (reply_tx, reply_rx) = channel();

    handle_set_ntp(
        &mut cfg,
        Path::new("/nonexistent/anyka.toml"),
        vec!["new.example".into()],
        &reply_tx,
    );

    assert_eq!(reply_rx.try_recv(), Ok(control::ToggleOutcome::Error));
    assert_eq!(cfg.time.servers, vec!["old.example".to_string()]);
}

#[test]
fn test_handle_set_ntp_rejects_a_bad_server_without_touching_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("anyka.toml");
    let original = "[time]\nservers = [\"old.example\"]\n";
    std::fs::write(&cfg_path, original).unwrap();

    let mut cfg = Config::default();
    let (reply_tx, reply_rx) = channel();
    handle_set_ntp(&mut cfg, &cfg_path, vec!["bad host".into()], &reply_tx);

    assert_eq!(reply_rx.try_recv(), Ok(control::ToggleOutcome::Error));
    assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), original);
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init handle_set_ntp
```

Expected: FAIL — no function `handle_set_ntp`.

**Step 3: Implement**

Add the message variant to `Msg`:

```rust
    /// Replace `[time].servers`. Persists to `anyka.toml` first, then updates
    /// in-memory state, the same order as `ToggleService`.
    SetNtpServers {
        servers: Vec<String>,
        reply: Sender<control::ToggleOutcome>,
    },
```

Add the handler. It takes `&mut Config` rather than `LoopCtx` because it needs
nothing else, which also keeps it directly testable:

```rust
fn handle_set_ntp(
    cfg: &mut Config,
    config_path: &Path,
    servers: Vec<String>,
    reply: &Sender<control::ToggleOutcome>,
) {
    if let Err(e) = Config::set_time_servers(config_path, &servers) {
        tracing::error!(error = %e, "set-ntp: config write failed; not applied");
        let _ = reply.send(control::ToggleOutcome::Error);
        return;
    }
    tracing::info!(?servers, "NTP servers updated");
    cfg.time.servers = servers;
    let _ = reply.send(control::ToggleOutcome::Ok);
}
```

In `dispatch_msg`, add the arm alongside `Msg::ToggleService`:

```rust
        Msg::SetNtpServers { servers, reply } => {
            handle_set_ntp(ctx.cfg, ctx.config_path, servers, &reply);
        }
```

In `handle_control_conn`, add the request arm. Reuse `send_toggle`'s
reply-timeout shape rather than inventing a second one — read `send_toggle`
(around line 589) and mirror it:

```rust
        Some(control::Request::SetNtp(servers)) => {
            let (reply_tx, reply_rx) = channel();
            if tx
                .send(Msg::SetNtpServers {
                    servers,
                    reply: reply_tx,
                })
                .is_err()
            {
                let _ = stream.write_all(b"error\n");
                return Ok(());
            }
            let word = match reply_rx.recv_timeout(CONTROL_REPLY_TIMEOUT) {
                Ok(control::ToggleOutcome::Ok) => "ok",
                Ok(control::ToggleOutcome::Unknown) => "unknown",
                Ok(control::ToggleOutcome::Error) => "error",
                Err(_) => "pending",
            };
            let _ = stream.write_all(format!("{word}\n").as_bytes());
        }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu -p anyka-init --all-targets -- -D warnings
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervisor_loop.rs
git commit -m "feat(anyka-init): apply set-ntp, file before memory

Same visible order as handle_toggle_service: a failed config write leaves
in-memory state untouched, so the two can never disagree.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: Re-read `[time]` per resync and publish `state/ntp.status`

Two changes in one task because they share the loop signature.

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs` — `resync_loop` (~line 302), new `reload_time_cfg` and `write_status`
- Modify: `cross-compile/anyka-init/src/main.rs:220-230` (call site)
- Test: `cross-compile/anyka-init/src/timesync.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_reload_time_cfg_picks_up_a_changed_server_list() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anyka.toml");
    std::fs::write(&path, "[time]\nservers = [\"new.example\"]\n").unwrap();

    let current = TimeCfg {
        servers: vec!["old.example".into()],
        ..TimeCfg::default()
    };
    let got = reload_time_cfg(&path, &current);
    assert_eq!(got.servers, vec!["new.example".to_string()]);
}

#[test]
fn test_reload_time_cfg_keeps_the_current_value_on_a_broken_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anyka.toml");
    std::fs::write(&path, "this is not toml {{{").unwrap();

    let current = TimeCfg {
        servers: vec!["old.example".into()],
        ..TimeCfg::default()
    };
    let got = reload_time_cfg(&path, &current);
    assert_eq!(
        got.servers,
        vec!["old.example".to_string()],
        "a hand-edit mid-flight must not blank the server list"
    );
}

#[test]
fn test_write_status_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    write_status(
        dir.path(),
        Some((1_760_000_000, "192.168.2.1", -3)),
        &["192.168.2.1".to_string(), "pool.example".to_string()],
    );
    let text = std::fs::read_to_string(dir.path().join("state/ntp.status")).unwrap();
    assert_eq!(
        text.trim_end(),
        "1760000000\t192.168.2.1\t-3\t192.168.2.1,pool.example"
    );
}

#[test]
fn test_write_status_with_no_sync_yet() {
    let dir = tempfile::tempdir().unwrap();
    write_status(dir.path(), None, &["pool.example".to_string()]);
    let text = std::fs::read_to_string(dir.path().join("state/ntp.status")).unwrap();
    assert_eq!(text.trim_end(), "0\t\t0\tpool.example");
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init reload_time_cfg
```

Expected: FAIL — no function `reload_time_cfg`.

**Step 3: Implement**

In `cross-compile/anyka-init/src/timesync.rs`:

```rust
/// `{update_root}/state/ntp.status` — advisory sync state for the WebUI.
///
/// A sibling of `ntp.disabled` rather than a key in `anyka.toml`: adding a key
/// there would be a hard parse error for an older anyka-init in the other A/B
/// slot (`deny_unknown_fields`). Unlike the marker this one *is* parsed, which
/// is safe because it is display-only — a garbled line shows "unknown", it
/// never changes behaviour.
pub fn ntp_status_path(update_root: &Path) -> PathBuf {
    update_root.join("state/ntp.status")
}

/// One TSV line: `last_unix \t server \t delta_s \t s1,s2,…`.
/// `last_unix` is 0 when no sync has succeeded yet.
pub fn write_status(update_root: &Path, last: Option<(i64, &str, i64)>, servers: &[String]) {
    let path = ntp_status_path(update_root);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let (unix, server, delta) = last.unwrap_or((0, "", 0));
    let line = format!("{unix}\t{server}\t{delta}\t{}\n", servers.join(","));
    let _ = std::fs::write(&path, line);
}

/// Re-read `[time]` so a `set-ntp` (or a hand-edit) takes effect without a
/// reboot. Any read or parse failure keeps `current` — a config being edited
/// under us must not blank the server list.
pub fn reload_time_cfg(config_path: &Path, current: &TimeCfg) -> TimeCfg {
    match crate::config::Config::load_without_overlay(&config_path.to_string_lossy()) {
        Ok(cfg) => cfg.time,
        Err(e) => {
            tracing::warn!(error = %e, "could not reload [time]; keeping the running value");
            current.clone()
        }
    }
}
```

Then rewrite `resync_loop`. Keep the existing doc comment about why the first
retry is fast, and add the re-read:

```rust
/// Background resync loop, started after P3.
pub fn resync_loop(sys: &dyn Sys, cfg: &TimeCfg, ntp_disabled: &Path, config_path: &Path) {
    // Until the clock has been set once, retry at `retry_interval_sec`, not
    // `resync_interval_sec`. P2.5 gives up after 15s so that boot is not held
    // hostage to the network, which means a slow wifi association routinely
    // lands here with the clock still at the epoch. Sleeping the full 6h resync
    // interval first would leave ws_security.rs:85 (clock_skew_seconds = 300)
    // rejecting every authenticated ONVIF request for those 6 hours.
    let update_root = ntp_disabled
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(Path::new("/"))
        .to_path_buf();
    let mut cfg = cfg.clone();
    let mut synced = false;
    let mut last: Option<(i64, String, i64)> = None;

    write_status(&update_root, None, &cfg.servers);

    loop {
        std::thread::sleep(Duration::from_secs(resync_wait_secs(synced, &cfg)));
        // ponytail: re-parsing the whole config once per resync (6h steady
        // state) is the cheap way to pick up a `set-ntp`. If servers ever need
        // to apply in seconds, share a TimeCfg cell with the supervisor loop
        // instead of shortening this sleep.
        cfg = reload_time_cfg(config_path, &cfg);
        if let Some(delta) = sync_once(sys, &cfg, None, ntp_disabled) {
            synced = true;
            let server = cfg.servers.first().cloned().unwrap_or_default();
            last = Some((now_unix(sys), server, delta));
        }
        let borrowed = last.as_ref().map(|(u, s, d)| (*u, s.as_str(), *d));
        write_status(&update_root, borrowed, &cfg.servers);
    }
}
```

`sync_once` returns `Option<i64>` (the applied delta) but not *which* server
answered. Recording `cfg.servers.first()` would be a lie when the first server
times out and the second answers.

**Change `sync_once` to return the server too.** Its signature becomes
`Option<(String, i64)>`; in the success arms return `Some((server.clone(), delta))`.
Update `first_sync`, which only checks `.is_some()`, and the existing
`sync_once` tests, which assert on the delta — they become
`assert_eq!(got.map(|(_, d)| d), Some(0))` and similar. Then use the real
server name in `resync_loop` above.

Add a small `now_unix(sys: &dyn Sys) -> i64` helper next to `write_status`
using the existing `sys.realtime()` accessor — read how `delta_secs` consumes
it (around line 240) and match that conversion.

Finally, update the call site in `cross-compile/anyka-init/src/main.rs:220-230`:

```rust
    if cfg.time.enabled {
        let s = Arc::clone(sysimpl);
        let tcfg = cfg.time.clone();
        let ntp_marker = timesync::ntp_disabled_marker_path(std::path::Path::new(&cfg.update.root));
        let config_path = std::path::PathBuf::from(CONFIG_PATH);
        let _ = std::thread::Builder::new()
            .name("timesync".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                timesync::resync_loop(s.as_ref(), &tcfg, &ntp_marker, &config_path);
            });
    }
```

`CONFIG_PATH` is already a module const in `main.rs:11`. Check whether
`spawn_optional_threads` has it in scope; if not, pass it in as a parameter
rather than re-declaring the literal.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu -p anyka-init --all-targets -- -D warnings
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/timesync.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): re-read [time] per resync and publish ntp.status

Server changes now apply within one resync interval instead of needing a
reboot, and sync state reaches onvif-rust through a state/ file rather than
a new anyka.toml key that would break rollback to the other A/B slot.

sync_once now reports which server answered, so the status line cannot
credit the first configured server when the second one replied.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Phase B — onvif-rust

### Task 6: Report the real clock mode

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs:196-244`
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_get_system_date_and_time_reports_ntp_when_the_marker_is_absent() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config_with_update_root(dir.path());

    let got = handle_get_system_date_and_time(&config, GetSystemDateAndTime {}).unwrap();
    assert_eq!(
        got.system_date_and_time.date_time_type,
        SetDateTimeType::NTP
    );
}

#[test]
fn test_get_system_date_and_time_reports_manual_when_the_marker_exists() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config_with_update_root(dir.path());
    crate::time::ntp_marker::NtpMarker::new(dir.path())
        .disable()
        .unwrap();

    let got = handle_get_system_date_and_time(&config, GetSystemDateAndTime {}).unwrap();
    assert_eq!(
        got.system_date_and_time.date_time_type,
        SetDateTimeType::Manual
    );
}
```

Look at the existing `SetSystemDateAndTime` tests in this file for the
established way to build a `ConfigRuntime` with a temp `update.root`; reuse
that helper instead of writing `test_config_with_update_root` if one already
exists.

**Step 2: Run to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust get_system_date_and_time_reports
```

Expected: FAIL — both assert `Manual`, so the NTP case fails.

**Step 3: Implement**

In `handle_get_system_date_and_time`, replace the hardcoded
`date_time_type: SetDateTimeType::Manual`. Clone `update.root` out of the guard
before use — the function's own doc comment warns about holding the config
guard, and `handle_set_system_date_and_time` already does exactly this.

```rust
    let (tz_string, update_root) = {
        let guard = config.read();
        (guard.time.timezone.clone(), guard.update.root.clone())
    };
    let ntp_enabled = crate::time::ntp_marker::NtpMarker::new(&update_root).ntp_enabled();
```

Then use `date_time_type: if ntp_enabled { SetDateTimeType::NTP } else { SetDateTimeType::Manual }`
and `tz: tz_string` in the `TimeZone`, dropping the second `config.read()`.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/ops/system.rs
git commit -m "fix(onvif): report the real clock mode, not a hardcoded Manual

GetSystemDateAndTime always answered Manual, so the WebUI landed on Manual
even while NTP was running and saving silently suspended it.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: Persist the timezone

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs:422-424`
- Test: `cross-compile/onvif-rust/src/onvif/device/service.rs`

**Step 1: Write the failing test**

Follow the pattern of the existing `SetHostname` persistence test in this file
— it asserts the store's save was requested. If there is no such test, assert
on the config generation counter instead:

```rust
#[test]
fn test_set_system_date_and_time_requests_a_save() {
    let service = test_device_service();
    let before = service.store.config.generation();

    let body = r#"<tds:SetSystemDateAndTime>
        <tds:DateTimeType>NTP</tds:DateTimeType>
        <tds:DaylightSavings>false</tds:DaylightSavings>
        <tds:TimeZone><tt:TZ>UTC0</tt:TZ></tds:TimeZone>
    </tds:SetSystemDateAndTime>"#;
    // dispatch through the same path the SOAP server uses
    futures::executor::block_on(service.dispatch("SetSystemDateAndTime", body)).unwrap();

    assert!(service.store.config.generation() > before);
    assert!(service.store.save_was_requested(), "timezone must survive a restart");
}
```

Adapt the harness to whatever `test_device_service()` equivalent already
exists in this file — do not invent a new one.

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust set_system_date_and_time_requests_a_save
```

**Step 3: Implement**

`cross-compile/onvif-rust/src/onvif/device/service.rs:422`:

```rust
            "SetSystemDateAndTime" => {
                let result = dispatch_sync(body_xml, |request: SetSystemDateAndTime| {
                    system_ops::handle_set_system_date_and_time(&config, request)
                });
                self.store.request_save();
                result
            }
```

Place the `request_save()` after a successful dispatch only if the surrounding
arms do so — check how `SetHostname` at line 176 orders it and match.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/service.rs
git commit -m "fix(onvif): persist the timezone set by SetSystemDateAndTime

Every sibling setter calls request_save(); this one did not, so the zone
reverted on the next restart.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: Implement `SetNTP`

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/services.rs` (client)
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/network.rs:504-515`
- Test: both files

**Step 1: Write the failing tests**

In `services.rs`, extend the existing mock-control-server test
(`test_round_trip_against_a_mock_control_server`) rather than writing a new
server:

```rust
#[test]
fn test_request_set_ntp_sends_space_separated_servers() {
    // Mirror test_request_toggle_sends_the_right_verb: capture the request
    // line the client writes and assert on it.
    let (path, received) = spawn_capturing_server("ok\n");
    assert_eq!(
        request_set_ntp(&path, &["a.example".into(), "192.168.2.1".into()]),
        ToggleReply::Accepted
    );
    assert_eq!(received.recv().unwrap(), "set-ntp a.example 192.168.2.1\n");
}

#[test]
fn test_request_set_ntp_on_a_missing_socket_is_unreachable() {
    assert_eq!(
        request_set_ntp(Path::new("/nonexistent/sock"), &["a.example".into()]),
        ToggleReply::Unreachable
    );
}
```

In `network.rs`:

```rust
#[tokio::test]
async fn test_set_ntp_rejects_a_server_with_whitespace_or_quotes() {
    for bad in ["a b", "a\"b", "a\\b", "", "a\nb"] {
        let req = SetNTP {
            from_dhcp: false,
            ntp_manual: vec![NetworkHost::dns(bad)],
        };
        let err = handle_set_ntp(Path::new("/nonexistent/sock"), req)
            .await
            .expect_err("must fault");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { .. }),
            "{bad:?} must be an InvalidArgVal fault, got {err:?}"
        );
    }
}

#[tokio::test]
async fn test_set_ntp_requires_at_least_one_server() {
    let req = SetNTP { from_dhcp: false, ntp_manual: vec![] };
    assert!(handle_set_ntp(Path::new("/nonexistent/sock"), req).await.is_err());
}
```

Match `OnvifError`'s actual constructors — `handle_set_system_date_and_time`
uses `OnvifError::invalid_arg("ter:InvalidArgVal", msg)`; use the same.

**Step 2: Run to verify they fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust request_set_ntp
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust set_ntp_rejects
```

**Step 3: Implement**

Client, in `cross-compile/onvif-rust/src/diagnostics/services.rs` next to
`request_toggle`:

```rust
/// Blocking. Sends `set-ntp <s1> <s2>…` to the supervisor, which owns
/// `anyka.toml`. Servers must already be validated by the caller.
pub fn request_set_ntp(path: &Path, servers: &[String]) -> ToggleReply {
    match round_trip(path, &format!("set-ntp {}\n", servers.join(" "))) {
        None => ToggleReply::Unreachable,
        Some(r) => match r.trim() {
            "ok" => ToggleReply::Accepted,
            "unknown" => ToggleReply::Unknown,
            "pending" => ToggleReply::Pending,
            _ => ToggleReply::Error,
        },
    }
}
```

Handler, replacing the `ActionNotSupported` body in `network.rs:507`:

```rust
/// Reject anything that cannot go into a TOML basic string unescaped. The
/// supervisor validates again before writing; this copy exists so the caller
/// gets a precise ONVIF fault instead of a generic write failure.
fn valid_ntp_server(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !s.chars().any(|c| {
            c.is_whitespace() || c.is_control() || c == '"' || c == '\\' || c == '#'
        })
}

/// Handle SetNTP request.
///
/// `from_dhcp` is refused: udhcpc on this camera never supplies NTP servers,
/// so accepting it would silently do nothing. The server list is owned by
/// `anyka.toml [time].servers` and written by the supervisor, never by us.
pub async fn handle_set_ntp(socket_path: &Path, request: SetNTP) -> OnvifResult<SetNTPResponse> {
    if request.from_dhcp {
        return Err(OnvifError::invalid_arg(
            "ter:InvalidArgVal",
            "from_dhcp is not supported: this camera's DHCP client does not supply NTP servers",
        ));
    }

    let servers: Vec<String> = request
        .ntp_manual
        .iter()
        .map(host_to_string)
        .collect();

    if servers.is_empty() {
        return Err(OnvifError::invalid_arg(
            "ter:InvalidArgVal",
            "at least one NTP server is required",
        ));
    }
    if let Some(bad) = servers.iter().find(|s| !valid_ntp_server(s)) {
        return Err(OnvifError::invalid_arg(
            "ter:InvalidArgVal",
            format!("rejected NTP server {bad:?}"),
        ));
    }

    match crate::diagnostics::services::request_set_ntp(socket_path, &servers) {
        crate::diagnostics::services::ToggleReply::Accepted
        | crate::diagnostics::services::ToggleReply::Pending => {
            tracing::info!(?servers, "NTP servers handed to the supervisor");
            Ok(SetNTPResponse {})
        }
        crate::diagnostics::services::ToggleReply::Unreachable => Err(
            OnvifError::HardwareFailure("supervisor control socket unreachable".to_string()),
        ),
        other => Err(OnvifError::HardwareFailure(format!(
            "supervisor refused the NTP update: {other:?}"
        ))),
    }
}
```

Write `host_to_string` as the inverse of the existing `to_network_host` closure
in `handle_get_ntp` — read that closure first and mirror its field access.

Update the dispatch arm at `service.rs:549` to pass
`Path::new(crate::diagnostics::services::SOCKET_PATH)`.

**Step 4: Run to verify they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/services.rs cross-compile/onvif-rust/src/onvif/device/ops/network.rs cross-compile/onvif-rust/src/onvif/device/service.rs
git commit -m "feat(onvif): implement SetNTP via the supervisor control socket

onvif-rust never opens anyka.toml; the supervisor owns that file. from_dhcp
is refused outright rather than accepted and ignored, because udhcpc here
never supplies NTP servers.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: Read the real servers and sync status

**Files:**
- Create: `cross-compile/onvif-rust/src/time/ntp_status.rs`
- Modify: `cross-compile/onvif-rust/src/time/mod.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/network_info.rs:118-132, 271-281`
- Test: `ntp_status.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_parse_reads_a_full_status_line() {
    let got = parse("1760000000\t192.168.2.1\t-3\t192.168.2.1,pool.example\n").unwrap();
    assert_eq!(got.last_sync_unix, Some(1_760_000_000));
    assert_eq!(got.last_server.as_deref(), Some("192.168.2.1"));
    assert_eq!(got.last_delta_s, Some(-3));
    assert_eq!(got.servers, vec!["192.168.2.1", "pool.example"]);
}

#[test]
fn test_parse_reads_a_never_synced_line() {
    let got = parse("0\t\t0\tpool.example\n").unwrap();
    assert_eq!(got.last_sync_unix, None);
    assert_eq!(got.last_server, None);
    assert_eq!(got.servers, vec!["pool.example"]);
}

#[test]
fn test_parse_rejects_garbage_rather_than_guessing() {
    assert!(parse("").is_none());
    assert!(parse("nonsense").is_none());
    assert!(parse("abc\tx\t0\ts").is_none());
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust ntp_status
```

**Step 3: Implement**

`cross-compile/onvif-rust/src/time/ntp_status.rs`:

```rust
//! Reader for `{update_root}/state/ntp.status`, written by anyka-init's
//! timesync thread.
//!
//! Display-only: a missing or malformed file means "unknown", never a
//! behaviour change. That is what makes it safe to parse, unlike the
//! deliberately unparsed `ntp.disabled` marker next to it.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct NtpStatus {
    /// `None` until the first successful sync since boot.
    pub last_sync_unix: Option<i64>,
    pub last_server: Option<String>,
    pub last_delta_s: Option<i64>,
    pub servers: Vec<String>,
}

pub fn path(update_root: impl AsRef<Path>) -> PathBuf {
    update_root.as_ref().join("state/ntp.status")
}

/// `last_unix \t server \t delta_s \t s1,s2,…`
pub fn parse(text: &str) -> Option<NtpStatus> {
    let line = text.lines().next()?;
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 4 {
        return None;
    }
    let unix: i64 = f[0].parse().ok()?;
    let delta: i64 = f[2].parse().ok()?;
    let synced = unix > 0;
    Some(NtpStatus {
        last_sync_unix: synced.then_some(unix),
        last_server: synced.then(|| f[1].to_owned()).filter(|s| !s.is_empty()),
        last_delta_s: synced.then_some(delta),
        servers: f[3]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}

/// `None` when the file is absent (NTP disabled, or an older anyka-init).
pub fn read(update_root: impl AsRef<Path>) -> Option<NtpStatus> {
    parse(&std::fs::read_to_string(path(update_root)).ok()?)
}
```

Add `pub mod ntp_status;` to `cross-compile/onvif-rust/src/time/mod.rs`.

In `network_info.rs`, give `AnykaNetworkInfo` a status path alongside
`overlay_path`, following the existing `with_overlay_path` test-constructor
precedent:

```rust
pub(super) struct AnykaNetworkInfo {
    overlay_path: std::path::PathBuf,
    ntp_status_path: std::path::PathBuf,
}
```

`new()` sets it from `crate::time::ntp_status::path("/mnt/anyka_hack")`; add a
`#[cfg(test)] with_ntp_status_path`. Then replace `read_ntp_config`'s body:

```rust
    /// Real servers come from the supervisor's status file. The `/etc/ntp.conf`
    /// and `timesyncd.conf` parsers below stay for the stub platform only —
    /// neither file exists on this camera.
    fn read_ntp_config(&self) -> NtpInfo {
        if let Ok(text) = std::fs::read_to_string(&self.ntp_status_path)
            && let Some(status) = crate::time::ntp_status::parse(&text)
        {
            return NtpInfo {
                // udhcpc here never supplies NTP; saying otherwise would be a lie.
                from_dhcp: false,
                ntp_from_dhcp: vec![],
                ntp_manual: status.servers,
            };
        }
        NtpInfo::default()
    }
```

`read_ntp_config` is currently an associated function (`Self::read_ntp_config()`);
it becomes a method, so update the `get_ntp_info` call site to `self.read_ntp_config()`.
Leave `parse_ntp_conf`/`parse_timesyncd_conf` and their tests in place — the
stub platform still exercises them.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/time/ cross-compile/onvif-rust/src/platform/anyka/network_info.rs
git commit -m "feat(onvif): read NTP servers and sync state from state/ntp.status

GetNTP parsed /etc/ntp.conf and timesyncd.conf, neither of which exists on
this camera, so it returned an empty list. The real servers live in
anyka.toml and are now published by the supervisor.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 10: Delete the dead NTP config fields

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs:436-438, 457-459`

**Step 1: Confirm they are unused**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
grep -rn "ntp_primary\|ntp_secondary" onvif-rust/src/
```

Expected: hits **only** in `config/types.rs`. Note that `ntp_from_dhcp` also
appears in `platform/common/traits.rs`, `platform/stub/mod.rs`,
`onvif/device/ops/network.rs` and `onvif/types/device.rs` — those are the
*`NtpInfo`/`NTPInformation`* fields, which stay. Only the three on
`NetworkConfig` go. If you see any other `NetworkConfig` hit, **stop** and say so.

**Step 2: Delete**

Remove `ntp_from_dhcp`, `ntp_primary` and `ntp_secondary` from `NetworkConfig`
and from its `Default` impl. Because `NetworkConfig` is `#[serde(default)]`,
a deployed `config.toml` still carrying those keys keeps loading.

**Step 3: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust --all-targets -- -D warnings
```

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/config/types.rs
git commit -m "refactor(onvif): drop NetworkConfig's dead NTP fields

Read by nothing. The live server list is anyka.toml's.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 11: Expose sync status on `/api/diagnostics`

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/state.rs` — `Snapshot` (~line 63) and the builder
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_snapshot_carries_ntp_status_when_the_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("state")).unwrap();
    std::fs::write(
        dir.path().join("state/ntp.status"),
        "1760000000\t192.168.2.1\t-3\t192.168.2.1\n",
    )
    .unwrap();

    let state = DiagnosticsState::for_test_with_update_root(dir.path());
    let snap = state.snapshot();
    let time = snap.time.expect("status file present");
    assert_eq!(time.last_server.as_deref(), Some("192.168.2.1"));
}

#[test]
fn test_snapshot_time_is_none_without_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let state = DiagnosticsState::for_test_with_update_root(dir.path());
    assert!(state.snapshot().time.is_none());
}
```

`DiagnosticsState` is constructed during the Network startup phase; read its
existing constructor and test helpers before adding `for_test_with_update_root`
— reuse whatever seam is already there.

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust snapshot_carries_ntp_status
```

**Step 3: Implement**

Add to `Snapshot`:

```rust
    /// NTP sync state from the supervisor. `None` when NTP is disabled or the
    /// supervisor is an older build that does not publish it.
    pub time: Option<crate::time::ntp_status::NtpStatus>,
```

Store the update root on `DiagnosticsState` (mirroring how
`UpdateState::from_update_root` receives it in `server.rs:699`) and populate
the field with `crate::time::ntp_status::read(&self.update_root)`.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust --all-targets -- -D warnings
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/state.rs cross-compile/onvif-rust/src/onvif/server.rs
git commit -m "feat(onvif): report NTP sync state on /api/diagnostics

A field on the snapshot the WebUI already fetches, rather than a new route.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Phase C — WebUI

All commands run from `cross-compile/www`.

### Task 12: Make `timeService` honest

**Files:**
- Modify: `cross-compile/www/src/services/timeService.ts`
- Modify: `cross-compile/www/src/services/diagnosticsService.ts` (type + guard)
- Test: `cross-compile/www/src/services/timeService.test.ts`

**Step 1: Write the failing test**

```ts
describe('getNtp', () => {
  it('returns the servers the camera reports', async () => {
    mockSoapResponse({
      GetNTPResponse: {
        NTPInformation: {
          FromDHCP: 'false',
          NTPManual: [{ DNSname: 'pool.example' }, { IPv4Address: '192.168.2.1' }],
        },
      },
    });
    await expect(getNtp()).resolves.toEqual(['pool.example', '192.168.2.1']);
  });

  it('returns an empty list when the camera reports none', async () => {
    mockSoapResponse({ GetNTPResponse: { NTPInformation: {} } });
    await expect(getNtp()).resolves.toEqual([]);
  });
});

describe('setNtp', () => {
  it('sends each server as an NTPManual entry', async () => {
    const body = await captureSoapBody(() => setNtp(['pool.example', '192.168.2.1']));
    expect(body).toContain('<tt:DNSname>pool.example</tt:DNSname>');
    expect(body).toContain('<tt:IPv4Address>192.168.2.1</tt:IPv4Address>');
    expect(body).toContain('<tds:FromDHCP>false</tds:FromDHCP>');
  });
});

describe('getDateTime', () => {
  it('reports NTP mode from the camera, not a stub', async () => {
    mockSoapResponse({
      GetSystemDateAndTimeResponse: {
        SystemDateAndTime: {
          DateTimeType: 'NTP',
          DaylightSavings: 'true',
          TimeZone: { TZ: 'CET-1CEST,M3.5.0,M10.5.0/3' },
          UTCDateTime: {
            Date: { Year: 2026, Month: 9, Day: 20 },
            Time: { Hour: 12, Minute: 0, Second: 0 },
          },
        },
      },
    });
    const got = await getDateTime();
    expect(got.ntp.enabled).toBe(true);
    expect(got.daylightSavings).toBe(true);
    expect(got.utcDateTime.toISOString()).toBe('2026-09-20T12:00:00.000Z');
    expect(got).not.toHaveProperty('ntp.fromDHCP');
  });
});
```

Use whatever SOAP mocking helpers `timeService.test.ts` and its siblings
already use — read `networkService.test.ts` first; do not invent
`mockSoapResponse`/`captureSoapBody` if equivalents exist.

**Step 2: Run to verify it fails**

```bash
npx vitest run src/services/timeService.test.ts
```

**Step 3: Implement**

In `cross-compile/www/src/services/timeService.ts`:

- Delete `fromDHCP` from `DateTimeConfig` and the `fromDHCP: true` stub at
  line 118. Add `daylightSavings: boolean` and keep `utcDateTime`.
- Delete `setNTP`. Replace `setDateTime`'s hardcoded `false` daylight-savings
  argument with the caller's value.
- Add `getNtp(): Promise<string[]>` using `soapBodies` + `soapRequest` against
  `ENDPOINTS.device` with `'GetNTPResponse'`, reading `NTPManual` entries'
  `DNSname` / `IPv4Address` / `IPv6Address` and tolerating a single object as
  well as an array (`fast-xml-parser` collapses one-element lists).
- Add `setNtp(servers: string[]): Promise<void>` emitting
  `<tds:SetNTP><tds:FromDHCP>false</tds:FromDHCP>` plus one
  `<tds:NTPManual>` per server, choosing `<tt:IPv4Address>` when the string
  parses as an IPv4 literal and `<tt:DNSname>` otherwise, with `Type` set to
  match. Run every value through the existing `escapeXml`.

In `diagnosticsService.ts`, add the optional `time` field to the `Diagnostics`
type and extend `isDiagnostics` to accept it — keep it optional so an older
camera still validates.

**Step 4: Run to verify it passes**

```bash
npx vitest run src/services/timeService.test.ts src/services/diagnosticsService.test.ts
```

**Step 5: Commit**

```bash
git add cross-compile/www/src/services/timeService.ts cross-compile/www/src/services/timeService.test.ts cross-compile/www/src/services/diagnosticsService.ts
git commit -m "feat(webui): real GetNTP/SetNTP, drop the fromDHCP stub

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 13: Show the camera's clock, not the browser's

This is the gap that matters most: the panel currently reads correct while the
camera sits at 1970.

**Files:**
- Modify: `cross-compile/www/src/pages/settings/TimePage.tsx:49-73, 171-197`
- Test: `cross-compile/www/src/pages/settings/TimePage.test.tsx`

**Step 1: Write the failing test**

```ts
it('should display the camera time, not the browser time', async () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-09-20T12:00:00Z'));
  // Camera is three hours behind the browser.
  vi.mocked(getDateTime).mockResolvedValue({
    ...mockTimeConfig,
    utcDateTime: new Date('2026-09-20T09:00:00Z'),
    timezone: 'UTC0',
  });

  await renderTimePage();

  expect(screen.getByTestId('time-device-clock')).toHaveTextContent('09:00:00');
  vi.useRealTimers();
});

it('should keep ticking from the camera offset', async () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-09-20T12:00:00Z'));
  vi.mocked(getDateTime).mockResolvedValue({
    ...mockTimeConfig,
    utcDateTime: new Date('2026-09-20T09:00:00Z'),
    timezone: 'UTC0',
  });

  await renderTimePage();
  await act(async () => {
    vi.advanceTimersByTime(2000);
  });

  expect(screen.getByTestId('time-device-clock')).toHaveTextContent('09:00:02');
  vi.useRealTimers();
});

it('should warn when the camera clock is implausible', async () => {
  vi.mocked(getDateTime).mockResolvedValue({
    ...mockTimeConfig,
    utcDateTime: new Date('1970-01-01T00:00:00Z'),
    timezone: 'UTC0',
  });

  await renderTimePage();

  expect(screen.getByTestId('time-clock-stale')).toBeInTheDocument();
});
```

**Step 2: Run to verify it fails**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx -t 'camera time'
```

Expected: FAIL — no `time-device-clock` testid; the clock renders browser time.

**Step 3: Implement**

Replace the two `useEffect`s at `TimePage.tsx:58-73`:

```tsx
  // The camera's clock, not ours. Capturing the offset once and ticking from
  // it is what makes a camera stuck at 1970 visible instead of showing the
  // operator their own correct browser clock.
  const [offsetMs, setOffsetMs] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (config) {
      setOffsetMs(config.utcDateTime.getTime() - Date.now());
    }
  }, [config]);

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const deviceTime = offsetMs === null ? null : new Date(now + offsetMs);
  const clockIsStale = deviceTime !== null && deviceTime.getUTCFullYear() < 2020;
```

Render the clock with `data-testid="time-device-clock"`, formatted in the
camera's zone. The camera's TZ is a POSIX string, which `Intl` will not accept,
so derive the display offset from the already-fetched UTC value rather than
passing `timezone` to `toLocaleTimeString`: format with
`timeZone: 'UTC'` against `deviceTime` shifted by the camera's current offset.
The backend already reports `LocalDateTime` in `GetSystemDateAndTimeResponse` —
prefer plumbing that through `getDateTime` as `localDateTime` and displaying it
directly, which avoids reimplementing POSIX TZ arithmetic in TypeScript.

When `clockIsStale`, render a red banner with
`data-testid="time-clock-stale"` reading roughly: *"The camera's clock is not
set. Authenticated requests will fail until NTP syncs."*

Remove the `eslint-disable-next-line react-hooks/set-state-in-effect` comment
only if the rule no longer fires; otherwise keep it.

**Step 4: Run to verify it passes**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx
```

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/settings/TimePage.tsx cross-compile/www/src/pages/settings/TimePage.test.tsx
git commit -m "fix(webui): show the camera's clock instead of the browser's

The panel ticked new Date() and discarded the fetched camera UTC, so a
camera stuck at 1970 displayed the operator's own correct time. A stale
clock now says so, because that is exactly what breaks ws_security.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 14: Two real modes, with a computer-time button

**Files:**
- Modify: `cross-compile/www/src/pages/settings/TimePage.tsx:35-146, 218-292`
- Test: `cross-compile/www/src/pages/settings/TimePage.test.tsx`

**Step 1: Write the failing test**

```ts
it('should preselect NTP when the camera reports NTP', async () => {
  vi.mocked(getDateTime).mockResolvedValue({ ...mockTimeConfig, ntp: { enabled: true } });
  await renderTimePage();
  expect(screen.getByTestId('time-page-ntp-radio-input')).toBeChecked();
});

it('should preselect Manual when the camera reports Manual', async () => {
  vi.mocked(getDateTime).mockResolvedValue({ ...mockTimeConfig, ntp: { enabled: false } });
  await renderTimePage();
  expect(screen.getByTestId('time-page-manual-radio-input')).toBeChecked();
});

it('should fill the manual inputs from the browser clock on demand', async () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-09-20T12:34:56Z'));
  vi.mocked(getDateTime).mockResolvedValue({ ...mockTimeConfig, ntp: { enabled: false } });
  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

  await renderTimePage();
  await user.click(screen.getByTestId('time-page-use-computer-time'));

  expect(screen.getByTestId('time-page-manual-date-input')).toHaveValue('2026-09-20');
  vi.useRealTimers();
});

it('should not offer a computer mode', async () => {
  await renderTimePage();
  expect(screen.queryByTestId('time-page-computer-radio-input')).not.toBeInTheDocument();
});
```

**Step 2: Run to verify it fails**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx -t 'computer'
```

**Step 3: Implement**

- Narrow the zod schema's `mode` to `z.enum(['ntp', 'manual'])` and delete
  `ntpFromDHCP` from the schema, defaults and `form.reset`.
- Delete the Computer `RadioGroupItem` block (lines ~249-268) and
  `handleSyncComputer`'s `form.setValue('mode', …)`. The grid becomes
  `md:grid-cols-2`.
- Inside the Manual panel, add a secondary button
  `data-testid="time-page-use-computer-time"` labelled "Use this computer's
  time" that fills `manualDate`/`manualTime` from `new Date()`. Give it
  `type="button"` so it does not submit the form.
- `form.reset` takes `mode: config.ntp.enabled ? 'ntp' : 'manual'` — the value
  is now real, since Task 6 made the backend report it.
- In the mutation, drop the `computer` branch and pass
  `config.daylightSavings` through to `setDateTime`.
- Delete the "NTP from DHCP" `FormField` and the `ntpFromDHCP` `useWatch`.

**Step 4: Run to verify it passes**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx
```

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/settings/TimePage.tsx cross-compile/www/src/pages/settings/TimePage.test.tsx
git commit -m "fix(webui): two real clock modes, computer time as a button

Computer was a one-shot action shaped like a persistent mode and always
round-tripped back as Manual. The DHCP switch is gone: udhcpc on this
camera never supplies NTP servers.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 15: Real server list and a sync status card

**Files:**
- Modify: `cross-compile/www/src/pages/settings/TimePage.tsx`
- Test: `cross-compile/www/src/pages/settings/TimePage.test.tsx`

**Step 1: Write the failing test**

```ts
it('should load the camera servers into the textarea', async () => {
  vi.mocked(getNtp).mockResolvedValue(['192.168.2.1', 'pool.example']);
  await renderTimePage();
  expect(screen.getByTestId('time-page-ntp-servers')).toHaveValue(
    '192.168.2.1\npool.example',
  );
});

it('should save one server per non-empty line', async () => {
  vi.mocked(getNtp).mockResolvedValue(['old.example']);
  const user = userEvent.setup();
  await renderTimePage();

  const box = screen.getByTestId('time-page-ntp-servers');
  await user.clear(box);
  await user.type(box, 'a.example\n\n  b.example  \n');
  await user.click(screen.getByTestId('time-page-save-button'));

  await waitFor(() => {
    expect(setNtp).toHaveBeenCalledWith(['a.example', 'b.example']);
  });
});

it('should show the last sync reported by diagnostics', async () => {
  vi.mocked(getDiagnostics).mockResolvedValue({
    ...mockDiagnostics,
    time: {
      last_sync_unix: 1760000000,
      last_server: '192.168.2.1',
      last_delta_s: -3,
      servers: ['192.168.2.1'],
    },
  });
  await renderTimePage();
  expect(screen.getByTestId('time-page-last-sync')).toHaveTextContent('192.168.2.1');
});

it('should say so when the camera has never synced', async () => {
  vi.mocked(getDiagnostics).mockResolvedValue({ ...mockDiagnostics, time: undefined });
  await renderTimePage();
  expect(screen.getByTestId('time-page-last-sync')).toHaveTextContent(/never|unknown/i);
});
```

Add `getNtp`, `setNtp` to the `vi.mock('@/services/timeService', …)` factory at
the top of the file, and mock `@/services/diagnosticsService`.

**Step 2: Run to verify it fails**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx -t 'server'
```

**Step 3: Implement**

- Add `ntpServers: z.string()` to the schema; drop `ntpServer1`/`ntpServer2`.
- Add a `useQuery` for `getNtp` (key `['ntpServers']`) and seed the textarea in
  `form.reset` with `servers.join('\n')`.
- Replace the two `Input`s with a `<textarea data-testid="time-page-ntp-servers">`
  styled like the existing `select` at line 431, plus a `FormDescription`
  reading: *"One per line. Changes apply within six hours, or immediately
  after a reboot."*
- In the mutation's `ntp` branch, call
  `setNtp(values.ntpServers.split('\n').map((s) => s.trim()).filter(Boolean))`
  and `setDateTime`/`SetSystemDateAndTime` with `NTP` so the marker clears.
  Invalidate `['ntpServers']` alongside `['timeConfig']` on success.
- Add a status card above the Actions row rendering last sync, server and
  offset from `getDiagnostics().time`, with
  `data-testid="time-page-last-sync"` and a "Never synced" fallback.

**Step 4: Run to verify it passes**

```bash
npx vitest run src/pages/settings/TimePage.test.tsx
```

**Step 5: Commit**

```bash
git add cross-compile/www/src/pages/settings/TimePage.tsx cross-compile/www/src/pages/settings/TimePage.test.tsx
git commit -m "feat(webui): editable NTP server list and sync status

The two server inputs were hardcoded strings nothing read. A textarea takes
any number of servers, so a three-server config is not silently truncated.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 16: Full verification and on-device check

**Step 1: Host gates**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
$CARGO test   --target x86_64-unknown-linux-gnu -p anyka-init -p onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
cd www && npm run test && npm run type-check && npm run lint
```

For formatting, run the **raw** binary and read the exit code — the RTK filter
has printed "All files formatted correctly" on a real exit-1 with drifted files:

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/www
./node_modules/.bin/prettier --check . ; echo "exit=$?"
```

Expected: `exit=0`.

**Step 2: ARM build**

Must run from the crate directory, not the workspace root, or cargo silently
links with the host toolchain.

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust && $CARGO build --release --target armv5te-unknown-linux-uclibceabi
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init && $CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

**Step 3: Deploy**

Use @anyka-firmware-upgrade to build and push a bundle. Both binaries changed,
so both must ship. `anyka.toml` needs no new keys — do not hand-edit it.

**Step 4: On-device checks**

The one that proves gap 1 is dead:

1. Open the Time tab. Note the displayed device time.
2. Switch to Manual, set a time three hours off, save.
3. Reload. The panel must show the **wrong** time you just set. If it shows
   your browser's correct time, Task 13 did not take.
4. Switch back to NTP, save. Confirm `state/ntp.disabled` disappears:
   `ls /mnt/anyka_hack/state/`
5. Within one resync interval, confirm the clock corrects itself and
   `/mnt/anyka_hack/state/ntp.status` gains a non-zero first field.

Then the server path:

6. Change the server list in the WebUI. Confirm `[time].servers` in
   `/mnt/anyka_hack/anyka.toml` changed **and every comment in the file
   survived**.
7. Confirm the WebUI reloads the new list.

Remember the camera's log level defaults to `error`, so a `grep -c` of 0 on an
`info!` line proves nothing. Read `/mnt/anyka_hack/anyka.toml`'s `[logging]`
section before drawing conclusions from missing log lines.

**Step 5: Commit field notes**

```bash
git add docs/plans/2026-09-20-time-settings.md
git commit -m "docs(plan): field notes from the on-device time settings run

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Notes for the Implementer

- **Do not add keys to `anyka.toml`.** `Config` uses `deny_unknown_fields`, so a
  new key is a hard parse error for the older anyka-init in the other A/B slot,
  and the camera loses its rollback. Everything new in this plan lives in
  `state/` or in existing keys.
- **`onvif-rust` must never open `anyka.toml`.** If a task seems to need it,
  the answer is a control-socket verb.
- **Task 5 changes `sync_once`'s return type.** Expect a handful of existing
  tests to need their assertion updated; that is the task, not a surprise.
- **Two timezones exist by design.** `anyka.toml [time].timezone` drives the
  supervisor's own log timestamps only; onvif-rust carries its own for OSD and
  ONVIF. They can disagree in logs and that is accepted — see the design doc's
  Consequences section.
