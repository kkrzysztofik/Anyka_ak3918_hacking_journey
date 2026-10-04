# anyka-init Ponytail Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove ~600 lines of duplication and dead flexibility from `cross-compile/anyka-init/` with byte-identical config defaults and only four deliberate behaviour changes.

**Architecture:** Ten commits on `refactor/anyka-init-ponytail-cleanup`. Task 0 pins current behaviour, Task 1 consolidates test helpers, Tasks 2–5 are pure refactors, Tasks 6–9 are the four behaviour changes (one revertable commit each). Design: `docs/plans/2026-10-04-anyka-init-ponytail-cleanup-design.md`.

**Tech Stack:** Rust 2024 (vendored rustc 1.97), serde/toml, mockall, tracing. Skills: @anyka-rust-testing for test style, @anyka-embedded-build for the ARM build, @anyka-firmware-upgrade for the .198 deploy.

---

## Ground rules (read once)

**Working directory** for every command, unless a step says otherwise:

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/anyka-init-ponytail-cleanup/cross-compile
export PATH=$PWD/../toolchain/arm-anykav200-crosstool-ng/bin:$PATH
```

The PATH prefix is mandatory — without it clippy dies with E0514.

**Gate `G`** — run at the end of every task; all three must pass:

```bash
cargo test --target x86_64-unknown-linux-gnu -p anyka-init 2>&1 | tail -5 \
 && cargo clippy --target x86_64-unknown-linux-gnu -p anyka-init --all-targets -- -D warnings 2>&1 | tail -3 \
 && cargo fmt -p anyka-init --check && echo GATE-OK
```

Baseline: 352 tests pass, clippy clean. The test count only moves where a task says so.

**Committing:** this repo's index is often pre-staged. Always commit with an explicit pathspec (`git commit -m … -- <paths>`), never a bare `git commit`. End every message with:

```
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Paths** below are relative to `cross-compile/anyka-init/` unless absolute. Line numbers are from the branch point (`1bc401c7`) and drift as tasks land — locate by the quoted code, not the number.

**Never** add a key to `anyka.toml`'s schema (`deny_unknown_fields` → the older anyka-init in the other A/B slot refuses to parse). Nothing in this plan does.

---

### Task 0: Pin current behaviour

**Files:**
- Modify: `src/config.rs` (tests module, end of file)
- Modify: `src/supervise.rs` (`backoff_tests`)

**Step 1: Add the config golden test**

Append inside `mod tests` in `src/config.rs`:

```rust
    /// Every default, frozen. Task 2 moves these values from `d_*` fns into
    /// `Default` impls; a typo there must fail here, not on a camera.
    const DEFAULTS_GOLDEN: &str = r#"Config { schema: 0, log: LogCfg { dir: "/mnt/logs", level: "info", max_bytes: 2000000, keep: 2 }, system: SystemCfg { sensor_module: None, telnet: false, ftp: true }, wifi: WifiCfg { ssid: "testnet", password: "secret", config_file: "/etc/jffs2/anyka_cfg.ini", chip: "auto", gpio_polarity: "low_high", interface: "wlan0", security: "wpa", dhcp: true, address: None, gateway: None, dns: [], connect_timeout_sec: 45, fallback_to_vendor: true }, time: TimeCfg { enabled: true, servers: ["0.ubuntu.pool.ntp.org", "1.ubuntu.pool.ntp.org"], timezone: "GMT+00:00", first_sync_timeout_sec: 15, retry_interval_sec: 30, resync_interval_sec: 21600, step_threshold_sec: 2, min_plausible_unix: 1767225600, max_plausible_unix: 2147483647 }, supervisor: SupervisorCfg { backoff_min_sec: 1, backoff_max_sec: 60, crashloop_count: 10, crashloop_window_sec: 600, storm_guard_max_reboots: 3, storm_guard_state: "/mnt/anyka_hack/state/boot.json", storm_guard_reset_uptime_sec: 600 }, monitor: MonitorCfg { enabled: true, interval_sec: 60, wifi: true, wifi_probe: true, wifi_dhcp_after_ticks: 3, wifi_supplicant_after_ticks: 5, wifi_reboot_after_ticks: 10, wifi_reboot_cap: 3, video: true, video_restart_after_ticks: 2, video_kill_after_ticks: 3, video_reboot_after_ticks: 5, video_heartbeat_path: "/tmp/vd_heartbeat" }, reboot: RebootCfg { enabled: false, interval_min: 720, jitter_max_sec: 0 }, update: Update { root: "/mnt/anyka_hack", trial_hold_sec: 30, trial_deadline_sec: 120, trial_ports: [80, 554, 8080] }, services: {} }"#;

    #[test]
    fn test_defaults_match_the_golden_when_sections_are_absent() {
        let cfg = Config::from_str(MINIMAL).expect("parses");
        assert_eq!(format!("{cfg:?}"), DEFAULTS_GOLDEN);
    }

    #[test]
    fn test_defaults_match_the_golden_when_sections_are_present_but_empty() {
        // The other serde path: a present `[log]` header with no keys fills
        // fields one by one, not via the section-level default.
        let src = format!(
            "{MINIMAL}\n[log]\n[system]\n[time]\n[supervisor]\n[monitor]\n[reboot]\n[update]\n"
        );
        let cfg = Config::from_str(&src).expect("parses");
        assert_eq!(format!("{cfg:?}"), DEFAULTS_GOLDEN);
    }
```

**Step 2: Add backoff edge pins**

Append inside `mod backoff_tests` in `src/supervise.rs`:

```rust
    #[test]
    fn test_backoff_pins_the_shift_width_boundaries() {
        // Task 5 rewrites backoff_delay around checked_shl; these are the
        // values on both sides of every guard the old code had.
        for attempt in [31, 32, 33, 62, 63, 64] {
            assert_eq!(backoff_delay(attempt, MIN, MAX), MAX, "attempt {attempt}");
        }
        assert_eq!(backoff_delay(0, MAX, MAX), MAX, "min == max at attempt 0");
        assert_eq!(backoff_delay(9, Duration::ZERO, MAX), Duration::ZERO);
    }
```

**Step 3: Run the new tests**

```bash
cargo test --target x86_64-unknown-linux-gnu -p anyka-init defaults_match_the_golden backoff_pins 2>&1 | tail -5
```

Expected: 3 passed. If a golden test fails, the golden is wrong for *this* code — regenerate it by printing `format!("{:?}", Config::from_str(MINIMAL)?)`; never edit the code to match.

**Step 4: Gate `G`** — expect 355 tests.

**Step 5: Commit**

```bash
git commit -m "test(anyka-init): pin config defaults and backoff edges before cleanup

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/config.rs anyka-init/src/supervise.rs
```

---

### Task 1: Consolidate test helpers

**Files:**
- Modify: `src/config.rs` (add `test_config`)
- Modify: `src/boot.rs` (`wifi_tests`, `tz_tests`)
- Modify: `src/supervisor_loop.rs` (`run_tests`, `reboot_delay_tests`)
- Modify: `src/wifi.rs` (tests)
- Modify: `src/sys.rs` (tests)

**Step 1: One TOML-backed test config**

In `src/config.rs`, directly above `#[cfg(test)] mod tests`, add:

```rust
/// A parsed, unvalidated config for unit tests elsewhere in the crate. Tests
/// mutate the fields they care about instead of hand-building every section.
#[cfg(test)]
pub(crate) fn test_config() -> Config {
    "[wifi]\nssid = \"test\"\npassword = \"testpass\"\nconfig_file = \"/nonexistent/anyka_cfg.ini\"\n"
        .parse()
        .expect("test config parses")
}
```

**Step 2: Use it in `boot.rs` tests**

In `mod wifi_tests`:
- Replace the `use crate::config::{LogCfg, MonitorCfg, RebootCfg, ServiceCfg, SupervisorCfg, SystemCfg, TimeCfg};` line with `use crate::config::{ServiceCfg, SystemCfg};`.
- Replace `fn wifi_cfg(...)` and `fn test_config(...)` with:

```rust
    fn wifi_cfg(config_file: &str, ssid: &str, password: &str) -> WifiCfg {
        let mut w = crate::config::test_config().wifi;
        w.config_file = config_file.into();
        w.ssid = ssid.into();
        w.password = password.into();
        w
    }

    fn test_config(wifi: WifiCfg, system: SystemCfg) -> Config {
        let mut cfg = crate::config::test_config();
        cfg.wifi = wifi;
        cfg.system = system;
        cfg
    }
```

- Drop the now-unused `use std::collections::BTreeMap;` only if the compiler says so (`config_with_supplicant` still uses it).

Delete the whole `test_spawn_env_carries_tz` test in `mod tz_tests` (it asserts nothing and leaks a zombie; `sys.rs::a_spawned_child_sees_the_tz_env` covers it). Then remove the imports it alone used (`use crate::sys::{RealSys, SpawnSpec};`, `use std::collections::BTreeMap;`) from `tz_tests`.

**Step 3: Use it in `supervisor_loop.rs` `run_tests`**

- Delete `fn minimal_wifi_cfg()`.
- Replace `fn test_config(services)` with:

```rust
    fn test_config(services: BTreeMap<String, ServiceCfg>) -> Config {
        let mut cfg = crate::config::test_config();
        cfg.services = services;
        cfg.supervisor.backoff_min_sec = 30;
        cfg.supervisor.crashloop_count = 100;
        cfg.supervisor.storm_guard_state = "/nonexistent/storm.json".into();
        cfg
    }
```

- Delete `fn minimal_config()` and replace its three call sites (`let mut cfg = minimal_config();`) with `let mut cfg = crate::config::test_config();`.
- Trim the `use crate::config::{…}` line to what the compiler still needs (expected: `Config, ServiceCfg`), and drop `use std::str::FromStr;` if unused.

**Step 4: Shared setup for the `rewrite_env` tests**

In `mod reboot_delay_tests`, add above the first test:

```rust
    /// A tempdir update root whose `active` pointer says `a`, plus the root as
    /// a string for building expected paths.
    fn slot_a_root() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("slots")).unwrap();
        std::fs::write(d.path().join("active"), "a").unwrap();
        let s = d.path().display().to_string();
        (d, s)
    }

    fn rewrite_into_a(root: &Path) -> impl Fn(&str) -> String + '_ {
        move |p| {
            crate::update::slot_path(root, crate::update::Slot::A, Path::new(p))
                .to_string_lossy()
                .into_owned()
        }
    }
```

Rewrite each of the three `test_rewrite_env_*` tests to open with:

```rust
        let (d, root_str) = slot_a_root();
        let rewrite = rewrite_into_a(d.path());
```

deleting their per-test `tempdir` / `Slots::new` / `create_dir_all` / `write("active")` / closure / `root_str` lines. The `assert_eq!` bodies stay as they are. (Using `Slot::A` directly is equivalent: the old closure read `slots.active()` from a pointer that says `a`.)

**Step 5: Shared layout setup in `wifi.rs` tests**

Add next to `test_layout`:

```rust
    /// `test_layout` with wlan0 present, carrying, and addressed.
    fn happy_layout(dir: &std::path::Path, carrier: bool) -> FsLayout {
        let layout = test_layout(dir);
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net)).expect("iface dir");
        if carrier {
            std::fs::write(format!("{}/wlan0/carrier", layout.sys_class_net), "1").expect("carrier");
        }
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");
        layout
    }
```

In the three happy-path tests replace the `let layout = test_layout(...)` line plus the following 4 setup lines with `let layout = happy_layout(dir.path(), true);`. In `test_a_failing_overlay_is_quarantined_and_the_baseline_is_retried` replace its `test_layout` line plus 3 setup lines with `let layout = happy_layout(dir.path(), false);`.

Delete `test_wpa_supplicant_conf_is_deterministic` (asserts a pure `format!` is pure).

**Step 6: Shared log poll in `sys.rs` tests**

Add inside `mod tests`, below `FORK_LOCK`:

```rust
    /// Poll `log` until the child has written to it, or panic after 5 s.
    fn read_log_when_written(log: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(s) = std::fs::read_to_string(log)
                && !s.is_empty()
            {
                return s;
            }
            assert!(Instant::now() < deadline, "child never wrote to {}", log.display());
            std::thread::sleep(Duration::from_millis(20));
        }
    }
```

In `a_spawned_child_runs_in_its_executable_directory` and `a_spawned_child_sees_the_tz_env`, replace the `let deadline = …; let contents = loop { … };` block with `let contents = read_log_when_written(&log);`.

**Step 7: Gate `G`** — expect 353 tests (355 − TZ spawn test − determinism test).

**Step 8: Commit**

```bash
git commit -m "test(anyka-init): one TOML-backed test config and shared fixtures

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src
```

---

### Task 2: Config defaults live in `Default` only

**Files:**
- Modify: `src/config.rs`

**Step 1: Container-level defaults**

For each of `Update`, `LogCfg`, `SystemCfg`, `TimeCfg`, `SupervisorCfg`, `MonitorCfg`, `RebootCfg`:
- change `#[serde(deny_unknown_fields)]` to `#[serde(deny_unknown_fields, default)]`;
- delete every field-level `#[serde(default)]` / `#[serde(default = "…")]` inside it (keep the doc comments).

Leave `Config`, `WifiCfg` and `ServiceCfg` untouched: they have required keys, so they keep field-level defaults.

**Step 2: Replace the `Default` impls with literals**

Replace the seven `impl Default` blocks (`Update` … `RebootCfg`) with:

```rust
impl Default for Update {
    fn default() -> Self {
        Self {
            root: "/mnt/anyka_hack".into(),
            trial_hold_sec: 30,
            trial_deadline_sec: 120,
            trial_ports: crate::update::TRIAL_PORTS.to_vec(),
        }
    }
}

impl Default for LogCfg {
    fn default() -> Self {
        Self {
            dir: "/mnt/logs".into(),
            level: "info".into(),
            max_bytes: 2_000_000,
            keep: 2,
        }
    }
}

impl Default for SystemCfg {
    fn default() -> Self {
        Self {
            sensor_module: None,
            telnet: false,
            ftp: true,
        }
    }
}

impl Default for TimeCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: vec![
                "0.ubuntu.pool.ntp.org".into(),
                "1.ubuntu.pool.ntp.org".into(),
            ],
            timezone: "GMT+00:00".into(),
            first_sync_timeout_sec: 15,
            retry_interval_sec: 30,
            resync_interval_sec: 21_600,
            step_threshold_sec: 2,
            min_plausible_unix: 1_767_225_600, // 2026-01-01
            // ARMv5 uClibc uses 32-bit time_t; stay below the 2038 overflow.
            max_plausible_unix: i32::MAX as u64,
        }
    }
}

impl Default for SupervisorCfg {
    fn default() -> Self {
        Self {
            backoff_min_sec: 1,
            backoff_max_sec: 60,
            crashloop_count: 10,
            crashloop_window_sec: 600,
            storm_guard_max_reboots: 3,
            storm_guard_state: "/mnt/anyka_hack/state/boot.json".into(),
            storm_guard_reset_uptime_sec: 600,
        }
    }
}

impl Default for MonitorCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_sec: 60,
            wifi: true,
            wifi_probe: true,
            wifi_dhcp_after_ticks: 3,
            wifi_supplicant_after_ticks: 5,
            wifi_reboot_after_ticks: 10,
            wifi_reboot_cap: 3,
            video: true,
            video_restart_after_ticks: 2,
            video_kill_after_ticks: 3,
            video_reboot_after_ticks: 5,
            video_heartbeat_path: "/tmp/vd_heartbeat".into(),
        }
    }
}

impl Default for RebootCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_min: 720,
            jitter_max_sec: 0,
        }
    }
}
```

**Step 3: Delete the orphaned default fns**

Delete every `d_*` / `default_*` fn except: `d_true`, `d_wifi_cfg_file`, `d_wifi_chip`, `d_wifi_polarity`, `d_wifi_interface`, `d_wifi_security`, `d_wifi_timeout`. Let the compiler confirm: `cargo build --target x86_64-unknown-linux-gnu -p anyka-init` must show no `dead_code` warnings and no missing fns.

**Step 4: Collapse the file editors**

- Delete `pub fn set_bool_in_text`.
- `parse_file` becomes:

```rust
    fn parse_file(path: &str) -> Result<Self, ConfigError> {
        read_config_text(std::path::Path::new(path))?.parse()
    }
```

- Add below `persist_text`:

```rust
/// Read, edit one `key = raw` line under `section`, write atomically.
fn edit_file(path: &std::path::Path, section: &str, key: &str, raw: &str) -> Result<(), ConfigError> {
    let text = read_config_text(path)?;
    persist_text(path, &set_value_in_text(&text, section, key, raw)?)
}
```

- The three writers' bodies become (docs unchanged):
  - `set_service_enabled`: `edit_file(path, &format!("[services.{name}]"), "enabled", &enabled.to_string())`
  - `set_system_telnet`: `edit_file(path, "[system]", "telnet", &enabled.to_string())`
  - `set_time_servers`: keep both validation `if`s and the `raw` construction, then `edit_file(path, "[time]", "servers", &raw)`.

**Step 5: Retarget the editor tests**

```bash
sed -i 's/set_bool_in_text(\(.*\), false)/set_value_in_text(\1, "false")/; s/set_bool_in_text(\(.*\), true)/set_value_in_text(\1, "true")/' anyka-init/src/config.rs
grep -n 'set_bool_in_text(' anyka-init/src/config.rs   # expect: no output
```

Delete `test_set_bool_in_text_still_works_after_generalization` (it tested the deleted wrapper). Test *names* that still say `set_bool_in_text` are fine — leave them.

**Step 6: Gate `G`** — expect 352. The two golden tests from Task 0 are the point of this task: if either fails, a default drifted — fix the literal, not the golden.

**Step 7: Commit**

```bash
git commit -m "refactor(anyka-init): config defaults live only in Default impls

Container-level #[serde(default)] reads missing keys from Default, so the
33 per-field default fns duplicated every value. No anyka.toml key added
or removed; the golden test pins every default.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/config.rs
```

---

### Task 3: Shared `atomic_write` and `spawn_named`

**Files:**
- Modify: `src/sys.rs`, `src/storm.rs`, `src/update.rs`, `src/config.rs`
- Modify: `src/supervisor_loop.rs`, `src/main.rs`

**Step 1: `atomic_write` in `sys.rs`**

Add as a free function below `impl Default for RealSys`:

```rust
/// Write `bytes` to `path` via `<path>.tmp`, fsync, rename, then sync(2), so
/// a power cut on the vfat/exFAT card leaves the old contents or the new,
/// never a half-written file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    // SAFETY: sync(2) takes no arguments and cannot fail.
    unsafe { libc::sync() };
    Ok(())
}
```

The temp names match all three old call sites exactly (`boot.json.tmp`, `active.tmp`, `anyka.toml.tmp`). The config writer gains a `sync(2)` it did not have — strictly more durable.

**Step 2: Use it**

`src/storm.rs` `save` body:

```rust
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::sys::atomic_write(std::path::Path::new(path), self.render().as_bytes())
    }
```

`src/update.rs` `Slots::set_active` body:

```rust
    pub fn set_active(&self, slot: Slot) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        crate::sys::atomic_write(&self.pointer(), slot.name().as_bytes())
    }
```

`src/config.rs` `persist_text` body:

```rust
fn persist_text(path: &std::path::Path, new_text: &str) -> Result<(), ConfigError> {
    crate::sys::atomic_write(path, new_text.as_bytes()).map_err(|source| ConfigError::Write {
        path: path.display().to_string(),
        source,
    })
}
```

Update the three doc comments that described the dance inline to say "via `sys::atomic_write`".

**Step 3: `spawn_named` in `supervisor_loop.rs`**

Make `thread_stack` private (`fn thread_stack`) and add below it:

```rust
/// Spawn a named background thread with the camera-sized stack.
pub fn spawn_named<F>(name: &str, f: F) -> std::io::Result<std::thread::JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(thread_stack())
        .spawn(f)
}
```

Convert the three sites in `supervisor_loop.rs`:

```rust
// spawn_reaper: replace the `match std::thread::Builder…{ Ok … Err … }` with
    spawn_named("reaper", move || {
        /* loop body unchanged */
    })
    .inspect_err(|e| {
        tracing::error!(
            error = %e,
            "failed to start the reaper thread; service exits will not be observed"
        )
    })
    .ok()

// spawn_signal_thread:
    if let Err(e) = spawn_named("signals", move || { /* body unchanged */ }) {
        tracing::error!(error = %e, "failed to start the signal thread; SIGTERM/SIGINT will not shut down cleanly");
    }

// spawn_control_thread: replace the Builder chain and `.map_err(std::io::Error::other)?` with
    spawn_named("supervisor-ctl", move || { /* body unchanged */ })?;
```

Convert the five sites in `src/main.rs` (`update-trial`, `update-poll`, `monitor`, `timesync`, `periodic-reboot`) from `let _ = std::thread::Builder::new().name("X".into()).stack_size(supervisor_loop::thread_stack()).spawn(move || { … });` to `let _ = supervisor_loop::spawn_named("X", move || { … });`.

```bash
grep -n "thread::Builder" anyka-init/src   # expect: only the one inside spawn_named
```

**Step 4: Gate `G`** — expect 352.

**Step 5: Commit**

```bash
git commit -m "refactor(anyka-init): one atomic_write and one spawn_named

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src
```

---

### Task 4: Supervisor loop

**Files:**
- Modify: `src/supervisor_loop.rs`, `src/main.rs`, `tests/supervision.rs`

**Step 1: Drop `make_channel`**

Delete `pub fn make_channel`. Replace callers:
- `src/main.rs`: `let (tx, rx) = std::sync::mpsc::channel();`
- `tests/supervision.rs:89`: `let (tx, rx) = std::sync::mpsc::channel();`
- the three `run_tests` sites: `let (tx, rx) = channel();`

**Step 2: Single-variant `ControlMsg` → plain `Vec`**

- Delete `pub enum ControlMsg`.
- `Msg::QueryStatus(Sender<ControlMsg>)` → `Msg::QueryStatus(Sender<Vec<control::ServiceStatus>>)`.
- `handle_query_status(.., reply_tx: &Sender<Vec<control::ServiceStatus>>)`, ending `let _ = reply_tx.send(rows);`.
- In `handle_control_conn`: `if let Ok(rows) = reply {`.
- In the two status tests: `let rows = rx.recv().expect("status");`.

**Step 3: Pending row reuses `from_svc_state`**

In `handle_query_status`, the `None =>` arm becomes:

```rust
                // Enabled but not yet in the vec: render as pending, never drop.
                None => control::ServiceStatus::from_svc_state(
                    name,
                    &SvcState::Backoff { until: now, attempt: 0 },
                    &RestartHistory::default(),
                    now,
                ),
```

**Step 4: One `signal_service` for restart and kill**

Replace `handle_restart_service` and `handle_kill_service` with:

```rust
/// Signal a supervised service at the monitor's request: SIGTERM to restart
/// it (the exit path respawns it under backoff), SIGKILL when SIGTERM did not
/// take. A task wedged in D state survives even SIGKILL; the monitor's next
/// rung is a reboot.
fn signal_service(sys: &dyn Sys, services: &[Service], name: &str, sig: i32) {
    match services.iter().find(|s| s.name == name) {
        Some(svc) => match svc.state.pid() {
            Some(pid) => {
                tracing::warn!(service = %name, pid, sig, "signalling service at the monitor's request");
                let _ = sys.kill(pid, sig);
            }
            None => tracing::info!(service = %name, sig, "signal requested but the service is not running"),
        },
        None => tracing::warn!(service = %name, sig, "signal requested for unknown service"),
    }
}
```

In `dispatch_msg`:

```rust
        Ok(Msg::RestartService(name)) => {
            signal_service(ctx.sys, services, &name, libc::SIGTERM);
            false
        }
        Ok(Msg::KillService(name)) => {
            signal_service(ctx.sys, services, &name, libc::SIGKILL);
            false
        }
```

**Step 5: One reply path for toggle and set-ntp**

Replace `fn send_toggle` with:

```rust
/// Queue `msg` for the loop and write its verdict back to the connection.
fn send_and_reply<W: std::io::Write>(
    writer: &mut W,
    tx: &Sender<Msg>,
    msg: Msg,
    reply_rx: Receiver<control::ToggleOutcome>,
) {
    let reply = match tx.send(msg) {
        // A timeout is NOT a failure. The message is still queued, and the
        // handler writes `anyka.toml` and updates state *before* it replies —
        // dropping the receiver cancels nothing. Saying "error" here would
        // tell an admin nothing changed while the change lands a moment
        // later. `pending` says what is actually true: it was accepted, the
        // outcome is unconfirmed, go look at the service list.
        Ok(()) => match reply_rx.recv_timeout(CONTROL_REPLY_TIMEOUT) {
            Ok(control::ToggleOutcome::Ok) => "ok\n",
            Ok(control::ToggleOutcome::Unknown) => "unknown\n",
            Ok(control::ToggleOutcome::Error) => "error\n",
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!("control reply timed out; the change may still apply");
                "pending\n"
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => "error\n",
        },
        // The loop is gone entirely (reboot in progress): nothing was queued.
        Err(_) => "error\n",
    };
    let _ = writer.write_all(reply.as_bytes());
}
```

In `handle_control_conn`, the `Enable`, `Disable` and `SetNtp` arms become:

```rust
        Some(control::Request::Enable(name)) => {
            let (reply, rx) = channel();
            send_and_reply(&mut stream, tx, Msg::ToggleService { name, enabled: true, reply }, rx);
        }
        Some(control::Request::Disable(name)) => {
            let (reply, rx) = channel();
            send_and_reply(&mut stream, tx, Msg::ToggleService { name, enabled: false, reply }, rx);
        }
        Some(control::Request::SetNtp(servers)) => {
            let (reply, rx) = channel();
            send_and_reply(&mut stream, tx, Msg::SetNtpServers { servers, reply }, rx);
        }
```

Fix the `Status` arm's comment: "Bounded like `send_toggle`" → "Bounded like `send_and_reply`". Wire format (`ok/unknown/error/pending`) is unchanged — onvif-rust's `diagnostics/services.rs` depends on it.

**Step 6: `Service::pending`**

Add after `struct Service`:

```rust
impl Service {
    /// The state a boot start gets: the next tick starts it under the normal
    /// backoff/crash-loop policy.
    fn pending(name: String, spec: SpawnSpec, now: Instant) -> Self {
        Self {
            name,
            spec,
            state: SvcState::Backoff { until: now, attempt: 0 },
            hist: RestartHistory::default(),
        }
    }
}
```

- `build_enabled_services`: `.map(|(name, s)| Service::pending(name.clone(), spec_of_slot(s, update_root, slots), sys.now()))`.
- `handle_toggle_service`, `None if enabled` arm: `services.push(Service::pending(name.clone(), spec_of_slot(s, ctx.update_root, ctx.slots), ctx.sys.now()));`.

**Step 7: Small shrinks**

- `tick_services`: `next_deadline = Some(next_deadline.map_or(until, |d| d.min(until)));`
- `periodic_reboot_delay` body:

```rust
    // saturating: `jitter_max_sec + 1` overflows at u64::MAX. At 0 the
    // modulus is 1, so the jitter is 0 without a special case.
    let jitter = entropy % jitter_max_sec.saturating_add(1);
    Duration::from_secs(interval_min.saturating_mul(60).saturating_add(jitter))
```

- Delete the test-only `fn ctx(…)`; in `ToggleFixture::toggle` build the struct inline:

```rust
            let mut c = LoopCtx {
                sys,
                cfg: &mut self.cfg,
                config_path: &self.cfg_path,
                update_root: self.dir.path(),
                slots: &self.slots,
                policy: &self.policy,
            };
```

- Doc-comment fix: the block starting `/// Runtime enable/disable. Order is deliberate: **file first**…` through `/// \`handle_toggle_telnet\`).` currently sits above `handle_set_ntp`. Move it to sit directly above `fn handle_toggle_service`, so each fn carries its own doc.

**Step 8: Gate `G`** — expect 352.

**Step 9: Commit**

```bash
git commit -m "refactor(anyka-init): collapse duplicated supervisor-loop paths

signal_service replaces the restart/kill twins; send_and_reply serves
toggle and set-ntp; Service::pending names the boot-start state. The
control-socket wire format is unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src anyka-init/tests/supervision.rs
```

---

### Task 5: Small module cuts

**Files:** `src/supervise.rs`, `src/control.rs`, `src/storm.rs`, `src/main.rs`, `src/monitor.rs`, `src/netstat.rs`, `src/update.rs`, `src/sys.rs`, `src/wifi.rs`, `src/timesync.rs`, `src/netoverlay.rs`, `src/config.rs`, `examples/config-check.rs`

Run `cargo test --target x86_64-unknown-linux-gnu -p anyka-init` after each numbered step; it is cheap and localises a slip.

**Step 1: `supervise.rs`**

```rust
/// Exponential backoff: `min << (attempt - 1)`, clamped to `max`.
pub fn backoff_delay(attempt: u32, min: Duration, max: Duration) -> Duration {
    // checked_shl fails past 31 and checked_mul on overflow: both mean the
    // delay is already past any sane max.
    1u32.checked_shl(attempt.saturating_sub(1))
        .and_then(|m| min.checked_mul(m))
        .filter(|d| *d < max)
        .unwrap_or(max)
}
```

`RestartHistory::prune` body:

```rust
        while self.stamps.front().is_some_and(|&f| now.duration_since(f) > window) {
            self.stamps.pop_front();
        }
```

`SvcState::pid`: merge the two `None` arms into `Self::Backoff { .. } | Self::Disabled => None,`.

`decide`, the `Backoff` arm:

```rust
        (SvcState::Backoff { until, .. }, _) => Decision {
            action: if now >= *until {
                Action::Start
            } else {
                Action::Sleep(until.duration_since(now))
            },
            next: *state,
        },
```

**Step 2: `control.rs`**

```rust
    pub fn from_svc_state(name: &str, state: &SvcState, hist: &RestartHistory, now: Instant) -> Self {
        let mut s = Self {
            name: name.to_owned(),
            state: "disabled",
            pid: None,
            uptime_s: 0,
            restarts: 0,
            retry_in_s: 0,
        };
        match state {
            SvcState::Running { pid, since } => {
                s.state = "running";
                s.pid = Some(*pid);
                s.uptime_s = now.duration_since(*since).as_secs();
                s.restarts = hist.len() as u64;
            }
            SvcState::Backoff { until, attempt } => {
                s.state = "backoff";
                s.restarts = u64::from(*attempt);
                s.retry_in_s = until.saturating_duration_since(now).as_secs();
            }
            SvcState::Disabled => {}
        }
        s
    }
```

```rust
pub fn encode_status(rows: &[ServiceStatus]) -> String {
    rows.iter().map(ServiceStatus::encode_tsv).collect::<String>() + "\n"
}
```

**Step 3: `storm.rs` + `main.rs`**

Delete `pub fn should_enter_safe_mode` and `test_should_enter_safe_mode_at_threshold`. In `main.rs`:

```rust
    let safe_mode = storm_state.fast_reboots >= cfg.supervisor.storm_guard_max_reboots;
```

**Step 4: `monitor.rs`**

(a) `sample_link`:

```rust
fn sample_link(iface: &str, probe: bool) -> Health {
    let carrier = std::fs::read_to_string(format!("/sys/class/net/{iface}/operstate"))
        .is_ok_and(|s| netstat::parse_operstate(&s))
        && std::fs::read_to_string(format!("/sys/class/net/{iface}/carrier"))
            .is_ok_and(|s| s.trim() == "1");
    let gw = std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|s| netstat::parse_default_route(&s, iface));
    let route = gw.is_some();
    let reachable = match &gw {
        Some(gw) if carrier && probe => netstat::gateway_reachable(gw),
        // Without a probe, treat route presence as reachability so we do not
        // escalate on L3 alone when probing is disabled.
        _ => route,
    };
    Health {
        carrier,
        route,
        reachable,
    }
}
```

(b) Delete `pub struct VideoPolicy`. `video_decide` takes the config directly:

```rust
pub fn video_decide(stalled_ticks: u32, cfg: &MonitorCfg) -> VideoAction {
    if stalled_ticks >= cfg.video_reboot_after_ticks {
        return VideoAction::Reboot;
    }
    if stalled_ticks >= cfg.video_kill_after_ticks {
        return VideoAction::Kill;
    }
    if stalled_ticks >= cfg.video_restart_after_ticks {
        return VideoAction::Restart;
    }
    VideoAction::Nothing
}
```

In `apply_video_actions` delete the `let policy = VideoPolicy { … };` block and call `video_decide(*ticks, cfg)`. In tests delete `fn video_policy()` and replace `&video_policy()` with `&cfg_with_video()` (same 2/3/5 thresholds):

```bash
sed -i 's/&video_policy()/\&cfg_with_video()/g' anyka-init/src/monitor.rs
```

(c) `TickState`:

```rust
/// What `tick` carries from one iteration to the next.
#[derive(Debug, Default)]
pub struct TickState {
    pub reset_done: bool,
    pub wifi_ticks: u32,
    pub video_last: Option<u64>,
    pub video_ticks: u32,
}
```

`tick` drops the `#[allow(clippy::too_many_arguments)]` and its four `&mut` params in favour of `st: &mut TickState`; inside, use `st.reset_done`, `&mut st.wifi_ticks`, `&mut st.video_last`, `&mut st.video_ticks`. `run` becomes:

```rust
    let interval = Duration::from_secs(cfg.interval_sec);
    let mut st = TickState::default();
    loop {
        tick(sys, cfg, iface, state_path, reset_after, &tx, &mut st);
        std::thread::sleep(interval);
    }
```

In the two `test_tick_*` tests replace `let mut reset_done = false; let mut ticks = 0;` and the four trailing args with `let mut st = TickState::default();` / `&mut st`, and assert on `st.reset_done`.

**Step 5: `netstat.rs`**

In `parse_local_ipv4` delete the `if subnets.is_empty() { return None; }` block (the loop below already finds nothing).

**Step 6: `update.rs`**

(a) Add above `reconcile`:

```rust
/// Point `active` back at `prev`, clear the marker, reboot.
///
/// Order matters: if power is lost between the first two steps the next boot
/// repeats a revert that is already correct, whereas clearing the marker
/// first would boot the broken slot with no marker and no way back. Returns
/// false when the pointer could not be restored.
fn revert(sys: &dyn crate::sys::Sys, root: &Path, slots: &Slots, prev: Slot) -> bool {
    if let Err(e) = slots.set_active(prev) {
        tracing::error!(error = %e, "could not restore the previous slot");
        return false;
    }
    if let Err(e) = Trial::clear(root) {
        tracing::error!(error = %e, "could not clear the trial marker");
    }
    let _ = sys.reboot();
    true
}
```

In `reconcile`'s `Outcome::Revert` arm keep the `tracing::error!` and the lock, then replace the rest with `revert(sys, root, &slots, prev);` (drop the now-duplicated "Order matters" comment). In `revert_now` keep the `tracing::error!`, then end with `revert(sys, root, &slots, prev)`.

(b) `evaluate_trial` loses its `ports: &[u16]` parameter and reads `policy.ports`:

```rust
        if policy.ports.iter().all(|p| probe(*p)) {
```

`reconcile` calls `evaluate_trial(&policy, probe, sleep)`. In the four trial tests delete the leading `&[…],` argument and move its list into the `Policy`: the first two tests pass `&[80, 554, 8080]` (= `TRIAL_PORTS`, no change needed), but `a_late_bind_still_confirms_inside_the_deadline` passes `&[554]` and `a_flapping_port_resets_the_hold_and_eventually_reverts` passes `&[80]` — those two must become `ports: vec![554]` and `ports: vec![80]`, or their probes see ports they never expected.

**Step 7: `sys.rs`**

In `spawn` and `spawn_detached` replace the `ManuallyDrop` lines with:

```rust
        // The reaper thread owns reaping via waitpid(-1). std's `Child` has no
        // Drop that waits or kills, so letting the handle go cannot race it.
        Ok(child.id() as Pid)
```

Remove `use std::mem::ManuallyDrop;`. Keep `impl Default for RealSys` (clippy `new_without_default`).

**Step 8: `wifi.rs`**

(a) Delete `pub fn bring_up` and `pub fn bring_up_with`. Update their tests:
- the three `bring_up(&sys, &cfg, "/nonexistent/storm.json")` calls →

```rust
            bring_up_with_overlay(
                &sys,
                &cfg,
                &cfg,
                "/nonexistent/storm.json",
                &FsLayout::production(),
                std::path::Path::new("/nonexistent/network.toml"),
            ),
```

- `bring_up_with(&sys, &cfg, storm_path…, &layout)` → `bring_up_with_overlay(&sys, &cfg, &cfg, storm_path.to_str().expect("utf8"), &layout, &dir.path().join("network.toml"))`.

Fix the doc on `bring_up_with_overlay` that links `[`bring_up_with`]`: it now reads "Full bring-up (steps numbered as in the design addendum), plus rung 2 of the rescue ladder." — keep the rest.

(b) Inline the path constants into `FsLayout::production()` (`hw_conf: "/etc/jffs2/hw.conf".into()`, …, `busybox: "/bin/busybox".into()`) and delete `HW_CONF`, `HW_CONF_FACTORY`, `GPIO_WIFI_EN`, `OTG_MODULE`, `RESOLV_CONF`, `KO_DIR`, `BUSYBOX`, `SYS_CLASS_NET`, `PROC_FIB_TRIE`, `PROC_ROUTE`, `WIFI_MANAGE_SCRIPT`, `WIFI_DRIVER_TGZ`, `WIFI_TOOL_TGZ`. Keep `WPA_CONF` (also used by `supplicant_args`), `SUPPLICANT_BIN`, `KILL_WPA_SHIM`, `DRIVER_PROBE_ORDER`. Use `wpa_conf: WPA_CONF.into()`.

(c) Delete the private `fn gateway_reachable` wrapper; `assign_address` calls `crate::netstat::gateway_reachable(&gw)`.

**Step 9: `timesync.rs`**

`first_sync` loop body:

```rust
    loop {
        let remaining = deadline.saturating_duration_since(sys.now());
        if remaining.is_zero() {
            tracing::warn!(
                timeout_sec = cfg.first_sync_timeout_sec,
                "no NTP sync before boot deadline; continuing with a wrong clock. \
                 Authenticated ONVIF requests will fail until the resync thread succeeds."
            );
            return false;
        }
        if sync_once(sys, cfg, Some(remaining), ntp_disabled).is_some() {
            return true;
        }
        // Bounded by `deadline`; the check above ends the loop once it
        // passes, so a retry_interval longer than the timeout means one attempt.
        let left = deadline.saturating_duration_since(sys.now());
        sys.sleep(Duration::from_secs(cfg.retry_interval_sec.min(2)).min(left));
    }
```

In `sync_once`:

```rust
        let timeout = remaining
            .unwrap_or(Duration::from_secs(5))
            .min(Duration::from_secs(5));
```

and delete the `if timeout.is_zero() { return None; }` block that followed (unreachable: a zero `left` already returned).

**Step 10: `netoverlay.rs` + `config.rs`**

- Remove `Serialize` from the derive and the `use`, and delete all seven `#[serde(skip_serializing_if = "Option::is_none")]` lines. (onvif-rust has its own copy that it serializes; this crate only reads.)
- Delete `pub fn has_content` and, in `config.rs` `merge_network_overlay`, the `if !overlay.has_content() { return Ok(()); }` block. An empty overlay applies as a no-op and the merged config is the already-validated baseline.

**Step 11: `examples/config-check.rs`**

```rust
fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        eprintln!("usage: config-check <anyka.toml>");
        return std::process::ExitCode::FAILURE;
    };
    match anyka_init::config::Config::load(&path) {
```

(rest unchanged). Smoke-test it:

```bash
cargo run -q --target x86_64-unknown-linux-gnu -p anyka-init --example config-check -- ../SD_card_contents/anyka_hack/anyka.toml
cargo run -q --target x86_64-unknown-linux-gnu -p anyka-init --example config-check; echo "exit=$?"
```

Expected: `OK   …(schema=N)`, then the usage line and `exit=1`.

**Step 12: Gate `G`** — expect 351 (352 − the `should_enter_safe_mode` test).

**Step 13: Commit**

```bash
git commit -m "refactor(anyka-init): small per-module cuts

Test-only bring_up wrappers, a single-use VideoPolicy, a no-op
ManuallyDrop, redundant guards in backoff/first_sync/sync_once, and a
10-arg tick that now takes a TickState. No behaviour change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src anyka-init/examples
```

---

### Task 6: Behaviour — post-merge validation replaces overlay validation

**Files:**
- Modify: `src/config.rs` (`merge_network_overlay`, tests)
- Modify: `src/netoverlay.rs` (`validate`)

**Step 1: Write the failing test**

Append to `mod tests` in `src/config.rs`:

```rust
    #[test]
    fn test_overlay_static_switch_may_borrow_the_baseline_address() {
        // The overlay only flips dhcp; the address and gateway come from the
        // operator's file. The merged config is valid, so the overlay stands.
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("anyka.toml");
        let overlay = dir.path().join("network.toml");
        std::fs::write(
            &base,
            "[wifi]\nssid = \"OperatorNet\"\npassword = \"operatorpass\"\n\
             address = \"192.168.2.50/24\"\ngateway = \"192.168.2.1\"\n",
        )
        .expect("write base");
        std::fs::write(&overlay, "dhcp = false\n").expect("write overlay");

        let cfg = Config::load_with_overlay(base.to_str().expect("utf8"), &overlay)
            .expect("merged config is valid");

        assert!(!cfg.wifi.dhcp);
        assert_eq!(cfg.wifi.address.as_deref(), Some("192.168.2.50/24"));
        assert!(overlay.exists(), "a valid merge must not quarantine the overlay");
    }

    #[test]
    fn test_overlay_with_an_unknown_security_value_is_still_quarantined() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("anyka.toml");
        let overlay = dir.path().join("network.toml");
        std::fs::write(&base, "[wifi]\nssid = \"OperatorNet\"\npassword = \"operatorpass\"\n")
            .expect("write base");
        std::fs::write(&overlay, "security = \"wpa3\"\n").expect("write overlay");

        let cfg = Config::load_with_overlay(base.to_str().expect("utf8"), &overlay)
            .expect("a bad overlay must not park the baseline load");

        assert_eq!(cfg.wifi.security, "wpa");
        assert!(!overlay.exists());
        assert!(dir.path().join("network.toml.bad").exists());
    }
```

**Step 2: Run them**

```bash
cargo test --target x86_64-unknown-linux-gnu -p anyka-init test_overlay_ 2>&1 | tail -8
```

Expected: `test_overlay_static_switch_may_borrow_the_baseline_address` FAILS (`dhcp` is still true — the overlay was quarantined by `NetworkOverlay::validate`). The security test PASSES (regression guard for the next step).

**Step 3: Remove the pre-merge check**

- Delete `pub fn validate` from `src/netoverlay.rs`.
- In `merge_network_overlay` delete:

```rust
        if let Err(err) = overlay.validate() {
            tracing::warn!(error = %err, "invalid network overlay; quarantining");
            crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
            return Ok(());
        }
```

Keep `let baseline_wifi = cfg.wifi.clone();` above `overlay.apply_to(…)`, and update the fn's doc to: "Merge `network.toml` onto `[wifi]`. The *merged* result is validated; a merge that fails quarantines the overlay and restores the baseline instead of parking the camera."

**Step 4: Run them again** — both PASS.

**Step 5: Gate `G`** — expect 353.

**Step 6: Commit**

```bash
git commit -m "fix(anyka-init): validate the merged network config, not the overlay alone

An overlay that only sets dhcp = false was quarantined even when the
operator's anyka.toml supplied the address and gateway. The post-merge
validate() already catches every invalid result, so the pre-merge check
only rejected valid merges.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/config.rs anyka-init/src/netoverlay.rs
```

---

### Task 7: Behaviour — OS-seeded nonce from std

**Files:**
- Modify: `src/timesync.rs`

**Step 1: Replace `random_nonce` and delete `fallback_nonce`**

```rust
/// A 64-bit nonce from std's OS-seeded `RandomState` (getrandom, falling back
/// to /dev/urandom). Panics only on a system with no randomness source at
/// all, where the old hand-mixed fallback would have been guessable anyway.
pub fn random_nonce() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(())
}
```

Delete `fn fallback_nonce` and `test_fallback_nonce_successive_calls_differ`. Remove `use std::io::Read;` if the compiler flags it unused.

**Step 2: Keep the remaining nonce test as is**

`test_random_nonce_returns_nonzero_entropy` already asserts 8 samples are non-zero *and* unique, which is the property the deleted fallback test guarded. std increments the per-thread key on every `RandomState::new()`, so uniqueness is deterministic, not flaky. Fix its comment: "On the host this usually hits /dev/urandom; either path…" → "std seeds RandomState from the OS; successive nonces must still differ."

**Step 3: Gate `G`** — expect 352.

**Step 4: Commit**

```bash
git commit -m "refactor(anyka-init): NTP nonce from std RandomState

Same OS randomness source as the /dev/urandom read it replaces; drops the
28-line hand-mixed fallback that only ran where no OS randomness exists.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/timesync.rs
```

---

### Task 8: Behaviour — drop loadavg from the `sys` sample

**Files:**
- Modify: `src/monitor.rs`

**Step 1:** Delete `pub fn parse_loadavg` and `test_parse_loadavg_first_field`. `sample` becomes:

```rust
fn sample() {
    // No loadavg: on this SoC it counts D-state wifi threads and reads 3–15
    // at 40 % idle CPU, so it misled every investigation that used it.
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| parse_mem_kb(&s));
    tracing::info!(mem_avail_kb = mem, "sys");
}
```

**Step 2: Gate `G`** — expect 351.

**Step 3: Commit**

```bash
git commit -m "refactor(anyka-init): drop load1 from the sys sample

loadavg on the AK3918 counts D-state wifi threads; it read 3.5-15 at 40%
idle CPU and misled the crash-hardening investigation.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/monitor.rs
```

---

### Task 9: Behaviour — untar without a shell

**Files:**
- Modify: `src/update.rs` (`stage_and_flip`, tests)

**Step 1: Add the test helper the new argv needs**

Next to `fake_rm` in `mod tests`:

```rust
    /// The `-C` target of a mocked `busybox tar -xf <tar> -C <dir>` call.
    fn fake_untar_dir(args: &[String]) -> Option<String> {
        (args.first().map(String::as_str) == Some("tar")).then(|| args[4].clone())
    }
```

Replace each of the five untar blocks:

```rust
            if let Some(cmd) = args.iter().find(|a| a.contains("tar -xf")) {
                let dir = cmd
                    .split("-C ")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('\'').to_string())
                    .expect("untar -C dir");
```

with `if let Some(dir) = fake_untar_dir(args) {`, and the one unconditional variant (`let cmd = args.iter().find(…).unwrap(); let dir = cmd.split(…)…;`) with `let dir = fake_untar_dir(args).expect("untar -C dir");`.

**Step 2: Run the apply tests — they must now FAIL**

```bash
cargo test --target x86_64-unknown-linux-gnu -p anyka-init update::tests 2>&1 | tail -8
```

Expected: the staging/apply tests fail (production still sends `sh -c "busybox tar …"`, which `fake_untar_dir` no longer recognises). This proves the mocks now assert the new argv.

**Step 3: Change production**

In `stage_and_flip`:

```rust
    // No `sh -c`: busybox dispatches on its first argument, so neither path
    // passes through a shell (same as `remove_tree`).
    let untar = [
        "tar".to_string(),
        "-xf".to_string(),
        root.join("spool/bundle.tar").to_string_lossy().into_owned(),
        "-C".to_string(),
        staging.to_string_lossy().into_owned(),
    ];
```

`shell_quote` stays — `verify_slot` still needs it for the `cd … && sha256sum`.

**Step 4: Gate `G`** — expect 351.

**Step 5: Commit**

```bash
git commit -m "refactor(anyka-init): run the bundle untar without a shell

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" -- anyka-init/src/update.rs
```

---

### Task 10: ARM build, .198 deploy, PR

**Step 1: Measure the result**

```bash
git diff --stat 1bc401c7 -- anyka-init | tail -1
```

Expected: roughly `-600` net in `anyka-init/src`. Record the real number for the PR body.

**Step 2: ARM cross-build** (@anyka-embedded-build). PR CI never builds ARM.

```bash
cd anyka-init
cargo build --release 2>&1 | tail -3
file ../target/armv5te-unknown-linux-uclibceabi/release/anyka-init
cd ..
```

Must run from `cross-compile/anyka-init/` (from the workspace root cargo silently links with the host toolchain). Expected: `ELF 32-bit LSB executable, ARM`.

**Step 3: Deploy to .198** (@anyka-firmware-upgrade)

Build and push an A/B bundle to 192.168.2.198 following the skill. Bundles never carry `anyka.toml`; this change adds no keys, so none is needed. Then verify, with the camera's telnet on :24:

| Check | How | Expect |
|---|---|---|
| Trial confirms (exercises Task 9 untar + Task 3 `set_active`) | `ls /mnt/anyka_hack/state/` after ~2 min | no `trial-*` marker; `cat /mnt/anyka_hack/active` names the new slot |
| Defaults unchanged (Task 2) | `grep -c . /mnt/logs/anyka-init.log` grows; boot reached services | services running |
| Supervisor control (Task 4) | WebUI Services page (onvif-rust reads `/tmp/anyka-supervisor.sock`); toggle `snmp` off and on | all rows `running`; toggle replies `ok` and persists to `anyka.toml` |
| NTP (Task 7) | `cat /mnt/anyka_hack/state/ntp.status` | non-zero `last_unix` |
| loadavg gone (Task 8) | `grep '"sys"\| sys' /mnt/logs/anyka-init.log \| tail -1` | `mem_avail_kb=` and no `load1` |
| Streams | `curl -sI http://192.168.2.198/` and an RTSP probe on :554 | 200, stream plays |

Remember the shipped log level filters INFO; if a check depends on an INFO line, read the status files instead. If anything fails, `git revert` the matching task's commit rather than patching forward.

**Step 4: Code review**

Use superpowers:requesting-code-review on `1bc401c7..HEAD`.

**Step 5: PR**

Use superpowers:finishing-a-development-branch. PR body: the design doc link, the measured line delta, the four behaviour changes (one line each), and the .198 verification table with results. End with:

```
🤖 Generated with [Claude Code](https://claude.com/claude-code)
```
