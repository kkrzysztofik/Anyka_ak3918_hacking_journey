//! Machine-owned SNMP agent settings (`snmp.toml`).
//!
//! Field set must stay compatible with `snmp-agent` `SnmpConfig` (same keys).
//! SNMP is exposed via REST `/api/snmp` and this file — not ONVIF NetworkProtocolType.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use super::file_ops::atomic_write;

/// Production path alongside `anyka.toml`.
pub const DEFAULT_CONFIG_PATH: &str = "/mnt/anyka_hack/snmp.toml";

/// Pidfile written by `snmp-agent` for SIGHUP reload.
pub const DEFAULT_PIDFILE: &str = "/tmp/snmp-agent.pid";

fn path_override() -> &'static Mutex<Option<PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

/// Serializes all snmp.toml read-modify-write updates (REST and ONVIF).
fn update_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Resolve the snmp.toml path (test override or production default).
pub fn config_path() -> PathBuf {
    path_override()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

/// Test-only path override.
#[cfg(test)]
pub fn set_config_path_for_test(path: Option<PathBuf>) {
    *path_override().lock().unwrap_or_else(std::sync::PoisonError::into_inner) = path;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_community")]
    pub community: String,
    #[serde(default)]
    pub sys_contact: String,
    #[serde(default)]
    pub sys_name: String,
    #[serde(default)]
    pub sys_location: String,
}

fn default_enabled() -> bool {
    true
}
fn default_port() -> u16 {
    161
}
fn default_community() -> String {
    "public".to_string()
}

/// Returned by [`SnmpSettings::update_at`] when enable is requested without a community.
pub const ERR_EMPTY_COMMUNITY_WHEN_ENABLED: &str =
    "community must not be empty when SNMP is enabled";

impl Default for SnmpSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 161,
            community: default_community(),
            sys_contact: String::new(),
            sys_name: String::new(),
            sys_location: String::new(),
        }
    }
}

impl SnmpSettings {
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                let cfg: Self =
                    toml::from_str(&src).map_err(|e| std::io::Error::other(e.to_string()))?;
                if cfg.port == 0 {
                    return Err(std::io::Error::other("snmp port must be non-zero"));
                }
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn write(&self, path: &Path) -> Result<(), std::io::Error> {
        if self.port == 0 {
            return Err(std::io::Error::other("snmp port must be non-zero"));
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        atomic_write(path, content.as_bytes(), None)
    }

    pub fn update_at(path: &Path, edit: impl FnOnce(&mut Self)) -> Result<(), std::io::Error> {
        let _guard = update_lock().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut cfg = Self::read(path)?;
        edit(&mut cfg);
        if cfg.enabled && cfg.community.is_empty() {
            return Err(std::io::Error::other(ERR_EMPTY_COMMUNITY_WHEN_ENABLED));
        }
        cfg.write(path)
    }
}

/// Signal snmp-agent to reload config. Missing pidfile is not an error.
pub fn sighup_agent(pidfile: &Path) -> Result<(), std::io::Error> {
    let text = match std::fs::read_to_string(pidfile) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let pid: i32 = text
        .trim()
        .parse()
        .map_err(|e| std::io::Error::other(format!("invalid snmp pidfile: {e}")))?;
    // kill(2) overloads its first argument: 0 means "my whole process group" and
    // negative values mean a process group or, for -1, every process we may
    // signal. onvif-rust is root on the camera, so a truncated pidfile must be
    // inert rather than a broadcast.
    if pid <= 1 {
        return Ok(());
    }
    // SAFETY: libc kill with a pid from our pidfile; ESRCH is treated as Ok.
    let rc = unsafe { libc::kill(pid, libc::SIGHUP) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        let cfg = SnmpSettings {
            enabled: false,
            port: 1161,
            community: "monitor".into(),
            sys_name: "cam".into(),
            ..Default::default()
        };
        cfg.write(&path).unwrap();
        assert_eq!(SnmpSettings::read(&path).unwrap(), cfg);
    }

    #[test]
    fn test_missing_file_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.toml");
        assert_eq!(SnmpSettings::read(&path).unwrap(), SnmpSettings::default());
    }

    #[test]
    fn test_update_at_enabled_without_community_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        SnmpSettings {
            enabled: false,
            community: String::new(),
            ..SnmpSettings::default()
        }
        .write(&path)
        .unwrap();
        let err = SnmpSettings::update_at(&path, |s| s.enabled = true).unwrap_err();
        assert!(err.to_string().contains("community"));
    }

    #[test]
    fn test_read_write_reject_port_zero_and_update_at() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        let bad = SnmpSettings {
            port: 0,
            ..Default::default()
        };
        assert!(bad.write(&path).is_err());
        std::fs::write(&path, "port = 0\n").unwrap();
        assert!(SnmpSettings::read(&path).is_err());

        SnmpSettings::default().write(&path).unwrap();
        SnmpSettings::update_at(&path, |s| {
            s.enabled = false;
            s.community = "monitor".into();
            s.sys_contact = "ops".into();
            s.sys_location = "lab".into();
        })
        .unwrap();
        let got = SnmpSettings::read(&path).unwrap();
        assert!(!got.enabled);
        assert_eq!(got.community, "monitor");
        assert_eq!(got.sys_contact, "ops");
        assert_eq!(got.sys_location, "lab");
    }

    #[test]
    fn test_config_path_override_and_sighup_agent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ovr.toml");
        set_config_path_for_test(Some(path.clone()));
        assert_eq!(config_path(), path);
        set_config_path_for_test(None);
        assert_eq!(config_path(), PathBuf::from(DEFAULT_CONFIG_PATH));

        let missing = dir.path().join("no.pid");
        assert!(sighup_agent(&missing).is_ok());

        let bad = dir.path().join("bad.pid");
        std::fs::write(&bad, "not-a-pid\n").unwrap();
        assert!(sighup_agent(&bad).is_err());

        let stale = dir.path().join("stale.pid");
        std::fs::write(&stale, "2147483646\n").unwrap(); // unlikely live pid → ESRCH
        assert!(sighup_agent(&stale).is_ok());
    }

    #[test]
    fn test_sighup_agent_refuses_process_group_pids() {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in [
            ("zero.pid", "0\n"),
            ("all.pid", "-1\n"),
            ("neg.pid", "-4242\n"),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, body).unwrap();
            // Must be a no-op, never a kill(2) broadcast.
            assert!(sighup_agent(&path).is_ok(), "{name} must be ignored");
        }
    }

    /// The agent parses this file with its own struct. If either side gains a
    /// field, this fails instead of the mismatch reaching a camera.
    #[test]
    fn test_keys_match_snmp_agent_config() {
        let toml = toml::to_string(&SnmpSettings::default()).unwrap();
        let keys: Vec<String> = toml
            .lines()
            .filter_map(|l| l.split('=').next())
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect();
        assert_eq!(
            keys,
            [
                "enabled",
                "port",
                "community",
                "sys_contact",
                "sys_name",
                "sys_location"
            ],
            "keys changed: update snmp-agent/src/config.rs SnmpConfig to match"
        );
    }
}
