//! Machine-owned SNMP agent settings (`snmp.toml`).
//!
//! The struct and its validation are the agent's own (`snmp_agent::config`),
//! so the two sides cannot drift. On/off is `[services.snmp]` in `anyka.toml`.
//! SNMP is exposed via REST `/api/snmp` and this file — not ONVIF NetworkProtocolType.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

pub use snmp_agent::config::{ConfigError, SnmpConfig as SnmpSettings};
pub use snmp_agent::server::DEFAULT_PIDFILE;

use super::file_ops::atomic_write;

/// Serializes all snmp.toml read-modify-write updates.
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Load, edit, validate and atomically rewrite `snmp.toml`.
pub fn update_at(path: &Path, edit: impl FnOnce(&mut SnmpSettings)) -> Result<(), ConfigError> {
    let _guard = UPDATE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut cfg = SnmpSettings::load(path)?;
    edit(&mut cfg);
    cfg.validate()?;
    let content = toml::to_string_pretty(&cfg).map_err(std::io::Error::other)?;
    atomic_write(path, content.as_bytes(), None)?;
    Ok(())
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
    fn test_update_at_round_trips_through_the_agent_loader() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        update_at(&path, |s| {
            s.port = 1161;
            s.community = "monitor".into();
            s.sys_contact = "ops".into();
        })
        .unwrap();
        let got = SnmpSettings::load(&path).unwrap();
        assert_eq!(got.port, 1161);
        assert_eq!(got.community, "monitor");
        assert_eq!(got.sys_contact, "ops");
    }

    #[test]
    fn test_update_at_rejects_invalid_edits_and_keeps_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        update_at(&path, |_| {}).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        assert!(matches!(
            update_at(&path, |s| s.port = 0),
            Err(ConfigError::InvalidPort)
        ));
        assert!(matches!(
            update_at(&path, |s| s.community.clear()),
            Err(ConfigError::EmptyCommunity)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn test_update_at_drops_the_legacy_enabled_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(&path, "enabled = false\ncommunity = \"public\"\n").unwrap();
        update_at(&path, |_| {}).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("enabled"));
    }

    #[test]
    fn test_sighup_agent_tolerates_missing_and_stale_pidfiles() {
        let dir = tempfile::tempdir().unwrap();

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
}
