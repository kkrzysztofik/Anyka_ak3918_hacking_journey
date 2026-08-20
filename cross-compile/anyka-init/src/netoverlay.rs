//! Machine-owned network overlay.
//!
//! `anyka.toml` is the operator's file: hand-edited, comment-rich, and holding
//! the Wi-Fi credentials. Nothing in this codebase writes it. Runtime network
//! changes made from the WebUI land here instead, in a file that has no
//! comments to lose and no operator intent to clobber, and that a support
//! engineer can neutralise with a single `rm`.
//!
//! Every field is `Option` so that "the user never touched this" is
//! distinguishable from "the user set this to false / to an empty list".

use serde::{Deserialize, Serialize};

use crate::config::WifiCfg;

/// Overlay applied over `[wifi]` from `anyka.toml`.
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
    /// Production location of the overlay, alongside `anyka.toml`.
    pub const DEFAULT_PATH: &str = "/mnt/anyka_hack/network.toml";

    /// Quarantine name used when a boot with this overlay fails to associate.
    pub const QUARANTINE_SUFFIX: &str = ".bad";

    /// Merge this overlay onto a baseline `[wifi]` config, in place.
    ///
    /// Absent keys leave the baseline alone. This is why `dns` is an
    /// `Option<Vec<_>>` and not a bare `Vec<_>`: `dns = []` must be able to
    /// clear servers the baseline inherited.
    pub fn apply_to(&self, cfg: &mut WifiCfg) {
        if let Some(v) = &self.ssid {
            cfg.ssid = v.clone();
        }
        if let Some(v) = &self.password {
            cfg.password = v.clone();
        }
        if let Some(v) = &self.security {
            cfg.security = v.clone();
        }
        if let Some(v) = self.dhcp {
            cfg.dhcp = v;
        }
        if let Some(v) = &self.address {
            cfg.address = Some(v.clone());
        }
        if let Some(v) = &self.gateway {
            cfg.gateway = Some(v.clone());
        }
        if let Some(v) = &self.dns {
            cfg.dns = v.clone();
        }
    }

    /// True when any overlay key is present (file exists and parsed non-empty).
    pub fn has_content(&self) -> bool {
        self.ssid.is_some()
            || self.password.is_some()
            || self.security.is_some()
            || self.dhcp.is_some()
            || self.address.is_some()
            || self.gateway.is_some()
            || self.dns.is_some()
    }

    /// Whether this overlay overrides Wi-Fi association inputs.
    pub fn overrides_association(&self) -> bool {
        self.ssid.is_some() || self.password.is_some() || self.security.is_some()
    }

    /// Validate overlay invariants before merge.
    pub fn validate(&self) -> Result<(), crate::config::ConfigError> {
        if let Some(sec) = &self.security
            && !matches!(sec.as_str(), "wpa" | "wep" | "open")
        {
            return Err(crate::config::ConfigError::Invalid(format!(
                "network overlay security = {sec:?} is not one of wpa, wep, open"
            )));
        }
        if self.dhcp == Some(false) {
            if self.address.is_none() {
                return Err(crate::config::ConfigError::Invalid(
                    "network overlay address is required when dhcp = false".into(),
                ));
            }
            if self.gateway.is_none() {
                return Err(crate::config::ConfigError::Invalid(
                    "network overlay gateway is required when dhcp = false".into(),
                ));
            }
        }
        Ok(())
    }

    /// Read the overlay from `path`.
    ///
    /// An absent file is the normal, unconfigured case and yields the default
    /// (all-absent) overlay. A present but unparseable file is an error: the
    /// caller must be able to tell "nothing configured" from "configuration
    /// present and broken".
    pub fn load(path: &std::path::Path) -> Result<Self, crate::config::ConfigError> {
        let src = match std::fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(crate::config::ConfigError::Read {
                    path: path.display().to_string(),
                    source,
                });
            }
        };

        Ok(toml::from_str(&src)?)
    }

    /// Move a failed overlay aside so the next boot uses the baseline.
    ///
    /// Mirrors the quarantine-and-revert semantics of the A/B slot updates in
    /// `update.rs`: keep the last known-good, retain the failure for
    /// inspection. Best-effort by design — a rename that fails must not stop a
    /// boot that is already recovering.
    pub fn quarantine(path: &std::path::Path) {
        if !path.exists() {
            return;
        }
        let mut bad = path.as_os_str().to_owned();
        bad.push(Self::QUARANTINE_SUFFIX);
        if let Err(e) = std::fs::rename(path, std::path::Path::new(&bad)) {
            tracing::error!(error = %e, "failed to quarantine the network overlay");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> WifiCfg {
        toml::from_str(
            r#"
            ssid = "OperatorNet"
            password = "operatorpass"
            dhcp = true
            "#,
        )
        .expect("baseline must parse")
    }

    #[test]
    fn test_empty_overlay_leaves_baseline_untouched() {
        let mut cfg = baseline();
        NetworkOverlay::default().apply_to(&mut cfg);
        assert_eq!(cfg.ssid, "OperatorNet");
        assert!(cfg.dhcp);
        assert!(cfg.address.is_none());
    }

    #[test]
    fn test_overlay_switches_to_static_address() {
        let mut cfg = baseline();
        let overlay: NetworkOverlay = toml::from_str(
            r#"
            dhcp = false
            address = "192.168.2.50/24"
            gateway = "192.168.2.1"
            dns = ["192.168.2.1"]
            "#,
        )
        .expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert!(!cfg.dhcp);
        assert_eq!(cfg.address.as_deref(), Some("192.168.2.50/24"));
        assert_eq!(cfg.gateway.as_deref(), Some("192.168.2.1"));
        assert_eq!(cfg.dns, vec!["192.168.2.1".to_string()]);
        // Credentials the overlay did not set must survive.
        assert_eq!(cfg.ssid, "OperatorNet");
        assert_eq!(cfg.password, "operatorpass");
    }

    #[test]
    fn test_overlay_replaces_credentials_only_when_present() {
        let mut cfg = baseline();
        let overlay: NetworkOverlay =
            toml::from_str(r#"ssid = "NewNet""#).expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert_eq!(cfg.ssid, "NewNet");
        assert_eq!(
            cfg.password, "operatorpass",
            "an overlay that sets only ssid must not blank the baseline password"
        );
    }

    #[test]
    fn test_overlay_can_clear_dns_with_an_explicit_empty_list() {
        let mut cfg = baseline();
        cfg.dns = vec!["8.8.8.8".to_string()];
        let overlay: NetworkOverlay = toml::from_str("dns = []").expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert!(
            cfg.dns.is_empty(),
            "an explicit empty list must clear, not be treated as absent"
        );
    }

    #[test]
    fn test_unknown_key_is_rejected() {
        let result: Result<NetworkOverlay, _> = toml::from_str(r#"chip = "ssv6355_ble""#);
        assert!(
            result.is_err(),
            "the overlay must not silently accept keys it does not apply"
        );
    }

    #[test]
    fn test_load_returns_default_when_the_file_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        let overlay = NetworkOverlay::load(&path).expect("absent overlay is not an error");

        assert_eq!(overlay, NetworkOverlay::default());
    }

    #[test]
    fn test_load_reads_a_present_overlay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "dhcp = false\naddress = \"10.0.0.5/24\"\n").expect("write");

        let overlay = NetworkOverlay::load(&path).expect("overlay must load");

        assert_eq!(overlay.dhcp, Some(false));
        assert_eq!(overlay.address.as_deref(), Some("10.0.0.5/24"));
    }

    #[test]
    fn test_load_reports_a_corrupt_overlay_instead_of_ignoring_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "dhcp = = =").expect("write");

        assert!(
            NetworkOverlay::load(&path).is_err(),
            "a corrupt overlay must be loud; silently ignoring it is indistinguishable from a save that never happened"
        );
    }

    #[test]
    fn test_quarantine_renames_the_overlay_out_of_the_way() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "ssid = \"TypoNet\"\n").expect("write");

        NetworkOverlay::quarantine(&path);

        assert!(!path.exists(), "the failing overlay must not be used again");
        let bad = dir.path().join("network.toml.bad");
        assert!(
            bad.exists(),
            "the failing overlay must be kept for the UI to report"
        );
        assert_eq!(
            std::fs::read_to_string(&bad).expect("read"),
            "ssid = \"TypoNet\"\n",
            "quarantine must preserve the content so the UI can show what failed"
        );
    }

    #[test]
    fn test_quarantine_on_an_absent_overlay_is_a_no_op() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Must not panic and must not create a stray .bad file.
        NetworkOverlay::quarantine(&dir.path().join("network.toml"));
        assert!(!dir.path().join("network.toml.bad").exists());
    }
}
