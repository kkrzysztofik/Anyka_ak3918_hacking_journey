//! Memory monitoring and enforcement for embedded targets
//!
//! Implements soft and hard memory limits with graceful degradation under pressure.
//! Uses the `cap` crate for hard-limit allocation enforcement.
//!
//! ## Architecture Decision: `cap` Crate
//!
//! We use the `cap` crate for allocation enforcement instead of custom implementation because:
//! - **Battle-tested**: Used by 194+ GitHub projects (Solayer, Quiknode, etc.)
//! - **Purpose-built**: Designed for hard-limit enforcement (not just observation)
//! - **Production-proven**: Used in blockchain/embedded systems with tight resource constraints
//! - **Zero-cost**: Lock-free atomic counter overhead is minimal on ARMv5TE
//! - **No platform-specific code**: Works on ARM5TE, x86_64, and all Rust targets
//! - **Maintenance-free**: Stable API, unlikely to change
//!
//! See [MEMORY_MANAGEMENT.md](../../docs/MEMORY_MANAGEMENT.md) for full architectural rationale.

use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};
use crate::config::ConfigRuntime;

/// Memory limits for embedded target (ARMv5TE with ~32MB available to ONVIF services)
pub struct MemoryLimits {
    /// Soft limit: log warning when exceeded, continue processing
    pub soft_limit: usize,
    /// Hard limit: return HTTP 503 when allocation would exceed this
    pub hard_limit: usize,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            soft_limit: 16 * 1024 * 1024,  // 16 MB
            hard_limit: 24 * 1024 * 1024,  // 24 MB
        }
    }
}

impl MemoryLimits {
    /// Create custom limits (primarily for testing)
    pub fn new(soft_limit: usize, hard_limit: usize) -> Result<Self> {
        if soft_limit >= hard_limit {
            return Err(anyhow!(
                "soft_limit ({}) must be less than hard_limit ({})",
                soft_limit,
                hard_limit
            ));
        }
        Ok(Self {
            soft_limit,
            hard_limit,
        })
    }
}

/// Memory monitor wrapping `cap` allocator with soft/hard limit handling
pub struct MemoryMonitor {
    pub limits: MemoryLimits,
    soft_limit_warned: Arc<AtomicBool>,
}

impl MemoryMonitor {
    /// Create memory monitor with default 16MB soft / 24MB hard limits
    pub fn new() -> Self {
        Self::with_limits(MemoryLimits::default())
    }

    /// Create memory monitor with custom limits
    pub fn with_limits(limits: MemoryLimits) -> Self {
        info!(
            "Memory monitoring initialized: soft={:.1}MB, hard={:.1}MB",
            limits.soft_limit as f64 / 1024.0 / 1024.0,
            limits.hard_limit as f64 / 1024.0 / 1024.0
        );

        Self {
            limits,
            soft_limit_warned: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Load limits from application configuration (if available)
    ///
    /// Falls back to defaults if not configured
    pub fn from_config(config: &ConfigRuntime) -> Result<Self> {
        let soft_mb = config.get_int("memory.soft_limit_mb").ok().map(|v| v as usize);
        let hard_mb = config.get_int("memory.hard_limit_mb").ok().map(|v| v as usize);

        let soft = soft_mb.unwrap_or(16) * 1024 * 1024;
        let hard = hard_mb.unwrap_or(24) * 1024 * 1024;

        let limits = MemoryLimits::new(soft, hard)?;
        Ok(Self::with_limits(limits))
    }

    /// Check if allocation would exceed hard limit
    ///
    /// Returns error if request would exceed hard limit
    /// Logs soft limit warning if approaching capacity
    pub fn check_available(&self, request_size: usize) -> Result<()> {
        let current = self.current_usage();

        // Check hard limit
        if current + request_size > self.limits.hard_limit {
            warn!(
                "Hard memory limit reached: current={:.1}MB, request={:.1}MB, hard={:.1}MB",
                current as f64 / 1024.0 / 1024.0,
                request_size as f64 / 1024.0,
                self.limits.hard_limit as f64 / 1024.0 / 1024.0
            );
            return Err(anyhow!("Memory hard limit would be exceeded"));
        }

        // Check soft limit (warn once per request, not per allocation)
        if current > self.limits.soft_limit && !self.soft_limit_warned.load(Ordering::Relaxed) {
            warn!(
                "Soft memory limit reached: {:.1}MB / {:.1}MB",
                current as f64 / 1024.0 / 1024.0,
                self.limits.soft_limit as f64 / 1024.0 / 1024.0
            );
            self.soft_limit_warned.store(true, Ordering::Relaxed);
        }

        // Reset warning flag if we've dropped below soft limit
        if current < self.limits.soft_limit {
            self.soft_limit_warned.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Get current memory usage from the global allocator
    pub fn current_usage(&self) -> usize {
        crate::allocated()
    }

    /// Get memory status formatted for logging
    pub fn usage_string(&self) -> String {
        let current = self.current_usage();
        let percent = (current as f64 / self.limits.hard_limit as f64) * 100.0;
        format!(
            "{:.1}MB / {:.1}MB ({:.0}%)",
            current as f64 / 1024.0 / 1024.0,
            self.limits.hard_limit as f64 / 1024.0 / 1024.0,
            percent
        )
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory Profiling (Feature-Gated)
// ============================================================================

/// Information about a tracked allocation
///
/// Used for debugging memory usage patterns during development.
#[cfg(feature = "memory-profiling")]
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Size of the allocation in bytes
    pub size: usize,
    /// Source file where allocation originated
    pub file: &'static str,
    /// Line number in source file
    pub line: u32,
}

/// Tracks memory allocations for profiling during development
///
/// This struct is only available when compiled with `--features memory-profiling`.
/// It uses a `DashMap` for concurrent access from multiple threads.
///
/// # Example
///
/// ```ignore
/// #[cfg(feature = "memory-profiling")]
/// {
///     let tracker = AllocationTracker::new();
///     tracker.track_allocation(0x1000, 1024, file!(), line!());
///     tracker.log_stats();
///     tracker.track_deallocation(0x1000);
/// }
/// ```
#[cfg(feature = "memory-profiling")]
pub struct AllocationTracker {
    /// Map of allocation address to allocation info
    allocations: dashmap::DashMap<usize, AllocationInfo>,
    /// Peak memory usage observed
    peak_usage: std::sync::atomic::AtomicUsize,
}

#[cfg(feature = "memory-profiling")]
impl AllocationTracker {
    /// Create a new allocation tracker
    pub fn new() -> Self {
        info!("Memory profiling enabled - allocation tracking active");
        Self {
            allocations: dashmap::DashMap::new(),
            peak_usage: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Track a new allocation
    ///
    /// # Arguments
    ///
    /// * `addr` - Memory address of the allocation
    /// * `size` - Size of the allocation in bytes
    /// * `file` - Source file where allocation originated (use `file!()`)
    /// * `line` - Line number in source file (use `line!()`)
    pub fn track_allocation(&self, addr: usize, size: usize, file: &'static str, line: u32) {
        self.allocations.insert(addr, AllocationInfo { size, file, line });

        // Update peak usage if necessary
        let current_total: usize = self.allocations.iter().map(|e| e.size).sum();
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current_total > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current_total,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
    }

    /// Stop tracking an allocation (on deallocation)
    ///
    /// # Arguments
    ///
    /// * `addr` - Memory address being deallocated
    pub fn track_deallocation(&self, addr: usize) {
        self.allocations.remove(&addr);
    }

    /// Log current memory statistics
    ///
    /// Outputs current usage, peak usage, and allocation count to tracing.
    pub fn log_stats(&self) {
        let total: usize = self.allocations.iter().map(|e| e.size).sum();
        let peak = self.peak_usage.load(Ordering::Relaxed);

        info!(
            current_kb = total / 1024,
            peak_kb = peak / 1024,
            allocation_count = self.allocations.len(),
            "Memory profiling stats"
        );
    }

    /// Get the peak memory usage observed
    ///
    /// Returns the maximum total allocation size observed since tracker creation.
    pub fn get_peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::Relaxed)
    }

    /// Get current total allocation size
    pub fn current_total(&self) -> usize {
        self.allocations.iter().map(|e| e.size).sum()
    }

    /// Get count of active allocations
    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }
}

#[cfg(feature = "memory-profiling")]
impl Default for AllocationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limits_validation() {
        // Valid limits
        assert!(MemoryLimits::new(16 * 1024 * 1024, 24 * 1024 * 1024).is_ok());

        // Invalid: soft >= hard
        assert!(MemoryLimits::new(24 * 1024 * 1024, 24 * 1024 * 1024).is_err());
        assert!(MemoryLimits::new(25 * 1024 * 1024, 24 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_memory_monitor_creation() {
        let _monitor = MemoryMonitor::new();
        let _monitor = MemoryMonitor::default();

        // Test from_config with default config
        let config = crate::config::ConfigRuntime::new(Default::default());
        assert!(MemoryMonitor::from_config(&config).is_ok());
    }

    #[test]
    fn test_memory_check() {
        let monitor = MemoryMonitor::new();
        let _current = monitor.current_usage();
        let limit = monitor.limits.hard_limit;

        // Small allocation should succeed
        let small = 1024;
        assert!(monitor.check_available(small).is_ok());

        // Requesting more than available should fail
        let too_large = limit + 1024;
        assert!(monitor.check_available(too_large).is_err());
    }

    #[test]
    fn test_usage_string() {
        let monitor = MemoryMonitor::new();
        let usage = monitor.usage_string();

        // Should contain MB and percentage
        assert!(usage.contains("MB"));
        assert!(usage.contains("%"));
    }

    #[cfg(feature = "memory-profiling")]
    mod profiling_tests {
        use super::*;

        #[test]
        fn test_allocation_tracker_creation() {
            let tracker = AllocationTracker::new();
            assert_eq!(tracker.allocation_count(), 0);
            assert_eq!(tracker.get_peak_usage(), 0);
        }

        #[test]
        fn test_allocation_tracking() {
            let tracker = AllocationTracker::new();

            // Track some allocations
            tracker.track_allocation(0x1000, 1024, file!(), line!());
            tracker.track_allocation(0x2000, 2048, file!(), line!());

            assert_eq!(tracker.allocation_count(), 2);
            assert_eq!(tracker.current_total(), 3072);
            assert_eq!(tracker.get_peak_usage(), 3072);

            // Deallocate one
            tracker.track_deallocation(0x1000);

            assert_eq!(tracker.allocation_count(), 1);
            assert_eq!(tracker.current_total(), 2048);
            // Peak should remain at 3072
            assert_eq!(tracker.get_peak_usage(), 3072);
        }

        #[test]
        fn test_peak_usage_tracking() {
            let tracker = AllocationTracker::new();

            // Allocate
            tracker.track_allocation(0x1000, 4096, file!(), line!());
            assert_eq!(tracker.get_peak_usage(), 4096);

            // Deallocate
            tracker.track_deallocation(0x1000);
            assert_eq!(tracker.current_total(), 0);

            // Peak should still be 4096
            assert_eq!(tracker.get_peak_usage(), 4096);

            // New allocation smaller than peak
            tracker.track_allocation(0x3000, 1024, file!(), line!());
            assert_eq!(tracker.get_peak_usage(), 4096);
        }
    }
}
