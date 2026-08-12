# Diagnostics View Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the entirely-mocked `DiagnosticsPage` with real device data served from two JSON endpoints.

**Architecture:** Two `GET` routes on the existing axum router (`/api/diagnostics`, `/api/logs`) sampled on demand — no background timer, so an unwatched page costs the camera nothing. `/proc` parsers take `&str` rather than paths so kernel formats are testable off-device. The frontend polls with TanStack Query, which pauses on a hidden tab by default.

**Tech Stack:** Rust (axum 0.8, serde, libc), React 19 + TanStack Query v5 + Vitest.

**Design doc:** `docs/plans/2026-08-11-diagnostics-view-design.md`

---

## Before You Start

```bash
cd /home/kmk/dev/anyka-dev
source setenv.sh          # exports $CARGO and fixes CARGO_HOME so vendored cargo-* wins
git checkout feat/diagnostics-view
```

Host-side verification commands used throughout:

```bash
cd cross-compile/onvif-rust
$CARGO test  --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check

cd cross-compile/www
pnpm test
pnpm type-check
pnpm lint
```

**Never** run bare `cargo` — the vendored toolchain at `toolchain/arm-anykav200-crosstool-ng/bin/cargo` is required, and `setenv.sh` wires it up.

### Real device fixtures

These were captured from `.198` on 2026-08-11 and are used verbatim as test fixtures. Do not invent different ones — the point is that they came off the actual kernel.

```text
/proc/stat    → cpu  4169306 0 2746808 33139786 9423 0 91014 0 0 0
/proc/uptime  → 421240.95 351066.01
/proc/meminfo → MemTotal: 36540 kB / MemFree: 2756 kB / Buffers: 4104 kB / Cached: 18356 kB
/proc/net/dev → wlan0: 21863020 136885 0 5684 0 0 0 0 1261807137 986538 0 0 0 0 0 0
```

Note this kernel has **no `MemAvailable`** — it predates Linux 3.14.

---

## Task 1: `/proc` parsers

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/mod.rs`
- Create: `cross-compile/onvif-rust/src/diagnostics/proc.rs`
- Modify: `cross-compile/onvif-rust/src/lib.rs` (add `pub mod diagnostics;`)

**Step 1: Write the failing tests**

Create `src/diagnostics/proc.rs` with only this test module at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from the AK3918 on .198, 2026-08-11.
    const STAT: &str = "cpu  4169306 0 2746808 33139786 9423 0 91014 0 0 0\ncpu0 1 2 3\n";
    const MEMINFO: &str = "MemTotal:          36540 kB\nMemFree:            2756 kB\n\
                           Buffers:            4104 kB\nCached:            18356 kB\n\
                           SwapCached:            0 kB\n";
    const NET_DEV: &str = "Inter-|   Receive                    |  Transmit\n\
         face |bytes    packets errs drop fifo frame compressed multicast|bytes\n\
         wlan0: 21863020  136885    0 5684    0     0          0         0 1261807137 986538 0 0 0 0 0 0\n\
          p2p0:       0       0    0    0    0     0          0         0        0       0 0 0 0 0 0 0\n";

    #[test]
    fn test_parse_stat_sums_busy_and_total() {
        let times = parse_stat(STAT).expect("aggregate cpu line parses");
        // total = 4169306 + 0 + 2746808 + 33139786 + 9423 + 0 + 91014 = 40156337
        assert_eq!(times.total, 40_156_337);
        // busy excludes idle (33139786) and iowait (9423)
        assert_eq!(times.busy, 7_007_128);
    }

    #[test]
    fn test_parse_stat_rejects_non_cpu_first_line() {
        assert!(parse_stat("intr 12345\n").is_none());
    }

    #[test]
    fn test_parse_meminfo_excludes_buffers_and_cache() {
        let mem = parse_meminfo(MEMINFO).expect("meminfo parses");
        assert_eq!(mem.total_kb, 36_540);
        // 36540 - 2756 - 4104 - 18356 = 11324. Reporting total-free would claim
        // 92% used at idle, because Linux gives free pages to buffers and cache.
        assert_eq!(mem.used_kb, 11_324);
    }

    #[test]
    fn test_parse_meminfo_ignores_swap_cached() {
        // "SwapCached" must not be mistaken for "Cached".
        let mem = parse_meminfo(MEMINFO).expect("meminfo parses");
        assert_eq!(mem.used_kb, 11_324);
    }

    #[test]
    fn test_parse_meminfo_missing_field_is_none() {
        assert!(parse_meminfo("MemTotal:  36540 kB\n").is_none());
    }

    #[test]
    fn test_parse_net_dev_reads_rx_and_tx() {
        let bytes = parse_net_dev(NET_DEV, "wlan0").expect("wlan0 present");
        assert_eq!(bytes.rx, 21_863_020);
        assert_eq!(bytes.tx, 1_261_807_137);
    }

    #[test]
    fn test_parse_net_dev_unknown_interface_is_none() {
        assert!(parse_net_dev(NET_DEV, "eth0").is_none());
    }

    #[test]
    fn test_parse_uptime_secs_truncates() {
        assert_eq!(parse_uptime_secs("421240.95 351066.01\n"), Some(421_240));
    }
}
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile/onvif-rust && $CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::proc
```
Expected: FAIL — `cannot find function parse_stat in this scope`.

**Step 3: Write the implementation**

Prepend to `src/diagnostics/proc.rs`:

```rust
//! Parsers for the `/proc` files backing the diagnostics endpoint.
//!
//! Every function takes `&str` rather than a path so the kernel's formats stay
//! testable on the host. The camera runs a kernel old enough to lack
//! `MemAvailable`, so these formats cannot be assumed to match a modern box.

/// Jiffy counters from the aggregate `cpu` line of `/proc/stat`.
///
/// These are cumulative since boot, so a single reading yields the since-boot
/// average (~17% here), not current load. Use [`cpu_percent`] against a prior
/// reading for a live figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    /// Jiffies spent doing anything other than idling.
    pub busy: u64,
    /// Jiffies across all states.
    pub total: u64,
}

/// Memory figures in kilobytes, cache-adjusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemInfo {
    pub total_kb: u64,
    pub used_kb: u64,
}

/// Cumulative interface byte counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetBytes {
    pub rx: u64,
    pub tx: u64,
}

/// Parse the aggregate `cpu` line of `/proc/stat`.
pub fn parse_stat(contents: &str) -> Option<CpuTimes> {
    let line = contents.lines().next()?;
    let mut fields = line.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values: Vec<u64> = fields.filter_map(|f| f.parse().ok()).collect();
    if values.len() < 5 {
        return None;
    }
    let total: u64 = values.iter().sum();
    // Fields 3 and 4 are idle and iowait; everything else is work.
    let idle = values[3] + values[4];
    Some(CpuTimes {
        busy: total.saturating_sub(idle),
        total,
    })
}

/// Parse `/proc/meminfo` into a cache-adjusted used figure.
///
/// `MemAvailable` does not exist on this kernel, so used is assembled as
/// `MemTotal - MemFree - Buffers - Cached`. `MemFree` alone reads 2756 kB of
/// 36540 on an idle camera, which would show a permanently alarming gauge.
pub fn parse_meminfo(contents: &str) -> Option<MemInfo> {
    let (mut total, mut free, mut buffers, mut cached) = (None, None, None, None);
    for line in contents.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = rest
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        match key {
            "MemTotal" => total = Some(value),
            "MemFree" => free = Some(value),
            "Buffers" => buffers = Some(value),
            // Exact match: "SwapCached" is a different number.
            "Cached" => cached = Some(value),
            _ => {}
        }
    }
    let total_kb = total?;
    let used_kb = total_kb
        .saturating_sub(free?)
        .saturating_sub(buffers?)
        .saturating_sub(cached?);
    Some(MemInfo { total_kb, used_kb })
}

/// Parse one interface's byte counters out of `/proc/net/dev`.
pub fn parse_net_dev(contents: &str, iface: &str) -> Option<NetBytes> {
    for line in contents.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() != iface {
            continue;
        }
        let values: Vec<u64> = rest.split_whitespace().filter_map(|f| f.parse().ok()).collect();
        // Receive bytes is the first column, transmit bytes the ninth.
        if values.len() < 9 {
            return None;
        }
        return Some(NetBytes {
            rx: values[0],
            tx: values[8],
        });
    }
    None
}

/// Parse whole seconds of system uptime from `/proc/uptime`.
pub fn parse_uptime_secs(contents: &str) -> Option<u64> {
    contents
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .map(|secs| secs as u64)
}
```

Create `src/diagnostics/mod.rs`:

```rust
//! On-demand device diagnostics served over JSON.
//!
//! Deliberately has no background sampler: everything is read when a request
//! arrives, so an unwatched page costs this single-core device nothing.

pub mod proc;
```

Add to `src/lib.rs` alongside the other `pub mod` lines:

```rust
pub mod diagnostics;
```

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::proc
```
Expected: PASS, 8 tests.

**Step 5: Commit**

```bash
rtk git add src/diagnostics src/lib.rs
rtk git commit -m "feat(diagnostics): parse /proc into cache-adjusted metrics"
```

---

## Task 2: Delta arithmetic

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/proc.rs`

**Step 1: Write the failing tests**

Add to the `tests` module:

```rust
#[test]
fn test_cpu_percent_from_delta() {
    let prev = CpuTimes { busy: 100, total: 1000 };
    let now = CpuTimes { busy: 150, total: 1200 };
    // 50 busy jiffies out of 200 elapsed = 25%
    let pct = cpu_percent(prev, now).expect("forward delta");
    assert!((pct - 25.0).abs() < 0.01, "got {pct}");
}

#[test]
fn test_cpu_percent_rejects_backwards_counter() {
    let prev = CpuTimes { busy: 150, total: 1200 };
    let now = CpuTimes { busy: 100, total: 1000 };
    // A counter that went backwards means a reset, not -25% CPU.
    assert!(cpu_percent(prev, now).is_none());
}

#[test]
fn test_cpu_percent_zero_elapsed_is_none() {
    let same = CpuTimes { busy: 100, total: 1000 };
    assert!(cpu_percent(same, same).is_none());
}

#[test]
fn test_bytes_per_sec_divides_by_elapsed() {
    let rate = bytes_per_sec(1000, 6000, Duration::from_secs(5)).expect("forward delta");
    assert_eq!(rate, 1000);
}

#[test]
fn test_bytes_per_sec_rejects_counter_reset() {
    // Interface down/up resets the counter; report nothing rather than a wild number.
    assert!(bytes_per_sec(6000, 1000, Duration::from_secs(5)).is_none());
}
```

Add `use std::time::Duration;` to the test module.

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::proc
```
Expected: FAIL — `cannot find function cpu_percent`.

**Step 3: Write the implementation**

Append to `src/diagnostics/proc.rs` (above the tests), and add `use std::time::Duration;` at the top:

```rust
/// Busy percentage between two `/proc/stat` readings.
///
/// Returns `None` when the counters did not move forward — that means a reset,
/// and a negative delta would render as a nonsense percentage.
pub fn cpu_percent(prev: CpuTimes, now: CpuTimes) -> Option<f32> {
    let total = now.total.checked_sub(prev.total)?;
    let busy = now.busy.checked_sub(prev.busy)?;
    if total == 0 {
        return None;
    }
    Some((busy as f32 / total as f32) * 100.0)
}

/// Throughput between two cumulative byte counters.
pub fn bytes_per_sec(prev: u64, now: u64, elapsed: Duration) -> Option<u64> {
    let delta = now.checked_sub(prev)?;
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return None;
    }
    Some((delta as f64 / secs) as u64)
}
```

**Step 4: Run tests**

Expected: PASS, 13 tests.

**Step 5: Commit**

```bash
rtk git add src/diagnostics/proc.rs
rtk git commit -m "feat(diagnostics): compute CPU and throughput from counter deltas"
```

---

## Task 3: Storage via `statvfs`

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/storage.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs`

**Step 1: Write the failing test**

Create `src/diagnostics/storage.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_usage_of_root_is_plausible() {
        // "/" always exists on any host running these tests.
        let usage = storage_usage("/").expect("root filesystem is stat-able");
        assert!(usage.total_kb > 0, "a mounted filesystem has capacity");
        assert!(usage.used_kb <= usage.total_kb, "used cannot exceed total");
    }

    #[test]
    fn test_storage_usage_missing_path_is_none() {
        assert!(storage_usage("/nonexistent-mount-point-for-tests").is_none());
    }
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::storage
```
Expected: FAIL — `cannot find function storage_usage`.

**Step 3: Write the implementation**

Prepend to `src/diagnostics/storage.rs`:

```rust
//! Filesystem capacity for the SD card mount.

use std::ffi::CString;

/// Used and total capacity of a mount, in kilobytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageUsage {
    pub total_kb: u64,
    pub used_kb: u64,
}

/// Stat a mount point for capacity.
///
/// Uses `statvfs` rather than shelling out to `df`: spawning a process to read
/// two numbers is not something to do every five seconds on a 199 BogoMIPS core.
///
/// Returns `None` if the path cannot be stat-ed, e.g. the SD card is absent.
pub fn storage_usage(mount: &str) -> Option<StorageUsage> {
    let path = CString::new(mount).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };

    // SAFETY: `path` is a valid NUL-terminated C string that outlives the call,
    // and `stats` is a live, correctly-sized, zero-initialised statvfs.
    let rc = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };
    if rc != 0 {
        return None;
    }

    // f_frsize is the fragment size, which is what the block counts are in.
    let block_kb = u64::from(stats.f_frsize as u32) / 1024;
    let total_kb = (stats.f_blocks as u64).checked_mul(block_kb)?;
    let free_kb = (stats.f_bfree as u64).checked_mul(block_kb)?;
    Some(StorageUsage {
        total_kb,
        used_kb: total_kb.saturating_sub(free_kb),
    })
}
```

Add to `src/diagnostics/mod.rs`:

```rust
pub mod storage;
```

**Step 4: Run tests**

Expected: PASS, 2 tests.

**Step 5: Commit**

```bash
rtk git add src/diagnostics
rtk git commit -m "feat(diagnostics): read SD capacity via statvfs"
```

---

## Task 4: Extract the health computation

`App::health()` (`src/app.rs:1662`) computes exactly what the endpoint needs, but it takes `&App`, which the router cannot reach. Extract the body into a free function so **both** callers route through one implementation — do not copy it.

**Files:**
- Modify: `cross-compile/onvif-rust/src/lifecycle/health.rs`
- Modify: `cross-compile/onvif-rust/src/app.rs:1662-1720`

**Step 1: Write the failing test**

Add to the `tests` module in `src/lifecycle/health.rs`:

```rust
#[test]
fn test_compute_health_flags_stalled_stream() {
    // 6 s of silence is past the 5 s threshold.
    let status = compute_health(Duration::from_secs(60), Some(Some(6_000)), &[]);
    assert_eq!(status.status, HealthState::Degraded);
    assert!(status.degraded_services.contains(&"stream_health".to_string()));
}

#[test]
fn test_compute_health_accepts_recent_frame() {
    let status = compute_health(Duration::from_secs(60), Some(Some(40)), &[]);
    assert_eq!(status.status, HealthState::Healthy);
}

#[test]
fn test_compute_health_flags_stream_with_no_frames_yet() {
    let status = compute_health(Duration::from_secs(60), Some(None), &[]);
    assert_eq!(status.status, HealthState::Degraded);
}

#[test]
fn test_compute_health_without_streaming_is_healthy() {
    let status = compute_health(Duration::from_secs(60), None, &[]);
    assert_eq!(status.status, HealthState::Healthy);
}

#[test]
fn test_compute_health_includes_startup_degraded_services() {
    let status = compute_health(Duration::from_secs(60), None, &["PTZ".to_string()]);
    assert_eq!(status.status, HealthState::Degraded);
    assert!(status.degraded_services.contains(&"PTZ".to_string()));
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib lifecycle::health
```
Expected: FAIL — `cannot find function compute_health`.

**Step 3: Write the implementation**

Add to `src/lifecycle/health.rs`:

```rust
/// Maximum silence from venc-read before `stream_health` is marked degraded.
pub const STREAM_HEALTH_SILENCE_SECS: u64 = 5;

/// Build a [`HealthStatus`] from already-gathered inputs.
///
/// Free-standing rather than a method on `App` so the HTTP diagnostics handler
/// and `App::health()` share one implementation instead of drifting apart.
///
/// `frame_age_ms` is `None` when streaming is disabled entirely, and
/// `Some(None)` when streaming is on but no frame has arrived yet.
pub fn compute_health(
    uptime: Duration,
    frame_age_ms: Option<Option<u64>>,
    degraded_services: &[String],
) -> HealthStatus {
    let mut status = HealthStatus::new(uptime);

    status.add_component("config", ComponentHealth::healthy("Configuration"));
    status.add_component("platform", ComponentHealth::healthy("Platform"));
    status.add_component("device", ComponentHealth::healthy("Device Service"));
    status.add_component("media", ComponentHealth::healthy("Media Service"));

    if let Some(age) = frame_age_ms {
        match age {
            Some(age_ms) if age_ms > STREAM_HEALTH_SILENCE_SECS * 1000 => {
                status.add_component(
                    "stream_health",
                    ComponentHealth::degraded(
                        "Stream Health",
                        format!("No frames for {}ms (venc-read likely stalled)", age_ms),
                    ),
                );
                status.mark_degraded("stream_health");
            }
            Some(_) => {
                status.add_component("stream_health", ComponentHealth::healthy("Stream Health"));
            }
            None => {
                status.add_component(
                    "stream_health",
                    ComponentHealth::degraded(
                        "Stream Health",
                        "Streaming enabled but no frames observed yet",
                    ),
                );
                status.mark_degraded("stream_health");
            }
        }
    }

    for service in degraded_services {
        status.mark_degraded(service);
        status.add_component(
            service.to_lowercase(),
            ComponentHealth::degraded(service, "Initialization failed"),
        );
    }

    status
}
```

Now replace the body of `App::health()` in `src/app.rs` (delete the old `STREAM_HEALTH_SILENCE_SECS` const and the inline logic) with:

```rust
    /// Get the current health status of the application.
    pub fn health(&self) -> HealthStatus {
        let frame_age = self.streaming_service.as_ref().map(|_| {
            self.app_state
                .as_ref()
                .and_then(|s| s.platform())
                .and_then(|p| p.stream_frame_age_ms())
        });
        crate::lifecycle::health::compute_health(
            self.started_at.elapsed(),
            frame_age,
            &self.degraded_services,
        )
    }
```

Update the `use` at `src/app.rs:26` to bring in `compute_health` if needed, and drop now-unused imports.

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```
Expected: PASS. The pre-existing `App::health()` tests at `app.rs:2042`, `2147` and `2161` must still pass — they are the regression guard proving the extraction was behaviour-preserving.

**Step 5: Commit**

```bash
rtk git add src/lifecycle/health.rs src/app.rs
rtk git commit -m "refactor(health): extract compute_health so HTTP can share it"
```

---

## Task 5: The diagnostics snapshot type

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/state.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs`

**Step 1: Write the failing tests**

Create `src/diagnostics/state.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_snapshot_has_no_rates() {
        let state = DiagnosticsState::new(None, Vec::new());
        let snap = state.snapshot();
        // Rates need two samples. Reporting 0% would be a lie, not a default.
        assert!(snap.cpu_percent.is_none());
        assert!(snap.network.is_none());
    }

    #[test]
    fn test_second_snapshot_produces_rates() {
        let state = DiagnosticsState::new(None, Vec::new());
        let _ = state.snapshot();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let snap = state.snapshot();
        assert!(snap.cpu_percent.is_some(), "a second sample yields a rate");
    }

    #[test]
    fn test_snapshot_reports_process_uptime() {
        let state = DiagnosticsState::new(None, Vec::new());
        let snap = state.snapshot();
        assert!(snap.uptime.system_s >= snap.uptime.process_s);
    }

    #[test]
    fn test_snapshot_includes_startup_degraded_services() {
        let state = DiagnosticsState::new(None, vec!["PTZ".to_string()]);
        let snap = state.snapshot();
        assert_eq!(snap.status, "degraded");
    }
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::state
```
Expected: FAIL — `cannot find type DiagnosticsState`.

**Step 3: Write the implementation**

Prepend to `src/diagnostics/state.rs`:

```rust
//! Snapshot assembly and the previous-sample state used for rate deltas.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::proc::{self, CpuTimes, NetBytes};
use super::storage;
use crate::lifecycle::health::compute_health;
use crate::platform::Platform;

/// Interface whose counters back the throughput figures.
const NET_IFACE: &str = "wlan0";
/// Mount reported as device storage. The rootfs is a full read-only squashfs
/// and would always read 100%; the SD card is the number anyone cares about.
const STORAGE_MOUNT: &str = "/mnt";

/// Raw counters from one poll, kept only to compute the next poll's rates.
#[derive(Debug, Clone, Copy)]
struct RawSample {
    at: Instant,
    cpu: Option<CpuTimes>,
    net: Option<NetBytes>,
}

#[derive(Debug, Serialize)]
pub struct Uptime {
    /// Seconds since this process started.
    pub process_s: u64,
    /// Seconds since the device booted. A process uptime far below this means
    /// the supervisor restarted onvif-rust.
    pub system_s: u64,
}

#[derive(Debug, Serialize)]
pub struct Memory {
    pub total_kb: u64,
    pub used_kb: u64,
}

#[derive(Debug, Serialize)]
pub struct Storage {
    pub total_kb: u64,
    pub used_kb: u64,
}

#[derive(Debug, Serialize)]
pub struct Network {
    pub rx_bps: u64,
    pub tx_bps: u64,
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

/// One diagnostics reading, serialised straight to the client.
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub status: String,
    pub uptime: Uptime,
    /// `None` on the first poll — a rate needs two samples.
    pub cpu_percent: Option<f32>,
    pub memory: Option<Memory>,
    pub storage: Option<Storage>,
    /// `None` on the first poll.
    pub network: Option<Network>,
    pub stream_frame_age_ms: Option<u64>,
    pub components: Vec<Component>,
    pub degraded_services: Vec<String>,
}

/// Holds everything the diagnostics endpoint needs, including the previous
/// sample used for rate deltas.
///
/// There is no background task here on purpose: sampling happens when a request
/// arrives, so nobody watching means zero cost on this single core.
pub struct DiagnosticsState {
    started_at: Instant,
    platform: Option<Arc<dyn Platform>>,
    degraded_services: Vec<String>,
    // ponytail: std Mutex around a Copy struct; the guard never spans an await
    // and the critical section is two field writes. Swap for atomics only if a
    // profile ever shows contention here.
    previous: Mutex<Option<RawSample>>,
}

impl DiagnosticsState {
    pub fn new(platform: Option<Arc<dyn Platform>>, degraded_services: Vec<String>) -> Self {
        Self {
            started_at: Instant::now(),
            platform,
            degraded_services,
            previous: Mutex::new(None),
        }
    }

    /// Read `/proc` and assemble a snapshot, folding in the previous sample.
    pub fn snapshot(&self) -> Snapshot {
        let cpu = read_file("/proc/stat").and_then(|s| proc::parse_stat(&s));
        let net = read_file("/proc/net/dev").and_then(|s| proc::parse_net_dev(&s, NET_IFACE));
        let memory = read_file("/proc/meminfo")
            .and_then(|s| proc::parse_meminfo(&s))
            .map(|m| Memory {
                total_kb: m.total_kb,
                used_kb: m.used_kb,
            });
        let system_s = read_file("/proc/uptime")
            .and_then(|s| proc::parse_uptime_secs(&s))
            .unwrap_or_default();

        let now = RawSample {
            at: Instant::now(),
            cpu,
            net,
        };

        // Take the previous sample and install the new one in one critical
        // section. Nothing awaits while the guard is held.
        let previous = {
            let mut guard = self.previous.lock().unwrap_or_else(|e| e.into_inner());
            guard.replace(now)
        };

        let (cpu_percent, network) = match previous {
            Some(prev) => {
                let elapsed = now.at.saturating_duration_since(prev.at);
                let cpu_percent = prev
                    .cpu
                    .zip(now.cpu)
                    .and_then(|(p, n)| proc::cpu_percent(p, n));
                let network = prev.net.zip(now.net).and_then(|(p, n)| {
                    Some(Network {
                        rx_bps: proc::bytes_per_sec(p.rx, n.rx, elapsed)?,
                        tx_bps: proc::bytes_per_sec(p.tx, n.tx, elapsed)?,
                    })
                });
                (cpu_percent, network)
            }
            None => (None, None),
        };

        let frame_age_ms = self.platform.as_ref().map(|p| p.stream_frame_age_ms());
        let health = compute_health(
            self.started_at.elapsed(),
            frame_age_ms,
            &self.degraded_services,
        );

        Snapshot {
            status: health.status.to_string(),
            uptime: Uptime {
                process_s: self.started_at.elapsed().as_secs(),
                system_s,
            },
            cpu_percent,
            memory,
            storage: storage::storage_usage(STORAGE_MOUNT).map(|s| Storage {
                total_kb: s.total_kb,
                used_kb: s.used_kb,
            }),
            network,
            stream_frame_age_ms: frame_age_ms.flatten(),
            components: health
                .components
                .values()
                .map(|c| Component {
                    name: c.name.clone(),
                    status: c.status.to_string(),
                    message: c.message.clone(),
                })
                .collect(),
            degraded_services: health.degraded_services.clone(),
        }
    }
}

/// Read a `/proc` file, treating any error as "metric unavailable".
///
/// A missing counter must degrade one field, never fail the whole request —
/// a diagnostics page that 500s when one metric is unreadable is useless
/// exactly when you need it.
fn read_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Silence an unused-import warning on hosts without the streaming platform.
const _: Option<Duration> = None;
```

Remove that last `const _` line if `Duration` ends up used; it is a placeholder to keep the example compiling and should not survive review.

Add to `src/diagnostics/mod.rs`:

```rust
pub mod state;
```

**Step 4: Run tests**

Expected: PASS, 4 tests. Note `test_snapshot_reports_process_uptime` passes on the host because `/proc/uptime` exists on Linux; if these tests are ever run on a non-Linux host it will need gating.

**Step 5: Commit**

```bash
rtk git add src/diagnostics
rtk git commit -m "feat(diagnostics): assemble on-demand snapshots with delta rates"
```

---

## Task 6: Log tailing with a source allowlist

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/logs.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs`

**Step 1: Write the failing tests**

Create `src/diagnostics/logs.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_log(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        file.write_all(contents.as_bytes()).expect("write");
        file.flush().expect("flush");
        file
    }

    #[test]
    fn test_tail_returns_whole_file_when_under_budget() {
        let file = temp_log("one\ntwo\nthree\n");
        let text = tail_bytes(file.path(), 4096).expect("tail");
        assert_eq!(text, "one\ntwo\nthree\n");
    }

    #[test]
    fn test_tail_drops_partial_first_line() {
        let file = temp_log("aaaaaaaaaa\nbbbb\ncccc\n");
        // A 12-byte budget lands mid-way through the first line.
        let text = tail_bytes(file.path(), 12).expect("tail");
        assert!(!text.contains('a'), "partial line must be dropped, got {text:?}");
        assert!(text.contains("bbbb"));
    }

    #[test]
    fn test_tail_missing_file_is_error() {
        assert!(tail_bytes(Path::new("/nonexistent-log-for-tests"), 4096).is_err());
    }

    #[test]
    fn test_log_source_paths_are_fixed() {
        // The client picks an enum variant; it never supplies a path.
        assert_eq!(LogSource::OnvifRust.path(), "/mnt/logs/onvif_rust.log");
        assert_eq!(LogSource::VendorDaemon.path(), "/mnt/logs/vendor_daemon.log");
    }

    #[test]
    fn test_filter_lines_by_level_keeps_matching_and_worse() {
        let text = "INFO started\nWARN slow\nERROR broke\nDEBUG noise\n";
        let kept = filter_lines(text, Some(LogLevel::Warn), 100);
        assert_eq!(kept, vec!["WARN slow", "ERROR broke"]);
    }

    #[test]
    fn test_filter_lines_caps_at_requested_count() {
        let text = "INFO a\nINFO b\nINFO c\n";
        let kept = filter_lines(text, None, 2);
        // The newest lines are the interesting ones.
        assert_eq!(kept, vec!["INFO b", "INFO c"]);
    }
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::logs
```
Expected: FAIL — `cannot find type LogSource`.

**Step 3: Write the implementation**

Prepend to `src/diagnostics/logs.rs`:

```rust
//! Tailing and filtering of the on-device log files.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Deserialize;

/// Default bytes read from the end of a log file.
///
/// `onvif_rust.log` was already 510 KB in August 2026 and only grows. Reading it
/// whole would allocate an uncomfortable fraction of this device's 35 MB.
pub const DEFAULT_TAIL_BYTES: u64 = 64 * 1024;

/// Maximum lines a client may request.
pub const MAX_LINES: usize = 1000;

/// The log files this endpoint will serve.
///
/// An enum mapped to fixed paths, not a client-supplied path: there is
/// deliberately no way for a request to name a file that is not listed here,
/// so the endpoint has no path-traversal surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    OnvifRust,
    VendorDaemon,
    AnykaInit,
    WpaSupplicant,
}

impl LogSource {
    pub fn path(self) -> &'static str {
        match self {
            LogSource::OnvifRust => "/mnt/logs/onvif_rust.log",
            LogSource::VendorDaemon => "/mnt/logs/vendor_daemon.log",
            LogSource::AnykaInit => "/mnt/logs/anyka-init.log",
            LogSource::WpaSupplicant => "/mnt/logs/wpa_supplicant.log",
        }
    }
}

/// Minimum severity to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Detect the level of a log line by its first recognised level token.
    fn of_line(line: &str) -> Option<Self> {
        for (token, level) in [
            ("ERROR", LogLevel::Error),
            ("WARN", LogLevel::Warn),
            ("INFO", LogLevel::Info),
            ("DEBUG", LogLevel::Debug),
            ("TRACE", LogLevel::Trace),
        ] {
            if line.contains(token) {
                return Some(level);
            }
        }
        None
    }
}

/// Read at most `budget` bytes from the end of a file.
pub fn tail_bytes(path: &Path, budget: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(budget);
    file.seek(SeekFrom::Start(start))?;

    let mut buf = Vec::with_capacity(budget.min(len) as usize);
    file.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();

    // Seeking to a byte offset lands mid-line; drop the fragment.
    if start > 0 {
        if let Some(newline) = text.find('\n') {
            return Ok(text[newline + 1..].to_string());
        }
    }
    Ok(text)
}

/// Keep the newest `limit` lines at or above `min_level`.
///
/// Filtering happens here rather than in the browser so the wire payload stays
/// small — the point of the level buttons is to send less, not to hide more.
pub fn filter_lines(text: &str, min_level: Option<LogLevel>, limit: usize) -> Vec<&str> {
    let matching = text.lines().filter(|line| {
        if line.trim().is_empty() {
            return false;
        }
        match min_level {
            // An unparseable line is kept: dropping unknown output would hide
            // panics and vendor-daemon messages that carry no level token.
            Some(min) => LogLevel::of_line(line).is_none_or(|level| level >= min),
            None => true,
        }
    });

    let kept: Vec<&str> = matching.collect();
    let start = kept.len().saturating_sub(limit.min(MAX_LINES));
    kept[start..].to_vec()
}
```

Add to `src/diagnostics/mod.rs`:

```rust
pub mod logs;
```

Note `test_filter_lines_by_level_keeps_matching_and_worse` expects `DEBUG noise` dropped — verify `LogLevel::of_line` finds `DEBUG` and `Debug < Warn`.

**Step 4: Run tests**

Expected: PASS, 6 tests.

**Step 5: Commit**

```bash
rtk git add src/diagnostics
rtk git commit -m "feat(diagnostics): tail allowlisted log files with level filtering"
```

---

## Task 7: Auth for non-SOAP routes

The existing credential check lives at `src/onvif/dispatcher/auth.rs:23` and is `pub(super)`. Widen it and reuse it. **Do not write a second credential check** — a parallel auth path is how bypasses appear.

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/dispatcher/auth.rs:23`
- Modify: `cross-compile/onvif-rust/src/onvif/dispatcher/mod.rs` (re-export)
- Create: `cross-compile/onvif-rust/src/diagnostics/http.rs`

**Step 1: Write the failing test**

Create `src/diagnostics/http.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn test_missing_authorization_is_rejected() {
        let headers = HeaderMap::new();
        assert!(extract_basic_credentials(&headers).is_none());
    }

    #[test]
    fn test_non_basic_scheme_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Bearer abc"));
        assert!(extract_basic_credentials(&headers).is_none());
    }

    #[test]
    fn test_basic_credentials_are_decoded() {
        let mut headers = HeaderMap::new();
        // "admin:secret" base64-encoded
        headers.insert("Authorization", HeaderValue::from_static("Basic YWRtaW46c2VjcmV0"));
        let (user, pass) = extract_basic_credentials(&headers).expect("decodes");
        assert_eq!(user, "admin");
        assert_eq!(pass, "secret");
    }

    #[test]
    fn test_malformed_base64_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Basic !!!not-base64!!!"));
        assert!(extract_basic_credentials(&headers).is_none());
    }
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics::http
```
Expected: FAIL — `cannot find function extract_basic_credentials`.

**Step 3: Write the implementation**

First read `src/onvif/dispatcher/auth.rs:23-92` to see exactly what `verify_basic_auth_self` takes and returns, then change its visibility:

```rust
pub(crate) fn verify_basic_auth_self(
```

and re-export from `src/onvif/dispatcher/mod.rs`:

```rust
pub(crate) use auth::verify_basic_auth_self;
```

Then write `src/diagnostics/http.rs`, reusing that function for the actual credential verification and only doing header decoding locally. If `verify_basic_auth_self` already does the header decode, call it directly and delete `extract_basic_credentials` along with its tests — **reuse beats the tests above.** Read the function first and take whichever path duplicates less.

Sketch of the handler layer:

```rust
//! HTTP surface for diagnostics: two GET routes behind Basic Auth.

use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use super::logs::{self, LogLevel, LogSource, DEFAULT_TAIL_BYTES, MAX_LINES};
use super::state::DiagnosticsState;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    source: LogSource,
    level: Option<LogLevel>,
    #[serde(default = "default_lines")]
    lines: usize,
}

fn default_lines() -> usize {
    200
}

/// `GET /api/diagnostics`
pub async fn handle_diagnostics(
    Extension(state): Extension<Arc<DiagnosticsState>>,
) -> Json<super::state::Snapshot> {
    // Sampling is blocking file I/O but only a few small reads; it is not worth
    // a spawn_blocking hop, which on this device costs a ~12 ms scheduler
    // quantum — far more than the reads themselves.
    Json(state.snapshot())
}

/// `GET /api/logs`
pub async fn handle_logs(Query(query): Query<LogQuery>) -> Response {
    let path = std::path::Path::new(query.source.path());
    match logs::tail_bytes(path, DEFAULT_TAIL_BYTES) {
        Ok(text) => {
            let lines = logs::filter_lines(&text, query.level, query.lines.min(MAX_LINES));
            Json(lines).into_response()
        }
        // A missing log file is a normal state (that service never ran), not a
        // server fault.
        Err(_) => (StatusCode::NOT_FOUND, "log source unavailable").into_response(),
    }
}
```

Auth middleware wraps both routes; logs additionally require Administrator. Follow the level names in `src/onvif/auth_requirements.rs`.

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 5: Commit**

```bash
rtk git add src/diagnostics src/onvif/dispatcher
rtk git commit -m "feat(diagnostics): add authed JSON handlers for metrics and logs"
```

---

## Task 8: Wire the routes into the router

`OnvifServerState` is constructed at **nine** sites (`server.rs:507` plus eight in tests). Do not add a field to it. Use the `axum::Extension` pattern already used for `memory_monitor` at `server.rs:593` — it touches none of those sites.

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs:183-202` (add `diagnostics` field to `OnvifServer`)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs:548-618` (`build_router`)
- Modify: `cross-compile/onvif-rust/src/app.rs:1103-1106`

**Step 1: Write the failing test**

Add to the tests module in `src/onvif/server.rs`:

```rust
#[tokio::test]
async fn test_diagnostics_route_requires_auth() {
    // Build a server with diagnostics attached, then request without credentials.
    // Expect 401, not 200 and not 404.
}

#[tokio::test]
async fn test_diagnostics_route_returns_json_when_authorized() {
    // Expect 200 and a body containing "uptime".
}

#[tokio::test]
async fn test_unknown_api_route_is_not_swallowed_by_static_fallback() {
    // /api/nonexistent must 404, not return index.html.
}
```

Model these on the existing router tests near `server.rs:884` — copy their setup rather than inventing new scaffolding.

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib onvif::server
```
Expected: FAIL — routes do not exist, request 404s.

**Step 3: Write the implementation**

Add the field to `OnvifServer` (`server.rs:183`):

```rust
    /// Optional diagnostics state. `None` disables the /api routes entirely.
    diagnostics: Option<Arc<crate::diagnostics::state::DiagnosticsState>>,
```

Initialise it to `None` in `with_app_state` and any other constructor, and add a builder method:

```rust
    /// Attach diagnostics state, enabling the `/api` routes.
    ///
    /// Builder-style rather than a constructor parameter so the eight existing
    /// test constructors stay untouched.
    #[must_use]
    pub fn with_diagnostics(
        mut self,
        diagnostics: Arc<crate::diagnostics::state::DiagnosticsState>,
    ) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }
```

In `build_router`, before the static-file block:

```rust
        // Diagnostics routes. Nested at /api so the static fallback cannot
        // swallow them, and added before ServeDir for the same reason.
        let app = match &self.diagnostics {
            Some(diagnostics) => app
                .nest(
                    "/api",
                    Router::new()
                        .route("/diagnostics", get(crate::diagnostics::http::handle_diagnostics))
                        .route("/logs", get(crate::diagnostics::http::handle_logs))
                        .layer(middleware::from_fn_with_state(
                            /* auth state */,
                            diagnostics_auth_middleware,
                        )),
                )
                .layer(axum::Extension(Arc::clone(diagnostics))),
            None => app,
        };
```

Import `axum::routing::get`.

At `src/app.rs:1103`, build the state and attach it:

```rust
        let diagnostics = Arc::new(crate::diagnostics::state::DiagnosticsState::new(
            app_state.platform().cloned(),
            degraded_services.clone(),
        ));

        let server = Arc::new(
            OnvifServer::with_app_state(server_config, app_state.clone())
                .map_err(|e| StartupError::Network(e.to_string()))?
                .with_diagnostics(Arc::clone(&diagnostics)),
        );
```

Confirm the exact accessor for `platform` on `AppState` before writing this — `app.rs:1676` uses `.platform()`.

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

**Step 5: Commit**

```bash
rtk git add src/onvif/server.rs src/app.rs
rtk git commit -m "feat(diagnostics): mount /api routes on the ONVIF router"
```

---

## Task 9: Verify on hardware before touching the UI

Do this now, not at the end. A backend that works on the host and not on the camera is the expensive thing to discover late.

**Steps:**

```bash
cd /home/kmk/dev/anyka-dev
source setenv.sh
cd cross-compile/onvif-rust && $CARGO build --release      # ARM target
cd /home/kmk/dev/anyka-dev && ./scripts/deploy_onvif.sh    # read it first for flags
```

Then, from the host:

```bash
curl -s -u admin:<password> http://192.168.2.198/api/diagnostics | jq
curl -s -u admin:<password> "http://192.168.2.198/api/logs?source=onvif_rust&level=warn&lines=50" | jq
curl -s -o /dev/null -w '%{http_code}\n' http://192.168.2.198/api/diagnostics   # expect 401
```

**Check specifically:**
- First call returns `"cpu_percent": null`; a second call a few seconds later returns a number.
- `memory.total_kb` is ~36540, **not** ~2000000. If it looks like gigabytes, the parser is wrong.
- `uptime.system_s` is far larger than `uptime.process_s`.
- `storage.total_kb` is ~31 GB worth (the SD), not the 1 MB rootfs.
- The unauthenticated call returns 401, not 200.

**Do not proceed to the frontend until all six hold.** Verify with the actual command output, not by assumption — see @superpowers:verification-before-completion.

---

## Task 10: Frontend service and types

**Files:**
- Modify: `cross-compile/www/src/services/api.ts` (export an authorized fetch)
- Create: `cross-compile/www/src/services/diagnosticsService.ts`
- Create: `cross-compile/www/src/services/diagnosticsService.test.ts`

**Step 1: Write the failing tests**

Create `diagnosticsService.test.ts` covering:

```
test_get_diagnostics_returns_parsed_snapshot
test_get_diagnostics_sends_authorization_header
test_get_diagnostics_dispatches_unauthorized_event_on_401
test_get_logs_encodes_source_and_level_params
test_get_logs_returns_empty_array_for_missing_source
```

Model the mocking on `deviceService.test.ts`.

**Step 2: Run to verify it fails**

```bash
cd cross-compile/www && pnpm test diagnosticsService
```

**Step 3: Write the implementation**

In `api.ts`, the auth injection and the 401 side effects (clearing `sessionStorage` and dispatching `auth:unauthorized`) currently live inside the POST-only `request` function. Extract them so the GET routes get the same behaviour — otherwise an expired session on a diagnostics poll silently fails instead of logging the user out:

```ts
/**
 * Fetch with the same auth injection and 401 handling as SOAP posts.
 *
 * The diagnostics endpoints are plain JSON GETs, but they must share the
 * session-expiry path: a poll that quietly 401s every 5 s would leave a dead
 * page with no sign-in prompt.
 */
export async function authorizedFetch(
  url: string,
  init: RequestInit = {},
  config: ApiRequestConfig = {},
): Promise<Response> {
  // ... header injection, timeout via raceSignals, 401 side effects
}
```

Refactor `request` to call it, so both routes share one implementation.

Then `diagnosticsService.ts`:

```ts
export interface Diagnostics {
  status: string;
  uptime: { process_s: number; system_s: number };
  cpu_percent: number | null;
  memory: { total_kb: number; used_kb: number } | null;
  storage: { total_kb: number; used_kb: number } | null;
  network: { rx_bps: number; tx_bps: number } | null;
  stream_frame_age_ms: number | null;
  components: Array<{ name: string; status: string; message: string | null }>;
  degraded_services: string[];
}

export type LogSource = 'onvif_rust' | 'vendor_daemon' | 'anyka_init' | 'wpa_supplicant';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

export async function getDiagnostics(signal?: AbortSignal): Promise<Diagnostics> { ... }
export async function getLogs(
  source: LogSource,
  level?: LogLevel,
  lines = 200,
): Promise<string[]> { ... }
```

**Step 4: Run tests** — `pnpm test diagnosticsService`, then `pnpm type-check`.

**Step 5: Commit**

```bash
rtk git add src/services
rtk git commit -m "feat(www): add diagnostics service over the JSON API"
```

---

## Task 11: Polling hook with client-side history

**Files:**
- Create: `cross-compile/www/src/hooks/useDiagnostics.ts`
- Create: `cross-compile/www/src/hooks/useDiagnostics.test.tsx`

**Step 1: Write the failing tests**

```
test_use_diagnostics_returns_snapshot
test_history_appends_one_point_per_update
test_history_caps_at_max_samples
test_history_does_not_duplicate_on_rerender
```

The last one matters: React re-renders for unrelated reasons, and appending on every render would corrupt the chart. Keying the append effect on TanStack Query's `dataUpdatedAt` makes duplicates impossible — assert it by re-rendering without changing data.

**Step 2: Run to verify it fails** — `pnpm test useDiagnostics`

**Step 3: Write the implementation**

```ts
/** 60 samples at a 5 s poll is 5 minutes of history. */
const MAX_SAMPLES = 60;
const POLL_MS = 5000;

export function useDiagnostics() {
  const query = useQuery({
    queryKey: ['diagnostics'],
    queryFn: ({ signal }) => getDiagnostics(signal),
    refetchInterval: POLL_MS,
    // refetchIntervalInBackground stays false (the default): a hidden tab must
    // cost this single-core device nothing.
  });

  const [history, setHistory] = useState<Point[]>([]);

  useEffect(() => {
    if (!query.data) return;
    setHistory((prev) => [...prev, toPoint(query.data)].slice(-MAX_SAMPLES));
    // Keyed on dataUpdatedAt so an unrelated re-render cannot append twice.
  }, [query.dataUpdatedAt]);

  return { ...query, history };
}
```

**Step 4: Run tests** — `pnpm test useDiagnostics`

**Step 5: Commit**

```bash
rtk git add src/hooks
rtk git commit -m "feat(www): poll diagnostics with client-side history"
```

---

## Task 12: Rewrite the stat cards

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:19-36` (delete both mock generators)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:108-145` (stat card row)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Step 1: Write the failing tests**

```
test_diagnostics_page_renders_real_cpu_percent
test_diagnostics_page_shows_dash_when_cpu_is_null
test_diagnostics_page_renders_memory_in_megabytes
test_diagnostics_page_renders_storage_card
test_diagnostics_page_has_no_temperature_card
test_diagnostics_page_flags_recent_restart
```

`test_diagnostics_page_has_no_temperature_card` is a real assertion, not a formality: it stops anyone reinstating a card for a sensor this hardware does not have.

**Step 2: Run to verify it fails** — `pnpm test DiagnosticsPage`

**Step 3: Write the implementation**

- Delete `generateData` and `generateNetworkData` entirely.
- Replace the Temperature card with Storage (`HardDrive` icon is taken; use `Database` or `Server` from lucide).
- Show `—` for any `null` metric. Never substitute `0`; a zero reads as a real measurement.
- Format memory in MB against the real 36 MB total, not GB.
- Uptime row renders both figures, plus a restart note:

```tsx
{/*
  Process uptime far below system uptime means the supervisor restarted
  onvif-rust — the signature of the dusk VI collapse and of the
  vendor-daemon restart pairing. One subtraction turns "it feels flaky"
  into "it restarted 40 minutes ago".
*/}
{data.uptime.system_s - data.uptime.process_s > RESTART_THRESHOLD_S && (
  <span data-testid="diagnostics-restart-note">
    Restarted {formatDuration(data.uptime.process_s)} ago
  </span>
)}
```

**Step 4: Run tests** — `pnpm test DiagnosticsPage && pnpm type-check && pnpm lint`

**Step 5: Commit**

```bash
rtk git add src/pages/DiagnosticsPage.tsx src/pages/DiagnosticsPage.test.tsx
rtk git commit -m "feat(www): drive diagnostics stat cards from real metrics"
```

---

## Task 13: Charts and stream metrics

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:147-301` (charts)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:356-406` (System Metrics card)

**Step 1: Write the failing tests**

```
test_charts_render_from_history
test_charts_show_empty_state_before_two_samples
test_stream_card_renders_frame_age
test_stream_card_flags_stalled_stream
```

**Step 2: Run to verify it fails** — `pnpm test DiagnosticsPage`

**Step 3: Write the implementation**

Feed the existing `Sparkline` from `history`. Its props are `data: Array<Record<string, number>>`, `series`, and optional `domain` — CPU and memory keep `domain={[0, 100]}`; network omits it so it scales to traffic.

Replace the hardcoded `00:00 / 00:15 / 00:30` axis labels with real relative times derived from history length, or drop them — a fake axis under a real chart is worse than no axis.

Replace the "System Metrics" card contents (storage/streams/dropped-frames/bitrate fiction) with `stream_frame_age_ms` and the health components list. **Do not add a dropped-frames counter** — it does not exist and would need new accounting in a callback that runs 25 times a second.

**Step 4: Run tests** — `pnpm test DiagnosticsPage && pnpm type-check`

**Step 5: Commit**

```bash
rtk git add src/pages/DiagnosticsPage.tsx src/pages/DiagnosticsPage.test.tsx
rtk git commit -m "feat(www): chart real history and stream health"
```

---

## Task 14: The log panel

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:408-628`

**Step 1: Write the failing tests**

```
test_log_panel_renders_fetched_lines
test_log_source_selector_switches_source
test_log_level_filter_refetches_with_level
test_log_export_downloads_loaded_lines
test_log_panel_shows_message_when_source_unavailable
```

**Step 2: Run to verify it fails** — `pnpm test DiagnosticsPage`

**Step 3: Write the implementation**

- Source dropdown over the four allowlisted sources. Use the existing shadcn `Select`; check `components/ui/` for what is already there before adding anything.
- The four level buttons (`All`/`Info`/`Warning`/`Error`) become real, driving the `level` query param so filtering happens server-side.
- Export becomes a `Blob` download of the currently loaded lines:

```ts
const blob = new Blob([lines.join('\n')], { type: 'text/plain' });
const url = URL.createObjectURL(blob);
// ... anchor click ...
URL.revokeObjectURL(url);
```

- Replace the hardcoded `<tbody>` rows. Log lines arrive as raw strings, so either render them monospace one per row, or parse timestamp/level/message with a regex — pick rendering raw first and only parse if it reads badly. YAGNI.

**Step 4: Run tests** — `pnpm test && pnpm type-check && pnpm lint`

**Step 5: Commit**

```bash
rtk git add src/pages/DiagnosticsPage.tsx src/pages/DiagnosticsPage.test.tsx
rtk git commit -m "feat(www): serve real logs with source and level filters"
```

---

## Task 15: Full verification

**Step 1: Everything green on the host**

```bash
cd cross-compile/onvif-rust
$CARGO test  --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check

cd ../www
pnpm test && pnpm type-check && pnpm lint
```

**Step 2: Build and deploy**

```bash
cd cross-compile/www && pnpm build     # also precompresses into SD_card_contents
cd /home/kmk/dev/anyka-dev && ./scripts/deploy_onvif.sh
```

**Step 3: Confirm in a browser at `http://192.168.2.198`**

- Open Diagnostics. CPU and network show `—` briefly, then real numbers.
- Charts fill in over the following minute.
- Memory reads in MB against ~36 MB, not GB.
- Switch log sources; each returns different content.
- Level filters change what comes back.
- Export downloads a file.
- Background the tab for a minute, then check `/mnt/logs/onvif_rust.log` — **there must be no diagnostics requests during that window.** This is the check that the polling actually pauses; it is the whole basis of the "costs nothing when unwatched" claim.

**Step 4: Request review**

Use @superpowers:requesting-code-review, and the project's `code-review` skill.

**Step 5: Open the PR**

```bash
rtk git push -u origin feat/diagnostics-view
rtk gh pr create --title "feat: wire the diagnostics view to real device data" --body "..."
```

---

## Notes for whoever executes this

- **Never run bare `cargo`.** `source setenv.sh` first; the vendored toolchain is mandatory and `CARGO_HOME` needs its fix-up or clippy dies with E0514.
- **Every metric is nullable.** The device can fail to produce any single one, and a diagnostics page that 500s because one counter was unreadable is useless precisely when it is needed. Render `—`, never `0`.
- **Do not add a background sampler.** It is the single design decision the whole approach rests on. If charts feeling empty on open becomes annoying, that is a UI problem, not a reason to poll a 199 BogoMIPS core forever.
- **Do not add temperature.** There is no sensor. `/sys/class/thermal` does not exist on this device.
- If `stream_frame_age_ms()` returns `None` on real hardware, check whether streaming is actually enabled before assuming the plumbing is broken.
