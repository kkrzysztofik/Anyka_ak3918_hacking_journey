//! Apache combined format access logging for static assets.
//!
//! This module provides middleware for logging static asset requests in the Apache
//! combined log format:
//!
//! ```text
//! IP - - [timestamp] "METHOD PATH HTTP/1.1" STATUS SIZE "Referer" "User-Agent"
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::logging::StaticAssetLogging;
//! use axum::middleware;
//!
//! app.layer(middleware::from_fn(StaticAssetLogging::middleware))
//! ```

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, header},
    middleware::Next,
    response::Response,
};
use chrono::Local;
use std::net::SocketAddr;

/// Configuration for static asset access logging.
#[derive(Debug, Clone)]
pub struct StaticAssetLogConfig {
    /// Enable static asset access logging.
    pub enabled: bool,
    /// Target name for tracing (e.g., "access_log").
    pub target: String,
}

impl Default for StaticAssetLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target: "static_assets".to_string(),
        }
    }
}

/// Middleware for logging static asset requests in Apache combined format.
///
/// This middleware logs all requests to static assets with the following information:
/// - Client IP address (from ConnectInfo)
/// - Request method and path
/// - HTTP status code
/// - Response Content-Length (or "-" if not available)
/// - Referer header
/// - User-Agent header
/// - Request timestamp in Apache format
pub async fn static_asset_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let client_ip = addr.ip().to_string();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let full_path = format!("{}{}", path, query);

    // Extract headers before consuming the request
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    // Get the current timestamp in Apache format
    let timestamp = format_apache_timestamp();

    // Call the next middleware/handler
    let response = next.run(req).await;

    // Extract response status and Content-Length
    let status = response.status().as_u16();
    let content_length = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    // Format Apache combined log entry
    let log_entry = format!(
        "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"{}\" \"{}\"",
        client_ip, timestamp, method, full_path, status, content_length, referer, user_agent
    );

    // Log to the static_assets target
    // Use the target-specific span for routing to the separate access log file
    let span = tracing::span!(target: "static_assets", tracing::Level::INFO, "");
    let _enter = span.enter();
    tracing::info!(target: "static_assets", "{}", log_entry);

    response
}

/// Format a timestamp in Apache combined log format.
///
/// Apache format: `[day/mon/year:hour:minute:second zone]`
/// Example: `[10/Oct/2000:13:55:36 -0700]`
fn format_apache_timestamp() -> String {
    let now = Local::now();
    let offset = now.format("%z");
    let formatted_offset = format!(
        "{}:{}",
        &offset.to_string()[..3],
        &offset.to_string()[3..]
    );
    let year_time = now.format("%Y:%H:%M:%S");
    format!(
        "{}/{}:{} {}",
        now.format("%d"),
        now.format("%b"),
        year_time,
        formatted_offset
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apache_timestamp_format() {
        let timestamp = format_apache_timestamp();
        // Check format: [day/mon/year:hour:minute:second zone]
        assert!(timestamp.contains('/'), "Timestamp should contain /");
        assert!(timestamp.contains(':'), "Timestamp should contain :");
    }

    #[test]
    fn test_static_asset_log_config_default() {
        let config = StaticAssetLogConfig::default();
        assert!(config.enabled);
        assert_eq!(config.target, "static_assets");
    }
}
