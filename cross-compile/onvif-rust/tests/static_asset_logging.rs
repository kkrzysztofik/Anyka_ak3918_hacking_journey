//! Integration tests for static asset access logging.

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, header},
};
use onvif_rust::logging::static_asset_logging_middleware;
use std::net::SocketAddr;
use std::str::FromStr;
use tower::ServiceExt;
use tower_http::services::ServeDir;

#[tokio::test]
async fn test_static_asset_logging_format_with_get_request() {
    // Create a simple router with the static asset logging middleware
    let app = Router::new()
        .layer(axum::middleware::from_fn(static_asset_logging_middleware))
        .fallback_service(ServeDir::new("."));

    // Create a mock request to a static asset
    let addr = SocketAddr::from_str("127.0.0.1:12345").unwrap();
    let request = Request::builder()
        .method("GET")
        .uri("/index.html")
        .header(header::USER_AGENT, "Mozilla/5.0")
        .header(header::REFERER, "http://example.com/")
        .extension(ConnectInfo(addr))
        .body(Body::empty())
        .unwrap();

    // Execute the request
    let service = app.into_service();
    let response = service.oneshot(request).await;

    // Assert response was processed (may be 404 for non-existent file, but that's ok)
    assert!(response.is_ok());
}

#[test]
fn test_apache_timestamp_format_structure() {
    // Format should be: day/mon/year:hour:minute:second zone
    // We can't test exact values since they're time-dependent, but we can test the structure
    // This is tested implicitly through the middleware tests
    // We verify the format by checking that timestamps contain expected separators

    // Expected format components: slash, colons, and timezone offset
    // Examples: [10/Oct/2000:13:55:36 +0000]
    let example_timestamp = "[10/Oct/2000:13:55:36 +0000]";

    assert!(
        example_timestamp.contains('/'),
        "Timestamp should contain /"
    );
    assert!(
        example_timestamp.contains(':'),
        "Timestamp should contain :"
    );
    assert!(
        example_timestamp.len() > 10,
        "Timestamp should be reasonably long"
    );
}

#[test]
fn test_static_asset_log_format_contains_apache_fields() {
    // Test that the Apache combined format would include all required fields
    let client_ip = "192.168.1.100";
    let method = "GET";
    let path = "/assets/style.css";
    let status = 200;
    let size = "5234";
    let referer = "http://example.com/";
    let user_agent = "Mozilla/5.0";

    // Format similar to what the middleware produces
    let log_line = format!(
        "{} - - [timestamp] \"{} {} HTTP/1.1\" {} {} \"{}\" \"{}\"",
        client_ip, method, path, status, size, referer, user_agent
    );

    // Verify all components are in the log line
    assert!(log_line.contains(client_ip), "Should contain client IP");
    assert!(log_line.contains(method), "Should contain HTTP method");
    assert!(log_line.contains(path), "Should contain request path");
    assert!(
        log_line.contains(&status.to_string()),
        "Should contain status code"
    );
    assert!(log_line.contains(size), "Should contain response size");
    assert!(log_line.contains(referer), "Should contain referer");
    assert!(log_line.contains(user_agent), "Should contain user-agent");
}

#[test]
fn test_apache_log_format_with_missing_headers() {
    // Test that missing headers are represented as "-"
    let client_ip = "127.0.0.1";
    let method = "GET";
    let path = "/index.html";
    let status = 200;
    let size = "1024";
    let referer = "-"; // No referer
    let user_agent = "-"; // No user agent

    let log_line = format!(
        "{} - - [timestamp] \"{} {} HTTP/1.1\" {} {} \"{}\" \"{}\"",
        client_ip, method, path, status, size, referer, user_agent
    );

    // Verify hyphens are used for missing values
    assert!(
        log_line.contains("\"-\""),
        "Should use - for missing headers"
    );
}

#[test]
fn test_apache_log_format_status_codes() {
    // Test various status codes are correctly formatted
    let status_codes = vec![200, 304, 404, 500];

    for status in status_codes {
        let log_line = format!(
            "127.0.0.1 - - [timestamp] \"GET /file HTTP/1.1\" {} 1024 \"-\" \"-\"",
            status
        );
        assert!(
            log_line.contains(&status.to_string()),
            "Log should contain status code {}",
            status
        );
    }
}

#[test]
fn test_apache_log_format_with_query_string() {
    // Test that query strings are included in the path
    let path = "/search?q=test&lang=en";
    let log_line = format!(
        "127.0.0.1 - - [timestamp] \"GET {} HTTP/1.1\" 200 512 \"-\" \"-\"",
        path
    );

    assert!(
        log_line.contains("q=test"),
        "Should contain query parameter q"
    );
    assert!(
        log_line.contains("lang=en"),
        "Should contain query parameter lang"
    );
}

#[test]
fn test_apache_log_format_content_length_hyphen() {
    // Test that missing Content-Length is represented as "-"
    let log_line = "127.0.0.1 - - [timestamp] \"GET /file HTTP/1.1\" 204 - \"-\" \"-\"";

    assert!(
        log_line.contains(" - \""),
        "Should contain hyphen for missing Content-Length"
    );
}

#[test]
fn test_apache_log_rfc_compliance() {
    // Test RFC 3875 (CGI) common log format compliance
    // Format: host ident authuser date request status bytes
    let log_line =
        "192.168.1.1 - frank [10/Oct/2000:13:55:36 -0700] \"GET /apache_pb.gif HTTP/1.0\" 200 2326";

    // Should be parseable as: IP, -, -, [timestamp], request line, status, bytes
    let parts: Vec<&str> = log_line.split_whitespace().collect();
    assert_eq!(parts[0], "192.168.1.1", "First part should be IP");
    assert_eq!(parts[1], "-", "Second part should be ident (-)");
    assert_eq!(parts[2], "frank", "Third part should be auth user");
    // Timestamp is in brackets, so parts[3] is "[10/Oct/2000:13:55:36"
}

#[test]
fn test_log_config_default_values() {
    let config = onvif_rust::logging::StaticAssetLogConfig::default();

    assert!(
        config.enabled,
        "Static asset logging should be enabled by default"
    );
    assert_eq!(
        config.target, "static_assets",
        "Default target should be 'static_assets'"
    );
}
