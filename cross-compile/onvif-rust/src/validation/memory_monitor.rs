use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Memory usage statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub current_mb: f64,
    pub peak_mb: f64,
    pub average_mb: f64,
    pub sample_count: u64,
}

/// Internal memory monitor state - consolidated to reduce lock contention
#[derive(Debug, Clone, Copy)]
struct MemoryState {
    current_mb: f64,
    peak_mb: f64,
    total_mb: f64,
    sample_count: u64,
}

/// Memory monitor for tracking usage during streaming
pub struct MemoryMonitor {
    state: Arc<RwLock<MemoryState>>,
    max_threshold_mb: f64,
}

/// RAII guard for a background monitoring task.
///
/// Dropping this guard aborts the background task to avoid leaking tasks across tests.
pub struct MonitoringGuard {
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for MonitoringGuard {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl MemoryMonitor {
    /// Create a new memory monitor with specified threshold
    ///
    /// # Arguments
    ///
    /// * `max_threshold_mb` - Maximum allowed memory in MB (24 for Anyka spec)
    pub fn new(max_threshold_mb: f64) -> Self {
        Self {
            state: Arc::new(RwLock::new(MemoryState {
                current_mb: 0.0,
                peak_mb: 0.0,
                total_mb: 0.0,
                sample_count: 0,
            })),
            max_threshold_mb,
        }
    }

    /// Start memory monitoring in background task
    ///
    /// Returns a guard that aborts the monitoring task on drop.
    pub fn start_monitoring(&self) -> MonitoringGuard {
        let state = Arc::clone(&self.state);
        let max_threshold = self.max_threshold_mb;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                interval.tick().await;

                // Get current heap usage via the crate's global allocator tracking.
                let usage_mb = Self::get_current_memory_mb();

                // Update all state fields in a single lock acquisition
                let mut s = state.write().await;
                s.current_mb = usage_mb;

                // Update peak if needed
                if usage_mb > s.peak_mb {
                    s.peak_mb = usage_mb;
                }

                // Update running total and count
                s.total_mb += usage_mb;
                s.sample_count += 1;

                let _avg = s.total_mb / (s.sample_count as f64);

                // Warn if approaching threshold
                if usage_mb > max_threshold * 0.8 {
                    warn!(
                        "Memory usage at {:.1}MB/{:.0}MB (80% threshold)",
                        usage_mb, max_threshold
                    );
                }

                if usage_mb > max_threshold {
                    warn!(
                        "Memory usage exceeded maximum: {:.1}MB > {:.0}MB",
                        usage_mb, max_threshold
                    );
                }
            }
        });

        MonitoringGuard { handle }
    }

    /// Get current memory usage in MB
    pub async fn get_current_usage_mb(&self) -> f64 {
        self.state.read().await.current_mb
    }

    /// Get peak memory usage in MB
    pub async fn get_peak_usage_mb(&self) -> f64 {
        self.state.read().await.peak_mb
    }

    /// Get average memory usage in MB
    pub async fn get_average_usage_mb(&self) -> f64 {
        let s = self.state.read().await;
        if s.sample_count > 0 {
            s.total_mb / (s.sample_count as f64)
        } else {
            0.0
        }
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        let s = self.state.read().await;
        let average = if s.sample_count > 0 {
            s.total_mb / (s.sample_count as f64)
        } else {
            0.0
        };

        MemoryStats {
            current_mb: s.current_mb,
            peak_mb: s.peak_mb,
            average_mb: average,
            sample_count: s.sample_count,
        }
    }

    /// Check if memory usage is within budget
    pub async fn is_within_budget(&self) -> bool {
        self.state.read().await.peak_mb <= self.max_threshold_mb
    }

    /// Get memory usage percentage of threshold
    pub async fn get_usage_percentage(&self) -> f64 {
        let current = self.state.read().await.current_mb;
        (current / self.max_threshold_mb) * 100.0
    }

    /// Print memory statistics to log
    pub async fn log_stats(&self) {
        let stats = self.get_stats().await;
        info!(
            "Memory Stats: current={:.1}MB, peak={:.1}MB, avg={:.1}MB, samples={}",
            stats.current_mb, stats.peak_mb, stats.average_mb, stats.sample_count
        );
    }

    /// Get current heap usage in MB using the crate's allocation tracking.
    ///
    /// This is intentionally based on heap allocations (via `cap::Cap`) rather than RSS.
    /// RSS includes code segments, thread stacks, and instrumentation overhead (e.g. coverage),
    /// which makes CI runs (tarpaulin) non-representative of the embedded target budget.
    fn get_current_memory_mb() -> f64 {
        (crate::allocated() as f64) / 1024.0 / 1024.0
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new(24.0) // 24MB default for Anyka
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_monitor_creation() {
        let monitor = MemoryMonitor::new(24.0);

        let stats = monitor.get_stats().await;
        assert_eq!(stats.current_mb, 0.0);
        assert_eq!(stats.peak_mb, 0.0);
        assert_eq!(stats.sample_count, 0);
    }

    #[tokio::test]
    async fn test_memory_stats_tracking() {
        let monitor = MemoryMonitor::new(24.0);

        // Simulate memory readings
        {
            let mut state = monitor.state.write().await;
            state.current_mb = 10.0;
            state.peak_mb = 10.0;
            state.total_mb = 10.0;
            state.sample_count = 1;
        }

        assert_eq!(monitor.get_current_usage_mb().await, 10.0);
        assert_eq!(monitor.get_peak_usage_mb().await, 10.0);
        assert_eq!(monitor.get_average_usage_mb().await, 10.0);
    }

    #[tokio::test]
    async fn test_memory_budget_check() {
        let monitor = MemoryMonitor::new(24.0);

        {
            let mut state = monitor.state.write().await;
            state.current_mb = 20.0;
            state.peak_mb = 20.0;
        }

        assert!(monitor.is_within_budget().await);

        {
            let mut state = monitor.state.write().await;
            state.current_mb = 25.0;
            state.peak_mb = 25.0;
        }

        assert!(!monitor.is_within_budget().await);
    }

    #[test]
    fn test_default_monitor() {
        let monitor = MemoryMonitor::default();
        assert_eq!(monitor.max_threshold_mb, 24.0);
    }
}
