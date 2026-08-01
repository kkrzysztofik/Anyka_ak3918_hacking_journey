//! P2 system setup: timezone, sensor module, wifi credentials, service kills.

use crate::config::{Config, WifiCfg};
use crate::sys::Sys;

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

/// P2: system setup. Every step is best-effort — a camera with no sensor
/// module is still worth reaching over SSH to diagnose.
pub fn system_setup(sys: &dyn Sys, cfg: &Config) {
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

    match sys.run_to_completion("/usr/sbin/wifi_manage.sh", &["start".to_string()]) {
        Ok(st) => tracing::info!(?st, "wifi_manage.sh start"),
        Err(e) => tracing::warn!(error = %e, "wifi_manage.sh failed"),
    }

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
}

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
