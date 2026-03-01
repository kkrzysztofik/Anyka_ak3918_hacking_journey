//! Security audit logging.
//!
//! Provides structured logging for security-relevant events such as:
//! - Authentication failures
//! - IP address blocking
//! - Attack detection
//! - Rate limit violations
//!
//! All security events are logged at INFO level or higher for visibility
//! in production logs.
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::security::audit::{log_auth_failure, log_ip_blocked, log_attack_detected};
//!
//! // Log failed authentication
//! log_auth_failure(&client_ip, "admin", "Invalid password digest");
//!
//! // Log IP being blocked
//! log_ip_blocked(&client_ip, 300);
//!
//! // Log attack detection
//! log_attack_detected(&client_ip, "XXE");
//! ```

use std::net::IpAddr;
use tracing::{info, warn};

/// Log an authentication failure event.
///
/// # Arguments
///
/// * `client_ip` - The IP address of the client
/// * `username` - The username that failed to authenticate
/// * `reason` - The reason for the failure
pub fn log_auth_failure(client_ip: &IpAddr, username: &str, reason: &str) {
    warn!(
        event = "auth_failure",
        client_ip = %client_ip,
        username = %username,
        reason = %reason,
        "Authentication failure"
    );
}

/// Log an IP address being blocked.
///
/// # Arguments
///
/// * `client_ip` - The IP address being blocked
/// * `duration_secs` - How long the IP is blocked for
pub fn log_ip_blocked(client_ip: &IpAddr, duration_secs: u64) {
    warn!(
        event = "ip_blocked",
        client_ip = %client_ip,
        duration_secs = duration_secs,
        "IP address blocked due to repeated failures"
    );
}

/// Log detection of an attack attempt.
///
/// # Arguments
///
/// * `client_ip` - The IP address of the attacker
/// * `attack_type` - The type of attack detected (e.g., "XXE", "XSS", "XML_BOMB")
pub fn log_attack_detected(client_ip: &IpAddr, attack_type: &str) {
    warn!(
        event = "attack_detected",
        client_ip = %client_ip,
        attack_type = %attack_type,
        "Security attack detected"
    );
}

/// Log a rate limit being exceeded.
///
/// # Arguments
///
/// * `client_ip` - The IP address that exceeded the rate limit
/// * `request_count` - The number of requests made
pub fn log_rate_limit_exceeded(client_ip: &IpAddr, request_count: u32) {
    info!(
        event = "rate_limit_exceeded",
        client_ip = %client_ip,
        request_count = request_count,
        "Rate limit exceeded"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    // These tests just ensure the functions compile and don't panic.
    // Actual log output can be verified with tracing-test in integration tests.

    #[test]
    fn test_log_auth_failure() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        log_auth_failure(&ip, "admin", "Invalid password");
        // No panic = success
    }

    #[test]
    fn test_log_ip_blocked() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        log_ip_blocked(&ip, 300);
    }

    #[test]
    fn test_log_attack_detected() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        log_attack_detected(&ip, "XXE");
    }

    #[test]
    fn test_log_rate_limit_exceeded() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        log_rate_limit_exceeded(&ip, 100);
    }
}
