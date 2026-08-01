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
    let cfg = match config::Config::load("/mnt/anyka_hack/anyka.toml") {
        Ok(c) => c,
        Err(e) => {
            logging::console(&format!("config load failed: {e}"));
            park();
        }
    };

    if let Err(e) = logging::init(&cfg.log.dir, &cfg.log.level) {
        logging::console(&format!("logging init failed: {e}"));
        park();
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
    }

    // P2
    boot::system_setup(sysimpl.as_ref(), &cfg);

    // P2.5
    timesync::first_sync(sysimpl.as_ref(), &cfg.time);

    if cfg.monitor.enabled {
        let s = Arc::clone(&sysimpl);
        let state_path = cfg.supervisor.storm_guard_state.clone();
        let reset_after = Duration::from_secs(cfg.supervisor.storm_guard_reset_uptime_sec);
        let interval = Duration::from_secs(cfg.monitor.interval_sec);
        let _ = std::thread::Builder::new()
            .name("monitor".into())
            .stack_size(supervisor_loop::thread_stack())
            .spawn(move || {
                monitor::run(s.as_ref(), interval, &state_path, reset_after);
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

    let (tx, rx) = supervisor_loop::make_channel();
    supervisor_loop::spawn_signal_thread(tx.clone());

    if safe_mode {
        // Telnet, logging and the monitor stay up; no services start.
        park();
    }

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    supervisor_loop::spawn_reaper(Arc::clone(&sysimpl), tx, Arc::clone(&stop));

    // P3 + P4
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
