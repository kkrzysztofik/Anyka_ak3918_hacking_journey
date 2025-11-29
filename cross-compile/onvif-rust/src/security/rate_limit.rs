//! Per-IP rate limiting.
//!
//! Limits the number of requests from a single IP address within a time window.
//! Uses DashMap for concurrent access without global locking.
//!
//! # Example
//!
//! ```
//! use onvif_rust::security::rate_limit::RateLimiter;
//! use std::net::IpAddr;
//!
//! let limiter = RateLimiter::new(60); // 60 requests per minute
//!
//! let ip: IpAddr = "192.168.1.100".parse().unwrap();
//!
//! // Check if request is allowed
//! if limiter.check_rate_limit(&ip) {
//!     // Process request
//! } else {
//!     // Return 429 Too Many Requests
//! }
//! ```

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default rate limit: requests per minute.
pub const DEFAULT_RATE_LIMIT: u32 = 60;

/// Default window duration in seconds.
pub const DEFAULT_WINDOW_SECONDS: u64 = 60;

/// Request count for a single IP within a time window.
#[derive(Debug, Clone)]
pub struct RequestCount {
    /// Number of requests in the current window.
    pub count: u32,
    /// When the current window started.
    pub window_start: Instant,
}

impl RequestCount {
    /// Create a new request count starting now.
    pub fn new() -> Self {
        Self {
            count: 1,
            window_start: Instant::now(),
        }
    }
}

impl Default for RequestCount {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-IP rate limiter using sliding window algorithm.
#[derive(Clone)]
pub struct RateLimiter {
    /// Maximum requests allowed per window.
    max_requests: u32,
    /// Window duration.
    window_duration: Duration,
    /// Request counts per IP address.
    counts: Arc<DashMap<IpAddr, RequestCount>>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    ///
    /// * `max_requests_per_minute` - Maximum requests allowed per minute per IP.
    pub fn new(max_requests_per_minute: u32) -> Self {
        Self {
            max_requests: max_requests_per_minute,
            window_duration: Duration::from_secs(DEFAULT_WINDOW_SECONDS),
            counts: Arc::new(DashMap::new()),
        }
    }

    /// Create a rate limiter with custom window duration.
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum requests allowed per window.
    /// * `window_seconds` - Window duration in seconds.
    pub fn with_window(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
            counts: Arc::new(DashMap::new()),
        }
    }

    /// Check if a request from the given IP is within rate limits.
    ///
    /// This method is atomic and thread-safe.
    ///
    /// # Arguments
    ///
    /// * `ip` - The client's IP address
    ///
    /// # Returns
    ///
    /// `true` if the request is allowed, `false` if rate limit exceeded.
    pub fn check_rate_limit(&self, ip: &IpAddr) -> bool {
        let now = Instant::now();

        // Use entry API for atomic update
        let mut entry = self.counts.entry(*ip).or_insert_with(|| RequestCount {
            count: 0,
            window_start: now,
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) > self.window_duration {
            // Reset window
            entry.count = 1;
            entry.window_start = now;
            return true;
        }

        // Increment counter and check limit
        entry.count += 1;

        entry.count <= self.max_requests
    }

    /// Get the current request count for an IP.
    ///
    /// Returns `None` if the IP has no recorded requests.
    pub fn get_count(&self, ip: &IpAddr) -> Option<u32> {
        self.counts.get(ip).map(|entry| entry.count)
    }

    /// Get the remaining requests allowed for an IP.
    ///
    /// # Arguments
    ///
    /// * `ip` - The client's IP address
    ///
    /// # Returns
    ///
    /// The number of remaining requests, or the full limit if no requests recorded.
    pub fn remaining(&self, ip: &IpAddr) -> u32 {
        match self.counts.get(ip) {
            Some(entry) => {
                // Check if window expired
                if entry.window_start.elapsed() > self.window_duration {
                    self.max_requests
                } else {
                    self.max_requests.saturating_sub(entry.count)
                }
            }
            None => self.max_requests,
        }
    }

    /// Clean up expired entries to prevent memory growth.
    ///
    /// Call this periodically (e.g., every minute).
    pub fn cleanup(&self) {
        self.counts
            .retain(|_, entry| entry.window_start.elapsed() <= self.window_duration);
    }

    /// Reset rate limit for a specific IP.
    pub fn reset(&self, ip: &IpAddr) {
        self.counts.remove(ip);
    }

    /// Get the maximum requests per window.
    pub fn max_requests(&self) -> u32 {
        self.max_requests
    }

    /// Get the window duration.
    pub fn window_duration(&self) -> Duration {
        self.window_duration
    }

    /// Get the number of tracked IPs.
    pub fn tracked_ips(&self) -> usize {
        self.counts.len()
    }
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter")
            .field("max_requests", &self.max_requests)
            .field("window_duration", &self.window_duration)
            .field("tracked_ips", &self.counts.len())
            .finish()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_RATE_LIMIT)
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
    fn test_first_request_allowed() {
        let limiter = RateLimiter::new(10);
        assert!(limiter.check_rate_limit(&test_ip()));
    }

    #[test]
    fn test_requests_within_limit_allowed() {
        let limiter = RateLimiter::new(5);
        let ip = test_ip();

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(&ip));
        }
    }

    #[test]
    fn test_requests_over_limit_denied() {
        let limiter = RateLimiter::new(3);
        let ip = test_ip();

        // First 3 allowed
        for _ in 0..3 {
            assert!(limiter.check_rate_limit(&ip));
        }

        // 4th denied
        assert!(!limiter.check_rate_limit(&ip));
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new(2);
        let ip1 = test_ip();
        let ip2 = other_ip();

        // Both IPs get their own limit
        assert!(limiter.check_rate_limit(&ip1));
        assert!(limiter.check_rate_limit(&ip1));
        assert!(!limiter.check_rate_limit(&ip1)); // Third blocked

        // IP2 still has full quota
        assert!(limiter.check_rate_limit(&ip2));
        assert!(limiter.check_rate_limit(&ip2));
        assert!(!limiter.check_rate_limit(&ip2)); // Third blocked
    }

    #[test]
    fn test_remaining_count() {
        let limiter = RateLimiter::new(5);
        let ip = test_ip();

        assert_eq!(limiter.remaining(&ip), 5);

        limiter.check_rate_limit(&ip);
        assert_eq!(limiter.remaining(&ip), 4);

        limiter.check_rate_limit(&ip);
        assert_eq!(limiter.remaining(&ip), 3);
    }

    #[test]
    fn test_get_count() {
        let limiter = RateLimiter::new(10);
        let ip = test_ip();

        assert_eq!(limiter.get_count(&ip), None);

        limiter.check_rate_limit(&ip);
        assert_eq!(limiter.get_count(&ip), Some(1));

        limiter.check_rate_limit(&ip);
        assert_eq!(limiter.get_count(&ip), Some(2));
    }

    #[test]
    fn test_reset() {
        let limiter = RateLimiter::new(2);
        let ip = test_ip();

        limiter.check_rate_limit(&ip);
        limiter.check_rate_limit(&ip);
        assert!(!limiter.check_rate_limit(&ip)); // Blocked

        limiter.reset(&ip);

        // Should be allowed again
        assert!(limiter.check_rate_limit(&ip));
    }

    #[test]
    fn test_window_reset() {
        // Use very short window for testing
        let limiter = RateLimiter::with_window(2, 0); // 0 second window

        let ip = test_ip();

        limiter.check_rate_limit(&ip);
        limiter.check_rate_limit(&ip);

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(10));

        // Should be allowed - window reset
        assert!(limiter.check_rate_limit(&ip));
    }

    #[test]
    fn test_cleanup() {
        let limiter = RateLimiter::with_window(10, 0); // Immediate expiration
        let ip = test_ip();

        limiter.check_rate_limit(&ip);
        assert_eq!(limiter.tracked_ips(), 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(10));

        limiter.cleanup();
        assert_eq!(limiter.tracked_ips(), 0);
    }

    #[test]
    fn test_debug_impl() {
        let limiter = RateLimiter::new(60);
        let debug_str = format!("{:?}", limiter);
        assert!(debug_str.contains("RateLimiter"));
        assert!(debug_str.contains("max_requests: 60"));
    }

    #[test]
    fn test_default() {
        let limiter = RateLimiter::default();
        assert_eq!(limiter.max_requests(), DEFAULT_RATE_LIMIT);
    }

    #[test]
    fn test_ipv6_address() {
        let limiter = RateLimiter::new(3);
        let ipv6: IpAddr = "2001:db8::1".parse().unwrap();

        assert!(limiter.check_rate_limit(&ipv6));
        assert!(limiter.check_rate_limit(&ipv6));
        assert!(limiter.check_rate_limit(&ipv6));
        assert!(!limiter.check_rate_limit(&ipv6)); // 4th blocked

        assert_eq!(limiter.get_count(&ipv6), Some(4));
    }

    #[test]
    fn test_ipv4_and_ipv6_independent() {
        let limiter = RateLimiter::new(2);
        let ipv4: IpAddr = "192.168.1.1".parse().unwrap();
        let ipv6: IpAddr = "::1".parse().unwrap();

        // Exhaust IPv4 limit
        limiter.check_rate_limit(&ipv4);
        limiter.check_rate_limit(&ipv4);
        assert!(!limiter.check_rate_limit(&ipv4));

        // IPv6 should have full quota
        assert!(limiter.check_rate_limit(&ipv6));
        assert!(limiter.check_rate_limit(&ipv6));
    }

    #[test]
    fn test_tracked_ips_count() {
        let limiter = RateLimiter::new(10);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();
        let ip3: IpAddr = "2001:db8::1".parse().unwrap();

        assert_eq!(limiter.tracked_ips(), 0);

        limiter.check_rate_limit(&ip1);
        assert_eq!(limiter.tracked_ips(), 1);

        limiter.check_rate_limit(&ip2);
        assert_eq!(limiter.tracked_ips(), 2);

        limiter.check_rate_limit(&ip3);
        assert_eq!(limiter.tracked_ips(), 3);
    }

    #[test]
    fn test_window_duration_getter() {
        let limiter = RateLimiter::with_window(10, 120);
        assert_eq!(limiter.window_duration(), Duration::from_secs(120));
    }

    #[test]
    fn test_max_requests_getter() {
        let limiter = RateLimiter::new(42);
        assert_eq!(limiter.max_requests(), 42);
    }
}
