//! `/api/snmp` — read/write `snmp.toml` for the WebUI.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::snmp::{self, SnmpSettings};

/// Shared state for SNMP REST handlers.
pub struct SnmpApiState {
    pub config_path: PathBuf,
    pub pidfile: PathBuf,
}

impl SnmpApiState {
    pub fn from_update_root(update_root: impl Into<PathBuf>) -> Self {
        let root = update_root.into();
        Self {
            config_path: root.join("snmp.toml"),
            pidfile: PathBuf::from(snmp::DEFAULT_PIDFILE),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnmpView {
    pub enabled: bool,
    pub port: u16,
    pub community: String,
    pub sys_contact: String,
    pub sys_name: String,
    pub sys_location: String,
}

impl From<&SnmpSettings> for SnmpView {
    fn from(s: &SnmpSettings) -> Self {
        Self {
            enabled: s.enabled,
            port: s.port,
            community: s.community.clone(),
            sys_contact: s.sys_contact.clone(),
            sys_name: s.sys_name.clone(),
            sys_location: s.sys_location.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SnmpPatch {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
    pub community: Option<String>,
    pub sys_contact: Option<String>,
    pub sys_name: Option<String>,
    pub sys_location: Option<String>,
}

fn validate_patch(patch: &SnmpPatch) -> Result<(), String> {
    if let Some(port) = patch.port
        && port == 0
    {
        return Err("port must be 1-65535".into());
    }
    if let Some(community) = &patch.community
        && community.is_empty()
    {
        return Err("community must not be empty".into());
    }
    Ok(())
}

fn apply_patch(settings: &mut SnmpSettings, patch: SnmpPatch) {
    if let Some(enabled) = patch.enabled {
        settings.enabled = enabled;
    }
    if let Some(port) = patch.port {
        settings.port = port;
    }
    if let Some(community) = patch.community {
        settings.community = community;
    }
    if let Some(sys_contact) = patch.sys_contact {
        settings.sys_contact = sys_contact;
    }
    if let Some(sys_name) = patch.sys_name {
        settings.sys_name = sys_name;
    }
    if let Some(sys_location) = patch.sys_location {
        settings.sys_location = sys_location;
    }
}

/// GET /api/snmp
pub async fn handle_get_snmp(
    Extension(state): Extension<Arc<SnmpApiState>>,
) -> Result<Json<SnmpView>, (StatusCode, String)> {
    let settings = SnmpSettings::read(&state.config_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(SnmpView::from(&settings)))
}

/// PUT /api/snmp
pub async fn handle_put_snmp(
    Extension(state): Extension<Arc<SnmpApiState>>,
    Json(patch): Json<SnmpPatch>,
) -> Result<StatusCode, (StatusCode, String)> {
    validate_patch(&patch).map_err(|reason| (StatusCode::BAD_REQUEST, reason))?;

    SnmpSettings::update_at(&state.config_path, |s| apply_patch(s, patch))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = snmp::sighup_agent(Path::new(&state.pidfile));
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rejects_empty_community() {
        let patch = SnmpPatch {
            community: Some(String::new()),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());
    }

    #[test]
    fn test_validate_rejects_port_zero() {
        let patch = SnmpPatch {
            port: Some(0),
            ..Default::default()
        };
        assert!(validate_patch(&patch).is_err());
    }

    #[test]
    fn test_apply_patch_merges_fields() {
        let mut s = SnmpSettings::default();
        apply_patch(
            &mut s,
            SnmpPatch {
                enabled: Some(false),
                community: Some("monitor".into()),
                ..Default::default()
            },
        );
        assert!(!s.enabled);
        assert_eq!(s.community, "monitor");
        assert_eq!(s.port, 161);
    }

    #[tokio::test]
    async fn test_get_and_put_snmp_handlers() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("snmp.toml");

        let state = Arc::new(SnmpApiState {
            config_path: config_path.clone(),
            pidfile: dir.path().join("missing.pid"),
        });

        let get = handle_get_snmp(Extension(Arc::clone(&state)))
            .await
            .expect("get defaults");
        assert!(get.enabled);
        assert_eq!(get.port, 161);
        assert_eq!(get.community, "public");

        let put = handle_put_snmp(
            Extension(Arc::clone(&state)),
            Json(SnmpPatch {
                enabled: Some(false),
                port: Some(1161),
                community: Some("ops".into()),
                sys_contact: Some("a".into()),
                sys_name: Some("b".into()),
                sys_location: Some("c".into()),
            }),
        )
        .await
        .expect("put");
        assert_eq!(put, StatusCode::NO_CONTENT);

        let stored = SnmpSettings::read(&config_path).unwrap();
        assert!(!stored.enabled);
        assert_eq!(stored.port, 1161);
        assert_eq!(stored.community, "ops");
        assert_eq!(stored.sys_contact, "a");
        assert_eq!(stored.sys_name, "b");
        assert_eq!(stored.sys_location, "c");

        let view = SnmpView::from(&stored);
        assert_eq!(view.sys_name, "b");
        assert_eq!(
            SnmpApiState::from_update_root(dir.path()).config_path,
            config_path
        );

        let bad = handle_put_snmp(
            Extension(state),
            Json(SnmpPatch {
                port: Some(0),
                ..Default::default()
            }),
        )
        .await;
        assert!(bad.is_err());
    }
}
