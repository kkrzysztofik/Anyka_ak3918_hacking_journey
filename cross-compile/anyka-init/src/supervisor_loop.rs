//! The P3 + P4 supervision loop.

use crate::config::{Config, ServiceCfg};
use crate::logging;
use crate::storm::StormState;
use crate::supervise::{Action, Event, Policy, RestartHistory, SvcState, decide};
use crate::sys::{ExitStatus, Pid, SpawnSpec, Sys};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Background thread stack. 64 KiB on the camera (four threads on a 36 MB
/// device); full default on host so integration tests can actually run.
pub fn thread_stack() -> usize {
    if cfg!(target_arch = "arm") {
        64 * 1024
    } else {
        2 * 1024 * 1024
    }
}

/// How long the reaper sleeps when no child has exited.
///
/// ponytail: this is a poll, not a blocking `waitpid`, purely so the thread can
/// observe the shutdown flag that host integration tests need. Upgrade path if
/// exit latency ever matters: block in `waitpid(-1, 0)` and wake it on shutdown
/// by sending the process SIGCHLD.
const REAP_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub enum Msg {
    Exited(Pid, ExitStatus),
    Shutdown,
    /// Recovery request from the monitor thread. The supervisor kills the named
    /// service; the normal exit path then restarts it under the usual backoff.
    RestartService(String),
    /// Escalation from the monitor when `RestartService` did not take. Sends
    /// SIGKILL. A task wedged in D state will not die even from this — the
    /// monitor's next rung is a reboot, which does not need the process to die.
    KillService(String),
}

struct Service {
    name: String,
    spec: SpawnSpec,
    state: SvcState,
    hist: RestartHistory,
}

/// Rewrite a `SpawnSpec` so exec and slot-owned env paths resolve inside the
/// active slot. `root` is `[update] root`; paths outside it pass through.
fn spec_of_slot(svc: &ServiceCfg, root: &Path, slots: &crate::update::Slots) -> SpawnSpec {
    // Where this supervisor was actually loaded from, not where `active`
    // claims. When `config.sh` falls back to the other slot it does not
    // rewrite the pointer, and resolving services against a stale pointer
    // would spawn them out of the slot that just failed to exec.
    let active = slots.running_slot();
    let rewrite = |p: &str| {
        crate::update::slot_path(root, active, Path::new(p))
            .to_string_lossy()
            .into_owned()
    };
    SpawnSpec {
        exec: rewrite(&svc.exec),
        args: svc.args.clone(),
        env: svc
            .env
            .iter()
            .map(|(k, v)| (k.clone(), rewrite_env(k, v, &rewrite)))
            .collect(),
        log: svc.log.clone(),
        core_dump: svc.core_dump,
    }
}

/// Rewrite one env value. `LD_LIBRARY_PATH` is a `:`-separated path list, so
/// each entry must be rewritten individually — rewriting the whole string as a
/// single path would leave the first entry pointing at the old slot's libs.
fn rewrite_env(key: &str, value: &str, rewrite: &impl Fn(&str) -> String) -> String {
    if key == "LD_LIBRARY_PATH" {
        let entries: Vec<String> = value.split(':').map(rewrite).collect();
        entries.join(":")
    } else {
        rewrite(value)
    }
}

pub fn make_channel() -> (Sender<Msg>, Receiver<Msg>) {
    channel()
}

pub fn spawn_reaper(
    sys: Arc<dyn Sys>,
    tx: Sender<Msg>,
    stop: Arc<AtomicBool>,
) -> Option<std::thread::JoinHandle<()>> {
    match std::thread::Builder::new()
        .name("reaper".into())
        .stack_size(thread_stack())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match sys.wait_any() {
                    Ok(Some((pid, st))) => {
                        if tx.send(Msg::Exited(pid, st)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {
                        // 1s, not 50ms. This poll exists only so the reaper can
                        // observe `stop`; nothing needs sub-second exit latency
                        // because backoff_min is 1s anyway. At 50ms this thread
                        // woke 20x/sec forever on a single core that also
                        // encodes and streams video.
                        std::thread::sleep(REAP_POLL_INTERVAL);
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "wait_any");
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        }) {
        Ok(handle) => Some(handle),
        Err(e) => {
            tracing::error!(
                error = %e,
                "failed to start the reaper thread; service exits will not be observed"
            );
            None
        }
    }
}

pub fn spawn_signal_thread(tx: Sender<Msg>) {
    let spawned = std::thread::Builder::new()
        .name("signals".into())
        .stack_size(thread_stack())
        .spawn(move || {
            use signal_hook::consts::{SIGINT, SIGTERM};
            let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGTERM, SIGINT]) else {
                tracing::error!("failed to install signal handler");
                return;
            };
            for _ in signals.forever() {
                let _ = tx.send(Msg::Shutdown);
            }
        });
    if let Err(e) = spawned {
        tracing::error!(
            error = %e,
            "failed to start the signal thread; SIGTERM/SIGINT will not shut down cleanly"
        );
    }
}

fn build_enabled_services(
    sys: &dyn Sys,
    cfg: &Config,
    update_root: &Path,
    slots: &crate::update::Slots,
) -> Vec<Service> {
    cfg.services
        .iter()
        .filter(|(_, s)| s.enabled)
        .map(|(name, s)| Service {
            name: name.clone(),
            spec: spec_of_slot(s, update_root, slots),
            state: SvcState::Backoff {
                until: sys.now(),
                attempt: 0,
            },
            hist: RestartHistory::default(),
        })
        .collect()
}

fn try_start_service(
    sys: &dyn Sys,
    cfg: &Config,
    svc: &mut Service,
    svc_idx: usize,
    by_pid: &mut BTreeMap<Pid, usize>,
    policy: &Policy,
) {
    if let Err(e) = logging::rotate_if_needed(&svc.spec.log, cfg.log.max_bytes, cfg.log.keep) {
        tracing::warn!(service = %svc.name, error = %e, "log rotate failed");
    }
    match sys.spawn(&svc.spec) {
        Ok(pid) => {
            tracing::info!(service = %svc.name, pid, "started");
            svc.state = SvcState::Running {
                pid,
                since: sys.now(),
            };
            by_pid.insert(pid, svc_idx);
        }
        Err(e) => {
            tracing::error!(service = %svc.name, error = %e, "start failed");
            let d = decide(
                &SvcState::Running {
                    pid: -1,
                    since: sys.now(),
                },
                &mut svc.hist,
                Event::Exited,
                sys.now(),
                policy,
            );
            svc.state = d.next;
            if let Action::Reboot(why) = d.action
                && !do_reboot(sys, cfg, &why)
            {
                apply_failed_reboot_backoff(svc, sys.now(), policy.backoff_max);
            }
        }
    }
}

fn tick_services(
    sys: &dyn Sys,
    cfg: &Config,
    services: &mut [Service],
    by_pid: &mut BTreeMap<Pid, usize>,
    policy: &Policy,
) -> Option<Instant> {
    let mut next_deadline: Option<Instant> = None;

    // Index loop: by_pid stores service indices; Start failure path also
    // needs random-access mutation of hist/state by index.
    #[allow(clippy::needless_range_loop)]
    for i in 0..services.len() {
        let now = sys.now();
        let state = services[i].state;
        let d = decide(&state, &mut services[i].hist, Event::Tick, now, policy);
        services[i].state = d.next;

        if matches!(d.action, Action::Start) {
            try_start_service(sys, cfg, &mut services[i], i, by_pid, policy);
        }

        if let SvcState::Backoff { until, .. } = services[i].state {
            next_deadline = Some(match next_deadline {
                Some(d) if d < until => d,
                _ => until,
            });
        }
    }

    next_deadline
}

fn handle_service_exited(
    sys: &dyn Sys,
    cfg: &Config,
    services: &mut [Service],
    by_pid: &mut BTreeMap<Pid, usize>,
    policy: &Policy,
    pid: Pid,
    st: ExitStatus,
) {
    let Some(i) = by_pid.remove(&pid) else {
        tracing::debug!(pid, ?st, "reaped an unknown child");
        return;
    };
    tracing::warn!(service = %services[i].name, pid, ?st, "service exited");
    let now = sys.now();
    let state = services[i].state;
    let d = decide(&state, &mut services[i].hist, Event::Exited, now, policy);
    services[i].state = d.next;
    if let Action::Reboot(why) = d.action
        && !do_reboot(sys, cfg, &why)
    {
        apply_failed_reboot_backoff(&mut services[i], sys.now(), policy.backoff_max);
    }
}

fn handle_restart_service(sys: &dyn Sys, services: &[Service], name: String) {
    match services.iter().find(|s| s.name == name) {
        Some(svc) => match svc.state.pid() {
            Some(pid) => {
                tracing::warn!(service = %name, pid, "restart requested by monitor");
                let _ = sys.kill(pid, libc::SIGTERM);
            }
            None => tracing::info!(
                service = %name,
                "restart requested but the service is not running"
            ),
        },
        None => {
            tracing::warn!(service = %name, "restart requested for unknown service")
        }
    }
}

fn handle_kill_service(sys: &dyn Sys, services: &[Service], name: String) {
    match services.iter().find(|s| s.name == name) {
        Some(svc) => match svc.state.pid() {
            Some(pid) => {
                tracing::warn!(service = %name, pid, "SIGTERM did not take; sending SIGKILL");
                let _ = sys.kill(pid, libc::SIGKILL);
            }
            None => tracing::info!(
                service = %name,
                "kill requested but the service is not running"
            ),
        },
        None => tracing::warn!(service = %name, "kill requested for unknown service"),
    }
}

/// Returns `true` when the supervisor loop should exit.
fn dispatch_msg(
    sys: &Arc<dyn Sys>,
    cfg: &Config,
    services: &mut [Service],
    by_pid: &mut BTreeMap<Pid, usize>,
    policy: &Policy,
    rx: &Receiver<Msg>,
    msg: Result<Msg, std::sync::mpsc::RecvTimeoutError>,
) -> bool {
    match msg {
        Ok(Msg::Exited(pid, st)) => {
            handle_service_exited(sys.as_ref(), cfg, services, by_pid, policy, pid, st);
            false
        }
        Ok(Msg::RestartService(name)) => {
            handle_restart_service(sys.as_ref(), services, name);
            false
        }
        Ok(Msg::KillService(name)) => {
            handle_kill_service(sys.as_ref(), services, name);
            false
        }
        Ok(Msg::Shutdown) => {
            tracing::info!("shutdown requested");
            shutdown(sys.as_ref(), by_pid, rx);
            true
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            tracing::error!("event channel closed");
            true
        }
    }
}

pub fn run(sys: Arc<dyn Sys>, cfg: &Config, rx: Receiver<Msg>) {
    let policy = Policy {
        backoff_min: Duration::from_secs(cfg.supervisor.backoff_min_sec),
        backoff_max: Duration::from_secs(cfg.supervisor.backoff_max_sec),
        crashloop_count: cfg.supervisor.crashloop_count,
        crashloop_window: Duration::from_secs(cfg.supervisor.crashloop_window_sec),
    };

    let slots = crate::update::Slots::new(&cfg.update.root);
    let update_root = Path::new(&cfg.update.root);

    let mut services = build_enabled_services(sys.as_ref(), cfg, update_root, &slots);
    let mut by_pid: BTreeMap<Pid, usize> = BTreeMap::new();

    loop {
        let next_deadline = tick_services(sys.as_ref(), cfg, &mut services, &mut by_pid, &policy);
        let timeout = next_deadline
            .map(|d| d.saturating_duration_since(sys.now()))
            .unwrap_or(Duration::from_secs(3600));

        if dispatch_msg(
            &sys,
            cfg,
            &mut services,
            &mut by_pid,
            &policy,
            &rx,
            rx.recv_timeout(timeout),
        ) {
            return;
        }
    }
}

/// Delay before a scheduled reboot: the configured interval plus up to
/// `jitter_max_sec`.
///
/// Jitter exists so that a fleet of cameras flashed from the same SD image does
/// not reboot in lockstep and brown out the recorder they all stream to. Pure
/// so the clamp is testable without waiting hours.
pub fn periodic_reboot_delay(interval_min: u64, jitter_max_sec: u64, entropy: u64) -> Duration {
    let base = interval_min.saturating_mul(60);
    let jitter = if jitter_max_sec == 0 {
        0
    } else {
        // saturating: `jitter_max_sec + 1` overflows at u64::MAX, and the
        // modulus must never be zero.
        entropy % jitter_max_sec.saturating_add(1)
    };
    Duration::from_secs(base.saturating_add(jitter))
}

/// Replaces `periodic_reboot.sh`. Only started when `[reboot].enabled` is true.
///
/// Deliberately does NOT touch the storm-guard counter: this is a scheduled
/// reboot, not a crash-loop one, and inflating that counter would push a
/// healthy camera into safe mode after three uneventful cycles.
pub fn periodic_reboot_loop(sys: &dyn Sys, interval_min: u64, jitter_max_sec: u64) {
    let delay = periodic_reboot_delay(
        interval_min,
        jitter_max_sec,
        crate::timesync::random_nonce(),
    );
    tracing::info!(delay_sec = delay.as_secs(), "periodic reboot scheduled");
    sys.sleep(delay);
    tracing::warn!("periodic reboot interval elapsed; rebooting");
    if let Err(e) = sys.reboot() {
        tracing::error!(error = %e, "periodic reboot failed");
    }
}

/// Returns `false` when `reboot()` fails so the caller can clear history and
/// apply a bounded backoff instead of spinning and rewriting flash.
fn do_reboot(sys: &dyn Sys, cfg: &Config, why: &str) -> bool {
    tracing::error!(reason = why, "crash-loop cap exceeded; rebooting");
    let mut st = StormState::load(&cfg.supervisor.storm_guard_state);
    st.fast_reboots = st.fast_reboots.saturating_add(1);
    if let Err(e) = st.save(&cfg.supervisor.storm_guard_state) {
        tracing::error!(error = %e, "failed to persist storm-guard state");
    }
    if let Err(e) = sys.reboot() {
        tracing::error!(error = %e, "reboot failed");
        return false;
    }
    true
}

fn apply_failed_reboot_backoff(svc: &mut Service, now: Instant, backoff_max: Duration) {
    svc.hist.clear();
    svc.state = SvcState::Backoff {
        until: now + backoff_max,
        attempt: u32::MAX / 2,
    };
}

fn shutdown(sys: &dyn Sys, by_pid: &BTreeMap<Pid, usize>, rx: &Receiver<Msg>) {
    use std::collections::BTreeSet;
    let mut pending: BTreeSet<Pid> = by_pid.keys().copied().collect();
    for &pid in &pending {
        let _ = sys.kill(pid, libc::SIGTERM);
    }
    let deadline = sys.now() + Duration::from_secs(5);
    while !pending.is_empty() {
        let left = deadline.saturating_duration_since(sys.now());
        if left.is_zero() {
            break;
        }
        match rx.recv_timeout(left) {
            Ok(Msg::Exited(pid, _)) => {
                pending.remove(&pid);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    for pid in pending {
        let _ = sys.kill(pid, libc::SIGKILL);
    }
}

#[cfg(test)]
mod reboot_delay_tests {
    use super::*;

    #[test]
    fn test_rewrite_env_rewrites_a_path_list_entry_by_entry() {
        // Two entries on the update root: rewriting the whole value as one
        // path would leave the second entry pointing at the old slot.
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let slots = crate::update::Slots::new(root);
        // Force active=a so slot_path resolves into slots/a regardless of the
        // host this test runs on.
        std::fs::create_dir_all(root.join("slots")).unwrap();
        std::fs::write(root.join("active"), "a").unwrap();
        let rewrite = |p: &str| {
            crate::update::slot_path(root, slots.active(), Path::new(p))
                .to_string_lossy()
                .into_owned()
        };
        let root_str = root.display().to_string();
        assert_eq!(
            rewrite_env(
                "LD_LIBRARY_PATH",
                &format!("{root_str}/vendor-daemon/lib:{root_str}/onvif/lib"),
                &rewrite,
            ),
            format!("{root_str}/slots/a/vendor-daemon/lib:{root_str}/slots/a/onvif/lib")
        );
    }

    #[test]
    fn test_rewrite_env_leaves_unbundled_path_list_entries_alone() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let slots = crate::update::Slots::new(root);
        std::fs::create_dir_all(root.join("slots")).unwrap();
        std::fs::write(root.join("active"), "a").unwrap();
        let rewrite = |p: &str| {
            crate::update::slot_path(root, slots.active(), Path::new(p))
                .to_string_lossy()
                .into_owned()
        };
        let root_str = root.display().to_string();
        // /lib is outside the slots and must pass through.
        assert_eq!(
            rewrite_env(
                "LD_LIBRARY_PATH",
                &format!("/lib:{root_str}/vendor-daemon/lib"),
                &rewrite,
            ),
            format!("/lib:{root_str}/slots/a/vendor-daemon/lib")
        );
    }

    #[test]
    fn test_rewrite_env_rewrites_non_path_list_values_verbatim() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let slots = crate::update::Slots::new(root);
        std::fs::create_dir_all(root.join("slots")).unwrap();
        std::fs::write(root.join("active"), "a").unwrap();
        let rewrite = |p: &str| {
            crate::update::slot_path(root, slots.active(), Path::new(p))
                .to_string_lossy()
                .into_owned()
        };
        let root_str = root.display().to_string();
        // A single bundled path is rewritten wholesale, not split on ':'.
        assert_eq!(
            rewrite_env(
                "OTHER",
                &format!("{root_str}/onvif/onvif-rust.bin"),
                &rewrite
            ),
            format!("{root_str}/slots/a/onvif/onvif-rust.bin")
        );
    }

    #[test]
    fn test_periodic_reboot_delay_converts_minutes_to_seconds() {
        assert_eq!(
            periodic_reboot_delay(720, 0, 12345),
            Duration::from_secs(43_200)
        );
    }

    #[test]
    fn test_periodic_reboot_zero_jitter_is_exact() {
        assert_eq!(
            periodic_reboot_delay(1, 0, u64::MAX),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn test_periodic_reboot_jitter_stays_within_bound() {
        for entropy in [0u64, 1, 59, 60, 61, u64::MAX] {
            let d = periodic_reboot_delay(10, 60, entropy).as_secs();
            assert!(
                (600..=660).contains(&d),
                "entropy {entropy} produced {d}s, outside 600..=660"
            );
        }
    }

    #[test]
    fn test_periodic_reboot_delay_saturates_instead_of_overflowing() {
        // A user typing a nonsense interval must not wrap to a near-zero delay
        // and reboot-loop the camera.
        let d = periodic_reboot_delay(u64::MAX, u64::MAX, 7);
        assert_eq!(d, Duration::from_secs(u64::MAX));
    }
}

#[cfg(test)]
mod periodic_reboot_loop_tests {
    use super::*;
    use crate::sys::MockSys;

    #[test]
    fn test_periodic_reboot_loop_sleeps_then_reboots() {
        let mut sys = MockSys::new();
        sys.expect_sleep().times(1).returning(|_| {});
        sys.expect_reboot().times(1).returning(|| Ok(()));

        periodic_reboot_loop(&sys, 1, 0);
    }

    #[test]
    fn test_periodic_reboot_loop_logs_and_returns_when_reboot_fails() {
        let mut sys = MockSys::new();
        sys.expect_sleep().times(1).returning(|_| {});
        sys.expect_reboot()
            .times(1)
            .returning(|| Err(crate::sys::SysError::Other("reboot() unsupported".into())));

        // Must not panic even though the reboot call failed.
        periodic_reboot_loop(&sys, 1, 0);
    }
}

#[cfg(test)]
mod run_tests {
    use super::*;
    use crate::config::{
        Config, LogCfg, MonitorCfg, RebootCfg, ServiceCfg, SupervisorCfg, SystemCfg, TimeCfg,
        WifiCfg,
    };
    use crate::sys::{MockSys, SysError};

    fn minimal_wifi_cfg() -> WifiCfg {
        WifiCfg {
            ssid: "test".into(),
            password: "test".into(),
            config_file: "/nonexistent/anyka_cfg.ini".into(),
            chip: "auto".into(),
            gpio_polarity: "low_high".into(),
            interface: "wlan0".into(),
            security: "wpa".into(),
            dhcp: true,
            address: None,
            gateway: None,
            dns: Vec::new(),
            connect_timeout_sec: 45,
            fallback_to_vendor: true,
        }
    }

    fn test_config(services: BTreeMap<String, ServiceCfg>) -> Config {
        Config {
            schema: 0,
            log: LogCfg::default(),
            system: SystemCfg::default(),
            wifi: minimal_wifi_cfg(),
            time: TimeCfg::default(),
            supervisor: SupervisorCfg {
                backoff_min_sec: 30,
                backoff_max_sec: 60,
                crashloop_count: 100,
                crashloop_window_sec: 600,
                storm_guard_max_reboots: 3,
                storm_guard_state: "/nonexistent/storm.json".into(),
                storm_guard_reset_uptime_sec: 600,
            },
            monitor: MonitorCfg::default(),
            reboot: RebootCfg::default(),
            update: crate::config::Update::default(),
            services,
        }
    }

    #[test]
    fn test_run_restart_message_for_unknown_service_is_ignored() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let cfg = Arc::new(test_config(BTreeMap::new()));
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || run(sys, &cfg, rx));

        tx.send(Msg::RestartService("nope".into()))
            .expect("send restart");
        tx.send(Msg::Shutdown).expect("send shutdown");
        handle.join().expect("run() must not panic");
    }

    #[test]
    fn test_run_start_failure_backs_off_and_restart_request_reports_not_running() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);
        sys.expect_spawn()
            .times(1)
            .returning(|_| Err(SysError::Other("boom".into())));

        let mut services = BTreeMap::new();
        services.insert(
            "flaky".to_string(),
            ServiceCfg {
                enabled: true,
                exec: "/bin/false".into(),
                args: Vec::new(),
                env: BTreeMap::new(),
                log: "/nonexistent/flaky.log".into(),
                core_dump: false,
            },
        );
        let cfg = Arc::new(test_config(services));
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || run(sys, &cfg, rx));

        // Give the loop time to run its first tick (spawn fails, service goes
        // to a 30s backoff) before the restart request arrives.
        std::thread::sleep(Duration::from_millis(100));
        tx.send(Msg::RestartService("flaky".into()))
            .expect("send restart");
        tx.send(Msg::Shutdown).expect("send shutdown");
        handle.join().expect("run() must not panic");
    }

    #[test]
    fn test_run_shuts_down_immediately_with_no_services() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let cfg = Arc::new(test_config(BTreeMap::new()));
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || run(sys, &cfg, rx));

        tx.send(Msg::Shutdown).expect("send shutdown");
        handle.join().expect("run() must not panic");
    }
}
