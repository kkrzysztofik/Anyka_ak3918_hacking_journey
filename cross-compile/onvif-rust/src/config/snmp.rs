//! Machine-owned SNMP agent settings (`snmp.toml`).
//!
//! Field set must stay compatible with `snmp-agent` `SnmpConfig` (same keys).
//! Official ONVIF `NetworkProtocolType` is HTTP/HTTPS/RTSP only — `SNMP` is a
//! vendor extension on that enum; community stays in this file / WebUI REST.

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

/// Resolve the snmp.toml path (test override or production default).
pub fn config_path() -> PathBuf {
    path_override()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

/// Test-only path override.
#[cfg(test)]
pub fn set_config_path_for_test(path: Option<PathBuf>) {
    *path_override().lock().unwrap_or_else(|e| e.into_inner()) = path;
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
        let mut cfg = Self::read(path)?;
        edit(&mut cfg);
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
}
