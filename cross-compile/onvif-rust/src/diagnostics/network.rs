//! `/api/network` — overlay state for the WebUI.
//!
//! Exists because ONVIF cannot express "saved but not yet applied", and
//! because Wi-Fi credentials over SOAP would mean the whole
//! `Dot11Configuration` type surface for one form.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::config::netoverlay::{DEFAULT_OVERLAY_PATH, NetworkOverlay};

/// Shared state for the network overlay endpoint.
#[derive(Clone)]
pub struct NetworkState {
    pub overlay_path: PathBuf,
    pub overlay_lock: Arc<Mutex<()>>,
}

impl NetworkState {
    pub fn new(overlay_path: impl Into<PathBuf>) -> Self {
        Self {
            overlay_path: overlay_path.into(),
            overlay_lock: NetworkOverlay::overlay_lock(),
        }
    }

    /// Test-only helper: overlay path under an update root.
    pub fn from_update_root(update_root: impl Into<PathBuf>) -> Self {
        Self::new(update_root.into().join("network.toml"))
    }
}

impl Default for NetworkState {
    fn default() -> Self {
        Self::new(DEFAULT_OVERLAY_PATH)
    }
}

/// Overlay fields exposed to the WebUI (password never returned).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NetworkOverlayView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    pub has_password: bool,
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

impl NetworkOverlayView {
    fn from_overlay(overlay: &NetworkOverlay) -> Self {
        Self {
            ssid: overlay.ssid.clone(),
            has_password: overlay.password.is_some(),
            security: overlay.security.clone(),
            dhcp: overlay.dhcp,
            address: overlay.address.clone(),
            gateway: overlay.gateway.clone(),
            dns: overlay.dns.clone(),
        }
    }
}

/// Partial overlay patch accepted by PUT /api/network.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetworkOverlayPatch {
    pub ssid: Option<String>,
    pub password: Option<String>,
    pub security: Option<String>,
    pub dhcp: Option<bool>,
    pub address: Option<String>,
    pub gateway: Option<String>,
    pub dns: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct NetworkStateResponse {
    pub pending: NetworkOverlayView,
    pub has_pending: bool,
    pub last_failure: Option<NetworkOverlayView>,
}

fn overlay_exists(path: &Path) -> bool {
    path.exists()
}

fn validate_patch(patch: &NetworkOverlayPatch) -> Result<(), String> {
    if patch.ssid.as_deref() == Some("") {
        return Err("SSID cannot be empty".to_string());
    }
    if let Some(sec) = patch.security.as_deref()
        && !matches!(sec, "wpa" | "wep" | "open")
    {
        return Err(format!("security must be wpa, wep, or open (got {sec})"));
    }
    if patch.dhcp == Some(false) {
        if patch.address.as_deref().is_none_or(|a| a.trim().is_empty()) {
            return Err("address is required when DHCP is disabled".to_string());
        }
        if patch.gateway.as_deref().is_none_or(|g| g.trim().is_empty()) {
            return Err("gateway is required when DHCP is disabled".to_string());
        }
    }
    if let Some(ref addr) = patch.address
        && !addr.is_empty()
    {
        validate_cidr(addr)?;
    }
    if let Some(ref gw) = patch.gateway
        && !gw.is_empty()
    {
        validate_ipv4(gw)?;
    }
    if let Some(ref servers) = patch.dns {
        for s in servers {
            validate_ipv4(s)?;
        }
    }
    Ok(())
}

fn validate_ipv4(addr: &str) -> Result<(), String> {
    if addr.parse::<Ipv4Addr>().is_ok() {
        Ok(())
    } else {
        Err(format!("invalid IPv4 address: {addr}"))
    }
}

fn validate_cidr(cidr: &str) -> Result<(), String> {
    let (addr, prefix) = cidr
        .split_once('/')
        .ok_or_else(|| format!("invalid CIDR address: {cidr}"))?;
    validate_ipv4(addr)?;
    let prefix: u8 = prefix
        .parse()
        .map_err(|_| format!("invalid CIDR prefix: {cidr}"))?;
    if !(1..=32).contains(&prefix) {
        return Err(format!("invalid CIDR prefix: {cidr}"));
    }
    Ok(())
}

fn merge_overlay(existing: &mut NetworkOverlay, patch: NetworkOverlayPatch) {
    if patch.ssid.is_some() {
        existing.ssid = patch.ssid;
    }
    if patch.password.is_some() {
        existing.password = patch.password;
    }
    if patch.security.is_some() {
        existing.security = patch.security;
    }
    if patch.dhcp.is_some() {
        existing.dhcp = patch.dhcp;
    }
    if patch.address.is_some() {
        existing.address = patch.address;
    }
    if patch.gateway.is_some() {
        existing.gateway = patch.gateway;
    }
    if patch.dns.is_some() {
        existing.dns = patch.dns;
    }
}

/// GET /api/network
pub async fn handle_get_network(
    Extension(state): Extension<Arc<NetworkState>>,
) -> Result<Json<NetworkStateResponse>, (StatusCode, String)> {
    let pending = NetworkOverlay::read(&state.overlay_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let last_failure = NetworkOverlay::last_failure(&state.overlay_path)
        .map(|o| NetworkOverlayView::from_overlay(&o));

    Ok(Json(NetworkStateResponse {
        has_pending: overlay_exists(&state.overlay_path),
        pending: NetworkOverlayView::from_overlay(&pending),
        last_failure,
    }))
}

/// PUT /api/network
pub async fn handle_put_network(
    Extension(state): Extension<Arc<NetworkState>>,
    Json(patch): Json<NetworkOverlayPatch>,
) -> Result<StatusCode, (StatusCode, String)> {
    validate_patch(&patch).map_err(|reason| (StatusCode::BAD_REQUEST, reason))?;

    let _guard = state.overlay_lock.lock();
    let mut overlay = NetworkOverlay::read(&state.overlay_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    merge_overlay(&mut overlay, patch);
    overlay
        .write(&state.overlay_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_view_never_serialises_password() {
        let overlay = NetworkOverlay {
            ssid: Some("Net".into()),
            password: Some("secret".into()),
            ..Default::default()
        };
        let view = NetworkOverlayView::from_overlay(&overlay);
        assert!(view.has_password);
        let json = serde_json::to_string(&view).expect("json");
        assert!(!json.contains("secret"));
        assert!(!json.contains("\"password\""));
    }

    #[test]
    fn test_validate_patch_rejects_empty_ssid() {
        let patch = NetworkOverlayPatch {
            ssid: Some(String::new()),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());
    }

    #[test]
    fn test_validate_patch_rejects_static_without_address_or_gateway() {
        let patch = NetworkOverlayPatch {
            dhcp: Some(false),
            address: Some(String::new()),
            gateway: Some("192.168.1.1".into()),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());

        let patch = NetworkOverlayPatch {
            dhcp: Some(false),
            address: Some("192.168.1.10/24".into()),
            gateway: Some(String::new()),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());
    }

    #[test]
    fn test_validate_patch_rejects_invalid_security() {
        let patch = NetworkOverlayPatch {
            security: Some("wpa3".into()),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());
    }

    #[test]
    fn test_merge_overlay_preserves_absent_keys() {
        let mut existing = NetworkOverlay {
            ssid: Some("Net".into()),
            dhcp: Some(false),
            address: Some("10.0.0.1/24".into()),
            ..Default::default()
        };
        merge_overlay(
            &mut existing,
            NetworkOverlayPatch {
                gateway: Some("10.0.0.254".into()),
                ..Default::default()
            },
        );
        assert_eq!(existing.ssid.as_deref(), Some("Net"));
        assert_eq!(existing.dhcp, Some(false));
        assert_eq!(existing.address.as_deref(), Some("10.0.0.1/24"));
        assert_eq!(existing.gateway.as_deref(), Some("10.0.0.254"));
    }

    #[test]
    fn test_merge_overlay_stores_explicit_empty_dns_list() {
        let mut existing = NetworkOverlay {
            dns: Some(vec!["8.8.8.8".into()]),
            ..Default::default()
        };
        merge_overlay(
            &mut existing,
            NetworkOverlayPatch {
                dns: Some(vec![]),
                ..Default::default()
            },
        );
        assert_eq!(existing.dns, Some(vec![]));
    }

    #[test]
    fn test_validate_cidr_rejects_missing_prefix_and_out_of_range() {
        assert!(validate_cidr("192.168.1.1").is_err());
        assert!(validate_cidr("192.168.1.1/0").is_err());
        assert!(validate_cidr("192.168.1.1/33").is_err());
        assert!(validate_cidr("192.168.1.1/24").is_ok());
    }

    #[tokio::test]
    async fn test_handle_put_network_round_trips_via_tempfile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let state = Arc::new(NetworkState::new(&path));

        let result = handle_put_network(
            Extension(Arc::clone(&state)),
            Json(NetworkOverlayPatch {
                ssid: Some("TestNet".into()),
                dhcp: Some(true),
                ..Default::default()
            }),
        )
        .await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let response = handle_get_network(Extension(state))
            .await
            .expect("get must succeed");
        assert_eq!(response.pending.ssid.as_deref(), Some("TestNet"));
        assert!(response.has_pending);
    }
}
