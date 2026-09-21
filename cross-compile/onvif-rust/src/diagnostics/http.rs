//! HTTP handlers for diagnostics JSON endpoints behind Basic Auth.
//!
//! Two routes are exposed:
//! - `GET /api/diagnostics` – system metrics snapshot (requires [`AuthLevel::User`])
//! - `GET /api/logs` – log tail/filter (requires [`AuthLevel::Administrator`])
//!
//! Auth is enforced by [`diagnostics_auth_middleware`], which delegates
//! credential verification entirely to [`verify_basic_auth_self`] — the same
//! function used for SOAP Basic Auth.  No parallel credential-check path exists.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Query, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::logs::{self, DEFAULT_TAIL_BYTES, LogLevel, LogSource, MAX_LINES};
use super::state::{DiagnosticsState, Snapshot};
use crate::config::UserAccount;
use crate::onvif::auth_requirements::AuthLevel;
use crate::onvif::dispatcher::verify_basic_auth_self;
use crate::onvif::error::OnvifError;
use crate::onvif::server::OnvifServerState;

/// Query parameters for `GET /api/logs`.
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    source: LogSource,
    level: Option<LogLevel>,
    #[serde(default = "default_lines")]
    lines: usize,
}

fn default_lines() -> usize {
    200
}

/// Serve a snapshot of system metrics as JSON.
///
/// Auth is enforced by the surrounding [`diagnostics_auth_middleware`].
/// When authentication is disabled the route still answers, but PTZ
/// `init_error` / `self_check` are redacted so unauthenticated clients
/// do not learn motor bring-up failure details.
pub async fn handle_diagnostics(
    State(server): State<OnvifServerState>,
    Extension(state): Extension<Arc<DiagnosticsState>>,
) -> Json<Snapshot> {
    let mut snapshot = state.snapshot().await;
    if !server.auth_enabled
        && let Some(ptz) = snapshot.ptz.as_mut()
    {
        ptz.init_error = None;
        ptz.self_check = None;
    }
    Json(snapshot)
}

/// Serve a filtered tail of one of the on-device log files as JSON.
///
/// Auth is enforced by the surrounding [`diagnostics_auth_middleware`];
/// this handler assumes the request has already been authenticated.
pub async fn handle_logs(Query(query): Query<LogQuery>) -> Response {
    let source = query.source;
    let level = query.level;
    let lines_limit = query.lines.min(MAX_LINES);

    match tokio::task::spawn_blocking(move || {
        let path = std::path::Path::new(source.path());
        logs::tail_bytes(path, DEFAULT_TAIL_BYTES)
            .map(|text| logs::filter_lines(&text, level, lines_limit))
    })
    .await
    {
        Ok(Ok(lines)) => Json(lines).into_response(),
        Ok(Err(e)) => {
            tracing::debug!(source = ?source, error = %e, "log tail failed");
            (StatusCode::NOT_FOUND, "log source unavailable").into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "log tail task failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Determine the required auth level for a given request path.
///
/// The middleware is nested under `/api`, so axum strips that prefix and
/// this function sees the remainder (e.g. `/logs`, `/diagnostics`). Some
/// callers still pass the full `/api/...` path; strip the prefix defensively.
fn required_level_for_path(path: &str) -> AuthLevel {
    let path = path.strip_prefix("/api").unwrap_or(path);
    match path {
        "/logs" | "/logs/" => AuthLevel::Administrator,
        "/update" | "/update/" => AuthLevel::Administrator,
        "/sound" | "/sound/" => AuthLevel::Administrator,
        "/sound/play" | "/sound/play/" => AuthLevel::Administrator,
        "/diagnostics" | "/diagnostics/" => AuthLevel::User,
        "/processes" | "/processes/" => AuthLevel::User,
        // Fail closed: unknown routes require Administrator until explicitly opened.
        _ => AuthLevel::Administrator,
    }
}

/// Map a credential-check result and required level to an HTTP failure status.
///
/// Returns `None` when access should be granted, `Some(status)` when denied.
///
/// Keeping this as a thin pure function makes the auth gate independently
/// testable without HTTP infrastructure.
fn check_required_level(
    result: Result<Option<UserAccount>, OnvifError>,
    required: AuthLevel,
) -> Option<StatusCode> {
    match result {
        Ok(Some(user)) => {
            if required.is_satisfied_by(Some(user.level)) {
                None
            } else {
                Some(StatusCode::FORBIDDEN)
            }
        }
        // No Basic Auth header present
        Ok(None) => Some(StatusCode::UNAUTHORIZED),
        // Header present but malformed or credentials invalid
        Err(_) => Some(StatusCode::UNAUTHORIZED),
    }
}

/// Axum middleware that enforces Basic Auth for the `/api` diagnostics routes.
///
/// When `auth_enabled` is `false` the request passes through unchanged —
/// matching the same bypass used by the SOAP dispatcher.
///
/// Credential verification is delegated entirely to [`verify_basic_auth_self`];
/// no separate credential-decode path exists here.
pub async fn diagnostics_auth_middleware(
    State(state): State<OnvifServerState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.auth_enabled {
        return next.run(request).await;
    }

    let path = request.uri().path().to_owned();
    let required = required_level_for_path(&path);
    let auth_ctx = state.auth_context();

    match check_required_level(
        verify_basic_auth_self(&state.dispatcher, &request, &auth_ctx),
        required,
    ) {
        None => next.run(request).await,
        Some(StatusCode::UNAUTHORIZED) => {
            tracing::warn!(
                target: "security",
                path = %path,
                ?required,
                "diagnostics access denied: unauthorized"
            );
            let mut resp = StatusCode::UNAUTHORIZED.into_response();
            resp.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static(r#"Basic realm="ONVIF Camera""#),
            );
            resp
        }
        Some(status) => {
            tracing::warn!(
                target: "security",
                path = %path,
                ?required,
                status = %status,
                "diagnostics access denied: insufficient privilege"
            );
            status.into_response()
        }
    }
}
