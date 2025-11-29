//! Brute force protection.
//!
//! Tracks authentication failures per IP and blocks IPs after too many failures.
//! Uses a sliding window approach to prevent brute force attacks while allowing
//! legitimate users who occasionally mistype passwords.
//!
//! # Example
//!
//! ```
//! use onvif_rust::security::brute_force::BruteForceProtection;
//! use std::net::IpAddr;
//!
//! let protection = BruteForceProtection::default();
//! let ip: IpAddr = "192.168.1.100".parse().unwrap();
//!
//! // Check if IP is currently blocked
//! if protection.is_blocked(&ip) {
//!     // Reject connection (EC-012)
//! }
//!
//! // Record a failed authentication attempt
//! protection.record_failure(&ip);
//!
//! // On successful authentication, clear failures
//! protection.clear_failures(&ip);
//! ```

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default maximum failures before blocking.
pub const DEFAULT_MAX_FAILURES: u32 = 5;

/// Default failure window in seconds.
pub const DEFAULT_FAILURE_WINDOW_SECONDS: u64 = 60;

/// Default block duration in seconds.
pub const DEFAULT_BLOCK_DURATION_SECONDS: u64 = 300;

/// Record of authentication failures for a single IP.
#[derive(Debug, Clone)]
pub struct FailureRecord {
    /// Number of failures in the current window.
    pub count: u32,
    /// When the first failure in this window occurred.
    pub first_failure: Instant,
    /// If blocked, when the block expires. `None` if not blocked.
    pub blocked_until: Option<Instant>,
}

impl FailureRecord {
    /// Create a new failure record with the first failure.
    pub fn new() -> Self {
        Self {
            count: 1,
            first_failure: Instant::now(),
            blocked_until: None,
        }
    }

    /// Check if this record is currently blocked.
    pub fn is_blocked(&self) -> bool {
        self.blocked_until
            .is_some_and(|until| Instant::now() < until)
    }
}

impl Default for FailureRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// Brute force protection with configurable thresholds.
#[derive(Clone)]
pub struct BruteForceProtection {
    /// Maximum failures before blocking.
    max_failures: u32,
    /// Time window for counting failures.
    failure_window: Duration,
    /// How long to block an IP.
    block_duration: Duration,
    /// Failure records per IP.
    records: Arc<DashMap<IpAddr, FailureRecord>>,
}

impl BruteForceProtection {
    /// Create a new brute force protector with custom settings.
    ///
    /// # Arguments
    ///
    /// * `max_failures` - Maximum failed attempts before blocking.
    /// * `failure_window_seconds` - Time window for counting failures.
    /// * `block_duration_seconds` - How long to block an IP after threshold.
    pub fn new(
        max_failures: u32,
        failure_window_seconds: u64,
        block_duration_seconds: u64,
    ) -> Self {
        Self {
            max_failures,
            failure_window: Duration::from_secs(failure_window_seconds),
            block_duration: Duration::from_secs(block_duration_seconds),
            records: Arc::new(DashMap::new()),
        }
    }

    /// Check if an IP address is currently blocked.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address to check
    ///
    /// # Returns
    ///
    /// `true` if the IP is blocked, `false` otherwise.
    pub fn is_blocked(&self, ip: &IpAddr) -> bool {
        if let Some(record) = self.records.get(ip) {
            return record.is_blocked();
        }
        false
    }

    /// Record an authentication failure for an IP.
    ///
    /// If the failure count exceeds the threshold within the window,
    /// the IP will be blocked.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address that failed authentication
    ///
    /// # Returns
    ///
    /// `true` if the IP is now blocked after this failure.
    pub fn record_failure(&self, ip: &IpAddr) -> bool {
        let now = Instant::now();

        let mut entry = self.records.entry(*ip).or_insert_with(|| FailureRecord {
            count: 0,
            first_failure: now,
            blocked_until: None,
        });

        // If already blocked, just return true
        if entry.is_blocked() {
            return true;
        }

        // Check if failure window has expired
        if now.duration_since(entry.first_failure) > self.failure_window {
            // Reset the window
            entry.count = 1;
            entry.first_failure = now;
            return false;
        }

        // Increment failure count
        entry.count += 1;

        // Check if we should block
        if entry.count >= self.max_failures {
            entry.blocked_until = Some(now + self.block_duration);
            return true;
        }

        false
    }

    /// Clear authentication failures for an IP.
    ///
    /// Call this on successful authentication to reset the failure count.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address to clear
    pub fn clear_failures(&self, ip: &IpAddr) {
        self.records.remove(ip);
    }

    /// Get the failure count for an IP.
    ///
    /// # Returns
    ///
    /// The current failure count, or 0 if no failures recorded.
    pub fn get_failure_count(&self, ip: &IpAddr) -> u32 {
        self.records.get(ip).map(|r| r.count).unwrap_or(0)
    }

    /// Get the remaining time an IP is blocked for.
    ///
    /// # Returns
    ///
    /// The remaining block duration, or `None` if not blocked.
    pub fn remaining_block_time(&self, ip: &IpAddr) -> Option<Duration> {
        self.records.get(ip).and_then(|record| {
            record.blocked_until.and_then(|until| {
                let now = Instant::now();
                if now < until { Some(until - now) } else { None }
            })
        })
    }

    /// Manually block an IP for a specified duration.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address to block
    /// * `duration` - How long to block for. If `None`, uses default block duration.
    pub fn block_ip(&self, ip: &IpAddr, duration: Option<Duration>) {
        let block_duration = duration.unwrap_or(self.block_duration);
        let until = Instant::now() + block_duration;

        let mut entry = self.records.entry(*ip).or_default();
        entry.blocked_until = Some(until);
    }

    /// Unblock an IP address immediately.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address to unblock
    pub fn unblock_ip(&self, ip: &IpAddr) {
        if let Some(mut record) = self.records.get_mut(ip) {
            record.blocked_until = None;
            record.count = 0;
        }
    }

    /// Clean up expired records.
    ///
    /// Removes records where the failure window has expired and the IP is not blocked.
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.records.retain(|_, record| {
            // Keep if blocked
            if record.blocked_until.is_some_and(|until| now < until) {
                return true;
            }
            // Keep if failure window hasn't expired
            now.duration_since(record.first_failure) <= self.failure_window
        });
    }

    /// Get the number of tracked IPs.
    pub fn tracked_ips(&self) -> usize {
        self.records.len()
    }

    /// Get the current configuration.
    pub fn config(&self) -> (u32, Duration, Duration) {
        (self.max_failures, self.failure_window, self.block_duration)
    }
}

impl std::fmt::Debug for BruteForceProtection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BruteForceProtection")
            .field("max_failures", &self.max_failures)
            .field("failure_window", &self.failure_window)
            .field("block_duration", &self.block_duration)
            .field("tracked_ips", &self.records.len())
            .finish()
    }
}

impl Default for BruteForceProtection {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_FAILURES,
            DEFAULT_FAILURE_WINDOW_SECONDS,
            DEFAULT_BLOCK_DURATION_SECONDS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ip() -> IpAddr {
        "192.168.1.100".parse().unwrap()
    }

    fn other_ip() -> IpAddr {
        "192.168.1.101".parse().unwrap()
    }

    #[test]
    fn test_first_failure_not_blocked() {
        let protection = BruteForceProtection::default();
        let ip = test_ip();

        assert!(!protection.is_blocked(&ip));
        assert!(!protection.record_failure(&ip));
        assert!(!protection.is_blocked(&ip));
    }

    #[test]
    fn test_blocked_after_threshold() {
        let protection = BruteForceProtection::new(3, 60, 300);
        let ip = test_ip();

        // First 2 failures don't block
        assert!(!protection.record_failure(&ip));
        assert!(!protection.record_failure(&ip));
        assert!(!protection.is_blocked(&ip));

        // Third failure triggers block
        assert!(protection.record_failure(&ip));
        assert!(protection.is_blocked(&ip));
    }

    #[test]
    fn test_different_ips_independent() {
        let protection = BruteForceProtection::new(2, 60, 300);
        let ip1 = test_ip();
        let ip2 = other_ip();

        // Block IP1
        protection.record_failure(&ip1);
        protection.record_failure(&ip1);
        assert!(protection.is_blocked(&ip1));

        // IP2 is not affected
        assert!(!protection.is_blocked(&ip2));
        protection.record_failure(&ip2);
        assert!(!protection.is_blocked(&ip2));
    }

    #[test]
    fn test_clear_failures() {
        let protection = BruteForceProtection::new(3, 60, 300);
        let ip = test_ip();

        protection.record_failure(&ip);
        protection.record_failure(&ip);
        assert_eq!(protection.get_failure_count(&ip), 2);

        protection.clear_failures(&ip);
        assert_eq!(protection.get_failure_count(&ip), 0);
    }

    #[test]
    fn test_block_expires() {
        // Very short block duration for testing
        let protection = BruteForceProtection::new(2, 60, 0);
        let ip = test_ip();

        protection.record_failure(&ip);
        protection.record_failure(&ip);

        // Block should expire immediately
        std::thread::sleep(Duration::from_millis(10));
        assert!(!protection.is_blocked(&ip));
    }

    #[test]
    fn test_failure_window_reset() {
        // Very short failure window
        let protection = BruteForceProtection::new(3, 0, 300);
        let ip = test_ip();

        protection.record_failure(&ip);
        protection.record_failure(&ip);

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(10));

        // Window reset, so this is the first failure again
        protection.record_failure(&ip);
        assert_eq!(protection.get_failure_count(&ip), 1);
        assert!(!protection.is_blocked(&ip));
    }

    #[test]
    fn test_manual_block() {
        let protection = BruteForceProtection::default();
        let ip = test_ip();

        assert!(!protection.is_blocked(&ip));

        protection.block_ip(&ip, Some(Duration::from_secs(300)));
        assert!(protection.is_blocked(&ip));
    }

    #[test]
    fn test_unblock() {
        let protection = BruteForceProtection::new(2, 60, 300);
        let ip = test_ip();

        protection.record_failure(&ip);
        protection.record_failure(&ip);
        assert!(protection.is_blocked(&ip));

        protection.unblock_ip(&ip);
        assert!(!protection.is_blocked(&ip));
        assert_eq!(protection.get_failure_count(&ip), 0);
    }

    #[test]
    fn test_remaining_block_time() {
        let protection = BruteForceProtection::new(1, 60, 10);
        let ip = test_ip();

        assert!(protection.remaining_block_time(&ip).is_none());

        protection.record_failure(&ip);
        let remaining = protection.remaining_block_time(&ip).unwrap();
        assert!(remaining.as_secs() <= 10);
    }

    #[test]
    fn test_cleanup() {
        let protection = BruteForceProtection::new(10, 0, 0);
        let ip = test_ip();

        protection.record_failure(&ip);

        // Wait for everything to expire
        std::thread::sleep(Duration::from_millis(10));

        protection.cleanup();
        assert_eq!(protection.tracked_ips(), 0);
    }

    #[test]
    fn test_debug_impl() {
        let protection = BruteForceProtection::default();
        let debug_str = format!("{:?}", protection);
        assert!(debug_str.contains("BruteForceProtection"));
        assert!(debug_str.contains("max_failures: 5"));
    }

    #[test]
    fn test_subsequent_failures_while_blocked() {
        let protection = BruteForceProtection::new(2, 60, 300);
        let ip = test_ip();

        // Get blocked
        protection.record_failure(&ip);
        protection.record_failure(&ip);
        assert!(protection.is_blocked(&ip));

        // Subsequent failure should still return blocked
        assert!(protection.record_failure(&ip));
    }

    #[test]
    fn test_ipv6_address() {
        let protection = BruteForceProtection::new(2, 60, 300);
        let ipv6: IpAddr = "2001:db8::1".parse().unwrap();

        assert!(!protection.is_blocked(&ipv6));
        protection.record_failure(&ipv6);
        assert!(!protection.is_blocked(&ipv6));
        protection.record_failure(&ipv6);
        assert!(protection.is_blocked(&ipv6));
    }

    #[test]
    fn test_ipv4_and_ipv6_independent() {
        let protection = BruteForceProtection::new(2, 60, 300);
        let ipv4: IpAddr = "192.168.1.1".parse().unwrap();
        let ipv6: IpAddr = "::1".parse().unwrap();

        // Block IPv4
        protection.record_failure(&ipv4);
        protection.record_failure(&ipv4);
        assert!(protection.is_blocked(&ipv4));

        // IPv6 should be unaffected
        assert!(!protection.is_blocked(&ipv6));
        protection.record_failure(&ipv6);
        assert!(!protection.is_blocked(&ipv6));
    }

    #[test]
    fn test_get_failure_count_no_failures() {
        let protection = BruteForceProtection::default();
        let ip = test_ip();

        assert_eq!(protection.get_failure_count(&ip), 0);
    }

    #[test]
    fn test_get_failure_count_with_failures() {
        let protection = BruteForceProtection::default();
        let ip = test_ip();

        protection.record_failure(&ip);
        assert_eq!(protection.get_failure_count(&ip), 1);

        protection.record_failure(&ip);
        assert_eq!(protection.get_failure_count(&ip), 2);

        protection.record_failure(&ip);
        assert_eq!(protection.get_failure_count(&ip), 3);
    }

    #[test]
    fn test_config_getter() {
        let protection = BruteForceProtection::new(10, 120, 600);
        let (max_failures, failure_window, block_duration) = protection.config();

        assert_eq!(max_failures, 10);
        assert_eq!(failure_window, Duration::from_secs(120));
        assert_eq!(block_duration, Duration::from_secs(600));
    }

    #[test]
    fn test_tracked_ips_count() {
        let protection = BruteForceProtection::default();
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        assert_eq!(protection.tracked_ips(), 0);

        protection.record_failure(&ip1);
        assert_eq!(protection.tracked_ips(), 1);

        protection.record_failure(&ip2);
        assert_eq!(protection.tracked_ips(), 2);
    }

    #[test]
    fn test_manual_block_with_default_duration() {
        let protection = BruteForceProtection::new(10, 60, 300);
        let ip = test_ip();

        protection.block_ip(&ip, None); // Uses default 300s
        assert!(protection.is_blocked(&ip));

        let remaining = protection.remaining_block_time(&ip).unwrap();
        assert!(remaining.as_secs() <= 300);
        assert!(remaining.as_secs() > 290); // Should be close to 300
    }
}
