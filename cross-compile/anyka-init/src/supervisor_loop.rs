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
    let _ = std::thread::Builder::new()
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
        });
}

pub fn spawn_signal_thread(tx: Sender<Msg>) {
    let _ = std::thread::Builder::new()
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
        });
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
                let state = services[i].state;
                let d = decide(&state, &mut services[i].hist, Event::Exited, now, &policy);
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
