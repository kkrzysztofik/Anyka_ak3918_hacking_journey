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
        "/diagnostics" | "/diagnostics/" => AuthLevel::User,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserLevel;

    /// Build a minimal `UserAccount` for test assertions.
    fn user(level: UserLevel) -> UserAccount {
        UserAccount::new("test", "x", level)
    }

    // ── check_required_level ─────────────────────────────────────────────

    #[test]
    fn test_check_required_level_no_credentials_returns_401() {
        // Ok(None) means no Authorization header was present
        assert_eq!(
            check_required_level(Ok(None), AuthLevel::User),
            Some(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn test_check_required_level_bad_credentials_returns_401() {
        // Err means the header was present but malformed/invalid
        assert_eq!(
            check_required_level(
                Err(OnvifError::NotAuthorized("Invalid credentials".into())),
                AuthLevel::User,
            ),
            Some(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn test_check_required_level_user_on_user_route_passes() {
        assert_eq!(
            check_required_level(Ok(Some(user(UserLevel::User))), AuthLevel::User),
            None
        );
    }

    #[test]
    fn test_check_required_level_operator_on_user_route_passes() {
        assert_eq!(
            check_required_level(Ok(Some(user(UserLevel::Operator))), AuthLevel::User),
            None
        );
    }

    #[test]
    fn test_check_required_level_admin_on_user_route_passes() {
        assert_eq!(
            check_required_level(Ok(Some(user(UserLevel::Administrator))), AuthLevel::User),
            None
        );
    }

    #[test]
    fn test_check_required_level_admin_on_admin_route_passes() {
        assert_eq!(
            check_required_level(
                Ok(Some(user(UserLevel::Administrator))),
                AuthLevel::Administrator,
            ),
            None
        );
    }

    #[test]
    fn test_check_required_level_user_on_admin_route_returns_403() {
        // Metrics require User; logs require Administrator.
        // A plain user must not reach the logs route.
        assert_eq!(
            check_required_level(Ok(Some(user(UserLevel::User))), AuthLevel::Administrator,),
            Some(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn test_check_required_level_operator_on_admin_route_returns_403() {
        assert_eq!(
            check_required_level(
                Ok(Some(user(UserLevel::Operator))),
                AuthLevel::Administrator,
            ),
            Some(StatusCode::FORBIDDEN)
        );
    }

    // ── required_level_for_path ──────────────────────────────────────────
    // The middleware is nested under /api, so axum strips that prefix.
    // Paths seen here are /logs, /diagnostics, etc.

    #[test]
    fn test_required_level_for_path_diagnostics_requires_user() {
        assert_eq!(required_level_for_path("/diagnostics"), AuthLevel::User);
    }

    #[test]
    fn test_required_level_for_path_diagnostics_trailing_slash_requires_user() {
        assert_eq!(required_level_for_path("/diagnostics/"), AuthLevel::User);
    }

    #[test]
    fn test_required_level_for_path_logs_requires_admin() {
        assert_eq!(required_level_for_path("/logs"), AuthLevel::Administrator);
    }

    #[test]
    fn test_required_level_for_path_logs_trailing_slash_requires_admin() {
        assert_eq!(required_level_for_path("/logs/"), AuthLevel::Administrator);
    }

    #[test]
    fn test_required_level_for_path_update_requires_admin() {
        assert_eq!(required_level_for_path("/update"), AuthLevel::Administrator);
    }

    #[test]
    fn test_required_level_for_path_update_trailing_slash_requires_admin() {
        assert_eq!(
            required_level_for_path("/update/"),
            AuthLevel::Administrator
        );
    }

    #[test]
    fn test_required_level_for_path_update_with_api_prefix_requires_admin() {
        assert_eq!(
            required_level_for_path("/api/update"),
            AuthLevel::Administrator
        );
    }

    #[test]
    fn test_required_level_for_path_unknown_fails_closed_to_admin() {
        // Fail-closed: unrecognised routes must not be default-open.
        assert_eq!(required_level_for_path("/status"), AuthLevel::Administrator);
    }

    #[test]
    fn test_required_level_for_path_root_fails_closed_to_admin() {
        assert_eq!(required_level_for_path("/"), AuthLevel::Administrator);
    }

    #[test]
    fn test_required_level_for_path_diagnostics_with_api_prefix_requires_user() {
        assert_eq!(required_level_for_path("/api/diagnostics"), AuthLevel::User);
    }

    // ── reuse proof: verify_basic_auth_self integration ──────────────────
    //
    // These tests exercise the same verify_basic_auth_self → check_required_level
    // pipeline that diagnostics_auth_middleware uses, confirming no duplicate
    // credential-check path was introduced.

    fn auth_pipeline_status(
        account: Option<(&str, &str, UserLevel)>,
        uri: &str,
        required: AuthLevel,
    ) -> Option<StatusCode> {
        use crate::config::{PasswordManager, UserStorage};
        use crate::onvif::dispatcher::{AuthContext, ServiceDispatcher};
        use crate::onvif::ws_security::WsSecurityValidator;
        use axum::body::Body;
        use axum::http::Request as HttpRequest;
        use base64::Engine;
        use std::sync::Arc;

        let user_storage = Arc::new(UserStorage::new());
        if let Some((username, password, level)) = account {
            user_storage.create_user(username, password, level).unwrap();
        }

        let dispatcher = ServiceDispatcher::new();
        let auth_ctx = AuthContext::new(
            Arc::new(WsSecurityValidator::with_defaults()),
            user_storage,
            Arc::new(PasswordManager::new()),
            true,
        );

        let mut builder = HttpRequest::builder().method("GET").uri(uri);
        if let Some((username, password, _)) = account {
            let credentials =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            builder = builder.header("Authorization", format!("Basic {credentials}"));
        }

        let request = builder.body(Body::empty()).unwrap();
        let result = verify_basic_auth_self(&dispatcher, &request, &auth_ctx);
        check_required_level(result, required)
    }

    #[test]
    fn test_reuse_verify_basic_auth_self_no_header_gives_unauthorized() {
        assert_eq!(
            auth_pipeline_status(None, "/api/diagnostics", AuthLevel::User),
            Some(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn test_reuse_verify_basic_auth_self_valid_user_on_metrics_route_passes() {
        assert_eq!(
            auth_pipeline_status(
                Some(("viewer", "pass", UserLevel::User)),
                "/api/diagnostics",
                AuthLevel::User,
            ),
            None,
            "valid User-level credentials must pass /api/diagnostics"
        );
    }

    #[test]
    fn test_reuse_verify_basic_auth_self_user_blocked_from_logs_route() {
        assert_eq!(
            auth_pipeline_status(
                Some(("viewer", "pass", UserLevel::User)),
                "/api/logs",
                required_level_for_path("/logs"),
            ),
            Some(StatusCode::FORBIDDEN),
            "User level must not access /logs"
        );
    }

    #[test]
    fn test_reuse_verify_basic_auth_self_admin_allowed_on_logs_route() {
        assert_eq!(
            auth_pipeline_status(
                Some(("admin", "secret", UserLevel::Administrator)),
                "/api/logs",
                required_level_for_path("/logs"),
            ),
            None,
            "Administrator must be allowed on /logs"
        );
    }
}
