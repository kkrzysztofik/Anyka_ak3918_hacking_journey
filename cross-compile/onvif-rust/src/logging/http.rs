//! HTTP request/response logging middleware.
//!
//! This module provides configurable logging middleware for HTTP requests and responses.
//! When verbose mode is enabled via `[logging] http_verbose = true`, it logs:
//!
//! - HTTP method and path
//! - Request/response status codes
//! - Request latency
//! - Truncated, sanitized SOAP payloads (for debugging)
//!
//! # Configuration
//!
//! The middleware behavior is controlled by the `[logging]` configuration section:
//!
//! ```toml
//! [logging]
//! http_verbose = true       # Enable detailed HTTP logging
//! ```
//!
//! # Security
//!
//! When logging SOAP payloads, the middleware:
//! - Truncates large payloads to prevent log flooding
//! - Sanitizes sensitive data (passwords, tokens) before logging
//! - Only logs when verbose mode is explicitly enabled
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::logging::{HttpLogConfig, HttpLoggingMiddleware};
//!
//! let config = HttpLogConfig {
//!     verbose: true,
//!     max_body_log_size: 4096,
//!     sanitize_passwords: true,
//! };
//!
//! let middleware = HttpLoggingMiddleware::new(config);
//!
//! let app = Router::new()
//!     .layer(middleware.layer());
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use tower::{Layer, Service};

/// Configuration for HTTP logging middleware.
#[derive(Debug, Clone)]
pub struct HttpLogConfig {
    /// Enable verbose HTTP logging (logs request/response bodies).
    pub verbose: bool,

    /// Maximum number of bytes to log from request/response bodies.
    /// Bodies larger than this are truncated with "...[truncated]" suffix.
    pub max_body_log_size: usize,

    /// Whether to sanitize passwords and tokens in logged bodies.
    pub sanitize_passwords: bool,
}

impl Default for HttpLogConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            max_body_log_size: 4096,
            sanitize_passwords: true,
        }
    }
}

/// HTTP logging middleware.
///
/// This middleware wraps an inner service and logs HTTP request/response
/// metadata and optionally the SOAP payloads when verbose mode is enabled.
#[derive(Debug, Clone)]
pub struct HttpLoggingMiddleware {
    config: Arc<HttpLogConfig>,
}

impl HttpLoggingMiddleware {
    /// Create a new HTTP logging middleware with the given configuration.
    pub fn new(config: HttpLogConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create a tower Layer from this middleware.
    pub fn layer(&self) -> HttpLoggingLayer {
        HttpLoggingLayer {
            config: Arc::clone(&self.config),
        }
    }
}

/// Tower Layer for HTTP logging middleware.
#[derive(Debug, Clone)]
pub struct HttpLoggingLayer {
    config: Arc<HttpLogConfig>,
}

impl<S> Layer<S> for HttpLoggingLayer {
    type Service = HttpLoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpLoggingService {
            inner,
            config: Arc::clone(&self.config),
        }
    }
}

/// Tower Service for HTTP logging middleware.
#[derive(Debug, Clone)]
pub struct HttpLoggingService<S> {
    inner: S,
    config: Arc<HttpLogConfig>,
}

impl<S> Service<Request<Body>> for HttpLoggingService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let config = Arc::clone(&self.config);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let method = request.method().clone();
            let uri = request.uri().clone();
            let path = uri.path().to_string();
            let start = Instant::now();

            // Extract SOAP action from header if present
            let soap_action = request
                .headers()
                .get("SOAPAction")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim_matches('"').to_string());

            // Handle verbose vs non-verbose logging
            let response = if config.verbose {
                // Buffer request body for logging
                let (parts, body) = request.into_parts();
                let body_bytes = match body.collect().await {
                    Ok(collected) => collected.to_bytes(),
                    Err(_) => Bytes::new(),
                };

                // Log request with body
                log_request_verbose(
                    method.as_ref(),
                    &path,
                    soap_action.as_deref(),
                    &body_bytes,
                    &config,
                );

                // Reconstruct request with buffered body
                let request = Request::from_parts(parts, Body::from(body_bytes));

                // Call inner service
                let response = inner.call(request).await?;

                // Buffer response body for logging
                let (parts, body) = response.into_parts();
                let body_bytes = match body.collect().await {
                    Ok(collected) => collected.to_bytes(),
                    Err(_) => Bytes::new(),
                };

                let duration = start.elapsed();
                let status = parts.status;

                // Log response with body
                log_response_verbose(
                    method.as_ref(),
                    &path,
                    status,
                    duration.as_millis(),
                    &body_bytes,
                    &config,
                );

                // Reconstruct response with buffered body
                Response::from_parts(parts, Body::from(body_bytes))
            } else {
                // Non-verbose mode: just log basic info
                tracing::debug!(
                    method = %method,
                    path = %path,
                    soap_action = ?soap_action,
                    "→ HTTP request"
                );

                let response = inner.call(request).await?;

                let duration = start.elapsed();
                let status = response.status();

                tracing::info!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    latency_ms = %duration.as_millis(),
                    "← HTTP response"
                );

                response
            };

            Ok(response)
        })
    }
}

/// Log verbose request details including sanitized body.
fn log_request_verbose(
    method: &str,
    path: &str,
    soap_action: Option<&str>,
    body: &Bytes,
    config: &HttpLogConfig,
) {
    let body_preview =
        sanitize_and_truncate_body(body, config.max_body_log_size, config.sanitize_passwords);

    tracing::info!(
        method = %method,
        path = %path,
        soap_action = ?soap_action,
        body_size = body.len(),
        "→ HTTP request"
    );
    tracing::info!("→ Request body: {}", body_preview);
}

/// Log verbose response details including sanitized body.
fn log_response_verbose(
    method: &str,
    path: &str,
    status: StatusCode,
    latency_ms: u128,
    body: &Bytes,
    config: &HttpLogConfig,
) {
    let body_preview =
        sanitize_and_truncate_body(body, config.max_body_log_size, config.sanitize_passwords);

    if status.is_success() {
        tracing::info!(
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = latency_ms,
            body_size = body.len(),
            "← HTTP response"
        );
        tracing::info!("← Response body: {}", body_preview);
    } else {
        tracing::warn!(
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = latency_ms,
            body_size = body.len(),
            "← HTTP response error"
        );
        tracing::warn!("← Response body: {}", body_preview);
    }
}

/// Sanitize and truncate a body for safe logging.
///
/// This function:
/// 1. Converts bytes to UTF-8 string (lossy)
/// 2. Sanitizes sensitive data (passwords, tokens, nonces) if enabled
/// 3. Truncates to max length with "[truncated]" suffix
/// 4. Replaces newlines with spaces for single-line logging
fn sanitize_and_truncate_body(body: &Bytes, max_bytes: usize, sanitize: bool) -> String {
    if body.is_empty() {
        return "<empty>".to_string();
    }

    // Convert to string (lossy for non-UTF8)
    let body_str = String::from_utf8_lossy(body);

    // Sanitize sensitive data if enabled
    let sanitized = if sanitize {
        sanitize_soap_body(&body_str)
    } else {
        body_str.to_string()
    };

    // Truncate if needed
    let truncated = if sanitized.len() > max_bytes {
        format!("{}...[truncated]", &sanitized[..max_bytes])
    } else {
        sanitized
    };

    // Replace newlines for single-line logging
    truncated.replace('\n', " ").replace('\r', "")
}

/// Sanitize SOAP body by masking sensitive data.
///
/// Masks:
/// - Password elements: `<Password>***</Password>`
/// - PasswordDigest: `<PasswordDigest>***</PasswordDigest>`
/// - Nonce values: `<Nonce>***</Nonce>`
///
/// This implementation uses simple string matching instead of regex
/// to keep the binary size small for embedded systems.
fn sanitize_soap_body(body: &str) -> String {
    let mut result = body.to_string();

    // List of sensitive element names to sanitize
    let sensitive_elements = [
        "Password",
        "wsse:Password",
        "PasswordDigest",
        "wsse:PasswordDigest",
        "Nonce",
        "wsse:Nonce",
    ];

    for element in sensitive_elements {
        result = sanitize_xml_element(&result, element);
    }

    result
}

/// Sanitize a specific XML element by replacing its content with "***".
///
/// Handles elements with attributes: `<Element attr="val">content</Element>`
fn sanitize_xml_element(body: &str, element_name: &str) -> String {
    let mut result = String::with_capacity(body.len());
    let mut remaining = body;

    loop {
        // Find opening tag (with potential attributes)
        let open_pattern = format!("<{}", element_name);
        if let Some(open_start) = remaining.find(&open_pattern) {
            // Add everything before the tag
            result.push_str(&remaining[..open_start]);

            // Find the end of the opening tag
            let after_open = &remaining[open_start..];
            if let Some(tag_end) = after_open.find('>') {
                // Check if it's a self-closing tag
                if tag_end > 0 && after_open.as_bytes()[tag_end - 1] == b'/' {
                    // Self-closing tag, keep as is
                    result.push_str(&after_open[..=tag_end]);
                    remaining = &after_open[tag_end + 1..];
                    continue;
                }

                // Find closing tag
                let close_pattern = format!("</{}>", element_name);
                let content_start = tag_end + 1;
                let after_content = &after_open[content_start..];

                if let Some(close_pos) = after_content.find(&close_pattern) {
                    // Add opening tag with attributes
                    result.push_str(&after_open[..content_start]);
                    // Add masked content
                    result.push_str("***");
                    // Add closing tag
                    result.push_str(&close_pattern);

                    remaining = &after_content[close_pos + close_pattern.len()..];
                    continue;
                }
            }

            // If we get here, tag structure is broken - just add the character and continue
            result.push_str(&remaining[..open_start + 1]);
            remaining = &remaining[open_start + 1..];
        } else {
            // No more tags found, add remaining content
            result.push_str(remaining);
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_password() {
        let input = r#"<Password>secret123</Password>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(result, r#"<Password>***</Password>"#);
    }

    #[test]
    fn test_sanitize_password_with_namespace() {
        let input = r#"<wsse:Password>secret123</wsse:Password>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(result, r#"<wsse:Password>***</wsse:Password>"#);
    }

    #[test]
    fn test_sanitize_password_with_attribute() {
        let input = r#"<wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">mysecret</wsse:Password>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(
            result,
            r#"<wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">***</wsse:Password>"#
        );
    }

    #[test]
    fn test_sanitize_password_digest() {
        let input = r#"<wsse:PasswordDigest>abc123def456</wsse:PasswordDigest>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(result, r#"<wsse:PasswordDigest>***</wsse:PasswordDigest>"#);
    }

    #[test]
    fn test_sanitize_nonce() {
        let input = r#"<wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">randomnonce==</wsse:Nonce>"#;
        let result = sanitize_soap_body(input);
        assert!(result.contains("<wsse:Nonce EncodingType="));
        assert!(result.contains(">***</wsse:Nonce>"));
    }

    #[test]
    fn test_sanitize_complex_soap() {
        let input = r#"<?xml version="1.0"?>
<soap:Envelope>
  <soap:Header>
    <wsse:Security>
      <wsse:UsernameToken>
        <wsse:Username>admin</wsse:Username>
        <wsse:Password Type="...">mysecret</wsse:Password>
        <wsse:Nonce>abc123</wsse:Nonce>
      </wsse:UsernameToken>
    </wsse:Security>
  </soap:Header>
  <soap:Body>
    <GetDeviceInformation/>
  </soap:Body>
</soap:Envelope>"#;

        let result = sanitize_soap_body(input);

        // Username should be preserved
        assert!(result.contains("<wsse:Username>admin</wsse:Username>"));
        // Password should be masked
        assert!(result.contains(">***</wsse:Password>"));
        // Nonce should be masked
        assert!(result.contains(">***</wsse:Nonce>"));
        // Body content should be preserved
        assert!(result.contains("<GetDeviceInformation/>"));
    }

    #[test]
    fn test_truncate_body() {
        let body = Bytes::from("a".repeat(2000));
        let result = sanitize_and_truncate_body(&body, 100, false);

        assert!(result.len() < 150); // 100 + "...[truncated]"
        assert!(result.ends_with("...[truncated]"));
    }

    #[test]
    fn test_empty_body() {
        let body = Bytes::new();
        let result = sanitize_and_truncate_body(&body, 100, true);
        assert_eq!(result, "<empty>");
    }

    #[test]
    fn test_config_default() {
        let config = HttpLogConfig::default();
        assert!(!config.verbose);
        assert_eq!(config.max_body_log_size, 4096);
        assert!(config.sanitize_passwords);
    }

    #[test]
    fn test_newlines_replaced() {
        let body = Bytes::from("line1\nline2\r\nline3");
        let result = sanitize_and_truncate_body(&body, 100, false);
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
        assert_eq!(result, "line1 line2 line3");
    }

    #[test]
    fn test_self_closing_tag() {
        let input = r#"<Password/>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(result, r#"<Password/>"#);
    }

    #[test]
    fn test_multiple_passwords() {
        let input = r#"<Password>first</Password><Password>second</Password>"#;
        let result = sanitize_soap_body(input);
        assert_eq!(
            result,
            r#"<Password>***</Password><Password>***</Password>"#
        );
    }
}
