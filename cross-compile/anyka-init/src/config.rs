//! Typed configuration, parsed from `/mnt/anyka_hack/anyka.toml`.
//!
//! This file is *parsed*, never evaluated. The predecessor
//! (`gergesettings.txt`) was `.`-sourced by `gergehack.sh`, which made any SD
//! card an unsandboxed root code-execution vector at boot.
//!
//! `deny_unknown_fields` everywhere is deliberate: a typo'd key in a config a
//! user edits by hand on an SD card must be a loud failure, not a silent
//! fallback to a default they did not intend.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub log: LogCfg,
    #[serde(default)]
    pub system: SystemCfg,
    pub wifi: WifiCfg,
    #[serde(default)]
    pub time: TimeCfg,
    #[serde(default)]
    pub supervisor: SupervisorCfg,
    #[serde(default)]
    pub monitor: MonitorCfg,
    #[serde(default)]
    pub reboot: RebootCfg,
    #[serde(default)]
    pub services: BTreeMap<String, ServiceCfg>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogCfg {
    #[serde(default = "d_log_dir")]
    pub dir: String,
    #[serde(default = "d_log_level")]
    pub level: String,
    #[serde(default = "d_log_max_bytes")]
    pub max_bytes: u64,
    #[serde(default = "d_log_keep")]
    pub keep: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemCfg {
    /// Sensor kernel module to load.
    ///
    /// Load-bearing despite `camera.sh:37-38` also loading sensor modules:
    /// the hack ships its module at `/data/sensor/`, which is on *none* of
    /// camera.sh's three search paths (`/etc/jffs2`, `/usr/modules`,
    /// `/data/sensor_ko_and_isp_conf`). Do not delete this as a duplicate.
    #[serde(default)]
    pub sensor_module: Option<String>,
    /// Keep the P0 recovery telnetd running after boot.
    #[serde(default)]
    pub telnet: bool,
    /// Keep the vendor's FTP server (`rc.local:14`) running.
    #[serde(default = "d_true")]
    pub ftp: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WifiCfg {
    pub ssid: String,
    pub password: String,
    #[serde(default = "d_wifi_cfg_file")]
    pub config_file: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    #[serde(default = "d_ntp_servers")]
    pub servers: Vec<String>,
    #[serde(default = "d_timezone")]
    pub timezone: String,
    #[serde(default = "d_first_sync_timeout")]
    pub first_sync_timeout_sec: u64,
    #[serde(default = "d_retry_interval")]
    pub retry_interval_sec: u64,
    #[serde(default = "d_resync_interval")]
    pub resync_interval_sec: u64,
    #[serde(default = "d_step_threshold")]
    pub step_threshold_sec: u64,
    #[serde(default = "d_min_plausible")]
    pub min_plausible_unix: u64,
    #[serde(default = "d_max_plausible")]
    pub max_plausible_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupervisorCfg {
    #[serde(default = "d_backoff_min")]
    pub backoff_min_sec: u64,
    #[serde(default = "d_backoff_max")]
    pub backoff_max_sec: u64,
    #[serde(default = "d_crashloop_count")]
    pub crashloop_count: u32,
    #[serde(default = "d_crashloop_window")]
    pub crashloop_window_sec: u64,
    #[serde(default = "d_storm_max")]
    pub storm_guard_max_reboots: u8,
    #[serde(default = "d_storm_state")]
    pub storm_guard_state: String,
    #[serde(default = "d_storm_reset_uptime")]
    pub storm_guard_reset_uptime_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    #[serde(default = "d_monitor_interval")]
    pub interval_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RebootCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "d_reboot_interval")]
    pub interval_min: u64,
    #[serde(default)]
    pub jitter_max_sec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    pub exec: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Injected verbatim into the child after clearing its environment.
    ///
    /// This is the structural fix for the loader-poisoning bug documented in
    /// `SD_card_contents/anyka_hack/onvif/onvif-rust`: two incompatible uClibc
    /// versions coexist on this device, and an inherited `LD_LIBRARY_PATH`
    /// breaks every busybox applet a service starts.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub log: String,
    #[serde(default)]
    pub core_dump: bool,
}

fn d_true() -> bool {
    true
}
fn d_log_dir() -> String {
    "/mnt/logs".into()
}
fn d_log_level() -> String {
    "info".into()
}
fn d_log_max_bytes() -> u64 {
    2_000_000
}
fn d_log_keep() -> u8 {
    2
}
fn d_wifi_cfg_file() -> String {
    "/etc/jffs2/anyka_cfg.ini".into()
}
fn d_ntp_servers() -> Vec<String> {
    vec![
        "0.ubuntu.pool.ntp.org".into(),
        "1.ubuntu.pool.ntp.org".into(),
    ]
}
fn d_timezone() -> String {
    "GMT+00:00".into()
}
fn d_first_sync_timeout() -> u64 {
    15
}
fn d_retry_interval() -> u64 {
    30
}
fn d_resync_interval() -> u64 {
    21_600
}
fn d_step_threshold() -> u64 {
    2
}
fn d_min_plausible() -> u64 {
    1_767_225_600
} // 2026-01-01
fn d_max_plausible() -> u64 {
    2_524_608_000
} // 2050-01-01
fn d_backoff_min() -> u64 {
    1
}
fn d_backoff_max() -> u64 {
    60
}
fn d_crashloop_count() -> u32 {
    10
}
fn d_crashloop_window() -> u64 {
    600
}
fn d_storm_max() -> u8 {
    3
}
fn d_storm_state() -> String {
    "/mnt/anyka_hack/state/boot.json".into()
}
fn d_storm_reset_uptime() -> u64 {
    600
}
fn d_monitor_interval() -> u64 {
    60
}
fn d_reboot_interval() -> u64 {
    720
}

impl Default for LogCfg {
    fn default() -> Self {
        Self {
            dir: d_log_dir(),
            level: d_log_level(),
            max_bytes: d_log_max_bytes(),
            keep: d_log_keep(),
        }
    }
}
impl Default for SystemCfg {
    fn default() -> Self {
        Self {
            sensor_module: None,
            telnet: false,
            ftp: true,
        }
    }
}
impl Default for TimeCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: d_ntp_servers(),
            timezone: d_timezone(),
            first_sync_timeout_sec: d_first_sync_timeout(),
            retry_interval_sec: d_retry_interval(),
            resync_interval_sec: d_resync_interval(),
            step_threshold_sec: d_step_threshold(),
            min_plausible_unix: d_min_plausible(),
            max_plausible_unix: d_max_plausible(),
        }
    }
}
impl Default for SupervisorCfg {
    fn default() -> Self {
        Self {
            backoff_min_sec: d_backoff_min(),
            backoff_max_sec: d_backoff_max(),
            crashloop_count: d_crashloop_count(),
            crashloop_window_sec: d_crashloop_window(),
            storm_guard_max_reboots: d_storm_max(),
            storm_guard_state: d_storm_state(),
            storm_guard_reset_uptime_sec: d_storm_reset_uptime(),
        }
    }
}
impl Default for MonitorCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_sec: d_monitor_interval(),
        }
    }
}
impl Default for RebootCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_min: d_reboot_interval(),
            jitter_max_sec: 0,
        }
    }
}

impl std::str::FromStr for Config {
    type Err = ConfigError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str(src)?)
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let src = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg: Self = src.parse()?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.supervisor.backoff_min_sec > self.supervisor.backoff_max_sec {
            return Err(ConfigError::Invalid(
                "supervisor.backoff_min_sec exceeds backoff_max_sec".into(),
            ));
        }
        if self.time.min_plausible_unix >= self.time.max_plausible_unix {
            return Err(ConfigError::Invalid(
                "time.min_plausible_unix must be below max_plausible_unix".into(),
            ));
        }
        if self.time.enabled && self.time.servers.is_empty() {
            return Err(ConfigError::Invalid(
                "time.enabled is true but time.servers is empty".into(),
            ));
        }
        for (name, svc) in &self.services {
            if svc.exec.is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "services.{name}.exec is empty"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const MINIMAL: &str = r#"
[wifi]
ssid = "testnet"
password = "secret"
"#;

    #[test]
    fn test_config_parse_minimal_applies_defaults() {
        let cfg = Config::from_str(MINIMAL).expect("minimal config must parse");
        assert_eq!(cfg.wifi.ssid, "testnet");
        assert_eq!(cfg.log.dir, "/mnt/logs");
        assert_eq!(cfg.supervisor.backoff_min_sec, 1);
        assert_eq!(cfg.supervisor.backoff_max_sec, 60);
        assert_eq!(cfg.supervisor.crashloop_count, 10);
        assert_eq!(cfg.time.resync_interval_sec, 21_600);
        assert!(cfg.services.is_empty());
    }

    #[test]
    fn test_config_parse_rejects_unknown_key() {
        let src = format!("{MINIMAL}\n[system]\nnot_a_real_key = 1\n");
        let err = Config::from_str(&src).expect_err("unknown key must be rejected");
        assert!(
            format!("{err}").contains("not_a_real_key"),
            "error should name the offending key, got: {err}"
        );
    }

    #[test]
    fn test_config_parse_rejects_wrong_type() {
        let src = format!("{MINIMAL}\n[supervisor]\nbackoff_min_sec = \"soon\"\n");
        assert!(Config::from_str(&src).is_err());
    }

    #[test]
    fn test_config_parse_rejects_missing_wifi() {
        assert!(Config::from_str("[log]\nlevel = \"info\"\n").is_err());
    }

    #[test]
    fn test_config_parse_rejects_shell_syntax() {
        // The old gergesettings.txt format must not silently parse as TOML.
        assert!(Config::from_str("run_ssh=1\nwifi_ssid=kmk\n").is_err());
    }

    #[test]
    fn test_config_parse_service_table() {
        let src = format!(
            r#"{MINIMAL}
[services.vendor-daemon]
enabled = true
exec = "/mnt/anyka_hack/vendor-daemon/vendor-daemon.bin"
log = "/mnt/logs/vendor_daemon.log"
core_dump = true
env = {{ LD_LIBRARY_PATH = "/mnt/anyka_hack/vendor-daemon/lib" }}
"#
        );
        let cfg = Config::from_str(&src).expect("service table must parse");
        let svc = cfg.services.get("vendor-daemon").expect("service present");
        assert!(svc.enabled);
        assert!(svc.core_dump);
        assert!(svc.args.is_empty());
        assert_eq!(
            svc.env.get("LD_LIBRARY_PATH").map(String::as_str),
            Some("/mnt/anyka_hack/vendor-daemon/lib")
        );
    }

    #[test]
    fn test_config_validate_rejects_backoff_min_above_max() {
        let src = format!("{MINIMAL}\n[supervisor]\nbackoff_min_sec = 90\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_implausible_time_bounds() {
        let src = format!("{MINIMAL}\n[time]\nmin_plausible_unix = 99\nmax_plausible_unix = 98\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());
    }
}
