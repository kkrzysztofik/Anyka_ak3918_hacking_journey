# Boot/Runtime Rust Supervisor — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace ~1,700 lines of unsupervised POSIX shell in the camera's boot path with one resident Rust supervisor binary.

**Architecture:** Vendor firmware runs `/mnt/Factory/config.sh` (a ~10-line shell wrapper that starts recovery telnet, then hands off to the ELF). The ELF parses TOML config, performs system setup, syncs the clock over SNTP, starts every service with a cleared environment, then blocks forever reaping children and restarting them with exponential backoff. No tokio, no async — a blocking `waitpid` thread feeding an `mpsc` channel that `main` waits on with `recv_timeout`.

**Tech Stack:** Rust 2024 edition, `std` only for the runtime, `serde`+`toml` for config, `tracing` for logs, `libc` for syscalls, `signal-hook` for SIGTERM, `mockall` for unit tests. Cross-compiled with the vendored toolchain at `toolchain/arm-anykav200-crosstool-ng/`.

**Design doc:** `docs/plans/2026-08-01-boot-runtime-rust-design.md` — read it before starting.

---

## Before You Start

**Load the vendored toolchain. Every `cargo` command in this plan assumes it.**

```bash
cd /home/kmk/dev/anyka-dev
source ./setenv.sh
echo "$CARGO"    # must print .../toolchain/arm-anykav200-crosstool-ng/bin/cargo
```

`setenv.sh` prepends the toolchain `bin/` to `PATH`. This matters: `cargo clippy`
fails with `E0514` (rustc version mismatch) if the toolchain `bin/` is not first
on `PATH`. Never substitute system `cargo`.

**The three commands you will run constantly, always from `cross-compile/anyka-init/`:**

```bash
$CARGO test   --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

**Project conventions** (`AGENTS.md`): `snake_case` for functions/vars, `CamelCase`
for types, `Result<T, E>` with `?` — no `unwrap()`/`expect()` outside `#[cfg(test)]`,
descriptive test names like `test_backoff_saturates_at_max` not `test1`.

**Working branch:** `design/boot-runtime-rust` already exists with the design doc
committed. Continue on it, or branch from it.

---

## Task 1: Scaffold the crate

**Files:**
- Create: `cross-compile/anyka-init/Cargo.toml`
- Create: `cross-compile/anyka-init/src/main.rs`
- Create: `cross-compile/anyka-init/src/lib.rs`
- Modify: `cross-compile/Cargo.toml:2`

**Step 1: Add the workspace member**

In `cross-compile/Cargo.toml`, change line 2 from:

```toml
members = ["onvif-rust", "streaming-lib"]
```

to:

```toml
members = ["onvif-rust", "streaming-lib", "anyka-init"]
```

**Step 2: Create `cross-compile/anyka-init/Cargo.toml`**

```toml
[package]
name = "anyka-init"
version = "0.1.0"
edition = "2024"
description = "Boot and process supervisor for the Anyka AK3918 SD-card hack"

[lib]
name = "anyka_init"
path = "src/lib.rs"
doctest = false

[[bin]]
name = "anyka-init"
path = "src/main.rs"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "1.1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
libc = "0.2"
signal-hook = "0.3"
thiserror = "2.0"
anyhow = "1.0"

[dev-dependencies]
mockall = "0.15"
tempfile = "3"
```

**Step 3: Create `cross-compile/anyka-init/src/lib.rs`**

```rust
//! Boot and process supervisor for the Anyka AK3918 SD-card hack.
//!
//! Entered from `/mnt/Factory/config.sh`, which the vendor's
//! `/usr/sbin/service.sh` runs when a `Factory/` directory is present on the
//! SD card. See `docs/plans/2026-08-01-boot-runtime-rust-design.md`.

pub mod config;
pub mod supervise;
pub mod sys;
pub mod timesync;
```

Create empty module files so it compiles:

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init/src
touch config.rs supervise.rs sys.rs timesync.rs
```

**Step 4: Create `cross-compile/anyka-init/src/main.rs`**

```rust
fn main() {
    println!("anyka-init: not implemented");
}
```

**Step 5: Verify it builds for the host**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init
$CARGO build --target x86_64-unknown-linux-gnu
```
Expected: `Finished` with no errors.

**Step 6: Verify it cross-compiles for the camera**

```bash
$CARGO build --release --target arm-anykav200-crosstool-ng
file target/arm-anykav200-crosstool-ng/release/anyka-init
```
Expected: `ELF 32-bit LSB executable, ARM, EABI5 ... dynamically linked, interpreter /mnt/anyka_hack/lib/ld-uClibc.so.1`

If the interpreter differs from that path, **stop** — the deploy layout in Task 17 depends on it.

**Step 7: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/Cargo.toml cross-compile/anyka-init/
git commit -m "feat(anyka-init): scaffold supervisor crate"
```

---

## Task 2: Config types and TOML parsing

The supervisor is the only component that reads config. Everything downstream
takes a `&Config`. Parsing must reject, never evaluate — this is what closes the
boot-time RCE (D2 in the design doc).

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs`

**Step 1: Write the failing tests**

Append to `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[wifi]
ssid = "testnet"
password = "secret"
"#;

    #[test]
    fn test_config_parse_minimal_applies_defaults() {
        let cfg = Config::from_str(MINIMAL).expect("minimal config must parse");
        assert_eq!(cfg.wifi.ssid, "testnet");
        assert_eq!(cfg.log.dir, "/mnt/logs");
        assert_eq!(cfg.supervisor.backoff_min_sec, 1);
        assert_eq!(cfg.supervisor.backoff_max_sec, 60);
        assert_eq!(cfg.supervisor.crashloop_count, 10);
        assert_eq!(cfg.time.resync_interval_sec, 21_600);
        assert!(cfg.services.is_empty());
    }

    #[test]
    fn test_config_parse_rejects_unknown_key() {
        let src = format!("{MINIMAL}\n[system]\nnot_a_real_key = 1\n");
        let err = Config::from_str(&src).expect_err("unknown key must be rejected");
        assert!(
            format!("{err}").contains("not_a_real_key"),
            "error should name the offending key, got: {err}"
        );
    }

    #[test]
    fn test_config_parse_rejects_wrong_type() {
        let src = format!("{MINIMAL}\n[supervisor]\nbackoff_min_sec = \"soon\"\n");
        assert!(Config::from_str(&src).is_err());
    }

    #[test]
    fn test_config_parse_rejects_missing_wifi() {
        assert!(Config::from_str("[log]\nlevel = \"info\"\n").is_err());
    }

    #[test]
    fn test_config_parse_rejects_shell_syntax() {
        // The old gergesettings.txt format must not silently parse as TOML.
        assert!(Config::from_str("run_ssh=1\nwifi_ssid=kmk\n").is_err());
    }

    #[test]
    fn test_config_parse_service_table() {
        let src = format!(
            r#"{MINIMAL}
[services.vendor-daemon]
enabled = true
exec = "/mnt/anyka_hack/vendor-daemon/vendor-daemon.bin"
log = "/mnt/logs/vendor_daemon.log"
core_dump = true
env = {{ LD_LIBRARY_PATH = "/mnt/anyka_hack/vendor-daemon/lib" }}
"#
        );
        let cfg = Config::from_str(&src).expect("service table must parse");
        let svc = cfg.services.get("vendor-daemon").expect("service present");
        assert!(svc.enabled);
        assert!(svc.core_dump);
        assert!(svc.args.is_empty());
        assert_eq!(
            svc.env.get("LD_LIBRARY_PATH").map(String::as_str),
            Some("/mnt/anyka_hack/vendor-daemon/lib")
        );
    }

    #[test]
    fn test_config_validate_rejects_backoff_min_above_max() {
        let src = format!("{MINIMAL}\n[supervisor]\nbackoff_min_sec = 90\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_implausible_time_bounds() {
        let src = format!("{MINIMAL}\n[time]\nmin_plausible_unix = 99\nmax_plausible_unix = 98\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());
    }
}
```

**Step 2: Run the tests to verify they fail**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init
$CARGO test --target x86_64-unknown-linux-gnu config
```
Expected: FAIL — `cannot find type Config in this scope`.

**Step 3: Write the implementation**

Prepend to `src/config.rs` (above the `#[cfg(test)]` block):

```rust
//! Typed configuration, parsed from `/mnt/anyka_hack/anyka.toml`.
//!
//! This file is *parsed*, never evaluated. The predecessor
//! (`gergesettings.txt`) was `.`-sourced by `gergehack.sh`, which made any SD
//! card an unsandboxed root code-execution vector at boot.
//!
//! `deny_unknown_fields` everywhere is deliberate: a typo'd key in a config a
//! user edits by hand on an SD card must be a loud failure, not a silent
//! fallback to a default they did not intend.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub log: LogCfg,
    #[serde(default)]
    pub system: SystemCfg,
    pub wifi: WifiCfg,
    #[serde(default)]
    pub time: TimeCfg,
    #[serde(default)]
    pub supervisor: SupervisorCfg,
    #[serde(default)]
    pub monitor: MonitorCfg,
    #[serde(default)]
    pub reboot: RebootCfg,
    #[serde(default)]
    pub services: BTreeMap<String, ServiceCfg>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogCfg {
    #[serde(default = "d_log_dir")]
    pub dir: String,
    #[serde(default = "d_log_level")]
    pub level: String,
    #[serde(default = "d_log_max_bytes")]
    pub max_bytes: u64,
    #[serde(default = "d_log_keep")]
    pub keep: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemCfg {
    /// Sensor kernel module to load.
    ///
    /// Load-bearing despite `camera.sh:37-38` also loading sensor modules:
    /// the hack ships its module at `/data/sensor/`, which is on *none* of
    /// camera.sh's three search paths (`/etc/jffs2`, `/usr/modules`,
    /// `/data/sensor_ko_and_isp_conf`). Do not delete this as a duplicate.
    #[serde(default)]
    pub sensor_module: Option<String>,
    /// Keep the P0 recovery telnetd running after boot.
    #[serde(default)]
    pub telnet: bool,
    /// Keep the vendor's FTP server (`rc.local:14`) running.
    #[serde(default = "d_true")]
    pub ftp: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WifiCfg {
    pub ssid: String,
    pub password: String,
    #[serde(default = "d_wifi_cfg_file")]
    pub config_file: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    #[serde(default = "d_ntp_servers")]
    pub servers: Vec<String>,
    #[serde(default = "d_timezone")]
    pub timezone: String,
    #[serde(default = "d_first_sync_timeout")]
    pub first_sync_timeout_sec: u64,
    #[serde(default = "d_retry_interval")]
    pub retry_interval_sec: u64,
    #[serde(default = "d_resync_interval")]
    pub resync_interval_sec: u64,
    #[serde(default = "d_step_threshold")]
    pub step_threshold_sec: u64,
    #[serde(default = "d_min_plausible")]
    pub min_plausible_unix: u64,
    #[serde(default = "d_max_plausible")]
    pub max_plausible_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupervisorCfg {
    #[serde(default = "d_backoff_min")]
    pub backoff_min_sec: u64,
    #[serde(default = "d_backoff_max")]
    pub backoff_max_sec: u64,
    #[serde(default = "d_crashloop_count")]
    pub crashloop_count: u32,
    #[serde(default = "d_crashloop_window")]
    pub crashloop_window_sec: u64,
    #[serde(default = "d_storm_max")]
    pub storm_guard_max_reboots: u8,
    #[serde(default = "d_storm_state")]
    pub storm_guard_state: String,
    #[serde(default = "d_storm_reset_uptime")]
    pub storm_guard_reset_uptime_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    #[serde(default = "d_monitor_interval")]
    pub interval_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RebootCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "d_reboot_interval")]
    pub interval_min: u64,
    #[serde(default)]
    pub jitter_max_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    pub exec: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Injected verbatim into the child after clearing its environment.
    ///
    /// This is the structural fix for the loader-poisoning bug documented in
    /// `SD_card_contents/anyka_hack/onvif/onvif-rust`: two incompatible uClibc
    /// versions coexist on this device, and an inherited `LD_LIBRARY_PATH`
    /// breaks every busybox applet a service starts.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub log: String,
    #[serde(default)]
    pub core_dump: bool,
}

fn d_true() -> bool { true }
fn d_log_dir() -> String { "/mnt/logs".into() }
fn d_log_level() -> String { "info".into() }
fn d_log_max_bytes() -> u64 { 2_000_000 }
fn d_log_keep() -> u8 { 2 }
fn d_wifi_cfg_file() -> String { "/etc/jffs2/anyka_cfg.ini".into() }
fn d_ntp_servers() -> Vec<String> {
    vec!["0.ubuntu.pool.ntp.org".into(), "1.ubuntu.pool.ntp.org".into()]
}
fn d_timezone() -> String { "GMT+00:00".into() }
fn d_first_sync_timeout() -> u64 { 15 }
fn d_retry_interval() -> u64 { 30 }
fn d_resync_interval() -> u64 { 21_600 }
fn d_step_threshold() -> u64 { 2 }
fn d_min_plausible() -> u64 { 1_767_225_600 } // 2026-01-01
fn d_max_plausible() -> u64 { 2_524_608_000 } // 2050-01-01
fn d_backoff_min() -> u64 { 1 }
fn d_backoff_max() -> u64 { 60 }
fn d_crashloop_count() -> u32 { 10 }
fn d_crashloop_window() -> u64 { 600 }
fn d_storm_max() -> u8 { 3 }
fn d_storm_state() -> String { "/mnt/anyka_hack/state/boot.json".into() }
fn d_storm_reset_uptime() -> u64 { 600 }
fn d_monitor_interval() -> u64 { 60 }
fn d_reboot_interval() -> u64 { 720 }

impl Default for LogCfg {
    fn default() -> Self {
        Self { dir: d_log_dir(), level: d_log_level(), max_bytes: d_log_max_bytes(), keep: d_log_keep() }
    }
}
impl Default for SystemCfg {
    fn default() -> Self {
        Self { sensor_module: None, telnet: false, ftp: true }
    }
}
impl Default for TimeCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: d_ntp_servers(),
            timezone: d_timezone(),
            first_sync_timeout_sec: d_first_sync_timeout(),
            retry_interval_sec: d_retry_interval(),
            resync_interval_sec: d_resync_interval(),
            step_threshold_sec: d_step_threshold(),
            min_plausible_unix: d_min_plausible(),
            max_plausible_unix: d_max_plausible(),
        }
    }
}
impl Default for SupervisorCfg {
    fn default() -> Self {
        Self {
            backoff_min_sec: d_backoff_min(),
            backoff_max_sec: d_backoff_max(),
            crashloop_count: d_crashloop_count(),
            crashloop_window_sec: d_crashloop_window(),
            storm_guard_max_reboots: d_storm_max(),
            storm_guard_state: d_storm_state(),
            storm_guard_reset_uptime_sec: d_storm_reset_uptime(),
        }
    }
}
impl Default for MonitorCfg {
    fn default() -> Self {
        Self { enabled: true, interval_sec: d_monitor_interval() }
    }
}
impl Default for RebootCfg {
    fn default() -> Self {
        Self { enabled: false, interval_min: d_reboot_interval(), jitter_max_sec: 0 }
    }
}

impl Config {
    pub fn from_str(src: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(src)?)
    }

    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let src = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg = Self::from_str(&src)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.supervisor.backoff_min_sec > self.supervisor.backoff_max_sec {
            return Err(ConfigError::Invalid(
                "supervisor.backoff_min_sec exceeds backoff_max_sec".into(),
            ));
        }
        if self.time.min_plausible_unix >= self.time.max_plausible_unix {
            return Err(ConfigError::Invalid(
                "time.min_plausible_unix must be below max_plausible_unix".into(),
            ));
        }
        if self.time.enabled && self.time.servers.is_empty() {
            return Err(ConfigError::Invalid(
                "time.enabled is true but time.servers is empty".into(),
            ));
        }
        for (name, svc) in &self.services {
            if svc.exec.is_empty() {
                return Err(ConfigError::Invalid(format!("services.{name}.exec is empty")));
            }
        }
        Ok(())
    }
}
```

**Step 4: Run the tests to verify they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu config
```
Expected: PASS, 8 tests.

**Step 5: Lint and format**

```bash
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 6: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs
git commit -m "feat(anyka-init): typed TOML config with strict unknown-key rejection"
```

---

## Task 3: The `Sys` trait

Every syscall goes behind this trait so that policy is testable on `x86_64`.
This is the only module containing `unsafe`.

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs`

**Step 1: Write the trait definition**

```rust
//! The syscall boundary.
//!
//! Everything the supervisor does to the outside world goes through `Sys`, so
//! that `supervise::decide` and friends can be exercised on the host with
//! `MockSys`. This is the only module in the crate that contains `unsafe`.

use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime};

pub type Pid = i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSpec {
    pub exec: String,
    pub args: Vec<String>,
    /// Applied after the child's environment is cleared. See
    /// `config::ServiceCfg::env`.
    pub env: BTreeMap<String, String>,
    pub log: String,
    pub core_dump: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

#[derive(Debug, thiserror::Error)]
pub enum SysError {
    #[error("start {exec} failed: {source}")]
    Spawn {
        exec: String,
        #[source]
        source: std::io::Error,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

#[cfg_attr(test, mockall::automock)]
pub trait Sys: Send + Sync {
    fn spawn(&self, spec: &SpawnSpec) -> Result<Pid, SysError>;
    /// Blocks until any child exits. Called only from the reaper thread.
    fn wait_any(&self) -> Result<(Pid, ExitStatus), SysError>;
    fn kill(&self, pid: Pid, sig: i32) -> Result<(), SysError>;
    fn reboot(&self) -> Result<(), SysError>;
    /// Monotonic. Never `SystemTime` — P2.5 steps the wall clock by decades.
    fn now(&self) -> Instant;
    fn realtime(&self) -> SystemTime;
    fn set_realtime(&self, t: SystemTime) -> Result<(), SysError>;
    /// Elapsed since this process started. Used by the storm-guard reset.
    fn uptime(&self) -> Duration;
    fn insmod(&self, path: &str) -> Result<(), SysError>;
    fn run_to_completion(&self, prog: &str, args: &[&str]) -> Result<ExitStatus, SysError>;
}
```

**Step 2: Add `RealSys` in the same file**

Implement each method with `libc`. Key points, in order:

- **`spawn`** — open `spec.log` with `create(true).append(true)`, `try_clone()`
  it for stderr, then build a `std::process::Command` with `.args(&spec.args)`,
  `.env_clear()`, `.envs(&spec.env)`, `.stdin(Stdio::null())` and both output
  streams pointed at the log file.
  If `spec.core_dump` is set, use `std::os::unix::process::CommandExt`'s
  pre-`exec` hook to call `libc::setrlimit(libc::RLIMIT_CORE, &lim)` with
  `RLIM_INFINITY` for both soft and hard limits, returning
  `std::io::Error::last_os_error()` on failure. Document the `unsafe` block:
  `setrlimit` is async-signal-safe and touches only the child's own rlimits.
  After `.spawn()`, take `child.id() as Pid` and `std::mem::forget(child)` —
  the reaper thread owns reaping via `waitpid(-1)`, and letting `std` also try
  to reap would race it.
- **`wait_any`** — `libc::waitpid(-1, &mut status, 0)`; negative return means
  `Err(SysError::Io(std::io::Error::last_os_error()))`. Map the status with
  `libc::WIFSIGNALED` / `libc::WTERMSIG` / `libc::WEXITSTATUS`.
- **`kill`** — `libc::kill(pid, sig)`, nonzero is an error.
- **`reboot`** — `libc::sync()` then `libc::reboot(libc::RB_AUTOBOOT)`. It does
  not return on success, so anything after it is the error path.
- **`now`** — `Instant::now()`. **`realtime`** — `SystemTime::now()`.
- **`set_realtime`** — `duration_since(UNIX_EPOCH)` into a `libc::timespec`,
  then `libc::clock_settime(libc::CLOCK_REALTIME, &ts)`.
- **`uptime`** — store an `Instant` in `RealSys::new()` and return `.elapsed()`.
- **`insmod`** — delegate to `run_to_completion("insmod", &[path])`, mapping a
  nonzero exit to `SysError::Other`.
- **`run_to_completion`** — `std::process::Command::new(prog).args(args).status()`.
  Add a comment: this uses `std`'s own wait, which races the reaper's
  `waitpid(-1)`, so it must only be called during P2, before the reaper starts.

**Step 3: Verify it builds and lints**

```bash
$CARGO build --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 4: Commit**

```bash
git add cross-compile/anyka-init/src/sys.rs
git commit -m "feat(anyka-init): Sys trait seam with libc-backed RealSys"
```

---

## Task 4: Backoff arithmetic

**Files:**
- Modify: `cross-compile/anyka-init/src/supervise.rs`

**Step 1: Write the failing tests**

Append to `src/supervise.rs`:

```rust
#[cfg(test)]
mod backoff_tests {
    use super::*;
    use std::time::Duration;

    const MIN: Duration = Duration::from_secs(1);
    const MAX: Duration = Duration::from_secs(60);

    #[test]
    fn test_backoff_doubles_from_min() {
        assert_eq!(backoff_delay(1, MIN, MAX), Duration::from_secs(1));
        assert_eq!(backoff_delay(2, MIN, MAX), Duration::from_secs(2));
        assert_eq!(backoff_delay(3, MIN, MAX), Duration::from_secs(4));
        assert_eq!(backoff_delay(4, MIN, MAX), Duration::from_secs(8));
        assert_eq!(backoff_delay(5, MIN, MAX), Duration::from_secs(16));
        assert_eq!(backoff_delay(6, MIN, MAX), Duration::from_secs(32));
    }

    #[test]
    fn test_backoff_saturates_at_max() {
        assert_eq!(backoff_delay(7, MIN, MAX), MAX);
        assert_eq!(backoff_delay(50, MIN, MAX), MAX);
        // Must not panic or wrap on a shift far past u64 width.
        assert_eq!(backoff_delay(u32::MAX, MIN, MAX), MAX);
    }

    #[test]
    fn test_backoff_attempt_zero_returns_min() {
        assert_eq!(backoff_delay(0, MIN, MAX), MIN);
    }
}
```

**Step 2: Run to verify failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu backoff
```
Expected: FAIL — `cannot find function backoff_delay`.

**Step 3: Implement**

Prepend to `src/supervise.rs`:

```rust
//! Restart policy. Everything here is pure so it can be exercised exhaustively
//! on the host; the caller applies the returned `Action` through `Sys`.
//!
//! All time is monotonic (`Instant`). P2.5 steps the wall clock by decades
//! seconds into supervision, so any policy built on `SystemTime` would either
//! fire instantly or never.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Exponential backoff: `min << (attempt - 1)`, clamped to `max`.
pub fn backoff_delay(attempt: u32, min: Duration, max: Duration) -> Duration {
    if attempt == 0 {
        return min;
    }
    // Shifts past 63 would overflow; anything that large is already >= max.
    let shift = attempt - 1;
    if shift >= 63 {
        return max;
    }
    match min.checked_mul(1u32 << shift.min(31)) {
        Some(d) if d < max => d,
        _ => max,
    }
}
```

**Step 4: Run to verify pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu backoff
```
Expected: PASS, 3 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervise.rs
git commit -m "feat(anyka-init): exponential backoff with saturation"
```

---

## Task 5: Crash-loop history window

**Files:**
- Modify: `cross-compile/anyka-init/src/supervise.rs`

**Step 1: Write the failing tests**

Append a new test module to `src/supervise.rs`:

```rust
#[cfg(test)]
mod history_tests {
    use super::*;
    use std::time::{Duration, Instant};

    const WINDOW: Duration = Duration::from_secs(600);

    #[test]
    fn test_history_counts_restarts_inside_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..5 {
            h.record(t0 + Duration::from_secs(i * 10));
        }
        h.prune(t0 + Duration::from_secs(50), WINDOW);
        assert_eq!(h.len(), 5);
    }

    #[test]
    fn test_history_prunes_entries_older_than_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.record(t0 + Duration::from_secs(100));
        // 700s after t0: the first entry is 700s old, outside a 600s window.
        h.prune(t0 + Duration::from_secs(700), WINDOW);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn test_history_entry_exactly_at_window_edge_is_kept() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.prune(t0 + WINDOW, WINDOW);
        assert_eq!(h.len(), 1, "an entry exactly `window` old is still inside");
    }

    #[test]
    fn test_history_empty_prune_is_noop() {
        let mut h = RestartHistory::default();
        h.prune(Instant::now(), WINDOW);
        assert_eq!(h.len(), 0);
    }
}
```

**Step 2: Run to verify failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu history
```
Expected: FAIL — `cannot find type RestartHistory`.

**Step 3: Implement**

Add to `src/supervise.rs`:

```rust
/// Sliding window of restart timestamps, used for the crash-loop cap.
#[derive(Debug, Default)]
pub struct RestartHistory {
    stamps: VecDeque<Instant>,
}

impl RestartHistory {
    pub fn record(&mut self, at: Instant) {
        self.stamps.push_back(at);
    }

    /// Drops entries strictly older than `window`. An entry exactly `window`
    /// old is still inside the window.
    pub fn prune(&mut self, now: Instant, window: Duration) {
        while let Some(&front) = self.stamps.front() {
            if now.duration_since(front) > window {
                self.stamps.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.stamps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stamps.is_empty()
    }

    pub fn clear(&mut self) {
        self.stamps.clear();
    }
}
```

**Step 4: Run to verify pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu history
```
Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervise.rs
git commit -m "feat(anyka-init): sliding-window restart history"
```

---

## Task 6: The `decide` state machine

The heart of the supervisor, and the reason for the whole trait seam. Pure
function, total, exhaustively testable.

**Files:**
- Modify: `cross-compile/anyka-init/src/supervise.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod decide_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn policy() -> Policy {
        Policy {
            backoff_min: Duration::from_secs(1),
            backoff_max: Duration::from_secs(60),
            crashloop_count: 10,
            crashloop_window: Duration::from_secs(600),
        }
    }

    #[test]
    fn test_decide_exit_after_short_run_enters_backoff() {
        let t0 = Instant::now();
        let st = SvcState::Running { pid: 42, since: t0 };
        let mut h = RestartHistory::default();
        let d = decide(&st, &mut h, Event::Exited, t0 + Duration::from_secs(1), &policy());
        assert!(matches!(d.action, Action::Sleep(x) if x == Duration::from_secs(1)));
        assert!(matches!(d.next, SvcState::Backoff { attempt: 1, .. }));
    }

    #[test]
    fn test_decide_stable_run_resets_attempt_counter() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.record(t0);
        h.record(t0);
        let running = SvcState::Running { pid: 7, since: t0 };
        // Ran 61s, above backoff_max of 60s => considered stable.
        let d = decide(
            &running,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(61),
            &policy(),
        );
        assert!(
            matches!(d.next, SvcState::Backoff { attempt: 1, .. }),
            "a run longer than backoff_max resets the escalation"
        );
    }

    #[test]
    fn test_decide_run_just_under_max_does_not_reset() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        // 59s < backoff_max: must keep escalating, or a service dying every
        // 59s would reset forever and never hit the cap.
        let st = SvcState::Running { pid: 7, since: t0 };
        h.record(t0);
        h.record(t0);
        let d = decide(&st, &mut h, Event::Exited, t0 + Duration::from_secs(59), &policy());
        assert!(matches!(d.next, SvcState::Backoff { attempt: 3, .. }));
    }

    #[test]
    fn test_decide_crashloop_cap_triggers_reboot() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..9 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        let d = decide(&st, &mut h, Event::Exited, t0 + Duration::from_secs(10), &policy());
        assert!(matches!(d.action, Action::Reboot(_)), "10th restart in window must reboot");
    }

    #[test]
    fn test_decide_crashloop_not_triggered_one_restart_early() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..8 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        let d = decide(&st, &mut h, Event::Exited, t0 + Duration::from_secs(10), &policy());
        assert!(matches!(d.action, Action::Sleep(_)), "9th restart must not reboot");
    }

    #[test]
    fn test_decide_crashloop_ignores_restarts_outside_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..20 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        // 1h later: every recorded restart is outside the 600s window.
        let d = decide(&st, &mut h, Event::Exited, t0 + Duration::from_secs(3600), &policy());
        assert!(matches!(d.action, Action::Sleep(_)));
    }

    #[test]
    fn test_decide_backoff_expired_yields_start() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Backoff { until: t0, attempt: 2 };
        let d = decide(&st, &mut h, Event::Tick, t0 + Duration::from_secs(1), &policy());
        assert!(matches!(d.action, Action::Start));
    }

    #[test]
    fn test_decide_backoff_not_yet_expired_sleeps_remaining() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Backoff { until: t0 + Duration::from_secs(10), attempt: 3 };
        let d = decide(&st, &mut h, Event::Tick, t0 + Duration::from_secs(4), &policy());
        assert!(matches!(d.action, Action::Sleep(x) if x == Duration::from_secs(6)));
    }

    #[test]
    fn test_decide_tick_while_running_is_noop() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Running { pid: 3, since: t0 };
        let d = decide(&st, &mut h, Event::Tick, t0 + Duration::from_secs(5), &policy());
        assert!(matches!(d.action, Action::None));
    }
}
```

**Step 2: Run to verify failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu decide
```
Expected: FAIL — missing types.

**Step 3: Implement**

Add to `src/supervise.rs`:

```rust
use crate::sys::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvcState {
    Running { pid: Pid, since: Instant },
    Backoff { until: Instant, attempt: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Exited,
    Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Start,
    Sleep(Duration),
    Reboot(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub next: SvcState,
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    pub backoff_min: Duration,
    pub backoff_max: Duration,
    pub crashloop_count: u32,
    pub crashloop_window: Duration,
}

/// The supervisor's entire restart policy, as a total function.
///
/// `hist` is `&mut` only so that pruning and recording happen in one place;
/// the function has no other effects and performs no I/O.
pub fn decide(
    state: &SvcState,
    hist: &mut RestartHistory,
    ev: Event,
    now: Instant,
    p: &Policy,
) -> Decision {
    match (state, ev) {
        (SvcState::Running { since, .. }, Event::Exited) => {
            hist.prune(now, p.crashloop_window);
            hist.record(now);

            if hist.len() as u32 >= p.crashloop_count {
                return Decision {
                    action: Action::Reboot(format!(
                        "{} restarts within {}s",
                        hist.len(),
                        p.crashloop_window.as_secs()
                    )),
                    next: SvcState::Backoff { until: now, attempt: 0 },
                };
            }

            // A run longer than the backoff ceiling counts as stable, so the
            // escalation resets. The threshold sits *above* the ceiling on
            // purpose: at or below it, a service dying every `backoff_max - 1`
            // seconds would reset forever and never reach the cap.
            let ran = now.duration_since(*since);
            let attempt = if ran > p.backoff_max {
                hist.clear();
                1
            } else {
                hist.len() as u32
            };

            let delay = backoff_delay(attempt, p.backoff_min, p.backoff_max);
            Decision {
                action: Action::Sleep(delay),
                next: SvcState::Backoff { until: now + delay, attempt },
            }
        }

        (SvcState::Backoff { until, attempt }, _) => {
            if now >= *until {
                Decision {
                    action: Action::Start,
                    next: SvcState::Backoff { until: *until, attempt: *attempt },
                }
            } else {
                Decision {
                    action: Action::Sleep(until.duration_since(now)),
                    next: *state,
                }
            }
        }

        (SvcState::Running { .. }, Event::Tick) => {
            Decision { action: Action::None, next: *state }
        }
    }
}
```

**Step 4: Run to verify pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu decide
```
Expected: PASS, 9 tests.

If `test_decide_stable_run_resets_attempt_counter` or
`test_decide_run_just_under_max_does_not_reset` fails, the `>` vs `>=` on the
stability threshold is the culprit — re-read the comment in `decide`.

**Step 5: Lint, format, commit**

```bash
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/supervise.rs
git commit -m "feat(anyka-init): pure restart-policy state machine"
```

---

## Task 7: Reboot-storm guard

Reboot-on-cap without this is a camera power-cycling forever, unattended, with
no SSH window.

**Files:**
- Create: `cross-compile/anyka-init/src/storm.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs` (add `pub mod storm;`)

**Step 1: Write the failing tests**

Create `src/storm.rs` with only the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storm_state_parses_valid_json() {
        assert_eq!(StormState::parse(r#"{"fast_reboots":2}"#).fast_reboots, 2);
    }

    #[test]
    fn test_storm_state_corrupt_input_is_treated_as_zero() {
        // vfat plus a power cut mid-write is expected, not exceptional.
        // Worst case of guessing zero is three extra reboots.
        assert_eq!(StormState::parse("").fast_reboots, 0);
        assert_eq!(StormState::parse("\0\0\0\0").fast_reboots, 0);
        assert_eq!(StormState::parse(r#"{"fast_reboots":"two"}"#).fast_reboots, 0);
        assert_eq!(StormState::parse(r#"{"fast_reboots":999}"#).fast_reboots, 0);
    }

    #[test]
    fn test_storm_state_render_roundtrips() {
        let s = StormState { fast_reboots: 3 };
        assert_eq!(StormState::parse(&s.render()).fast_reboots, 3);
    }

    #[test]
    fn test_should_enter_safe_mode_at_threshold() {
        assert!(!should_enter_safe_mode(0, 3));
        assert!(!should_enter_safe_mode(2, 3));
        assert!(should_enter_safe_mode(3, 3));
        assert!(should_enter_safe_mode(4, 3));
    }
}
```

**Step 2: Run to verify failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu storm
```
Expected: FAIL.

**Step 3: Implement**

Prepend to `src/storm.rs`:

```rust
//! Reboot-storm guard.
//!
//! The restart policy reboots the camera when a service exceeds its crash-loop
//! cap. Unguarded, a permanently broken service turns that into an unattended
//! power-cycle loop with no window to log in. This bounds it: after
//! `max_reboots` consecutive fast reboots the supervisor enters safe mode —
//! telnet, logging and the monitor thread only, no camera services — and waits
//! for a human.
//!
//! State lives on a vfat SD card and will occasionally be torn by a power cut.
//! Anything unparseable is read as zero; the cost of guessing wrong is three
//! extra reboots, and the cost of failing closed would be a camera that never
//! starts.

const MAX_SANE_REBOOTS: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StormState {
    pub fast_reboots: u8,
}

impl StormState {
    /// Deliberately hand-rolled rather than pulling in serde_json for one
    /// integer. Any input that is not exactly what `render` produces reads
    /// as zero.
    pub fn parse(src: &str) -> Self {
        let Some(rest) = src.trim().strip_prefix(r#"{"fast_reboots":"#) else {
            return Self::default();
        };
        let Some(num) = rest.strip_suffix('}') else {
            return Self::default();
        };
        match num.trim().parse::<u8>() {
            Ok(n) if n <= MAX_SANE_REBOOTS => Self { fast_reboots: n },
            _ => Self::default(),
        }
    }

    pub fn render(&self) -> String {
        format!(r#"{{"fast_reboots":{}}}"#, self.fast_reboots)
    }

    /// Write via temp file + rename so a power cut leaves either the old
    /// contents or the new ones, never a half-written file.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = format!("{path}.tmp");
        std::fs::write(&tmp, self.render())?;
        std::fs::rename(&tmp, path)?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }

    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .map(|s| Self::parse(&s))
            .unwrap_or_default()
    }
}

pub fn should_enter_safe_mode(fast_reboots: u8, max_reboots: u8) -> bool {
    fast_reboots >= max_reboots
}
```

Add `pub mod storm;` to `src/lib.rs`.

**Step 4: Run to verify pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu storm
```
Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/storm.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): reboot-storm guard with safe mode"
```

---

## Task 8: SNTP response parsing

Security-relevant: this parses unauthenticated UDP from the network and the
result sets the system clock. Every rejection below has a test.

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod parse_tests {
    use super::*;

    const NONCE: u64 = 0xDEAD_BEEF_CAFE_F00D;
    /// 2026-08-01T00:00:00Z as NTP seconds (unix + 2_208_988_800).
    const GOOD_NTP_SECS: u64 = 1_785_542_400 + NTP_UNIX_OFFSET;

    fn bounds() -> Bounds {
        Bounds { min_unix: 1_767_225_600, max_unix: 2_524_608_000 }
    }

    /// A well-formed server reply: LI=0, VN=4, Mode=4, stratum 2.
    fn good_packet() -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = 0b00_100_100;
        p[1] = 2;
        p[24..32].copy_from_slice(&NONCE.to_be_bytes());
        p[40..48].copy_from_slice(&(GOOD_NTP_SECS << 32).to_be_bytes());
        p
    }

    #[test]
    fn test_parse_accepts_well_formed_reply() {
        let t = parse_response(&good_packet(), NONCE, &bounds()).expect("must accept");
        let unix = t
            .duration_since(std::time::UNIX_EPOCH)
            .expect("post-epoch")
            .as_secs();
        assert_eq!(unix, 1_785_542_400);
    }

    #[test]
    fn test_parse_rejects_non_server_mode() {
        let mut p = good_packet();
        p[0] = 0b00_100_011; // mode 3 = client
        assert_eq!(parse_response(&p, NONCE, &bounds()), Err(NtpError::BadMode(3)));
    }

    #[test]
    fn test_parse_rejects_leap_indicator_alarm() {
        let mut p = good_packet();
        p[0] |= 0b11_000_000; // LI = 3, server says it is unsynchronised
        assert_eq!(parse_response(&p, NONCE, &bounds()), Err(NtpError::Unsynchronised));
    }

    #[test]
    fn test_parse_rejects_kiss_of_death_stratum_zero() {
        let mut p = good_packet();
        p[1] = 0;
        assert_eq!(parse_response(&p, NONCE, &bounds()), Err(NtpError::BadStratum(0)));
    }

    #[test]
    fn test_parse_rejects_unsynchronised_stratum_16() {
        let mut p = good_packet();
        p[1] = 16;
        assert_eq!(parse_response(&p, NONCE, &bounds()), Err(NtpError::BadStratum(16)));
    }

    #[test]
    fn test_parse_rejects_nonce_mismatch() {
        // This is the anti-spoofing check. Without it any host that can guess
        // our query can set the camera's clock.
        let p = good_packet();
        assert_eq!(parse_response(&p, NONCE ^ 1, &bounds()), Err(NtpError::NonceMismatch));
    }

    #[test]
    fn test_parse_rejects_zero_transmit_timestamp() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&0u64.to_be_bytes());
        assert_eq!(parse_response(&p, NONCE, &bounds()), Err(NtpError::ZeroTimestamp));
    }

    #[test]
    fn test_parse_rejects_time_before_lower_bound() {
        let mut p = good_packet();
        // 2000-01-01, well before min_plausible.
        let secs = 946_684_800u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_time_after_upper_bound() {
        let mut p = good_packet();
        let secs = 4_000_000_000u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_pre_1900_timestamp_without_panicking() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&(1u64 << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }
}
```

**Step 2: Run to verify failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu parse_tests
```
Expected: FAIL.

**Step 3: Implement**

Prepend to `src/timesync.rs`:

```rust
//! SNTP client, replacing the fire-and-forget `ntpd -n -N -p <server> &` in
//! `gergehack.sh:357`.
//!
//! This is not cosmetic. `onvif-rust`'s `ws_security.rs:85` sets
//! `clock_skew_seconds: 300` and `:234-239` rejects any WS-UsernameToken
//! `Created` timestamp outside +/- 5 minutes of now. A camera at the epoch
//! rejects **every** authenticated ONVIF request.
//!
//! The response is unauthenticated UDP from the network and its content sets
//! the system clock, so `parse_response` validates aggressively.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch.
pub const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NtpError {
    #[error("not a server reply (mode {0})")]
    BadMode(u8),
    #[error("server reports itself unsynchronised (LI=3)")]
    Unsynchronised,
    #[error("unusable stratum {0}")]
    BadStratum(u8),
    #[error("originate timestamp does not echo our nonce")]
    NonceMismatch,
    #[error("transmit timestamp is zero")]
    ZeroTimestamp,
    #[error("implausible time: unix {0}")]
    Implausible(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_unix: u64,
    pub max_unix: u64,
}

fn be64(pkt: &[u8; 48], off: usize) -> u64 {
    // Offsets are compile-time constants inside a fixed-size array, so this
    // slice is always exactly 8 bytes.
    let mut b = [0u8; 8];
    b.copy_from_slice(&pkt[off..off + 8]);
    u64::from_be_bytes(b)
}

/// Build a client request. `nonce` goes in the transmit-timestamp field; a
/// conformant server echoes it verbatim into the originate field, which is how
/// `parse_response` rejects off-path spoofing.
pub fn build_request(nonce: u64) -> [u8; 48] {
    let mut p = [0u8; 48];
    p[0] = 0b00_100_011; // LI=0, VN=4, Mode=3 (client)
    p[40..48].copy_from_slice(&nonce.to_be_bytes());
    p
}

pub fn parse_response(
    pkt: &[u8; 48],
    sent_nonce: u64,
    bounds: &Bounds,
) -> Result<SystemTime, NtpError> {
    let li = pkt[0] >> 6;
    let mode = pkt[0] & 0b111;
    if mode != 4 {
        return Err(NtpError::BadMode(mode));
    }
    if li == 3 {
        return Err(NtpError::Unsynchronised);
    }
    let stratum = pkt[1];
    if stratum == 0 || stratum > 15 {
        return Err(NtpError::BadStratum(stratum));
    }
    if be64(pkt, 24) != sent_nonce {
        return Err(NtpError::NonceMismatch);
    }
    let transmit = be64(pkt, 40);
    if transmit == 0 {
        return Err(NtpError::ZeroTimestamp);
    }

    let secs_1900 = transmit >> 32;
    let unix = secs_1900
        .checked_sub(NTP_UNIX_OFFSET)
        .ok_or(NtpError::Implausible(0))?;
    if unix < bounds.min_unix || unix > bounds.max_unix {
        return Err(NtpError::Implausible(unix));
    }

    let frac_nanos = (((transmit & 0xFFFF_FFFF) * 1_000_000_000) >> 32) as u32;
    Ok(UNIX_EPOCH + Duration::new(unix, frac_nanos))
}
```

**Step 4: Run to verify pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu parse_tests
```
Expected: PASS, 10 tests.

**Step 5: Commit**

```bash
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/timesync.rs
git commit -m "feat(anyka-init): SNTP response parsing with spoof and sanity checks"
```

---

## Task 9: SNTP query and the time-sync phase

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs`

**Step 1: Add the network I/O**

```rust
use std::net::{ToSocketAddrs, UdpSocket};

/// Read a 64-bit nonce from `/dev/urandom`. Falls back to a monotonic-derived
/// value if unavailable — weaker, but the alternative is skipping the
/// anti-spoofing check entirely.
pub fn random_nonce() -> u64 {
    use std::io::Read;
    let mut buf = [0u8; 8];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        if f.read_exact(&mut buf).is_ok() {
            return u64::from_be_bytes(buf);
        }
    }
    std::time::Instant::now().elapsed().as_nanos() as u64 ^ 0x5DEE_CE66_Du64
}

pub fn query(server: &str, timeout: Duration, bounds: &Bounds) -> anyhow::Result<SystemTime> {
    let addr = format!("{server}:123")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no address for {server}"))?;

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(timeout))?;
    sock.set_write_timeout(Some(timeout))?;

    let nonce = random_nonce();
    sock.send_to(&build_request(nonce), addr)?;

    let mut buf = [0u8; 48];
    let (n, from) = sock.recv_from(&mut buf)?;
    if n != 48 {
        anyhow::bail!("short NTP reply: {n} bytes");
    }
    if from.ip() != addr.ip() {
        anyhow::bail!("reply from {} but queried {}", from.ip(), addr.ip());
    }
    Ok(parse_response(&buf, nonce, bounds)?)
}
```

**Step 2: Add the step-the-clock helper**

```rust
use crate::config::TimeCfg;
use crate::sys::Sys;

/// Query each configured server in turn; step the clock on the first success.
/// Returns the applied delta in seconds, or `None` if nothing was applied.
pub fn sync_once(sys: &dyn Sys, cfg: &TimeCfg) -> Option<i64> {
    let bounds = Bounds { min_unix: cfg.min_plausible_unix, max_unix: cfg.max_plausible_unix };
    for server in &cfg.servers {
        match query(server, Duration::from_secs(5), &bounds) {
            Ok(t) => {
                let before = sys.realtime();
                let delta = delta_secs(before, t);
                if delta.unsigned_abs() < cfg.step_threshold_sec {
                    tracing::debug!(server, delta, "clock already within threshold");
                    return Some(0);
                }
                match sys.set_realtime(t) {
                    Ok(()) => {
                        tracing::info!(server, delta_sec = delta, "stepped system clock");
                        return Some(delta);
                    }
                    Err(e) => tracing::error!(server, error = %e, "clock_settime failed"),
                }
            }
            Err(e) => tracing::warn!(server, error = %e, "NTP query failed"),
        }
    }
    None
}

fn delta_secs(from: SystemTime, to: SystemTime) -> i64 {
    match to.duration_since(from) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// P2.5: bounded best-effort first sync. Never blocks boot beyond
/// `first_sync_timeout_sec`.
pub fn first_sync(sys: &dyn Sys, cfg: &TimeCfg) -> bool {
    if !cfg.enabled {
        return false;
    }
    let deadline = sys.now() + Duration::from_secs(cfg.first_sync_timeout_sec);
    loop {
        if sync_once(sys, cfg).is_some() {
            return true;
        }
        if sys.now() >= deadline {
            tracing::warn!(
                timeout_sec = cfg.first_sync_timeout_sec,
                "no NTP sync before boot deadline; continuing with a wrong clock. \
                 Authenticated ONVIF requests will fail until the resync thread succeeds."
            );
            return false;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Background resync loop, started after P3.
pub fn resync_loop(sys: &dyn Sys, cfg: &TimeCfg) {
    loop {
        std::thread::sleep(Duration::from_secs(cfg.resync_interval_sec));
        sync_once(sys, cfg);
    }
}
```

**Step 3: Add a test for the delta helper**

```rust
#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn test_delta_secs_positive_when_target_is_later() {
        let a = UNIX_EPOCH + Duration::from_secs(1000);
        let b = UNIX_EPOCH + Duration::from_secs(1090);
        assert_eq!(delta_secs(a, b), 90);
    }

    #[test]
    fn test_delta_secs_negative_when_target_is_earlier() {
        let a = UNIX_EPOCH + Duration::from_secs(1090);
        let b = UNIX_EPOCH + Duration::from_secs(1000);
        assert_eq!(delta_secs(a, b), -90);
    }
}
```

**Step 4: Run tests, lint, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu timesync
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/timesync.rs
git commit -m "feat(anyka-init): SNTP query, bounded first sync, resync loop"
```

---

## Task 10: Logging setup and rotation

**Files:**
- Create: `cross-compile/anyka-init/src/logging.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_rotate_is_noop_below_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.log");
        std::fs::write(&p, b"small").expect("write");
        rotate_if_needed(p.to_str().expect("utf8"), 1000, 2).expect("rotate");
        assert!(p.exists());
        assert!(!dir.path().join("a.log.1").exists());
    }

    #[test]
    fn test_rotate_moves_oversized_file_to_dot_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.log");
        let mut f = std::fs::File::create(&p).expect("create");
        f.write_all(&vec![b'x'; 2048]).expect("fill");
        drop(f);
        rotate_if_needed(p.to_str().expect("utf8"), 1024, 2).expect("rotate");
        assert!(dir.path().join("a.log.1").exists());
        assert!(!p.exists(), "current log is moved aside, not copied");
    }

    #[test]
    fn test_rotate_discards_beyond_keep_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("a.log");
        for name in ["a.log.1", "a.log.2"] {
            std::fs::write(dir.path().join(name), b"old").expect("write");
        }
        let mut f = std::fs::File::create(&base).expect("create");
        f.write_all(&vec![b'x'; 2048]).expect("fill");
        drop(f);
        rotate_if_needed(base.to_str().expect("utf8"), 1024, 2).expect("rotate");
        assert!(dir.path().join("a.log.1").exists());
        assert!(dir.path().join("a.log.2").exists());
        assert!(!dir.path().join("a.log.3").exists(), "keep=2 must not create .3");
    }

    #[test]
    fn test_rotate_missing_file_is_ok() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("nope.log");
        assert!(rotate_if_needed(p.to_str().expect("utf8"), 1024, 2).is_ok());
    }
}
```

**Step 2: Run to verify failure, then implement**

```rust
//! Logging.
//!
//! Rotation is size-based and never time-based. `tracing-appender`'s
//! `Rotation::DAILY` names files from the wall clock, and P2.5 steps that clock
//! from the epoch to the real date mid-boot: the boot record would land in
//! `anyka-init.log.1970-01-01` with a discontinuity at every boundary.

use std::io::Write;

/// Rotate `path` if it exceeds `max_bytes`, keeping `keep` generations.
///
/// ponytail: service logs rotate only at start time. The supervisor holds the
/// child's fd, so renaming underneath a live child leaves it writing to the
/// renamed inode; correcting that needs the fd reopened and dup2'd, which is
/// only possible when the child is (re)started. Self-corrects for a
/// crash-looping service; a stable chatty one grows until its next restart.
/// Upgrade path: move service logs to syslog, or SIGSTOP/reopen/SIGCONT from
/// the monitor thread.
pub fn rotate_if_needed(path: &str, max_bytes: u64, keep: u8) -> std::io::Result<()> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(()); // nothing to rotate yet
    };
    if meta.len() <= max_bytes {
        return Ok(());
    }
    let oldest = format!("{path}.{keep}");
    let _ = std::fs::remove_file(&oldest);
    for n in (1..keep).rev() {
        let from = format!("{path}.{n}");
        let to = format!("{path}.{}", n + 1);
        let _ = std::fs::rename(&from, &to);
    }
    std::fs::rename(path, format!("{path}.1"))
}

/// Install the tracing subscriber writing to `<dir>/anyka-init.log`, with
/// ERROR-level events additionally reaching stderr (which `service.sh` leaves
/// attached to the boot console).
pub fn init(dir: &str, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = format!("{dir}/anyka-init.log");
    let file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;

    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::EnvFilter::try_new(level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(file))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .with_filter(tracing_subscriber::filter::LevelFilter::ERROR),
        )
        .init();
    Ok(())
}

/// Last-resort output when the subscriber is not up yet or the log is
/// unwritable. Goes to the boot console.
pub fn console(msg: &str) {
    let _ = writeln!(std::io::stderr(), "anyka-init: {msg}");
}
```

Add `pub mod logging;` to `src/lib.rs`.

**Step 3: Run tests, lint, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu logging
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/logging.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): size-based log rotation and tracing setup"
```

---

## Task 11: Wifi credential rewrite

Ports `gergehack.sh:53-120` (`input_wifi_creds`). The rewrite itself is a pure
string transform, so it is fully testable.

**Files:**
- Create: `cross-compile/anyka-init/src/boot.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod wifi_tests {
    use super::*;

    const SAMPLE: &str = "\
[wlan]
ssid = oldnet
password = oldpass
channel = 6
";

    #[test]
    fn test_rewrite_replaces_ssid_and_password() {
        let out = rewrite_wifi_cfg(SAMPLE, "newnet", "newpass");
        assert!(out.contains("ssid = newnet"));
        assert!(out.contains("password = newpass"));
        assert!(!out.contains("oldnet"));
        assert!(!out.contains("oldpass"));
    }

    #[test]
    fn test_rewrite_preserves_other_lines_and_order() {
        let out = rewrite_wifi_cfg(SAMPLE, "n", "p");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "[wlan]");
        assert_eq!(lines[3], "channel = 6");
    }

    #[test]
    fn test_rewrite_is_idempotent() {
        let once = rewrite_wifi_cfg(SAMPLE, "n", "p");
        let twice = rewrite_wifi_cfg(&once, "n", "p");
        assert_eq!(once, twice);
    }

    #[test]
    fn test_rewrite_ignores_keys_that_merely_contain_ssid() {
        // The shell original matched with `case "$line" in ssid*)`, which also
        // matched ssid_hidden and only missed bssid by luck of ordering.
        let src = "bssid = aa:bb\nssid = old\n";
        let out = rewrite_wifi_cfg(src, "new", "p");
        assert!(out.contains("bssid = aa:bb"), "must not clobber bssid");
        assert!(out.contains("ssid = new"));
    }

    #[test]
    fn test_needs_update_detects_matching_credentials() {
        let cur = rewrite_wifi_cfg(SAMPLE, "n", "p");
        assert!(!needs_wifi_update(&cur, "n", "p"));
        assert!(needs_wifi_update(&cur, "other", "p"));
        assert!(needs_wifi_update(&cur, "n", "other"));
    }
}
```

**Step 2: Implement**

```rust
//! P2 system setup: timezone, sensor module, wifi credentials, service kills.

/// Rewrite `ssid`/`password` in an `anyka_cfg.ini`, preserving every other
/// line verbatim. Matches only lines whose key is exactly `ssid` or `password`.
pub fn rewrite_wifi_cfg(src: &str, ssid: &str, password: &str) -> String {
    let mut out = String::with_capacity(src.len() + 32);
    for line in src.lines() {
        let key = line.split('=').next().unwrap_or("").trim();
        match key {
            "ssid" => out.push_str(&format!("ssid = {ssid}\n")),
            "password" => out.push_str(&format!("password = {password}\n")),
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

pub fn needs_wifi_update(src: &str, ssid: &str, password: &str) -> bool {
    let mut have_ssid = false;
    let mut have_pass = false;
    for line in src.lines() {
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let val = parts.next().unwrap_or("").trim();
        match key {
            "ssid" => have_ssid = val == ssid,
            "password" => have_pass = val == password,
            _ => {}
        }
    }
    !(have_ssid && have_pass)
}
```

**Step 3: Add the file-level apply, with backup and readback**

```rust
use crate::config::WifiCfg;

/// Apply credentials to the on-disk config, backing up first and restoring the
/// backup if the write or the readback fails. A camera that loses its wifi
/// config cannot be reached to fix it.
pub fn apply_wifi(cfg: &WifiCfg) -> anyhow::Result<bool> {
    let path = &cfg.config_file;
    let current = std::fs::read_to_string(path)?;
    if !needs_wifi_update(&current, &cfg.ssid, &cfg.password) {
        tracing::debug!("wifi credentials already current");
        return Ok(false);
    }

    std::fs::write(format!("{path}.old"), &current)?;

    let updated = rewrite_wifi_cfg(&current, &cfg.ssid, &cfg.password);
    if let Err(e) = std::fs::write(path, &updated) {
        tracing::error!(error = %e, "wifi config write failed; restoring backup");
        std::fs::write(path, &current)?;
        return Err(e.into());
    }

    let readback = std::fs::read_to_string(path)?;
    if needs_wifi_update(&readback, &cfg.ssid, &cfg.password) {
        tracing::error!("wifi config readback mismatch; restoring backup");
        std::fs::write(path, &current)?;
        anyhow::bail!("wifi config verification failed");
    }

    tracing::info!(ssid = %cfg.ssid, "wifi credentials updated");
    Ok(true)
}
```

Add `pub mod boot;` to `src/lib.rs`.

**Step 4: Run tests, lint, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu wifi
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/boot.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): wifi credential rewrite with backup and readback"
```

---

## Task 12: The rest of P2

**Files:**
- Modify: `cross-compile/anyka-init/src/boot.rs`

**Step 1: Add the phase function**

```rust
use crate::config::Config;
use crate::sys::Sys;

/// P2: system setup. Every step is best-effort — a camera with no sensor
/// module is still worth reaching over SSH to diagnose.
pub fn system_setup(sys: &dyn Sys, cfg: &Config) {
    // SAFETY: set_var is not thread-safe, and P2 runs before any thread is
    // started. Do not move this call after P3.
    unsafe { std::env::set_var("TZ", &cfg.time.timezone) };
    tracing::info!(tz = %cfg.time.timezone, "timezone set");

    if let Some(module) = &cfg.system.sensor_module {
        match sys.insmod(module) {
            Ok(()) => tracing::info!(module, "sensor module loaded"),
            Err(e) => tracing::error!(
                module,
                error = %e,
                "sensor module load failed; video will be unavailable"
            ),
        }
    }

    match apply_wifi(&cfg.wifi) {
        Ok(true) => tracing::info!("wifi config rewritten"),
        Ok(false) => {}
        Err(e) => tracing::error!(error = %e, "wifi config update failed"),
    }

    match sys.run_to_completion("/usr/sbin/wifi_manage.sh", &["start"]) {
        Ok(st) => tracing::info!(?st, "wifi_manage.sh start"),
        Err(e) => tracing::warn!(error = %e, "wifi_manage.sh failed"),
    }

    // The P0 wrapper started telnetd on port 24 before config was readable.
    // Only now can we honour the setting.
    if !cfg.system.telnet {
        let _ = sys.run_to_completion("killall", &["telnetd"]);
        tracing::info!("telnetd disabled per config");
    }
    if !cfg.system.ftp {
        // tcpsvd is the vendor's FTP server, started at rc.local:14.
        let _ = sys.run_to_completion("killall", &["tcpsvd"]);
        tracing::info!("ftp disabled per config");
    }
}
```

**Step 2: Build, lint, commit**

```bash
$CARGO build --target x86_64-unknown-linux-gnu
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/boot.rs
git commit -m "feat(anyka-init): P2 system setup phase"
```

---

## Task 13: System monitor thread

Ports `sys_monitor.sh` (261 lines) into a thread. Parsing `/proc` is pure and
therefore tested; the sampling loop is not.

**Files:**
- Create: `cross-compile/anyka-init/src/monitor.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str = "\
MemTotal:          36864 kB
MemFree:            4096 kB
MemAvailable:       8192 kB
Buffers:             512 kB
";

    #[test]
    fn test_parse_mem_available_preferred_over_free() {
        assert_eq!(parse_mem_kb(MEMINFO), Some(8192));
    }

    #[test]
    fn test_parse_mem_falls_back_to_free_when_available_absent() {
        let src = "MemTotal: 36864 kB\nMemFree: 4096 kB\n";
        assert_eq!(parse_mem_kb(src), Some(4096));
    }

    #[test]
    fn test_parse_mem_returns_none_on_garbage() {
        assert_eq!(parse_mem_kb(""), None);
        assert_eq!(parse_mem_kb("MemFree: not-a-number kB"), None);
    }

    #[test]
    fn test_parse_loadavg_takes_first_field() {
        assert_eq!(parse_loadavg("0.52 0.31 0.19 1/84 1234"), Some(0.52));
        assert_eq!(parse_loadavg(""), None);
    }
}
```

**Step 2: Implement**

```rust
//! System resource sampling, replacing `sys_monitor.sh`.

use crate::storm::StormState;
use crate::sys::Sys;
use std::time::Duration;

pub fn parse_mem_kb(meminfo: &str) -> Option<u64> {
    let field = |name: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
    };
    field("MemAvailable:").or_else(|| field("MemFree:"))
}

pub fn parse_loadavg(src: &str) -> Option<f32> {
    src.split_whitespace().next().and_then(|v| v.parse().ok())
}

fn sample() {
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| parse_mem_kb(&s));
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| parse_loadavg(&s));
    tracing::info!(mem_avail_kb = mem, load1 = load, "sys");
}

/// Sampling loop. Also owns the storm-guard reset: once this process has been
/// up longer than the configured threshold, the boot is considered good.
pub fn run(sys: &dyn Sys, interval: Duration, state_path: &str, reset_after: Duration) {
    let mut reset_done = false;
    loop {
        sample();
        if !reset_done && sys.uptime() > reset_after {
            match (StormState { fast_reboots: 0 }).save(state_path) {
                Ok(()) => tracing::info!("boot considered good; storm-guard counter reset"),
                Err(e) => tracing::warn!(error = %e, "failed to reset storm-guard state"),
            }
            reset_done = true;
        }
        std::thread::sleep(interval);
    }
}
```

Add `pub mod monitor;` to `src/lib.rs`.

**Step 3: Run tests, lint, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu monitor
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/monitor.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): system monitor thread with storm-guard reset"
```

---

## Task 14: The supervisor loop

Wires the threads and the channel. This is where `decide` gets applied.

**Files:**
- Create: `cross-compile/anyka-init/src/supervisor_loop.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs`

**Design constraints, in order of importance:**

1. `waitpid(-1)` blocks forever and cannot be woken by a backoff timer
   expiring. Put it in its own thread and turn child exits into channel
   messages, making `rx.recv_timeout(next_deadline)` the single wait point.
2. Every background thread uses
   `std::thread::Builder::new().stack_size(64 * 1024)` — four threads at
   Rust's 2 MiB default is pointless on a 36 MB device.
3. A failed start is *not* a special case. Feed it through `decide` as an
   `Event::Exited` so it takes the normal backoff path.

**Step 1: Implement**

```rust
//! The P3 + P4 supervision loop.

use crate::config::{Config, ServiceCfg};
use crate::logging;
use crate::storm::StormState;
use crate::supervise::{Action, Event, Policy, RestartHistory, SvcState, decide};
use crate::sys::{ExitStatus, Pid, SpawnSpec, Sys};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

pub enum Msg {
    Exited(Pid, ExitStatus),
    Shutdown,
}

struct Service {
    name: String,
    spec: SpawnSpec,
    state: SvcState,
    hist: RestartHistory,
}

fn spec_of(svc: &ServiceCfg) -> SpawnSpec {
    SpawnSpec {
        exec: svc.exec.clone(),
        args: svc.args.clone(),
        env: svc.env.clone(),
        log: svc.log.clone(),
        core_dump: svc.core_dump,
    }
}

pub fn make_channel() -> (Sender<Msg>, Receiver<Msg>) {
    channel()
}

pub fn spawn_reaper(sys: Arc<dyn Sys>, tx: Sender<Msg>) {
    std::thread::Builder::new()
        .name("reaper".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            loop {
                match sys.wait_any() {
                    Ok((pid, st)) => {
                        if tx.send(Msg::Exited(pid, st)).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        // ECHILD with no children yet is normal at startup.
                        tracing::debug!(error = %e, "wait_any");
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        })
        .ok();
}

pub fn spawn_signal_thread(tx: Sender<Msg>) {
    std::thread::Builder::new()
        .name("signals".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            use signal_hook::consts::{SIGINT, SIGTERM};
            let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGTERM, SIGINT]) else {
                tracing::error!("failed to install signal handler");
                return;
            };
            for _ in signals.forever() {
                let _ = tx.send(Msg::Shutdown);
            }
        })
        .ok();
}

pub fn run(sys: Arc<dyn Sys>, cfg: &Config, rx: Receiver<Msg>) {
    let policy = Policy {
        backoff_min: Duration::from_secs(cfg.supervisor.backoff_min_sec),
        backoff_max: Duration::from_secs(cfg.supervisor.backoff_max_sec),
        crashloop_count: cfg.supervisor.crashloop_count,
        crashloop_window: Duration::from_secs(cfg.supervisor.crashloop_window_sec),
    };

    let mut services: Vec<Service> = cfg
        .services
        .iter()
        .filter(|(_, s)| s.enabled)
        .map(|(name, s)| Service {
            name: name.clone(),
            spec: spec_of(s),
            state: SvcState::Backoff { until: sys.now(), attempt: 0 },
            hist: RestartHistory::default(),
        })
        .collect();

    let mut by_pid: BTreeMap<Pid, usize> = BTreeMap::new();

    loop {
        let mut next_deadline: Option<Instant> = None;

        for i in 0..services.len() {
            let now = sys.now();
            let d = decide(&services[i].state, &mut services[i].hist, Event::Tick, now, &policy);
            services[i].state = d.next;

            if matches!(d.action, Action::Start) {
                if let Err(e) = logging::rotate_if_needed(
                    &services[i].spec.log,
                    cfg.log.max_bytes,
                    cfg.log.keep,
                ) {
                    tracing::warn!(service = %services[i].name, error = %e, "log rotate failed");
                }
                match sys.spawn(&services[i].spec) {
                    Ok(pid) => {
                        tracing::info!(service = %services[i].name, pid, "started");
                        services[i].state = SvcState::Running { pid, since: sys.now() };
                        by_pid.insert(pid, i);
                    }
                    Err(e) => {
                        tracing::error!(service = %services[i].name, error = %e, "start failed");
                        let d = decide(
                            &SvcState::Running { pid: -1, since: sys.now() },
                            &mut services[i].hist,
                            Event::Exited,
                            sys.now(),
                            &policy,
                        );
                        services[i].state = d.next;
                        if let Action::Reboot(why) = d.action {
                            do_reboot(sys.as_ref(), cfg, &why);
                        }
                    }
                }
            }

            if let SvcState::Backoff { until, .. } = services[i].state {
                next_deadline = Some(match next_deadline {
                    Some(d) if d < until => d,
                    _ => until,
                });
            }
        }

        let timeout = next_deadline
            .map(|d| d.saturating_duration_since(sys.now()))
            .unwrap_or(Duration::from_secs(3600));

        match rx.recv_timeout(timeout) {
            Ok(Msg::Exited(pid, st)) => {
                let Some(i) = by_pid.remove(&pid) else {
                    tracing::debug!(pid, ?st, "reaped an unknown child");
                    continue;
                };
                tracing::warn!(service = %services[i].name, pid, ?st, "service exited");
                let now = sys.now();
                let d = decide(&services[i].state, &mut services[i].hist, Event::Exited, now, &policy);
                services[i].state = d.next;
                if let Action::Reboot(why) = d.action {
                    do_reboot(sys.as_ref(), cfg, &why);
                }
            }
            Ok(Msg::Shutdown) => {
                tracing::info!("shutdown requested");
                shutdown(sys.as_ref(), &by_pid);
                return;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                tracing::error!("event channel closed");
                return;
            }
        }
    }
}

fn do_reboot(sys: &dyn Sys, cfg: &Config, why: &str) {
    tracing::error!(reason = why, "crash-loop cap exceeded; rebooting");
    let mut st = StormState::load(&cfg.supervisor.storm_guard_state);
    st.fast_reboots = st.fast_reboots.saturating_add(1);
    if let Err(e) = st.save(&cfg.supervisor.storm_guard_state) {
        tracing::error!(error = %e, "failed to persist storm-guard state");
    }
    if let Err(e) = sys.reboot() {
        tracing::error!(error = %e, "reboot failed");
    }
}

fn shutdown(sys: &dyn Sys, by_pid: &BTreeMap<Pid, usize>) {
    for &pid in by_pid.keys() {
        let _ = sys.kill(pid, libc::SIGTERM);
    }
    std::thread::sleep(Duration::from_secs(5));
    for &pid in by_pid.keys() {
        let _ = sys.kill(pid, libc::SIGKILL);
    }
}
```

Add `pub mod supervisor_loop;` to `src/lib.rs`.

**Step 2: Build, lint, commit**

```bash
$CARGO build --target x86_64-unknown-linux-gnu
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): supervision loop with reaper and signal threads"
```

---

## Task 15: `main.rs` phase sequencing

**Files:**
- Modify: `cross-compile/anyka-init/src/main.rs`

**Step 1: Implement**

Sequence, in this exact order:

1. Install a panic hook that writes to both `tracing::error!` and
   `logging::console` — a panic orphans every child, so it must be visible.
2. **P1** `Config::load("/mnt/anyka_hack/anyka.toml")`. On error, call
   `logging::console` with the message and `park()` forever. Do **not** fall
   back to defaults: guessing `wifi_ssid` joins a wrong network and guessing
   `sensor_module` loads a wrong kernel module.
3. `logging::init(&cfg.log.dir, &cfg.log.level)`; log the version.
4. Build `let sysimpl: Arc<dyn sys::Sys> = Arc::new(sys::RealSys::new());`
5. **Storm guard** — `storm::StormState::load(...)` then
   `storm::should_enter_safe_mode(...)`. If safe mode, log an ERROR naming the
   reboot count and telling the operator to log in over telnet :24.
6. **P2** `boot::system_setup(sysimpl.as_ref(), &cfg)`.
7. **P2.5** `timesync::first_sync(sysimpl.as_ref(), &cfg.time)`.
8. Start the monitor thread (if `cfg.monitor.enabled`) and the timesync resync
   thread (if `cfg.time.enabled`), both with `stack_size(64 * 1024)`. Clone the
   config values each thread needs into the closure — do not hold a borrow.
   The resync thread body is `timesync::resync_loop(s.as_ref(), &tcfg)`.
9. `supervisor_loop::spawn_signal_thread(tx.clone())`.
10. **If safe mode: `park()` here.** Telnet, logging and the monitor stay up;
    no services start.
11. `supervisor_loop::spawn_reaper(Arc::clone(&sysimpl), tx)`.
12. **P3 + P4** `supervisor_loop::run(sysimpl, &cfg, rx)`.

`park()` is:

```rust
/// Block forever without spinning. The recovery telnet started by the P0
/// wrapper stays reachable.
fn park() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
```

**Step 2: Build for both targets**

```bash
$CARGO build --target x86_64-unknown-linux-gnu
$CARGO build --release --target arm-anykav200-crosstool-ng
ls -la target/arm-anykav200-crosstool-ng/release/anyka-init
```

**Step 3: Lint and commit**

```bash
$CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): phase sequencing entry point"
```

---

## Task 16: Host integration tests

**Files:**
- Create: `cross-compile/anyka-init/tests/supervision.rs`

**Step 1: Write the tests**

Three tests, each building a temp `anyka.toml` with `[time].enabled = false`
and `[monitor].enabled = false`, then running `supervisor_loop::run` on a
background thread and sending `Msg::Shutdown` to stop it:

1. `test_service_that_exits_immediately_is_restarted` — service `exec =
   "/bin/false"`, `backoff_min_sec = 1`. After ~4 s the log file must show
   more than one start. Assert on the count of `started` lines in
   `/mnt/logs`-equivalent temp output, not on timing.
2. `test_stable_service_is_not_restarted` — service `exec = "/bin/sleep"`,
   `args = ["300"]`. After ~3 s the child PID must be unchanged.
3. `test_env_is_cleared_for_children` — the important one. Set
   `ANYKA_TEST_LEAK` in the test process, declare
   `ANYKA_TEST_INJECTED = "present"` in the service's `env`, run
   `exec = "/usr/bin/env"`, then assert the captured log **contains**
   `ANYKA_TEST_INJECTED=present` and **does not contain** `ANYKA_TEST_LEAK`.
   This is the regression guard for the two-uClibc loader-poisoning bug.

**Step 2: Run**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
```
Expected: PASS, 3 tests.

`--test-threads=1` is mandatory: these tests all call `waitpid(-1)`, which
reaps *any* child in the process, so parallel tests would steal each other's
exits.

**Step 3: Commit**

```bash
git add cross-compile/anyka-init/tests/supervision.rs
git commit -m "test(anyka-init): host integration tests for supervision and env isolation"
```

---

## Task 17: The `Factory/config.sh` wrapper and deploy wiring

**Files:**
- Rewrite: `SD_card_contents/Factory/config.sh`
- Create: `SD_card_contents/anyka_hack/anyka.toml`
- Modify: `scripts/build_sd_contents.sh`

**Step 1: Replace `SD_card_contents/Factory/config.sh` entirely**

```sh
#!/bin/sh
# P0: the only phase that must never fail.
#
# Called by the vendor's /usr/sbin/service.sh:91 when /mnt/Factory exists.
# Deliberately shell, not the ELF: anyka-init.bin needs the bundled uClibc
# loader at /mnt/anyka_hack/lib/ld-uClibc.so.1, and if that is missing the
# kernel cannot even start it. Shell has no such dependency, so telnet comes up
# regardless.
#
# service.sh:85 runs `killall telnetd` immediately before calling us, killing
# the telnetd that rcS:8 started. Restarting it here restores the only remote
# recovery channel.

telnetd -p 24 -l /bin/sh 2>/dev/null &

BIN=/mnt/anyka_hack/anyka-init.bin

if [ ! -x "$BIN" ]; then
  echo "anyka-init: missing or non-executable $BIN" >&2
  exit 1
fi

exec "$BIN"
```

**Step 2: Create `SD_card_contents/anyka_hack/anyka.toml`**

Use the full schema from the design doc's "Config schema" section. Fill
`[wifi]` with placeholder values and a comment telling the user to edit them.

**Step 3: Teach `build_sd_contents.sh` to build and install anyka-init**

Read the existing `onvif-rust` build step first and match its style. The new
step must run:

```bash
"${ANYKA_TOOLCHAIN_BIN}/cargo" build --release \
  --target arm-anykav200-crosstool-ng \
  --manifest-path "${ANYKA_REPO_ROOT}/cross-compile/anyka-init/Cargo.toml"

install -m 0755 \
  "${ANYKA_REPO_ROOT}/cross-compile/anyka-init/target/arm-anykav200-crosstool-ng/release/anyka-init" \
  "${ANYKA_REPO_ROOT}/SD_card_contents/anyka_hack/anyka-init.bin"
```

**Step 4: Verify the assembled payload**

```bash
cd /home/kmk/dev/anyka-dev
./scripts/build_sd_contents.sh --skip-www
file SD_card_contents/anyka_hack/anyka-init.bin
```
Expected: `ELF 32-bit LSB executable, ARM, EABI5 ... interpreter /mnt/anyka_hack/lib/ld-uClibc.so.1`

**Step 5: Commit**

```bash
git add SD_card_contents/Factory/config.sh SD_card_contents/anyka_hack/anyka.toml scripts/build_sd_contents.sh
git commit -m "feat(sd): P0 shell wrapper, anyka.toml, and anyka-init build step"
```

---

## Task 18: Delete the shell boot system

Do this **last**. Until this task the old and new systems coexist harmlessly,
because nothing calls the old `gergehack.sh` once `config.sh` is rewritten.

**Files:**
- Delete: `gergehack.sh`, `common.sh`, `init_logs.sh`, `sys_monitor.sh`, `periodic_reboot.sh`, `gergesettings.txt` (all under `SD_card_contents/anyka_hack/`)
- Delete: `SD_card_contents/anyka_hack/vendor-daemon/run_vendor_daemon.sh`
- Delete: `SD_card_contents/anyka_hack/onvif/run_onvif_rust.sh`
- Delete: `SD_card_contents/anyka_hack/dropbear/start_dropbear.sh`
- Modify: `SD_card_contents/anyka_hack/verify_logs.sh`
- Modify: `SD_card_contents/anyka_hack/ffmpeg/app_restarter.sh`
- Modify: `SD_card_contents/anyka_hack/ffmpeg/wrap_mp4.sh`
- Modify: `SD_card_contents/anyka_hack/README.md`

**Step 1: Fix the three survivors before deleting their dependency**

Each sources `common.sh` for `log()`. Replace those source lines with the
self-contained fallback that `run_vendor_daemon.sh:53-59` already used:

```sh
log() {
  level="$1"
  shift
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [${level}] $*"
}
```

Also remove their `. /data/gergesettings.txt` lines — the supervisor now
injects their config as environment variables. Verify:

```bash
grep -n "common.sh\|init_logs.sh\|gergesettings" \
  SD_card_contents/anyka_hack/verify_logs.sh \
  SD_card_contents/anyka_hack/ffmpeg/app_restarter.sh \
  SD_card_contents/anyka_hack/ffmpeg/wrap_mp4.sh
```
Expected: no output.

**Step 2: Delete the nine files**

```bash
cd /home/kmk/dev/anyka-dev
git rm SD_card_contents/anyka_hack/gergehack.sh \
       SD_card_contents/anyka_hack/common.sh \
       SD_card_contents/anyka_hack/init_logs.sh \
       SD_card_contents/anyka_hack/sys_monitor.sh \
       SD_card_contents/anyka_hack/periodic_reboot.sh \
       SD_card_contents/anyka_hack/gergesettings.txt \
       SD_card_contents/anyka_hack/vendor-daemon/run_vendor_daemon.sh \
       SD_card_contents/anyka_hack/onvif/run_onvif_rust.sh \
       SD_card_contents/anyka_hack/dropbear/start_dropbear.sh
```

**Step 3: Verify nothing still references them**

```bash
grep -rn "gergehack\|gergesettings\|run_onvif_rust\|run_vendor_daemon\|start_dropbear\|sys_monitor\|periodic_reboot" \
  --include="*.sh" --include="*.md" --include="*.rs" \
  SD_card_contents/ scripts/ docs/ AGENTS.md CLAUDE.md
```
Every remaining hit must be either (a) in `docs/plans/` describing history, or
(b) in `docs/reference/hack-process.md` describing the upstream project. Fix
anything else.

**Step 4: Update `SD_card_contents/anyka_hack/README.md`**

Replace the `gergesettings.txt` and "Script version updates" sections with
documentation of `anyka.toml` and the fact that updating means swapping the SD
card. Delete the SSH-settings section — those keys are now
`[services.dropbear].args`.

**Step 5: Verify the SD payload still assembles**

```bash
./scripts/build_sd_contents.sh --skip-www
```

**Step 6: Commit**

```bash
git add -A SD_card_contents/
git commit -m "refactor(sd): delete the shell boot system, superseded by anyka-init

Removes 1,724 lines: gergehack.sh, common.sh, init_logs.sh, sys_monitor.sh,
periodic_reboot.sh, gergesettings.txt and the three run_*.sh launchers.
verify_logs.sh and the two ffmpeg wrappers get an inline log() so they no
longer depend on common.sh."
```

---

## Task 19: Hardware smoke checklist

**Files:**
- Create: `docs/reference/anyka-init-smoke-test.md`

**Step 1: Write the checklist**, each item with the exact command and exact
expected result:

1. **Boot** — SD in, power on. `telnet <ip> 24` gives a root shell.
2. **Config parsed** — `grep "anyka-init starting" /mnt/logs/anyka-init.log` returns a line.
3. **Services up** — `ps | grep -E "vendor-daemon|onvif-rust"` shows both.
4. **Env isolation** — `cat /proc/$(pidof vendor-daemon.bin)/environ | tr '\0' '\n'` shows *only* the keys declared in `anyka.toml`.
5. **Clock** — `date` shows the correct year, and `grep "stepped system clock" /mnt/logs/anyka-init.log` appears *before* the first `started` line for `onvif`.
6. **ONVIF auth** — an authenticated ONVIF request from a client succeeds. This is the check that would have failed with the old fire-and-forget `ntpd`.
7. **Restart** — `kill -9 $(pidof vendor-daemon.bin)`; within ~1 s a new PID appears and the log records `service exited` then `started`.
8. **Backoff** — point `[services.dropbear].exec` at a nonexistent path; the log shows 1s, 2s, 4s, 8s gaps.
9. **Bad config** — put a syntax error in `anyka.toml`; camera boots, telnet :24 works, an error appears on the UART console, no services run.
10. **Safe mode** — write `{"fast_reboots":3}` to `/mnt/anyka_hack/state/boot.json`, reboot; log shows `SAFE MODE`, no services start, telnet works.
11. **Clean fallback** — power off, remove SD, power on. The camera boots stock firmware with the vendor app running.

**Step 2: Run it on hardware and record real results in the same file.**

**Step 3: Commit**

```bash
git add docs/reference/anyka-init-smoke-test.md
git commit -m "docs: hardware smoke checklist for anyka-init"
```

---

## Task 20: Final verification

**Step 1: Full test suite**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init
$CARGO test --target x86_64-unknown-linux-gnu -- --test-threads=1
```
Expected: all tests pass. Record the count.

**Step 2: Lint and format across the workspace**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 3: Cross-build the whole SD payload**

```bash
cd /home/kmk/dev/anyka-dev
./scripts/build_sd_contents.sh
```

**Step 4: Confirm the Task 19 checklist has real recorded results**, not placeholders.

**Step 5: Request review**

REQUIRED SUB-SKILL: use `superpowers:requesting-code-review` before merging.

---

## Notes for the Implementer

- **Never substitute system `cargo`.** `source ./setenv.sh` first, every session. Clippy fails with `E0514` otherwise.
- **`--test-threads=1` for anything touching `waitpid`.** It reaps *any* child in the process, so parallel tests steal each other's exits.
- **Monotonic clocks only in policy code.** P2.5 steps the wall clock by decades. Any `SystemTime` arithmetic in backoff or window logic is a bug.
- **Clearing the child environment is not optional.** Two incompatible uClibc versions coexist on this device; an inherited `LD_LIBRARY_PATH` breaks every busybox applet a service starts. See the comment in `SD_card_contents/anyka_hack/onvif/onvif-rust`.
- **Do not "clean up" `[system].sensor_module`.** It looks like a duplicate of `camera.sh:37-38` and is not — read the comment on the field.
- **`Factory/config.sh` must keep that exact path and name:** `service.sh:91` hardcodes `/mnt/Factory/config.sh`.
