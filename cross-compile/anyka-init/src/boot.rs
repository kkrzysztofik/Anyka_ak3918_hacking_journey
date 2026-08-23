//! P2 system setup: timezone, sensor module, wifi credentials, service kills.

use crate::config::{Config, WifiCfg};
use crate::sys::Sys;

/// Rewrite `ssid`/`password` in an `anyka_cfg.ini`, preserving every other
/// line verbatim. Matches only lines whose key is exactly `ssid` or `password`.
/// Missing keys are appended so a file without them still verifies on readback.
pub fn rewrite_wifi_cfg(src: &str, ssid: &str, password: &str) -> String {
    let mut out = String::with_capacity(src.len() + 32);
    let mut saw_ssid = false;
    let mut saw_password = false;
    for line in src.lines() {
        let key = line.split('=').next().unwrap_or("").trim();
        match key {
            "ssid" => {
                saw_ssid = true;
                out.push_str(&format!("ssid = {ssid}\n"));
            }
            "password" => {
                saw_password = true;
                out.push_str(&format!("password = {password}\n"));
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    if !saw_ssid {
        out.push_str(&format!("ssid = {ssid}\n"));
    }
    if !saw_password {
        out.push_str(&format!("password = {password}\n"));
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

    let verified = std::fs::read_to_string(path)
        .map(|readback| !needs_wifi_update(&readback, &cfg.ssid, &cfg.password))
        .unwrap_or(false);
    if !verified {
        tracing::error!("wifi config readback failed or mismatched; restoring backup");
        std::fs::write(path, &current)?;
        anyhow::bail!("wifi config verification failed");
    }

    tracing::info!(ssid = %cfg.ssid, "wifi credentials updated");
    Ok(true)
}

/// Reboot 10s after a panic instead of halting forever.
///
/// This kernel ships `kernel.panic = 0`, so a panic leaves the camera dead
/// until someone power-cycles it. `proc_root` is a parameter only so the test
/// can point at a tempdir; production passes `/proc/sys`. Called from `main`
/// rather than `system_setup` so the host test suite never touches the real
/// `/proc/sys` — every other side effect in `system_setup` routes through the
/// injected `Sys` or a config-supplied path, and this one cannot.
///
/// Best-effort: a missing knob is normal on other kernels and must not abort
/// boot.
pub fn write_panic_sysctls(proc_root: &std::path::Path) {
    for (knob, value) in [("kernel/panic", "10"), ("kernel/panic_on_oops", "1")] {
        let path = proc_root.join(knob);
        if let Err(e) = std::fs::write(&path, value) {
            tracing::warn!(knob, error = %e, "could not set panic sysctl");
        }
    }
}

/// Make this process the OOM killer's last choice.
///
/// The supervisor is the one process whose death cannot be recovered from in
/// software: `/usr/sbin/service.sh` has no respawn loop, so if anyka-init dies
/// every service is orphaned and only a power cycle brings the camera back.
/// `path` is a parameter for the same reason `write_panic_sysctls` takes
/// `proc_root`: production passes `/proc/self/oom_score_adj`, the test a
/// tempdir, so the host suite never rewrites the real machine's score when it
/// happens to run as root. Best-effort — a write failure must not abort boot.
pub fn protect_from_oom_killer(path: &std::path::Path) {
    if let Err(e) = std::fs::write(path, "-1000") {
        tracing::warn!(error = %e, "could not set oom_score_adj");
    }
}

/// Who holds `/var/run/wpa_supplicant/<iface>` when bring-up returns.
///
/// Only one process can own that control socket. Collapsing the vendor-fallback
/// case into "no driver probed" is what caused the 2026-08-23 `.127` outage: the
/// handover `killall` was skipped exactly when a foreign supplicant held the
/// socket, so the supervised service exited 255 forever, tripped the crash-loop
/// cap, and rebooted the camera into SAFE MODE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupplicantOwnership {
    /// Bring-up associated. The supervised service takes over on this `-D` flag.
    Ours(&'static str),
    /// The vendor chain owns the interface and its supplicant holds the socket.
    /// Supervising a second one can only crash-loop, so stand down for this boot.
    Vendor,
    /// Nothing holds the socket. Let the supervised service keep retrying — it
    /// is the only remaining route back to a link.
    Unowned,
}

/// P2: system setup. Every step is best-effort — a camera with no sensor
/// module is still worth reaching over SSH to diagnose.
///
/// Returns who owns the supplicant control socket, so P3 can either take it
/// over with the probed `-D` flag (F2/F3) or stand down when the vendor chain
/// already holds it.
pub fn system_setup(
    sys: &dyn Sys,
    cfg: &Config,
    baseline_wifi: &WifiCfg,
    overlay_path: &std::path::Path,
) -> SupplicantOwnership {
    // Affects this process only. `gergehack.sh:358` exported TZ for children to
    // inherit; `Sys::spawn` calls `env_clear()`, so a service sees TZ only if
    // its own `[services.X].env` declares it. Kept because it costs nothing and
    // makes any libc time formatting inside the supervisor correct — but do not
    // read this line as "services run in the configured timezone". They do not.
    // (onvif-rust does not care either way: it hardcodes `tz: "UTC"` at
    // onvif/device/ops/system.rs:191.)
    //
    // SAFETY: set_var is not thread-safe, and P2 runs before any thread is
    // started. Do not move this call after P3.
    unsafe { std::env::set_var("TZ", &cfg.time.timezone) };
    tracing::info!(tz = %cfg.time.timezone, "timezone set (supervisor process only)");

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

    let probed = match crate::wifi::bring_up_with_overlay(
        sys,
        &cfg.wifi,
        baseline_wifi,
        &cfg.supervisor.storm_guard_state,
        &crate::wifi::FsLayout::production(),
        overlay_path,
    ) {
        crate::wifi::Outcome::Up {
            chip,
            ref ssid,
            ref addr,
            driver,
        } => {
            tracing::info!(chip, ssid, addr, driver, "wifi up");
            SupplicantOwnership::Ours(driver)
        }
        crate::wifi::Outcome::FellBack => {
            tracing::error!("wifi came up via the vendor fallback; check the chip dispatch");
            SupplicantOwnership::Vendor
        }
        crate::wifi::Outcome::Failed => {
            tracing::error!("wifi is down and the fallback is disabled");
            SupplicantOwnership::Unowned
        }
    };

    // The P0 wrapper started telnetd on port 24 before config was readable.
    // Only now can we honour the setting.
    if !cfg.system.telnet {
        let _ = sys.run_to_completion("killall", &["telnetd".to_string()]);
        tracing::info!("telnetd disabled per config");
    }
    if !cfg.system.ftp {
        // tcpsvd is the vendor's FTP server, started at rc.local:14.
        let _ = sys.run_to_completion("killall", &["tcpsvd".to_string()]);
        tracing::info!("ftp disabled per config");
    }

    probed
}

#[cfg(test)]
mod wifi_tests {
    use super::*;
    use crate::config::{LogCfg, MonitorCfg, RebootCfg, SupervisorCfg, SystemCfg, TimeCfg};
    use crate::sys::{ExitStatus, MockSys, SysError};
    use std::collections::BTreeMap;

    const SAMPLE: &str = "\
[wlan]
ssid = oldnet
password = oldpass
channel = 6
";

    fn wifi_cfg(config_file: &str, ssid: &str, password: &str) -> WifiCfg {
        WifiCfg {
            ssid: ssid.into(),
            password: password.into(),
            config_file: config_file.into(),
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

    fn test_config(wifi: WifiCfg, system: SystemCfg) -> Config {
        Config {
            schema: 0,
            log: LogCfg::default(),
            system,
            wifi,
            time: TimeCfg::default(),
            supervisor: SupervisorCfg::default(),
            monitor: MonitorCfg::default(),
            reboot: RebootCfg::default(),
            update: crate::config::Update::default(),
            services: BTreeMap::new(),
        }
    }

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
    fn test_rewrite_appends_missing_ssid_and_password() {
        let src = "[wlan]\nchannel = 6\n";
        let out = rewrite_wifi_cfg(src, "newnet", "newpass");
        assert!(out.contains("ssid = newnet"));
        assert!(out.contains("password = newpass"));
        assert!(!needs_wifi_update(&out, "newnet", "newpass"));
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

    #[test]
    fn test_apply_wifi_updates_file_and_creates_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anyka_cfg.ini");
        std::fs::write(&path, SAMPLE).expect("write original");
        let cfg = wifi_cfg(path.to_str().expect("utf8 path"), "newnet", "newpass");

        let changed = apply_wifi(&cfg).expect("apply_wifi should succeed");
        assert!(changed);

        let updated = std::fs::read_to_string(&path).expect("read updated");
        assert!(updated.contains("ssid = newnet"));
        assert!(updated.contains("password = newpass"));

        let backup_path = format!("{}.old", path.to_str().unwrap());
        let backup = std::fs::read_to_string(backup_path).expect("read backup");
        assert_eq!(backup, SAMPLE, "backup must hold the pre-update content");
    }

    #[test]
    fn test_apply_wifi_is_a_noop_when_credentials_are_already_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anyka_cfg.ini");
        let current = rewrite_wifi_cfg(SAMPLE, "samenet", "samepass");
        std::fs::write(&path, &current).expect("write original");
        let cfg = wifi_cfg(path.to_str().expect("utf8 path"), "samenet", "samepass");

        let changed = apply_wifi(&cfg).expect("apply_wifi should succeed");
        assert!(!changed);

        assert_eq!(
            std::fs::read_to_string(&path).expect("read back"),
            current,
            "file must be untouched"
        );
        let backup_path = format!("{}.old", path.to_str().unwrap());
        assert!(
            !std::path::Path::new(&backup_path).exists(),
            "no-op must not create a backup"
        );
    }

    #[test]
    fn test_apply_wifi_restores_backup_when_verification_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anyka_cfg.ini");
        std::fs::write(&path, SAMPLE).expect("write original");
        // A leading/trailing space in the ssid survives the write but is
        // trimmed away on readback (`needs_wifi_update` trims `val`), so the
        // post-write verification always disagrees with what was written.
        // That deterministically exercises the restore-on-verify-failure path
        // without needing to inject filesystem failures.
        let cfg = wifi_cfg(path.to_str().expect("utf8 path"), " test ", "newpass");

        let err = apply_wifi(&cfg).expect_err("mismatched readback must fail verification");
        assert!(format!("{err}").contains("verification"));

        let restored = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(restored, SAMPLE, "backup must be restored byte-for-byte");
    }

    #[test]
    fn test_write_panic_sysctls_writes_both_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let kernel = dir.path().join("kernel");
        std::fs::create_dir_all(&kernel).expect("mkdir");
        std::fs::write(kernel.join("panic"), "0\n").expect("seed panic");
        std::fs::write(kernel.join("panic_on_oops"), "0\n").expect("seed oops");

        write_panic_sysctls(dir.path());

        assert_eq!(
            std::fs::read_to_string(kernel.join("panic")).expect("read panic"),
            "10"
        );
        assert_eq!(
            std::fs::read_to_string(kernel.join("panic_on_oops")).expect("read oops"),
            "1"
        );
    }

    #[test]
    fn test_write_panic_sysctls_does_not_panic_when_the_knobs_are_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        // No kernel/ subdirectory at all — must not panic.
        write_panic_sysctls(dir.path());
    }

    #[test]
    fn test_protect_from_oom_killer_writes_the_minimum_score() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("oom_score_adj");
        std::fs::write(&path, "0\n").expect("seed");

        protect_from_oom_killer(&path);

        assert_eq!(std::fs::read_to_string(&path).expect("read"), "-1000");
    }

    /// Regression for the 2026-08-23 `.127` outage. When bring-up hands off to
    /// the vendor chain, its supplicant owns the ctrl socket, so the result must
    /// be `Vendor` and not be collapsed into the same "nothing probed" case as
    /// an outright failure. `main` keys the stand-down off exactly this
    /// distinction; without it the supervised service exits 255 on every restart
    /// until the crash-loop cap reboots the camera into SAFE MODE.
    #[test]
    fn test_system_setup_reports_vendor_ownership_when_it_falls_back() {
        let mut sys = MockSys::new();
        sys.expect_insmod().returning(|_| Ok(()));
        sys.expect_run_to_completion()
            .returning(|_, _| Ok(ExitStatus::Code(0)));

        let mut wifi = wifi_cfg("/nonexistent/anyka_cfg.ini", "net", "pass");
        wifi.chip = "unknown_chip".into();
        // The one bit that differs from the failure case below.
        wifi.fallback_to_vendor = true;
        let system = SystemCfg {
            sensor_module: Some("sensor.ko".into()),
            ..Default::default()
        };
        let cfg = test_config(wifi, system);

        let dir = tempfile::tempdir().expect("tempdir");
        let overlay_path = dir.path().join("network.toml");

        assert_eq!(
            system_setup(&sys, &cfg, &cfg.wifi, &overlay_path),
            SupplicantOwnership::Vendor,
            "vendor fallback must be distinguishable from an outright failure"
        );
    }

    #[test]
    fn test_system_setup_loads_sensor_kills_disabled_services_and_reports_wifi_failure() {
        let mut sys = MockSys::new();
        sys.expect_insmod()
            .withf(|module| module == "sensor.ko")
            .times(1)
            .returning(|_| Ok(()));
        sys.expect_run_to_completion()
            .withf(|prog, args| prog == "killall" && args == ["telnetd".to_string()])
            .times(1)
            .returning(|_, _| Ok(ExitStatus::Code(0)));

        let mut wifi = wifi_cfg("/nonexistent/anyka_cfg.ini", "net", "pass");
        wifi.chip = "unknown_chip".into();
        wifi.fallback_to_vendor = false;
        // telnet=false (default) drives the killall telnetd expectation above.
        // ftp stays at its default `true`, so killall tcpsvd must not fire.
        let system = SystemCfg {
            sensor_module: Some("sensor.ko".into()),
            ..SystemCfg::default()
        };
        let cfg = test_config(wifi, system);

        let dir = tempfile::tempdir().expect("tempdir");
        let overlay_path = dir.path().join("network.toml");
        let probed = system_setup(&sys, &cfg, &cfg.wifi, &overlay_path);
        assert_eq!(
            probed,
            SupplicantOwnership::Unowned,
            "unknown chip with fallback disabled must fail, leaving the socket unowned"
        );
    }

    #[test]
    fn test_system_setup_logs_sensor_failure_and_kills_ftp_when_disabled() {
        let mut sys = MockSys::new();
        sys.expect_insmod()
            .times(1)
            .returning(|_| Err(SysError::Other("no such device".into())));
        sys.expect_run_to_completion()
            .withf(|prog, args| prog == "killall" && args == ["tcpsvd".to_string()])
            .times(1)
            .returning(|_, _| Ok(ExitStatus::Code(0)));

        let mut wifi = wifi_cfg("/nonexistent/anyka_cfg.ini", "net", "pass");
        wifi.chip = "unknown_chip".into();
        wifi.fallback_to_vendor = false;
        let system = SystemCfg {
            sensor_module: Some("sensor.ko".into()),
            telnet: true, // suppress the killall telnetd expectation
            ftp: false,   // drives the killall tcpsvd expectation above
        };
        let cfg = test_config(wifi, system);

        let dir = tempfile::tempdir().expect("tempdir");
        let overlay_path = dir.path().join("network.toml");
        let probed = system_setup(&sys, &cfg, &cfg.wifi, &overlay_path);
        assert_eq!(
            probed,
            SupplicantOwnership::Unowned,
            "unknown chip with fallback disabled must fail, leaving the socket unowned"
        );
    }
}
