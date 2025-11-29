//! Security hardening for ONVIF services.
//!
//! This module implements various security mechanisms to protect against
//! common attacks and abuse:
//!
//! - **Rate Limiting**: Prevent abuse by limiting requests per IP
//! - **Brute Force Protection**: Block IPs after failed authentication attempts
//! - **XML Security**: Detect and block XXE and XML bomb attacks
//! - **Audit Logging**: Log security events for monitoring and forensics
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::security::{RateLimiter, BruteForceProtection, XmlSecurityValidator};
//!
//! // Create security components
//! let rate_limiter = RateLimiter::new(60); // 60 requests/minute
//! let brute_force = BruteForceProtection::new(5, 60, 300);
//! let xml_validator = XmlSecurityValidator::default();
//!
//! // Check rate limit
//! if !rate_limiter.check_rate_limit(&client_ip) {
//!     // Return HTTP 429 Too Many Requests
//! }
//!
//! // Record authentication failure
//! brute_force.record_failure(&client_ip);
//!
//! // Check if IP is blocked
//! if brute_force.is_blocked(&client_ip) {
//!     // Reject connection
//! }
//! ```

mod audit;
mod brute_force;
mod rate_limit;
mod xml_security;

pub use audit::{log_attack_detected, log_auth_failure, log_ip_blocked, log_rate_limit_exceeded};
pub use brute_force::{BruteForceProtection, FailureRecord};
pub use rate_limit::{RateLimiter, RequestCount};
pub use xml_security::{XmlSecurityError, XmlSecurityValidator};
