//! The P3 + P4 supervision loop.

use crate::config::{Config, ServiceCfg};
use crate::control;
use crate::logging;
use crate::storm::StormState;
use crate::supervise::{Action, Event, Policy, RestartHistory, SvcState, decide};
use crate::sys::{ExitStatus, Pid, SpawnSpec, Sys};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
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

pub enum ControlMsg {
    Status(Vec<control::ServiceStatus>),
}

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
    QueryStatus(Sender<ControlMsg>),
    /// Runtime enable/disable of a configured service. The handler persists to
    /// `anyka.toml` first, then transitions in-memory state.
    ToggleService {
        name: String,
        enabled: bool,
        reply: Sender<control::ToggleOutcome>,
    },
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
        // The zone from the onvif config (source of truth); a service's own
        // `[services.X].env` TZ entry still overrides this one.
        tz: crate::boot::resolve_timezone(root, slots),
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

/// The recovery telnet (port 24). Deliberately *outside* the supervised
/// table: the P0 wrapper starts it **before** anyka-init exists, and a kill
/// must stay final — a supervisor would just resurrect it. So this (1)
/// persists `[system].telnet` (file first, like every other toggle) and
/// (2) makes the live process match: spawn the P0-equivalent
/// `telnetd -p 24 -l /bin/sh`, or killall it. Reboots follow the persisted
/// flag through the existing P0 (always start) and P2 (kill iff false) steps
/// — no boot-code change. Safe mode force-enables the flag at boot, which is
/// the documented escape hatch, not a bug this should fight.
fn handle_toggle_telnet(
    ctx: &mut LoopCtx<'_>,
    enabled: bool,
    reply: &Sender<control::ToggleOutcome>,
) {
    if let Err(e) = Config::set_system_telnet(ctx.config_path, enabled) {
        tracing::error!(error = %e, "telnet toggle: config write failed; not applied");
        let _ = reply.send(control::ToggleOutcome::Error);
        return;
    }
    ctx.cfg.system.telnet = enabled;

    if enabled {
        // pidof exits non-zero when the process is absent: port 24 is
        // single-instance, so spawn only when nothing holds it.
        let running = matches!(
            ctx.sys.run_to_completion("pidof", &["telnetd".to_string()]),
            Ok(status) if status.success()
        );
        if !running
            && let Err(e) = ctx.sys.spawn_detached(
                "telnetd",
                &[
                    "-p".to_string(),
                    "24".to_string(),
                    "-l".to_string(),
                    "/bin/sh".to_string(),
                ],
            )
        {
            tracing::warn!(error = %e, "telnet toggle: telnetd spawn failed");
        }
    } else {
        // killall exits non-zero when nothing matched — that is success here;
        // only a spawn/wait failure is worth a warning.
        if let Err(e) = ctx
            .sys
            .run_to_completion("killall", &["telnetd".to_string()])
        {
            tracing::warn!(error = %e, "telnet toggle: killall failed");
        }
    }
    tracing::info!(enabled, "recovery telnet toggled");
    let _ = reply.send(control::ToggleOutcome::Ok);
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
    // Only the service's *current* pid may drive its state machine.
    //
    // `disable` SIGTERMs the child and marks the service inert, but the exit
    // report arrives up to a reap-poll later. Re-enabling inside that window
    // starts a fresh pid while the old mapping is still in `by_pid`; without
    // this check the stale exit would be charged to the new instance —
    // recording a restart it never had, knocking it into backoff, and letting
    // the next tick spawn a second copy alongside the one already running.
    if services[i].state.pid() != Some(pid) {
        tracing::debug!(
            service = %services[i].name,
            pid,
            ?st,
            "ignoring an exit from a superseded pid"
        );
        return;
    }
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

/// One row per **configured** service, not per running one: a disabled
/// service has to be listable or the UI has no way to offer re-enabling it.
/// `cfg` is the single source of truth — both the boot path and the toggle
/// handler write it before anything else.
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

/// Services that may not be toggled at runtime.
///
/// `wpa_supplicant`: the monitor's wifi ladder reboots the camera on an
/// unhealthy link (monitor.rs:80) and is driven by link health, not by a file
/// we can stand down. Disabling the supplicant on a Wi-Fi camera also loses
/// the device outright.
const NON_TOGGLEABLE: [&str; 1] = ["wpa_supplicant"];

/// Runtime enable/disable. Order is deliberate: **file first**, then
/// in-memory cfg, then state/kill. A failed write means nothing changes — a
/// "disabled" service that silently re-enabled itself on reboot would defeat
/// the crash-loop escape hatch this exists for.
///
/// `telnetd` is special-cased before any service lookup: it is not a
/// supervised service, it is the `[system].telnet` switch (see
/// `handle_toggle_telnet`).
fn handle_toggle_service(
    ctx: &mut LoopCtx<'_>,
    services: &mut Vec<Service>,
    name: String,
    enabled: bool,
    reply: &Sender<control::ToggleOutcome>,
) {
    if name == "telnetd" {
        handle_toggle_telnet(ctx, enabled, reply);
        return;
    }
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

fn handle_control_conn(mut stream: UnixStream, tx: &Sender<Msg>) -> std::io::Result<()> {
    use std::io::{BufRead, Write};
    let mut reader = std::io::BufReader::new(&stream);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(());
    }
    match control::parse_request(&line) {
        Some(control::Request::Status) => {
            let (reply_tx, reply_rx) = channel();
            if tx.send(Msg::QueryStatus(reply_tx)).is_ok() {
                // Bounded like `send_toggle`. This thread serves connections
                // one at a time, so an unbounded wait on a wedged loop would
                // not just hang this status call — every later status,
                // restart, enable and disable would queue behind it forever.
                let reply = reply_rx.recv_timeout(CONTROL_REPLY_TIMEOUT);
                if let Ok(ControlMsg::Status(rows)) = reply {
                    let _ = stream.write_all(control::encode_status(&rows).as_bytes());
                }
            }
        }
        Some(control::Request::Restart(name)) => {
            let _ = tx.send(Msg::RestartService(name.clone()));
            // The onvif-rust client treats exactly "ok" as accepted and anything
            // else as a failure — keep the reply minimal, not chatty.
            let _ = stream.write_all(b"ok\n");
        }
        Some(control::Request::Enable(name)) => send_toggle(&mut stream, tx, name, true),
        Some(control::Request::Disable(name)) => send_toggle(&mut stream, tx, name, false),
        None => {
            let _ = stream.write_all(b"unknown\n");
        }
    }
    Ok(())
}

/// How long the control thread waits for the supervisor loop to answer.
///
/// Deliberately below the client's 2 s socket timeout
/// (`onvif-rust/src/diagnostics/services.rs`): answering later than the client
/// waits turns every slow reply into a connection error.
const CONTROL_REPLY_TIMEOUT: Duration = Duration::from_secs(1);

/// Ask the loop to toggle a service and write its verdict back to the
/// connection.
fn send_toggle<W: std::io::Write>(writer: &mut W, tx: &Sender<Msg>, name: String, enabled: bool) {
    let (reply_tx, reply_rx) = channel();
    let sent = tx.send(Msg::ToggleService {
        name,
        enabled,
        reply: reply_tx,
    });
    let reply = match sent {
        // A timeout is NOT a failure. The message is still queued, and
        // `handle_toggle_service` writes `anyka.toml` and updates state
        // *before* it replies — dropping the receiver cancels nothing. Saying
        // "error" here would tell an admin nothing changed while the toggle
        // lands a moment later. `pending` says what is actually true: it was
        // accepted, the outcome is unconfirmed, go look at the service list.
        Ok(()) => match reply_rx.recv_timeout(CONTROL_REPLY_TIMEOUT) {
            Ok(control::ToggleOutcome::Ok) => "ok\n",
            Ok(control::ToggleOutcome::Unknown) => "unknown\n",
            Ok(control::ToggleOutcome::Error) => "error\n",
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!("toggle reply timed out; the change may still apply");
                "pending\n"
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => "error\n",
        },
        // The loop is gone entirely (reboot in progress): nothing was queued.
        Err(_) => "error\n",
    };
    let _ = writer.write_all(reply.as_bytes());
}

pub fn spawn_control_thread(tx: Sender<Msg>) -> std::io::Result<()> {
    let socket_path = control::SOCKET_PATH;
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    // Not best-effort: this socket accepts restart/enable/disable. If it stays
    // world-writable the toggles are open to any local process, so a failure
    // here has to be visible rather than swallowed.
    if let Err(e) = std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600)) {
        tracing::error!(
            error = %e,
            path = socket_path,
            "could not restrict the control socket; it may be reachable by other local users"
        );
    }

    std::thread::Builder::new()
        .name("supervisor-ctl".into())
        .stack_size(thread_stack())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                if let Err(e) = handle_control_conn(stream, &tx) {
                    tracing::warn!(error = %e, "control connection failed");
                }
            }
        })
        .map_err(std::io::Error::other)?;
    Ok(())
}

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

/// Returns `true` when the supervisor loop should exit.
// `&mut Vec` deliberately: the enable path in `handle_toggle_service` pushes
// a service that was disabled at boot; a slice cannot grow.
#[allow(clippy::ptr_arg)]
fn dispatch_msg(
    ctx: &mut LoopCtx<'_>,
    services: &mut Vec<Service>,
    by_pid: &mut BTreeMap<Pid, usize>,
    rx: &Receiver<Msg>,
    msg: Result<Msg, std::sync::mpsc::RecvTimeoutError>,
) -> bool {
    match msg {
        Ok(Msg::Exited(pid, st)) => {
            handle_service_exited(ctx.sys, ctx.cfg, services, by_pid, ctx.policy, pid, st);
            false
        }
        Ok(Msg::RestartService(name)) => {
            handle_restart_service(ctx.sys, services, name);
            false
        }
        Ok(Msg::KillService(name)) => {
            handle_kill_service(ctx.sys, services, name);
            false
        }
        Ok(Msg::QueryStatus(reply_tx)) => {
            handle_query_status(ctx.cfg, services, &reply_tx);
            false
        }
        Ok(Msg::ToggleService {
            name,
            enabled,
            reply,
        }) => {
            handle_toggle_service(ctx, services, name, enabled, &reply);
            false
        }
        Ok(Msg::Shutdown) => {
            tracing::info!("shutdown requested");
            shutdown(ctx.sys, by_pid, rx);
            true
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            tracing::error!("event channel closed");
            true
        }
    }
}

pub fn run(sys: Arc<dyn Sys>, cfg: &mut Config, config_path: &Path, rx: Receiver<Msg>) {
    let policy = Policy {
        backoff_min: Duration::from_secs(cfg.supervisor.backoff_min_sec),
        backoff_max: Duration::from_secs(cfg.supervisor.backoff_max_sec),
        crashloop_count: cfg.supervisor.crashloop_count,
        crashloop_window: Duration::from_secs(cfg.supervisor.crashloop_window_sec),
    };

    // Owned, not borrowed from `cfg`: `LoopCtx` holds `&mut Config`, so a
    // live immutable borrow of `cfg.update.root` would conflict. Neither
    // value changes at runtime.
    let slots = crate::update::Slots::new(cfg.update.root.clone());
    let update_root = std::path::PathBuf::from(&cfg.update.root);

    let mut services = build_enabled_services(sys.as_ref(), cfg, &update_root, &slots);
    let mut by_pid: BTreeMap<Pid, usize> = BTreeMap::new();

    loop {
        let next_deadline = tick_services(sys.as_ref(), cfg, &mut services, &mut by_pid, &policy);
        let timeout = next_deadline
            .map(|d| d.saturating_duration_since(sys.now()))
            .unwrap_or(Duration::from_secs(3600));

        let mut ctx = LoopCtx {
            sys: sys.as_ref(),
            cfg,
            config_path,
            update_root: &update_root,
            slots: &slots,
            policy: &policy,
        };
        if dispatch_msg(
            &mut ctx,
            &mut services,
            &mut by_pid,
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

    #[test]
    fn test_handle_service_exited_ignores_a_superseded_pid() {
        // The disable -> re-enable race: the old pid was SIGTERM'd and its
        // mapping is still in by_pid when the service is already running again
        // under a new pid. Charging that stale exit to the new instance would
        // record a restart it never had, knock it into backoff, and let the
        // next tick spawn a second copy beside the one already running.
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);
        let cfg = test_config(BTreeMap::new());

        let running_since = Instant::now();
        let mut services = vec![Service {
            name: "snmp".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 200,
                since: running_since,
            },
            hist: RestartHistory::default(),
        }];
        let mut by_pid = BTreeMap::new();
        by_pid.insert(100, 0); // the superseded pid
        by_pid.insert(200, 0);

        handle_service_exited(
            &sys,
            &cfg,
            &mut services,
            &mut by_pid,
            &test_policy(),
            100,
            ExitStatus::Code(0),
        );

        // Still running under the new pid, no crash recorded, and the stale
        // mapping is gone.
        assert_eq!(
            services[0].state,
            SvcState::Running {
                pid: 200,
                since: running_since
            }
        );
        assert_eq!(services[0].hist.len(), 0);
        assert!(!by_pid.contains_key(&100));
        assert_eq!(by_pid.get(&200), Some(&0));
    }

    #[test]
    fn test_handle_service_exited_still_handles_the_current_pid() {
        // The guard must not swallow the normal case.
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);
        let cfg = test_config(BTreeMap::new());

        let mut services = vec![Service {
            name: "snmp".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 200,
                since: Instant::now(),
            },
            hist: RestartHistory::default(),
        }];
        let mut by_pid = BTreeMap::new();
        by_pid.insert(200, 0);

        handle_service_exited(
            &sys,
            &cfg,
            &mut services,
            &mut by_pid,
            &test_policy(),
            200,
            ExitStatus::Code(1),
        );

        assert!(matches!(services[0].state, SvcState::Backoff { .. }));
        assert!(by_pid.is_empty());
    }

    /// Mirrors the `Policy` that `run` builds from `test_config`.
    fn test_policy() -> Policy {
        Policy {
            backoff_min: Duration::from_secs(30),
            backoff_max: Duration::from_secs(60),
            crashloop_count: 100,
            crashloop_window: Duration::from_secs(600),
        }
    }

    /// A `LoopCtx` over test-owned parts. Returned by value so each test can
    /// keep its tempdir alive.
    fn ctx<'a>(
        sys: &'a dyn Sys,
        cfg: &'a mut Config,
        config_path: &'a Path,
        update_root: &'a Path,
        slots: &'a crate::update::Slots,
        policy: &'a Policy,
    ) -> LoopCtx<'a> {
        LoopCtx {
            sys,
            cfg,
            config_path,
            update_root,
            slots,
            policy,
        }
    }

    #[test]
    fn test_run_restart_message_for_unknown_service_is_ignored() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let mut cfg = test_config(BTreeMap::new());
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || {
            run(sys, &mut cfg, Path::new("/nonexistent/anyka.toml"), rx)
        });

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
        let mut cfg = test_config(services);
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || {
            run(sys, &mut cfg, Path::new("/nonexistent/anyka.toml"), rx)
        });

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

        let mut cfg = test_config(BTreeMap::new());
        let (tx, rx) = make_channel();
        let sys: Arc<dyn Sys> = Arc::new(sys);
        let handle = std::thread::spawn(move || {
            run(sys, &mut cfg, Path::new("/nonexistent/anyka.toml"), rx)
        });

        tx.send(Msg::Shutdown).expect("send shutdown");
        handle.join().expect("run() must not panic");
    }

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
            state: SvcState::Running {
                pid: 55,
                since: Instant::now(),
            },
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
        assert!(matches!(
            svcs[0].state,
            SvcState::Backoff { attempt: 0, .. }
        ));
    }

    #[test]
    fn test_toggle_unknown_service_replies_unknown_and_writes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        let before = "[services.snmp]\nenabled = true\nexec = \"/bin/true\"\n".to_string();
        std::fs::write(&cfg_path, &before).expect("seed");

        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let mut services = BTreeMap::new();
        services.insert("snmp".to_string(), svc_cfg("/bin/true", true));
        let mut cfg = test_config(services);
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs = vec![Service {
            name: "snmp".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 55,
                since: Instant::now(),
            },
            hist: RestartHistory::default(),
        }];

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "nope".into(), false, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Unknown);
        assert_eq!(std::fs::read_to_string(&cfg_path).expect("read"), before);
    }

    #[test]
    fn test_toggle_of_wpa_supplicant_is_refused() {
        // Configured and enabled, but not toggleable: disabling it would let the
        // monitor's wifi ladder reboot the camera, and there is no way back in.
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        let before =
            "[services.wpa_supplicant]\nenabled = true\nexec = \"/bin/true\"\n".to_string();
        std::fs::write(&cfg_path, &before).expect("seed");

        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let mut services = BTreeMap::new();
        services.insert("wpa_supplicant".to_string(), svc_cfg("/bin/true", true));
        let mut cfg = test_config(services);
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs = vec![Service {
            name: "wpa_supplicant".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 55,
                since: Instant::now(),
            },
            hist: RestartHistory::default(),
        }];

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "wpa_supplicant".into(), false, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Unknown);
        assert_eq!(std::fs::read_to_string(&cfg_path).expect("read"), before);
        assert!(cfg.services["wpa_supplicant"].enabled);
    }

    #[test]
    fn test_toggle_is_an_idempotent_noop_when_already_in_that_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        let before = "[services.snmp]\nenabled = false\nexec = \"/bin/true\"\n".to_string();
        std::fs::write(&cfg_path, &before).expect("seed");

        // MockSys with no kill expectation — calling it would fail the test.
        let sys = MockSys::new();

        let mut services = BTreeMap::new();
        services.insert("snmp".to_string(), svc_cfg("/bin/true", false));
        let mut cfg = test_config(services);
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs: Vec<Service> = Vec::new();

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "snmp".into(), false, &rtx);

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

        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);

        let mut services = BTreeMap::new();
        services.insert("snmp".to_string(), svc_cfg("/bin/true", true));
        let mut cfg = test_config(services);
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs = vec![Service {
            name: "snmp".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 55,
                since: Instant::now(),
            },
            hist: RestartHistory::default(),
        }];

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "snmp".into(), false, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Error);
        assert!(cfg.services["snmp"].enabled); // in-memory untouched
        assert!(matches!(svcs[0].state, SvcState::Running { .. }));
    }

    #[test]
    fn test_disabling_vendor_daemon_removes_the_video_heartbeat() {
        // Without this the monitor keeps reading a stale counter and reboots
        // the camera five ticks later — the exact thing disabling is meant to stop.
        let dir = tempfile::tempdir().expect("tempdir");
        let hb = dir.path().join("video.heartbeat");
        std::fs::write(&hb, "12345\n").expect("seed heartbeat");

        let cfg_path = dir.path().join("anyka.toml");
        std::fs::write(
            &cfg_path,
            "[services.vendor-daemon]\nenabled = true\nexec = \"/bin/true\"\n",
        )
        .expect("seed");

        let mut sys = MockSys::new();
        sys.expect_now().returning(Instant::now);
        sys.expect_kill()
            .withf(|pid, sig| *pid == 77 && *sig == libc::SIGTERM)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut services = BTreeMap::new();
        services.insert("vendor-daemon".to_string(), svc_cfg("/bin/true", true));
        let mut cfg = test_config(services);
        cfg.monitor.video_heartbeat_path = hb.display().to_string();
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs = vec![Service {
            name: "vendor-daemon".into(),
            spec: dummy_spec(),
            state: SvcState::Running {
                pid: 77,
                since: Instant::now(),
            },
            hist: RestartHistory::default(),
        }];

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "vendor-daemon".into(), false, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
        assert!(!hb.exists());
    }

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
        // Defensive: cfg says enabled, the vec does not have it (should not
        // happen after Task 4's insertion). The row must still appear, never
        // vanish.
        let mut services = BTreeMap::new();
        services.insert("snmp".to_string(), svc_cfg("/bin/true", true));
        let cfg = test_config(services);

        let (tx, rx) = channel();
        handle_query_status(&cfg, &[], &tx);
        let ControlMsg::Status(rows) = rx.recv().expect("status");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].state, "backoff");
    }

    #[test]
    fn test_toggle_telnet_enable_persists_then_spawns() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        std::fs::write(&cfg_path, "[system]\ntelnet = false\n").expect("seed");

        let mut sys = MockSys::new();
        sys.expect_run_to_completion()
            .times(1)
            .returning(|prog, _args| {
                assert_eq!(prog, "pidof");
                // pidof exits non-zero when the process is absent.
                Ok(crate::sys::ExitStatus::Code(1))
            });
        sys.expect_spawn_detached()
            .times(1)
            .returning(|prog, args| {
                assert_eq!(prog, "telnetd");
                assert_eq!(
                    args,
                    &[
                        "-p".to_string(),
                        "24".to_string(),
                        "-l".to_string(),
                        "/bin/sh".to_string()
                    ]
                );
                Ok(77)
            });

        let mut cfg = test_config(BTreeMap::new());
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs: Vec<Service> = Vec::new();

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "telnetd".into(), true, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
        // File first, then memory — and never a supervised service.
        assert!(
            std::fs::read_to_string(&cfg_path)
                .expect("read")
                .contains("telnet = true")
        );
        assert!(cfg.system.telnet);
        assert_eq!(svcs.len(), 0);
    }

    #[test]
    fn test_toggle_telnet_enable_is_idempotent_when_already_running() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        std::fs::write(&cfg_path, "[system]\ntelnet = false\n").expect("seed");

        let mut sys = MockSys::new();
        sys.expect_run_to_completion()
            .times(1)
            .returning(|prog, _| {
                assert_eq!(prog, "pidof");
                // pidof found it: port 24 already held, do not double-spawn.
                Ok(crate::sys::ExitStatus::Code(0))
            });
        sys.expect_spawn_detached().times(0);

        let mut cfg = test_config(BTreeMap::new());
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs: Vec<Service> = Vec::new();

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "telnetd".into(), true, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
        assert!(cfg.system.telnet);
    }

    #[test]
    fn test_toggle_telnet_disable_persists_then_killalls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        std::fs::write(&cfg_path, "[system]\ntelnet = true\n").expect("seed");

        let mut sys = MockSys::new();
        sys.expect_run_to_completion()
            .times(1)
            .returning(|prog, _| {
                assert_eq!(prog, "killall");
                // killall exits non-zero when nothing matched — success here.
                Ok(crate::sys::ExitStatus::Code(1))
            });
        sys.expect_spawn_detached().times(0);

        let mut cfg = test_config(BTreeMap::new());
        cfg.system.telnet = true;
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs: Vec<Service> = Vec::new();

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "telnetd".into(), false, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Ok);
        assert!(
            std::fs::read_to_string(&cfg_path)
                .expect("read")
                .contains("telnet = false")
        );
        assert!(!cfg.system.telnet);
    }

    #[test]
    fn test_toggle_telnet_write_failure_changes_nothing() {
        // A directory where the config file should be: the read fails, so
        // neither the file, the memory, nor any process may change.
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("anyka.toml");
        std::fs::create_dir(&cfg_path).expect("mkdir");

        let mut sys = MockSys::new();
        sys.expect_run_to_completion().times(0);
        sys.expect_spawn_detached().times(0);

        let mut cfg = test_config(BTreeMap::new());
        let slots = crate::update::Slots::new(dir.path());
        let policy = test_policy();
        let mut svcs: Vec<Service> = Vec::new();

        let (rtx, rrx) = channel();
        let mut c = ctx(&sys, &mut cfg, &cfg_path, dir.path(), &slots, &policy);
        handle_toggle_service(&mut c, &mut svcs, "telnetd".into(), true, &rtx);

        assert_eq!(rrx.recv().expect("reply"), control::ToggleOutcome::Error);
        assert!(!cfg.system.telnet);
    }
}
