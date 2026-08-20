//! Per-IP rate limiting.
//!
//! Limits the number of requests from a single IP address within a time window.
//! State lives behind one mutex: the target is a single-core armv5te, where a sharded map
//! costs a dependency and an inexact capacity bound to stripe locks nothing contends on.
//!
//! ## Important Limitations
//!
//! **Per-Instance Rate Limiting**: This implementation maintains rate limit state
//! in memory on each server instance. Without shared state (Redis/database), each
//! server instance has its own independent rate limit counter. This means:
//!
//! - In a single-instance deployment: Rate limiting works as expected
//! - In a multi-instance deployment (load balancer): Each instance tracks rate limits
//!   independently, effectively multiplying the allowed rate by the number of instances
//!
//! For distributed rate limiting across multiple instances, you would need to:
//! - Use a shared state store (Redis, database, etc.)
//! - Implement consistent hashing to route same IP to same instance
//! - Or accept per-instance limits as acceptable for your use case
//!
//! This per-instance approach is acceptable for single-instance deployments and
//! provides protection against abuse without requiring external dependencies.
//!
//! ## Tracked IP cap
//!
//! The number of distinct IP keys is capped at [`MAX_TRACKED_IPS`]. Before inserting a
//! new key, if the map is at capacity [`RateLimiter::cleanup`] may run so expired windows
//! can free slots; inline cleanup is **throttled** (hundreds of ms) to avoid repeated
//! `retain()` scans under sustained load at the cap. If still full—or cleanup was skipped
//! due to throttle—the new IP is denied. Existing IPs continue to be rate-limited normally.
//! The cap is exact: the check and the insert happen under one held lock, so no concurrent
//! caller can slip a key in past it.
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

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Default rate limit: requests per minute.
pub const DEFAULT_RATE_LIMIT: u32 = 60;

/// Default window duration in seconds.
pub const DEFAULT_WINDOW_SECONDS: u64 = 60;

/// Maximum distinct IP addresses tracked in the map.
///
/// Without a cap, a LAN scan with unique source IPs can grow the map without
/// bound and exhaust memory on embedded targets.
pub const MAX_TRACKED_IPS: usize = 10_000;

/// Minimum time between inline [`RateLimiter::cleanup`] calls triggered from [`RateLimiter::check_rate_limit`]
/// when the map is at [`MAX_TRACKED_IPS`].
const INLINE_CLEANUP_MIN_INTERVAL: Duration = Duration::from_millis(500);

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

/// Everything the limiter mutates, behind one lock.
///
/// A single lock rather than a sharded map: the target is a single-core armv5te, so striping
/// buys nothing, and one lock lets [`RateLimiter::check_rate_limit`] hold the guard across
/// check-cap-insert. That makes [`MAX_TRACKED_IPS`] an exact bound instead of a racy one.
#[derive(Debug)]
struct RateLimitState {
    /// Request counts per IP address.
    counts: HashMap<IpAddr, RequestCount>,
    /// Last time inline cleanup ran from [`RateLimiter::check_rate_limit`] (throttle for `cleanup` cost).
    last_inline_cleanup: Option<Instant>,
}

/// Per-IP rate limiter using sliding window algorithm.
#[derive(Clone)]
pub struct RateLimiter {
    /// Maximum requests allowed per window.
    max_requests: u32,
    /// Window duration.
    window_duration: Duration,
    /// Mutable tracking state.
    state: Arc<Mutex<RateLimitState>>,
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
            state: Self::new_state(),
        }
    }

    fn new_state() -> Arc<Mutex<RateLimitState>> {
        Arc::new(Mutex::new(RateLimitState {
            counts: HashMap::new(),
            last_inline_cleanup: None,
        }))
    }

    /// Lock the tracking state, recovering from poisoning.
    ///
    /// A panicked request must not take the server's rate limiting down with it; the counters
    /// are advisory and a torn update is worth less than a dead listener.
    fn state(&self) -> MutexGuard<'_, RateLimitState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
            state: Self::new_state(),
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
        let mut state = self.state();

        if let Some(entry) = state.counts.get_mut(ip) {
            return Self::apply_request_window(entry, self, now);
        }

        // Bound memory for new keys. The guard is held across check-cap-insert, so the cap is
        // exact: no window in which another thread can insert past it.
        if state.counts.len() >= MAX_TRACKED_IPS {
            let due = state
                .last_inline_cleanup
                .map(|last| now.duration_since(last) >= INLINE_CLEANUP_MIN_INTERVAL)
                .unwrap_or(true);
            if !due {
                return false;
            }
            // Must not call `self.cleanup()` here: `std::sync::Mutex` is not reentrant and we
            // already hold the guard, so that would deadlock instantly.
            Self::retain_unexpired(&mut state.counts, self.window_duration);
            // Stamp after the scan, not with the `now` from entry: the throttle bounds the
            // duty cycle of `retain_unexpired`, and anchoring it at scan *start* would let a
            // scan that outran the interval be followed immediately by another.
            state.last_inline_cleanup = Some(Instant::now());
            if state.counts.len() >= MAX_TRACKED_IPS {
                return false;
            }
        }

        match state.counts.entry(*ip) {
            Entry::Occupied(mut occ) => Self::apply_request_window(occ.get_mut(), self, now),
            Entry::Vacant(vac) => {
                let entry = vac.insert(RequestCount {
                    count: 0,
                    window_start: now,
                });
                Self::apply_request_window(entry, self, now)
            }
        }
    }

    /// Drop entries whose window has already closed. Caller holds the state lock.
    fn retain_unexpired(counts: &mut HashMap<IpAddr, RequestCount>, window_duration: Duration) {
        counts.retain(|_, entry| entry.window_start.elapsed() <= window_duration);
    }

    fn apply_request_window(entry: &mut RequestCount, limiter: &RateLimiter, now: Instant) -> bool {
        // 0 means "no limit", not "deny everything". Every other 0-valued knob in
        // this config reads as off/auto (snapshot_port = 0 -> use the main port),
        // and the deny reading silently closed the whole ONVIF API whenever the
        // key was absent from config.toml: the UI served its static assets but
        // every SOAP call 429'd, which looks like a broken login, not a limiter.
        if limiter.max_requests == 0 {
            return true;
        }
        if now.duration_since(entry.window_start) > limiter.window_duration {
            entry.count = 1;
            entry.window_start = now;
            return true;
        }
        entry.count += 1;
        entry.count <= limiter.max_requests
    }

    /// Get the current request count for an IP.
    ///
    /// Returns `None` if the IP has no recorded requests.
    pub fn get_count(&self, ip: &IpAddr) -> Option<u32> {
        self.state().counts.get(ip).map(|entry| entry.count)
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
        match self.state().counts.get(ip) {
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
        let window_duration = self.window_duration;
        Self::retain_unexpired(&mut self.state().counts, window_duration);
    }

    /// Reset rate limit for a specific IP.
    pub fn reset(&self, ip: &IpAddr) {
        self.state().counts.remove(ip);
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
        self.state().counts.len()
    }

    /// Start a background task that periodically cleans up expired entries.
    ///
    /// The task will call `cleanup()` at the specified interval until
    /// a shutdown signal is received. This prevents memory growth from
    /// accumulating expired rate limit entries.
    ///
    /// # Arguments
    ///
    /// * `cleanup_interval` - How often to run cleanup (e.g., `Duration::from_secs(60)` for 1 minute)
    /// * `shutdown_rx` - Receiver for shutdown signals. The task will exit when a signal is received.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` that can be used to await the task's completion.
    pub fn start_cleanup_task(
        self: Arc<Self>,
        cleanup_interval: Duration,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(cleanup_interval);
            // Skip the first tick (it fires immediately)
            interval_timer.tick().await;

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        self.cleanup();
                        tracing::debug!("Rate limiter cleanup: {} tracked IPs", self.tracked_ips());
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Rate limiter cleanup task shutting down");
                        break;
                    }
                }
            }
        })
    }
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("RateLimiter");
        out.field("max_requests", &self.max_requests)
            .field("window_duration", &self.window_duration);
        // `try_lock`, not `lock`: formatting a `RateLimiter` from inside a section that already
        // holds the state lock would otherwise deadlock, and a diagnostic is never worth that.
        match self.state.try_lock() {
            Ok(state) => out.field("tracked_ips", &state.counts.len()),
            Err(_) => out.field("tracked_ips", &"<locked>"),
        };
        out.finish()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_RATE_LIMIT)
    }
}

// ============================================================================
// Axum Middleware Integration
// ============================================================================

// Note: The rate limiting middleware is implemented in src/onvif/server.rs
// as `rate_limit_middleware()` to avoid feature gate issues and because
// it needs access to OnvifServerState. The middleware is integrated into
// the server router during server initialization.

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
    fn test_zero_limit_means_unlimited_not_denied() {
        // A limit of 0 used to reject every request, which closed the whole
        // ONVIF API whenever config.toml omitted rate_limit_per_minute.
        let limiter = RateLimiter::new(0);
        let ip: IpAddr = "192.168.30.121".parse().expect("ip");

        for i in 0..200 {
            assert!(
                limiter.check_rate_limit(&ip),
                "request {i} denied by a 0 (unlimited) rate limit"
            );
        }
    }

    #[test]
    fn test_server_config_default_does_not_deny_every_request() {
        // Guards the pairing: the shipped config omits the key, so the serde
        // default is what production actually runs with.
        let cfg = crate::config::types::ServerConfig::default();
        let limiter = RateLimiter::new(cfg.rate_limit_per_minute);
        let ip: IpAddr = "192.168.30.121".parse().expect("ip");

        assert!(
            limiter.check_rate_limit(&ip),
            "the default ServerConfig must allow at least one request"
        );
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

    /// Filling the map with distinct IPs must not grow past [`MAX_TRACKED_IPS`];
    /// additional unique IPs are denied until entries expire and `cleanup` runs.
    #[test]
    #[ignore = "iterates MAX_TRACKED_IPS (10k); run with: cargo test test_tracked_ip_cap_rejects_new_ip_when_full -- --ignored"]
    fn test_tracked_ip_cap_rejects_new_ip_when_full() {
        // One request per long window so each IP stays in the map as a distinct key.
        let limiter = RateLimiter::with_window(10, 3600);

        for i in 0..MAX_TRACKED_IPS {
            let ip = IpAddr::V4(std::net::Ipv4Addr::from(i as u32));
            assert!(
                limiter.check_rate_limit(&ip),
                "request for ip index {i} should be allowed"
            );
        }

        assert_eq!(limiter.tracked_ips(), MAX_TRACKED_IPS);

        let overflow_ip = IpAddr::V4(std::net::Ipv4Addr::from(MAX_TRACKED_IPS as u32));
        assert!(
            !limiter.check_rate_limit(&overflow_ip),
            "new IP should be rejected when map is at capacity"
        );

        // Known IP still works (cap applies only to new keys).
        let first = IpAddr::V4(std::net::Ipv4Addr::from(0u32));
        assert!(limiter.check_rate_limit(&first));
    }

    /// The sharded map made each read-modify-write atomic behind its entry guard. The single
    /// mutex has to keep that: a lost increment would let the effective limit drift upward
    /// with the number of concurrent callers, which is a rate limiter that does not limit.
    #[test]
    fn test_concurrent_requests_on_one_ip_do_not_lose_increments() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const THREADS: usize = 8;
        const PER_THREAD: usize = 50;
        const LIMIT: u32 = 100;

        // Long window so nothing expires mid-test.
        let limiter = RateLimiter::with_window(LIMIT, 3600);
        let allowed = AtomicUsize::new(0);
        let ip = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1));

        std::thread::scope(|scope| {
            for _ in 0..THREADS {
                scope.spawn(|| {
                    for _ in 0..PER_THREAD {
                        if limiter.check_rate_limit(&ip) {
                            allowed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        assert_eq!(
            allowed.load(Ordering::Relaxed),
            LIMIT as usize,
            "exactly the limit may pass, regardless of how the attempts interleave"
        );
        assert_eq!(
            limiter.get_count(&ip),
            Some((THREADS * PER_THREAD) as u32),
            "every attempt must be counted; a lost update means a dropped increment"
        );
    }
}
