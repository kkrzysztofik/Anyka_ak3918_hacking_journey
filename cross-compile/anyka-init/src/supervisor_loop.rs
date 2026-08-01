//! The P3 + P4 supervision loop.

use crate::config::{Config, ServiceCfg};
use crate::logging;
use crate::storm::StormState;
use crate::supervise::{Action, Event, Policy, RestartHistory, SvcState, decide};
use crate::sys::{ExitStatus, Pid, SpawnSpec, Sys};
use std::collections::BTreeMap;
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
            state: SvcState::Backoff {
                until: sys.now(),
                attempt: 0,
            },
            hist: RestartHistory::default(),
        })
        .collect();

    let mut by_pid: BTreeMap<Pid, usize> = BTreeMap::new();

    loop {
        let mut next_deadline: Option<Instant> = None;

        // Index loop: by_pid stores service indices; Start failure path also
        // needs random-access mutation of hist/state by index.
        #[allow(clippy::needless_range_loop)]
        for i in 0..services.len() {
            let now = sys.now();
            let state = services[i].state;
            let d = decide(&state, &mut services[i].hist, Event::Tick, now, &policy);
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
                        services[i].state = SvcState::Running {
                            pid,
                            since: sys.now(),
                        };
                        by_pid.insert(pid, i);
                    }
                    Err(e) => {
                        tracing::error!(service = %services[i].name, error = %e, "start failed");
                        let d = decide(
                            &SvcState::Running {
                                pid: -1,
                                since: sys.now(),
                            },
                            &mut services[i].hist,
                            Event::Exited,
                            sys.now(),
                            &policy,
                        );
                        services[i].state = d.next;
                        if let Action::Reboot(why) = d.action
                            && !do_reboot(sys.as_ref(), cfg, &why)
                        {
                            apply_failed_reboot_backoff(
                                &mut services[i],
                                sys.now(),
                                policy.backoff_max,
                            );
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
                let state = services[i].state;
                let d = decide(&state, &mut services[i].hist, Event::Exited, now, &policy);
                services[i].state = d.next;
                if let Action::Reboot(why) = d.action
                    && !do_reboot(sys.as_ref(), cfg, &why)
                {
                    apply_failed_reboot_backoff(&mut services[i], sys.now(), policy.backoff_max);
                }
            }
            Ok(Msg::RestartService(name)) => match services.iter().find(|s| s.name == name) {
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
            },
            Ok(Msg::Shutdown) => {
                tracing::info!("shutdown requested");
                shutdown(sys.as_ref(), &by_pid, &rx);
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
