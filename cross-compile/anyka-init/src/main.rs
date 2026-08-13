//! anyka-init entry point: phase sequencing for the boot/runtime supervisor.

use anyka_init::{boot, config, logging, monitor, storm, supervisor_loop, sys, timesync};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        tracing::error!(panic = %msg, "anyka-init panicked; children are orphaned");
        logging::console(&format!("PANIC: {msg}"));
    }));

    // P1: config. Never fall back to defaults — a wrong wifi_ssid or
    // sensor_module is worse than parking with recovery telnet up.
    let mut cfg = match config::Config::load("/mnt/anyka_hack/anyka.toml") {
        Ok(c) => c,
        Err(e) => {
            logging::console(&format!("config load failed: {e}"));
            park();
        }
    };

    if let Err(e) = logging::init(
        &cfg.log.dir,
        &cfg.log.level,
        cfg.log.max_bytes,
        cfg.log.keep,
    ) {
        // Do NOT park here. Parking on a bad config is right — a guessed
        // wifi_ssid or sensor_module does real damage. Losing the log directory
        // is not in that class: /mnt is vfat and can come back read-only after
        // an unclean shutdown, and a camera that still streams but keeps no
        // logs beats one that does nothing at all. Errors still reach the boot
        // console via `console`.
        logging::console(&format!(
            "logging init failed ({e}); continuing without file logs"
        ));
    }
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "anyka-init starting");

    let sysimpl: Arc<dyn sys::Sys> = Arc::new(sys::RealSys::new());

    let storm_state = storm::StormState::load(&cfg.supervisor.storm_guard_state);
    let safe_mode = storm::should_enter_safe_mode(
        storm_state.fast_reboots,
        cfg.supervisor.storm_guard_max_reboots,
    );
    if safe_mode {
        tracing::error!(
            fast_reboots = storm_state.fast_reboots,
            max = cfg.supervisor.storm_guard_max_reboots,
            "SAFE MODE: reboot-storm threshold reached. Log in over telnet :24, \
             fix the failing service, clear /mnt/anyka_hack/state/boot.json, reboot."
        );
        // system_setup honouring [system].telnet=false would kill the P0
        // recovery channel before we park — leave no way in. Force it on.
        cfg.system.telnet = true;
    }

    // P2
    boot::protect_from_oom_killer(std::path::Path::new("/proc/self/oom_score_adj"));
    boot::write_panic_sysctls(std::path::Path::new("/proc/sys"));
    let probed = boot::system_setup(sysimpl.as_ref(), &cfg);

    // P2.5
    timesync::first_sync(sysimpl.as_ref(), &cfg.time);

    // Channel is created before the monitor so link recovery can request
    // service restarts without racing the supervisor's own spawn path (R15).
    let (tx, rx) = supervisor_loop::make_channel();
    supervisor_loop::spawn_signal_thread(tx.clone());

    if cfg.monitor.enabled {
        let s = Arc::clone(&sysimpl);
        let state_path = cfg.supervisor.storm_guard_state.clone();
        let reset_after = Duration::from_secs(cfg.supervisor.storm_guard_reset_uptime_sec);
        let mon = cfg.monitor.clone();
        let iface = cfg.wifi.interface.clone();
        let tx_mon = tx.clone();
        let _ = std::thread::Builder::new()
            .name("monitor".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                monitor::run(s.as_ref(), &mon, &iface, &state_path, reset_after, tx_mon);
            });
    }

    if cfg.time.enabled {
        let s = Arc::clone(&sysimpl);
        let tcfg = cfg.time.clone();
        let _ = std::thread::Builder::new()
            .name("timesync".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                timesync::resync_loop(s.as_ref(), &tcfg);
            });
    }

    if cfg.reboot.enabled && !safe_mode {
        let s = Arc::clone(&sysimpl);
        let interval_min = cfg.reboot.interval_min;
        let jitter = cfg.reboot.jitter_max_sec;
        let _ = std::thread::Builder::new()
            .name("periodic-reboot".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                supervisor_loop::periodic_reboot_loop(s.as_ref(), interval_min, jitter);
            });
    }

    if safe_mode {
        // Reaching safe mode means repeated fast reboots. If an update is
        // still unconfirmed it is the prime suspect, and parking with the
        // marker intact would leave the camera on the broken slot forever —
        // the trial thread below is never reached from here. Revert first;
        // this reboots and does not return when there is something to revert.
        anyka_init::update::revert_now(sysimpl.as_ref(), std::path::Path::new(&cfg.update.root));
        // Telnet, logging and the monitor stay up; no services start.
        park();
    }

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _reaper = supervisor_loop::spawn_reaper(Arc::clone(&sysimpl), tx, Arc::clone(&stop));

    // The supplicant bring-up spawned is unsupervised and holds the ctrl
    // socket. Hand the interface over to the supervised instance here, not
    // earlier: P2.5 time sync needs the link up, and the address stays
    // configured across the swap so the gap is one reassociation.
    if let Some(driver) = probed {
        if let Some(svc) = cfg.services.get_mut("wpa_supplicant")
            && !anyka_init::wifi::patch_driver_arg(&mut svc.args, driver)
        {
            tracing::warn!(
                driver,
                "wpa_supplicant service has no -D flag to patch; using argv as-is"
            );
        }
        let _ = sysimpl.run_to_completion("killall", &["wpa_supplicant".to_string()]);
    }

    // P3 + P4
    // An unconfirmed update resolves on its own thread, because
    // `supervisor_loop::run` below blocks forever.
    //
    // The thread is spawned just before services start rather than after, so
    // the deadline budget has to cover service startup as well as the hold:
    // 120 s deadline against a 30 s hold leaves ~90 s for onvif-rust to walk
    // its five startup phases and bind. Judging a slow-but-healthy boot a
    // failure costs a revert to a known-good slot, so the safe direction is
    // the one it already errs in.
    //
    // The trial thread and the poll thread below both mutate the active
    // pointer and the trial marker. One lock serializes them: only one of
    // reconcile/apply may be mutating durable slot state at a time, so a
    // revert cannot interleave with a flip and leave `active` agreeing with
    // neither the marker nor the other writer.
    let slot_lock = Arc::new(std::sync::Mutex::new(()));
    {
        let s = Arc::clone(&sysimpl);
        let root = cfg.update.root.clone();
        let running = anyka_init::update::Slots::new(&root).running_slot();
        let policy = anyka_init::update::Policy {
            hold_secs: cfg.update.trial_hold_sec,
            deadline_secs: cfg.update.trial_deadline_sec,
            ports: cfg.update.trial_ports.clone(),
        };
        let lock = Arc::clone(&slot_lock);
        let _ = std::thread::Builder::new()
            .name("update-trial".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                anyka_init::update::reconcile(
                    s.as_ref(),
                    std::path::Path::new(&root),
                    running,
                    policy,
                    anyka_init::netstat::listening,
                    std::thread::sleep,
                    &lock,
                );
            });
    }

    // Poll `spool/` for a dropped bundle. Reuses the monitor cadence — a
    // `stat` per minute costs nothing and adds no new tunable.
    //
    // The interval is floored at 60 s: `monitor.interval_sec` is only
    // validated as non-zero when the monitor is enabled, so a disabled monitor
    // with a zero interval would otherwise busy-spin this thread.
    {
        let s = Arc::clone(&sysimpl);
        let root = cfg.update.root.clone();
        let schema = cfg.schema;
        let interval = Duration::from_secs(cfg.monitor.interval_sec.max(60));
        let lock = Arc::clone(&slot_lock);
        let _ = std::thread::Builder::new()
            .name("update-poll".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                let root = std::path::Path::new(&root);
                loop {
                    std::thread::sleep(interval);
                    if anyka_init::update::pending(root) {
                        anyka_init::update::apply(s.as_ref(), root, schema, &lock);
                    }
                }
            });
    }

    supervisor_loop::run(sysimpl, &cfg, rx);
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Block forever without spinning. The recovery telnet started by the P0
/// wrapper stays reachable.
fn park() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
