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

    if let Err(e) = logging::init(&cfg.log.dir, &cfg.log.level) {
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
    }

    // P2
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

    if cfg.reboot.enabled {
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
        // Telnet, logging and the monitor stay up; no services start.
        park();
    }

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    supervisor_loop::spawn_reaper(Arc::clone(&sysimpl), tx, Arc::clone(&stop));

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
