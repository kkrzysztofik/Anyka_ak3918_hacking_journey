//! Machine-owned network overlay, written here and consumed by anyka-init.
//!
//! The field set must stay identical to `anyka-init/src/netoverlay.rs` — a mismatch
//! means anyka-init's `deny_unknown_fields` rejects a file onvif-rust wrote,
//! surfacing as a boot failure.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::file_ops::atomic_write;

/// Production location, alongside `anyka.toml` under the update root.
pub const DEFAULT_OVERLAY_PATH: &str = "/mnt/anyka_hack/network.toml";

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkOverlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<String>>,
}

impl NetworkOverlay {
    /// Read the overlay; an absent file yields the default.
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        match std::fs::read_to_string(path) {
            Ok(src) => toml::from_str(&src).map_err(|e| std::io::Error::other(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Write the overlay atomically.
    ///
    /// A half-written overlay would fail anyka-init's `deny_unknown_fields`
    /// parse on every boot.
    pub fn write(&self, path: &Path) -> Result<(), std::io::Error> {
        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        atomic_write(path, content.as_bytes(), None)
    }

    /// Contents of a quarantined overlay from the previous boot, if any.
    pub fn last_failure(path: &Path) -> Option<Self> {
        let mut bad = path.as_os_str().to_owned();
        bad.push(".bad");
        let src = std::fs::read_to_string(Path::new(&bad)).ok()?;
        toml::from_str(&src).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        let overlay = NetworkOverlay {
            dhcp: Some(false),
            address: Some("192.168.2.50/24".to_string()),
            gateway: Some("192.168.2.1".to_string()),
            dns: Some(vec!["192.168.2.1".to_string()]),
            ..Default::default()
        };
        overlay.write(&path).expect("write must succeed");

        assert_eq!(NetworkOverlay::read(&path).expect("read"), overlay);
    }

    #[test]
    fn test_absent_keys_are_not_serialised() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        NetworkOverlay {
            dhcp: Some(true),
            ..Default::default()
        }
        .write(&path)
        .expect("write");

        let src = std::fs::read_to_string(&path).expect("read");
        assert!(src.contains("dhcp = true"));
        assert!(
            !src.contains("ssid"),
            "an overlay that never set ssid must not write an empty one; anyka-init would merge it over the operator's real credentials"
        );
    }

    #[test]
    fn test_read_of_an_absent_file_is_the_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let overlay = NetworkOverlay::read(&dir.path().join("nope.toml")).expect("absent is ok");
        assert_eq!(overlay, NetworkOverlay::default());
    }

    #[test]
    fn test_quarantined_overlay_is_detected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(dir.path().join("network.toml.bad"), "ssid = \"TypoNet\"\n").expect("write");

        assert!(
            NetworkOverlay::last_failure(&path).is_some(),
            "the UI must be able to report that the previous settings failed"
        );
    }

    #[test]
    fn test_serialised_keys_match_the_anyka_init_schema() {
        let all = NetworkOverlay {
            ssid: Some("s".into()),
            password: Some("p".into()),
            security: Some("wpa".into()),
            dhcp: Some(false),
            address: Some("10.0.0.1/24".into()),
            gateway: Some("10.0.0.254".into()),
            dns: Some(vec!["10.0.0.254".into()]),
        };
        let src = toml::to_string_pretty(&all).expect("serialise");
        let mut keys: Vec<&str> = src
            .lines()
            .filter_map(|l| l.split('=').next())
            .map(str::trim)
            .filter(|k| !k.is_empty() && !k.starts_with('['))
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "address", "dhcp", "dns", "gateway", "password", "security", "ssid"
            ],
            "overlay keys changed: update anyka-init/src/netoverlay.rs to match"
        );
    }
}
