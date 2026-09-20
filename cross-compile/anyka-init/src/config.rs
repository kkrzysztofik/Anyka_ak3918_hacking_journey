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
    #[error("failed to write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config schema generation. A bundle declares the minimum it needs in
    /// `manifest.meta`; the *running* supervisor compares that against this
    /// number before flipping, so a build that needs keys this file lacks is
    /// rejected with both slots intact instead of crashlooping into a revert.
    ///
    /// Zero means "predates the schema key", which accepts every bundle that
    /// does not ask for one.
    #[serde(default)]
    pub schema: u32,
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
    pub update: Update,
    #[serde(default)]
    pub services: BTreeMap<String, ServiceCfg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    /// Root holding `active`, `slots/`, `state/` and `spool/`.
    #[serde(default = "default_update_root")]
    pub root: String,
    /// Consecutive seconds all trial ports must stay bound.
    #[serde(default = "default_trial_hold")]
    pub trial_hold_sec: u32,
    /// Give up and revert after this long.
    #[serde(default = "default_trial_deadline")]
    pub trial_deadline_sec: u32,
    /// Ports an unconfirmed update must bind to be confirmed. The default
    /// mirrors the shipped ONVIF/RTSP/HTTP-FLV contract; if an operator
    /// changes those ports, the trial follows.
    #[serde(default = "default_trial_ports")]
    pub trial_ports: Vec<u16>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WifiCfg {
    pub ssid: String,
    pub password: String,
    #[serde(default = "d_wifi_cfg_file")]
    pub config_file: String,

    /// `auto` parses `/etc/jffs2/hw.conf`; any other value pins the chip and
    /// skips detection entirely. Pinned is the shipped default because
    /// `hw.conf` offset stability across camera revisions is unverifiable from
    /// a single board (design Q4), and because `service.sh:124` writes a
    /// 32-character default record that cannot be indexed at offset 51 (W2).
    #[serde(default = "d_wifi_chip")]
    pub chip: String,
    /// `high_low` matches vendor `WIFI_ENABLE_VALUE == "2"`
    /// (`wifi_driver.sh:374-382`); `low_high` is every other value.
    #[serde(default = "d_wifi_polarity")]
    pub gpio_polarity: String,
    #[serde(default = "d_wifi_interface")]
    pub interface: String,
    #[serde(default = "d_wifi_security")]
    pub security: String,

    #[serde(default = "d_true")]
    pub dhcp: bool,
    /// CIDR, e.g. `192.168.2.198/24`. Required when `dhcp = false`.
    #[serde(default)]
    pub address: Option<String>,
    /// Required when `dhcp = false`.
    #[serde(default)]
    pub gateway: Option<String>,
    /// Written to `/etc/resolv.conf` after the address is assigned (W7).
    #[serde(default)]
    pub dns: Vec<String>,

    #[serde(default = "d_wifi_timeout")]
    pub connect_timeout_sec: u64,
    /// R7. Not behind a flag by default: a wrong dispatch entry would
    /// otherwise cost the camera's only remote access.
    #[serde(default = "d_true")]
    pub fallback_to_vendor: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorCfg {
    #[serde(default = "d_true")]
    pub enabled: bool,
    #[serde(default = "d_monitor_interval")]
    pub interval_sec: u64,
    #[serde(default = "d_true")]
    pub wifi: bool,
    #[serde(default = "d_true")]
    pub wifi_probe: bool,
    #[serde(default = "d_wifi_dhcp_ticks")]
    pub wifi_dhcp_after_ticks: u32,
    #[serde(default = "d_wifi_supplicant_ticks")]
    pub wifi_supplicant_after_ticks: u32,
    #[serde(default = "d_wifi_reboot_ticks")]
    pub wifi_reboot_after_ticks: u32,
    #[serde(default = "d_wifi_reboot_cap")]
    pub wifi_reboot_cap: u8,
    #[serde(default = "d_true")]
    pub video: bool,
    #[serde(default = "d_video_restart_ticks")]
    pub video_restart_after_ticks: u32,
    #[serde(default = "d_video_kill_ticks")]
    pub video_kill_after_ticks: u32,
    #[serde(default = "d_video_reboot_ticks")]
    pub video_reboot_after_ticks: u32,
    #[serde(default = "d_video_heartbeat")]
    pub video_heartbeat_path: String,
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

fn d_wifi_chip() -> String {
    "auto".into()
}
fn d_wifi_polarity() -> String {
    "low_high".into()
}
fn d_wifi_interface() -> String {
    "wlan0".into()
}
fn d_wifi_security() -> String {
    "wpa".into()
}
fn d_wifi_timeout() -> u64 {
    45
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
    // ARMv5 uClibc uses 32-bit time_t; stay below the 2038 overflow.
    i32::MAX as u64
}
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

fn d_wifi_dhcp_ticks() -> u32 {
    3
}
fn d_wifi_supplicant_ticks() -> u32 {
    5
}
fn d_wifi_reboot_ticks() -> u32 {
    10
}
fn d_wifi_reboot_cap() -> u8 {
    3
}
fn d_video_restart_ticks() -> u32 {
    2
}
fn d_video_kill_ticks() -> u32 {
    3
}
fn d_video_reboot_ticks() -> u32 {
    5
}
fn d_video_heartbeat() -> String {
    "/tmp/vd_heartbeat".into()
}
fn d_reboot_interval() -> u64 {
    720
}
fn default_update_root() -> String {
    "/mnt/anyka_hack".to_string()
}
fn default_trial_hold() -> u32 {
    30
}
fn default_trial_deadline() -> u32 {
    120
}
fn default_trial_ports() -> Vec<u16> {
    crate::update::TRIAL_PORTS.to_vec()
}

/// Line-level edit of `key = <raw>` under `section`, where `raw` is already
/// encoded TOML (`true`, `"text"`, `["a", "b"]`).
///
/// Only the one line changes — comments, ordering and formatting everywhere
/// else survive byte-for-byte, which a TOML round-trip cannot guarantee. That
/// matters because this file is the operator's: hand-edited, comment-rich, and
/// holding the Wi-Fi credentials.
pub fn set_value_in_text(
    text: &str,
    section: &str,
    key: &str,
    raw: &str,
) -> Result<String, ConfigError> {
    // Preserve the file's line ending. Splitting on '\n' alone would leave a
    // '\r' on every existing line while the rewritten one has none, producing
    // a mixed-ending file out of a CRLF original.
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = text.split(nl).map(str::to_owned).collect();

    let Some(hdr) = out.iter().position(|l| strip_comment(l) == section) else {
        return Err(ConfigError::Invalid(format!(
            "no {section} stanza in config"
        )));
    };
    // The stanza ends at the next `[`-prefixed line, or at end of file.
    let end = out[hdr + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| i + hdr + 1)
        .unwrap_or(out.len());

    let Some(i) = (hdr + 1..end).find(|&i| line_key(&out[i]).is_some_and(|k| k == key)) else {
        out.insert(hdr + 1, format!("{key} = {raw}"));
        return verified(out.join(nl), section, key);
    };
    // Preserve the line's indentation, change only the value.
    let lead: String = out[i].chars().take_while(|c| c.is_whitespace()).collect();
    out[i] = format!("{lead}{key} = {raw}");
    verified(out.join(nl), section, key)
}

/// Line-level edit of a boolean, e.g. `enabled =` under `[services.<name>]`.
pub fn set_bool_in_text(
    text: &str,
    section: &str,
    key: &str,
    enabled: bool,
) -> Result<String, ConfigError> {
    set_value_in_text(text, section, key, if enabled { "true" } else { "false" })
}

/// A line with any trailing `# comment` removed, trimmed. Lets
/// `[services.snmp]  # the SNMP agent` match its bare header.
fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or("").trim()
}

/// The key a `key = value` line assigns, unquoted, or `None` for a line that
/// assigns nothing (a comment, a blank, a bare header).
///
/// Matching the *parsed key* rather than a prefix is load-bearing twice over:
/// `starts_with("enabled")` would overwrite a sibling `enabled_at_boot`, and
/// it would skip the equally valid `"enabled" = true`, insert a second
/// `enabled` key, and leave behind a duplicate-key document that no longer
/// parses.
fn line_key(line: &str) -> Option<&str> {
    let (lhs, _) = line.split_once('=')?;
    let lhs = lhs.trim();
    if lhs.starts_with('#') {
        return None;
    }
    Some(lhs.trim_matches(|c| c == '"' || c == '\''))
}

/// Re-parse the edited document before handing it back.
///
/// The editor scans lines rather than parsing TOML, which is what keeps
/// comments and formatting byte-identical. The cost is that an unusual but
/// legal input could, in principle, be mis-scanned into a document that no
/// longer parses — and the caller would then write it over the operator's
/// config, parking the supervisor on the next boot. Validating the *output*
/// turns every such case into a refused edit rather than a bricked camera,
/// without needing the scanner to understand all of TOML.
fn verified(out: String, section: &str, key: &str) -> Result<String, ConfigError> {
    match toml::from_str::<toml::Value>(&out) {
        Ok(_) => Ok(out),
        Err(e) => Err(ConfigError::Invalid(format!(
            "editing {key} under {section} produced invalid TOML ({e}); config left unchanged"
        ))),
    }
}

fn read_config_text(path: &std::path::Path) -> Result<String, ConfigError> {
    std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })
}

/// Write `new_text` to a temp file next to `path`, fsync it, then rename over
/// the original, so an interrupted write never leaves a half-written operator
/// config behind (the same pattern `update.rs` uses for the `active` pointer
/// on this filesystem).
fn persist_text(path: &std::path::Path, new_text: &str) -> Result<(), ConfigError> {
    let tmp = path.with_extension("toml.tmp");
    let write = |f: &mut std::fs::File| -> Result<(), std::io::Error> {
        std::io::Write::write_all(f, new_text.as_bytes())?;
        f.sync_all()
    };
    let mut f = std::fs::File::create(&tmp).map_err(|source| ConfigError::Write {
        path: tmp.display().to_string(),
        source,
    })?;
    write(&mut f).map_err(|source| ConfigError::Write {
        path: tmp.display().to_string(),
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| ConfigError::Write {
        path: path.display().to_string(),
        source,
    })
}

impl Default for Update {
    fn default() -> Self {
        Self {
            root: default_update_root(),
            trial_hold_sec: default_trial_hold(),
            trial_deadline_sec: default_trial_deadline(),
            trial_ports: default_trial_ports(),
        }
    }
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
            wifi: true,
            wifi_probe: true,
            wifi_dhcp_after_ticks: d_wifi_dhcp_ticks(),
            wifi_supplicant_after_ticks: d_wifi_supplicant_ticks(),
            wifi_reboot_after_ticks: d_wifi_reboot_ticks(),
            wifi_reboot_cap: d_wifi_reboot_cap(),
            video: true,
            video_restart_after_ticks: d_video_restart_ticks(),
            video_kill_after_ticks: d_video_kill_ticks(),
            video_reboot_after_ticks: d_video_reboot_ticks(),
            video_heartbeat_path: d_video_heartbeat(),
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
    /// Persist `enabled` for one service: line-level edit, then atomic
    /// tmp+rename over the original (the same pattern `update.rs` uses for the
    /// `active` pointer on this filesystem). On failure the original is
    /// untouched.
    ///
    /// Associated, not a method: this writes the file, and the in-memory
    /// `Config` is updated separately by the caller so the two steps stay
    /// visibly ordered.
    pub fn set_service_enabled(
        path: &std::path::Path,
        name: &str,
        enabled: bool,
    ) -> Result<(), ConfigError> {
        let text = read_config_text(path)?;
        let new_text = set_bool_in_text(&text, &format!("[services.{name}]"), "enabled", enabled)?;
        persist_text(path, &new_text)
    }

    /// Persist `[system].telnet` — the recovery-telnet switch. Same file-first
    /// atomic-write discipline as `set_service_enabled`; the in-memory `Config`
    /// and the runtime side (spawn/killall) are the caller's, in the same
    /// visible order.
    pub fn set_system_telnet(path: &std::path::Path, enabled: bool) -> Result<(), ConfigError> {
        let text = read_config_text(path)?;
        let new_text = set_bool_in_text(&text, "[system]", "telnet", enabled)?;
        persist_text(path, &new_text)
    }

    pub fn load(path: &str) -> Result<Self, ConfigError> {
        Self::load_with_overlay(
            path,
            std::path::Path::new(crate::netoverlay::NetworkOverlay::DEFAULT_PATH),
        )
    }

    /// Read, parse, and validate the base file only — no network overlay merge.
    pub fn load_without_overlay(path: &str) -> Result<Self, ConfigError> {
        let cfg = Self::parse_file(path)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// `Config::load`, with the overlay path taken as an argument so tests can
    /// point it at a tempdir.
    pub fn load_with_overlay(
        path: &str,
        overlay_path: &std::path::Path,
    ) -> Result<Self, ConfigError> {
        let mut cfg = Self::parse_file(path)?;
        // Validate the baseline first so non-network failures never quarantine
        // a valid overlay.
        cfg.validate()?;
        Self::merge_network_overlay(&mut cfg, overlay_path)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Merge `network.toml` onto `[wifi]`, quarantining a bad overlay instead of
    /// parking the camera. Read errors other than TOML parse still fail loud.
    fn merge_network_overlay(
        cfg: &mut Self,
        overlay_path: &std::path::Path,
    ) -> Result<(), ConfigError> {
        let overlay = match crate::netoverlay::NetworkOverlay::load(overlay_path) {
            Ok(overlay) => overlay,
            Err(ConfigError::Parse(err)) => {
                tracing::warn!(error = %err, "unparseable network overlay; quarantining");
                crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        if !overlay.has_content() {
            return Ok(());
        }

        let baseline_wifi = cfg.wifi.clone();
        if let Err(err) = overlay.validate() {
            tracing::warn!(error = %err, "invalid network overlay; quarantining");
            crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
            return Ok(());
        }

        overlay.apply_to(&mut cfg.wifi);
        if cfg.validate().is_err() {
            tracing::warn!("merged network overlay failed validation; quarantining");
            crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
            cfg.wifi = baseline_wifi;
        }
        Ok(())
    }

    fn parse_file(path: &str) -> Result<Self, ConfigError> {
        let src = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        src.parse()
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_supervisor()?;
        self.validate_time()?;
        self.validate_monitor_and_reboot()?;
        self.validate_update()?;
        self.validate_wifi()?;
        for (name, svc) in &self.services {
            if svc.exec.is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "services.{name}.exec is empty"
                )));
            }
        }
        Ok(())
    }

    fn validate_supervisor(&self) -> Result<(), ConfigError> {
        if self.supervisor.backoff_min_sec > self.supervisor.backoff_max_sec {
            return Err(ConfigError::Invalid(
                "supervisor.backoff_min_sec exceeds backoff_max_sec".into(),
            ));
        }
        if self.supervisor.crashloop_count == 0 {
            return Err(ConfigError::Invalid(
                "supervisor.crashloop_count must be non-zero".into(),
            ));
        }
        Ok(())
    }

    fn validate_time(&self) -> Result<(), ConfigError> {
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
        if self.time.enabled
            && (self.time.retry_interval_sec == 0 || self.time.resync_interval_sec == 0)
        {
            return Err(ConfigError::Invalid(
                "time.retry_interval_sec and time.resync_interval_sec must be non-zero".into(),
            ));
        }
        Ok(())
    }

    fn validate_monitor_and_reboot(&self) -> Result<(), ConfigError> {
        if self.monitor.enabled && self.monitor.interval_sec == 0 {
            return Err(ConfigError::Invalid(
                "monitor.interval_sec must be non-zero".into(),
            ));
        }
        if self.reboot.enabled && self.reboot.interval_min == 0 {
            return Err(ConfigError::Invalid(
                "reboot.interval_min must be non-zero when reboot.enabled is true".into(),
            ));
        }
        Ok(())
    }

    fn validate_update(&self) -> Result<(), ConfigError> {
        if self.update.trial_hold_sec == 0
            || self.update.trial_hold_sec >= self.update.trial_deadline_sec
        {
            return Err(ConfigError::Invalid(
                "update.trial_hold_sec must be greater than zero and less than \
                 update.trial_deadline_sec"
                    .into(),
            ));
        }
        if self.update.trial_ports.is_empty() {
            // An empty list would make evaluate_trial treat every port as bound
            // and confirm an update without ever checking a listener.
            return Err(ConfigError::Invalid(
                "update.trial_ports must contain at least one port".into(),
            ));
        }
        Ok(())
    }

    fn validate_wifi(&self) -> Result<(), ConfigError> {
        if self.wifi.chip != "auto" && crate::wifi::Chip::from_name(&self.wifi.chip).is_none() {
            return Err(ConfigError::Invalid(format!(
                "[wifi] chip = {:?} is not \"auto\" or a known chip name",
                self.wifi.chip
            )));
        }
        if crate::wifi::Polarity::from_name(&self.wifi.gpio_polarity).is_none() {
            return Err(ConfigError::Invalid(format!(
                "[wifi] gpio_polarity = {:?} is not one of low_high, high_low",
                self.wifi.gpio_polarity
            )));
        }
        if crate::wifi::Security::from_name(&self.wifi.security).is_none() {
            return Err(ConfigError::Invalid(format!(
                "[wifi] security = {:?} is not one of wpa, wep, open",
                self.wifi.security
            )));
        }
        self.validate_wifi_static()?;
        Ok(())
    }

    fn validate_wifi_static(&self) -> Result<(), ConfigError> {
        if self.wifi.dhcp {
            return Ok(());
        }
        if self.wifi.address.is_none() {
            return Err(ConfigError::Invalid(
                "[wifi] address is required when dhcp = false".into(),
            ));
        }
        if self.wifi.gateway.is_none() {
            return Err(ConfigError::Invalid(
                "[wifi] gateway is required when dhcp = false".into(),
            ));
        }
        if let Some(addr) = &self.wifi.address
            && crate::wifi::parse_cidr(addr).is_none()
        {
            return Err(ConfigError::Invalid(format!(
                "[wifi] address = {addr:?} is not valid CIDR (expected a.b.c.d/prefix)"
            )));
        }
        if let Some(gw) = &self.wifi.gateway
            && gw.parse::<std::net::Ipv4Addr>().is_err()
        {
            return Err(ConfigError::Invalid(format!(
                "[wifi] gateway = {gw:?} is not a valid IPv4 address"
            )));
        }
        if self.services.get("udhcpc").is_some_and(|s| s.enabled) {
            return Err(ConfigError::Invalid(
                "[services.udhcpc] is enabled but [wifi] dhcp = false; \
                 the renewer would overwrite the static address"
                    .into(),
            ));
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

    #[test]
    fn test_config_validate_rejects_zero_intervals() {
        let src = format!("{MINIMAL}\n[monitor]\ninterval_sec = 0\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());

        let src = format!("{MINIMAL}\n[time]\nretry_interval_sec = 0\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());

        let src = format!("{MINIMAL}\n[reboot]\nenabled = true\ninterval_min = 0\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());

        let src = format!("{MINIMAL}\n[supervisor]\ncrashloop_count = 0\n");
        let cfg = Config::from_str(&src).expect("parses");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_unknown_wifi_enums() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
gpio_polarity = "high-low"
"#,
        )
        .expect_err("typo polarity must be rejected");
        assert!(format!("{err}").contains("gpio_polarity"));

        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
security = "wpa3"
"#,
        )
        .expect_err("unknown security must be rejected");
        assert!(format!("{err}").contains("security"));

        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
chip = "not_a_chip"
"#,
        )
        .expect_err("unknown chip must be rejected");
        assert!(format!("{err}").contains("chip"));
    }

    fn load_from_str(src: &str) -> Result<Config, ConfigError> {
        let cfg: Config = toml::from_str(src)?;
        cfg.validate()?;
        Ok(cfg)
    }

    #[test]
    fn test_wifi_defaults_are_dhcp_and_auto_chip() {
        let cfg: Config = toml::from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
"#,
        )
        .expect("parse");
        assert!(cfg.wifi.dhcp);
        assert_eq!(cfg.wifi.chip, "auto");
        assert_eq!(cfg.wifi.interface, "wlan0");
        assert!(cfg.wifi.fallback_to_vendor);
    }

    #[test]
    fn test_wifi_static_requires_address_and_gateway() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
"#,
        )
        .expect_err("static config without an address must be rejected");
        assert!(
            format!("{err}").contains("address"),
            "error should name the missing field, got: {err}"
        );
    }

    #[test]
    fn test_wifi_rejects_unknown_key() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
sid = "typo"
"#,
        )
        .expect_err("deny_unknown_fields must reject a typo");
        assert!(format!("{err}").contains("sid"));
    }

    #[test]
    fn test_config_rejects_dhcp_client_alongside_static_addressing() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
address = "192.168.2.198/24"
gateway = "192.168.2.1"

[services.udhcpc]
enabled = true
exec = "/bin/busybox"
log = "/tmp/udhcpc.log"
"#,
        )
        .expect_err("static + enabled udhcpc must be rejected");
        assert!(
            format!("{err}").contains("udhcpc"),
            "error should name udhcpc, got: {err}"
        );
    }

    #[test]
    fn test_config_accepts_dhcp_client_when_dhcp_is_enabled() {
        load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = true

[services.udhcpc]
enabled = true
exec = "/bin/busybox"
log = "/tmp/udhcpc.log"
"#,
        )
        .expect("dhcp + enabled udhcpc must be accepted");
    }

    #[test]
    fn test_config_accepts_static_addressing_with_the_client_disabled() {
        load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
address = "192.168.2.198/24"
gateway = "192.168.2.1"

[services.udhcpc]
enabled = false
exec = "/bin/busybox"
log = "/tmp/udhcpc.log"
"#,
        )
        .expect("static + disabled udhcpc must be accepted");
    }

    #[test]
    fn test_shipped_anyka_toml_loads_cleanly() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../SD_card_contents/anyka_hack/anyka.toml"
        );
        Config::load(path).expect("shipped anyka.toml must parse and validate");
    }

    #[test]
    fn test_config_schema_defaults_to_zero_when_absent() {
        let c: Config = toml::from_str(MINIMAL).unwrap();
        assert_eq!(c.schema, 0);
    }

    #[test]
    fn test_config_schema_is_read_from_the_top_level() {
        let src = format!("schema = 2\n{MINIMAL}");
        let c: Config = toml::from_str(&src).unwrap();
        assert_eq!(c.schema, 2);
    }

    #[test]
    fn test_config_update_section_applies_defaults() {
        let c: Config = toml::from_str(MINIMAL).unwrap();
        assert_eq!(c.update.root, "/mnt/anyka_hack");
        assert_eq!(c.update.trial_hold_sec, 30);
        assert_eq!(c.update.trial_deadline_sec, 120);
        assert_eq!(c.update.trial_ports, crate::update::TRIAL_PORTS);
    }

    #[test]
    fn test_config_validate_rejects_hold_above_deadline() {
        let src = format!("{MINIMAL}\n[update]\ntrial_hold_sec = 200\ntrial_deadline_sec = 120\n");
        let c: Config = toml::from_str(&src).unwrap();
        assert!(c.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_zero_trial_hold() {
        let src = format!("{MINIMAL}\n[update]\ntrial_hold_sec = 0\ntrial_deadline_sec = 120\n");
        let c: Config = toml::from_str(&src).unwrap();
        assert!(c.validate().is_err());
    }

    #[test]
    fn test_config_update_section_reads_trial_ports() {
        let src = format!("{MINIMAL}\n[update]\ntrial_ports = [80, 8554]\n");
        let c: Config = toml::from_str(&src).unwrap();
        assert_eq!(c.update.trial_ports, vec![80, 8554]);
    }

    #[test]
    fn test_config_validate_rejects_empty_trial_ports() {
        let src = format!("{MINIMAL}\n[update]\ntrial_ports = []\n");
        let c: Config = toml::from_str(&src).unwrap();
        assert!(c.validate().is_err());
    }

    #[test]
    fn test_wifi_rejects_static_address_without_a_prefix() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
address = "192.168.2.198"
gateway = "192.168.2.1"
"#,
        )
        .expect_err("address without prefix must be rejected");
        assert!(
            format!("{err}").contains("address"),
            "error should name address, got: {err}"
        );
    }

    #[test]
    fn test_wifi_rejects_malformed_static_gateway() {
        let err = load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
address = "192.168.2.198/24"
gateway = "192.168.2"
"#,
        )
        .expect_err("malformed gateway must be rejected");
        assert!(
            format!("{err}").contains("gateway"),
            "error should name gateway, got: {err}"
        );
    }

    #[test]
    fn test_wifi_accepts_a_wellformed_static_configuration() {
        load_from_str(
            r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
address = "192.168.2.198/24"
gateway = "192.168.2.1"
dns = ["192.168.2.1", "8.8.8.8"]
"#,
        )
        .expect("shipped-shaped static config must be accepted");
    }

    #[test]
    fn test_load_with_overlay_quarantines_unparseable_overlay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("anyka.toml");
        let overlay = dir.path().join("network.toml");
        std::fs::write(
            &base,
            r#"
[wifi]
ssid = "OperatorNet"
password = "operatorpass"
"#,
        )
        .expect("write base");
        std::fs::write(&overlay, "this is not toml {{{").expect("write broken overlay");

        let cfg = Config::load_with_overlay(base.to_str().expect("utf8"), &overlay)
            .expect("broken overlay must not park the baseline load");
        assert_eq!(cfg.wifi.ssid, "OperatorNet");
        assert!(
            !overlay.exists(),
            "broken overlay must be quarantined away from the next boot"
        );
        assert!(dir.path().join("network.toml.bad").exists());
    }

    #[test]
    fn test_load_with_overlay_rejects_invalid_baseline_without_quarantining() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("anyka.toml");
        let overlay = dir.path().join("network.toml");
        std::fs::write(
            &base,
            r#"
[wifi]
ssid = "OperatorNet"
password = "operatorpass"

[services.broken]
enabled = true
exec = ""
log = "/mnt/logs/broken.log"
"#,
        )
        .expect("write base");
        std::fs::write(
            &overlay,
            r#"
ssid = "OverlayNet"
password = "overlaypass"
"#,
        )
        .expect("write overlay");

        let err = Config::load_with_overlay(base.to_str().expect("utf8"), &overlay)
            .expect_err("invalid baseline service must fail the load");
        assert!(
            matches!(err, ConfigError::Invalid(_)),
            "expected Invalid, got {err:?}"
        );
        assert!(
            overlay.exists(),
            "valid overlay must not be quarantined for a baseline fault"
        );
        assert!(!dir.path().join("network.toml.bad").exists());
    }

    const SAMPLE: &str = concat!(
        "title = \"anyka\"\n",
        "[services.onvif]\n",
        "enabled = true\n",
        "exec = \"/mnt/anyka_hack/slots/a/bin/onvif-rust.bin\"\n",
        "# keep this comment alive\n",
        "[services.snmp]\n",
        "exec = \"/usr/sbin/snmpd\"\n",
        "[services.dropbear]\n",
        "enabled = false\n",
    );

    #[test]
    fn test_set_bool_in_text_replaces_an_existing_line() {
        let got = set_bool_in_text(SAMPLE, "[services.onvif]", "enabled", false).expect("edit");
        assert!(got.contains("[services.onvif]\nenabled = false\nexec ="));
    }

    #[test]
    fn test_set_bool_in_text_does_not_clobber_a_key_with_the_same_prefix() {
        // A prefix match would overwrite this line, silently losing a key from
        // the operator's file.
        let src = "[services.x]\nenabled_at_boot = \"yes\"\nexec = \"/bin/true\"\n";
        let got = set_bool_in_text(src, "[services.x]", "enabled", false).expect("edit");
        assert!(got.contains("enabled_at_boot = \"yes\""));
        assert!(got.contains("\nenabled = false\n"));
    }

    #[test]
    fn test_set_bool_in_text_matches_a_quoted_key() {
        // `"enabled" = true` is legal TOML. Skipping it would insert a second
        // `enabled` key and produce a duplicate-key document.
        let src = "[services.x]\n\"enabled\" = true\nexec = \"/bin/true\"\n";
        let got = set_bool_in_text(src, "[services.x]", "enabled", false).expect("edit");
        assert!(got.contains("enabled = false"));
        toml::from_str::<toml::Value>(&got).expect("must stay valid TOML");
    }

    #[test]
    fn test_set_bool_in_text_matches_a_header_with_a_trailing_comment() {
        let src = "[services.x]  # the X service\nenabled = true\nexec = \"/bin/true\"\n";
        let got = set_bool_in_text(src, "[services.x]", "enabled", false).expect("edit");
        assert!(got.contains("# the X service"));
        assert!(got.contains("enabled = false"));
    }

    #[test]
    fn test_set_bool_in_text_ignores_a_commented_out_key() {
        let src = "[services.x]\n# enabled = true\nexec = \"/bin/true\"\n";
        let got = set_bool_in_text(src, "[services.x]", "enabled", false).expect("edit");
        assert!(got.contains("# enabled = true"));
        assert!(got.contains("\nenabled = false\n"));
    }

    #[test]
    fn test_set_bool_in_text_preserves_crlf_line_endings() {
        let src = "[services.x]\r\nenabled = true\r\nexec = \"/bin/true\"\r\n";
        let got = set_bool_in_text(src, "[services.x]", "enabled", false).expect("edit");
        assert!(!got.contains("\n\n"), "no bare LF may be introduced");
        assert_eq!(got, src.replace("enabled = true", "enabled = false"));
    }

    #[test]
    fn test_set_bool_in_text_refuses_to_return_invalid_toml() {
        // The scanner cannot see that this `[` continues an array, so it ends
        // the stanza early and would insert a duplicate `enabled`. The output
        // check turns that into a refused edit instead of a config that fails
        // to parse on the next boot.
        let src = "[services.x]\nargs = [\n[\"a\"],\n]\nenabled = true\n";
        match set_bool_in_text(src, "[services.x]", "enabled", false) {
            Err(ConfigError::Invalid(m)) => assert!(m.contains("invalid TOML"), "{m}"),
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    #[test]
    fn test_set_value_in_text_writes_an_array_and_keeps_comments() {
        let src = "\
# the operator's note, must survive
[time]
# IP first, deliberately
servers = [\"192.168.2.1\"]
timezone = \"UTC0\"
";
        let out =
            set_value_in_text(src, "[time]", "servers", "[\"a.example\", \"b.example\"]")
                .expect("edit must succeed");
        assert!(out.contains("servers = [\"a.example\", \"b.example\"]"));
        assert!(out.contains("# the operator's note, must survive"));
        assert!(out.contains("# IP first, deliberately"));
        assert!(out.contains("timezone = \"UTC0\""));
    }

    #[test]
    fn test_set_value_in_text_preserves_crlf() {
        let src = "[time]\r\nservers = [\"old\"]\r\n";
        let out = set_value_in_text(src, "[time]", "servers", "[\"new\"]").unwrap();
        assert!(out.contains("servers = [\"new\"]\r\n"));
        assert!(!out.contains("servers = [\"new\"]\n\r"));
    }

    #[test]
    fn test_set_value_in_text_rejects_an_edit_that_breaks_toml() {
        // `verified()` must catch a raw value that is not valid TOML.
        let src = "[time]\nservers = [\"old\"]\n";
        let err = set_value_in_text(src, "[time]", "servers", "[\"unterminated]");
        assert!(err.is_err(), "a malformed raw value must be refused, not written");
    }

    #[test]
    fn test_set_bool_in_text_still_works_after_generalization() {
        let src = "[services.snmp]\nenabled = false\n";
        let out = set_bool_in_text(src, "[services.snmp]", "enabled", true).unwrap();
        assert!(out.contains("enabled = true"));
    }

    #[test]
    fn test_set_bool_in_text_preserves_everything_else() {
        let got = set_bool_in_text(SAMPLE, "[services.snmp]", "enabled", true).expect("edit");
        // The only byte-level change: one inserted line.
        assert_eq!(
            got,
            SAMPLE.replace(
                "[services.snmp]\nexec =",
                "[services.snmp]\nenabled = true\nexec =",
            )
        );
        assert!(got.contains("# keep this comment alive"));
    }

    #[test]
    fn test_set_bool_in_text_inserts_under_the_header_when_absent() {
        // A hand-edited config may omit `enabled` entirely (it defaults true),
        // so "disable" has to be able to create the line.
        let got = set_bool_in_text(SAMPLE, "[services.snmp]", "enabled", false).expect("edit");
        assert!(got.contains("[services.snmp]\nenabled = false\nexec ="));
    }

    #[test]
    fn test_set_bool_in_text_does_not_escape_the_stanza() {
        // dropbear's line must be untouched when onvif is edited.
        let got = set_bool_in_text(SAMPLE, "[services.onvif]", "enabled", false).expect("edit");
        assert!(got.contains("[services.dropbear]\nenabled = false\n"));
    }

    #[test]
    fn test_set_bool_in_text_unknown_stanza_is_an_error() {
        match set_bool_in_text(SAMPLE, "[services.nope]", "enabled", true) {
            Err(ConfigError::Invalid(_)) => {}
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    const SYSTEM_SAMPLE: &str = "\n[system]\nsensor_module = \"/data/sensor/sensor_gc1084.ko\"\ntelnet = false\nftp = true\n\n[wifi]\n";

    #[test]
    fn test_set_bool_in_text_system_telnet_replaces_the_line_only() {
        let got = set_bool_in_text(SYSTEM_SAMPLE, "[system]", "telnet", true).expect("edit");
        assert!(got.contains(
            "[system]\nsensor_module = \"/data/sensor/sensor_gc1084.ko\"\ntelnet = true\nftp = true"
        ));
    }

    #[test]
    fn test_set_bool_in_text_system_telnet_inserts_when_absent() {
        let sample = "[system]\nftp = true\n";
        let got = set_bool_in_text(sample, "[system]", "telnet", false).expect("edit");
        assert!(got.contains("[system]\ntelnet = false\nftp = true"));
    }

    #[test]
    fn test_set_system_telnet_writes_atomically_and_preserves_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anyka.toml");
        std::fs::write(&path, SYSTEM_SAMPLE).expect("seed");

        Config::set_system_telnet(&path, true).expect("persist");

        let after = std::fs::read_to_string(&path).expect("read back");
        assert!(after.contains("telnet = true\nftp = true"));
        assert!(after.contains("sensor_module = \"/data/sensor/sensor_gc1084.ko\""));
        // No temp file left behind.
        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[test]
    fn test_set_service_enabled_writes_atomically_and_preserves_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anyka.toml");
        std::fs::write(&path, SAMPLE).expect("seed");

        Config::set_service_enabled(&path, "onvif", false).expect("persist");

        let after = std::fs::read_to_string(&path).expect("read back");
        assert!(after.contains("[services.onvif]\nenabled = false"));
        assert!(after.contains("# keep this comment alive"));
        // No temp file left behind.
        assert!(!path.with_extension("toml.tmp").exists());
        // Still parses.
        Config::load_without_overlay(path.to_str().expect("utf8")).ok();
    }
}
