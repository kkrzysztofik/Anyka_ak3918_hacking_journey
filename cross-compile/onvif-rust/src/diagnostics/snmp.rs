//! `/api/snmp` — read/write `snmp.toml` for the WebUI.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::config::snmp::{self, ConfigError, SnmpSettings};

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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SnmpPatch {
    pub port: Option<u16>,
    pub community: Option<String>,
    pub sys_contact: Option<String>,
    pub sys_name: Option<String>,
    pub sys_location: Option<String>,
}

fn apply_patch(settings: &mut SnmpSettings, patch: SnmpPatch) {
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
) -> Result<Json<SnmpSettings>, (StatusCode, String)> {
    SnmpSettings::load(&state.config_path)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// PUT /api/snmp
pub async fn handle_put_snmp(
    Extension(state): Extension<Arc<SnmpApiState>>,
    Json(patch): Json<SnmpPatch>,
) -> Result<StatusCode, (StatusCode, String)> {
    snmp::update_at(&state.config_path, |s| apply_patch(s, patch)).map_err(|e| {
        let status = match e {
            ConfigError::InvalidPort | ConfigError::EmptyCommunity => StatusCode::BAD_REQUEST,
            ConfigError::Io(_) | ConfigError::Parse(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, e.to_string())
    })?;

    snmp::sighup_agent(Path::new(&state.pidfile)).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("snmp settings saved but agent reload failed: {e}"),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patch_merges_fields() {
        let mut s = SnmpSettings::default();
        apply_patch(
            &mut s,
            SnmpPatch {
                community: Some("monitor".into()),
                ..Default::default()
            },
        );
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
        assert_eq!(get.port, 161);
        assert_eq!(get.community, "public");

        let put = handle_put_snmp(
            Extension(Arc::clone(&state)),
            Json(SnmpPatch {
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

        let stored = SnmpSettings::load(&config_path).unwrap();
        assert_eq!(stored.port, 1161);
        assert_eq!(stored.community, "ops");
        assert_eq!(stored.sys_contact, "a");
        assert_eq!(stored.sys_name, "b");
        assert_eq!(stored.sys_location, "c");

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
        assert_eq!(bad.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_put_snmp_empty_community_returns_bad_request() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(SnmpApiState {
            config_path: dir.path().join("snmp.toml"),
            pidfile: dir.path().join("missing.pid"),
        });
        let err = handle_put_snmp(
            Extension(state),
            Json(SnmpPatch {
                community: Some(String::new()),
                ..Default::default()
            }),
        )
        .await
        .expect_err("empty community");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
