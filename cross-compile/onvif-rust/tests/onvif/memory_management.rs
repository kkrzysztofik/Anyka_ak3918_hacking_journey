//! Integration tests for memory management and enforcement
//!
//! Tests verify:
//! - Memory hard limit (24MB) rejection of requests (HTTP 503)
//! - Memory soft limit (16MB) warning without rejection
//! - Connection persistence under memory pressure
//! - Memory monitor initialization from configuration

use std::sync::Arc;
use tokio::task;

use onvif_rust::{
    config::ConfigRuntime,
    utils::{MemoryLimits, MemoryMonitor},
};

/// Test that memory monitor initializes with correct default limits
#[tokio::test]
async fn test_memory_monitor_default_limits() {
    let monitor = MemoryMonitor::new();

    // Verify defaults match spec (T510-T511)
    assert_eq!(monitor.limits.soft_limit, 16 * 1024 * 1024);  // 16MB
    assert_eq!(monitor.limits.hard_limit, 24 * 1024 * 1024);  // 24MB
}

/// Test that memory monitor rejects requests exceeding hard limit
///
/// This test validates the hard limit enforcement logic by setting limits
/// that are intentionally BELOW current process memory usage, ensuring
/// that the hard limit check correctly fails.
#[tokio::test]
async fn test_memory_hard_limit_rejection() {
    // Create monitor with default limits (16MB soft, 24MB hard)
    // These are well above typical test process memory, so normal requests succeed
    let monitor_ok = Arc::new(MemoryMonitor::new());

    // Small request should succeed (current + 64KB < 24MB hard limit)
    let result = monitor_ok.check_available(64 * 1024);
    assert!(result.is_ok(), "Small request should succeed with default limits");

    // Now create a monitor with artificially LOW hard limit (1KB)
    // This is below current process memory, so ANY request should fail
    let limits = MemoryLimits::new(512, 1024).expect("Valid limits");
    let monitor_fail = Arc::new(MemoryMonitor::with_limits(limits));

    // Any request should fail since current_usage() >> 1KB
    let result = monitor_fail.check_available(1);
    assert!(result.is_err(), "Request should fail when current usage exceeds hard limit");
}

/// Test memory soft limit warning behavior
///
/// This test verifies that requests succeed even when above soft limit
/// but below hard limit. The monitor logs warnings but allows the request.
#[tokio::test]
async fn test_memory_soft_limit_warning() {
    // Use default limits: 16MB soft, 24MB hard
    // With typical test process memory ~few MB, this tests the "below soft limit" path
    let monitor = Arc::new(MemoryMonitor::new());

    // Request should succeed - current + 64KB is well below 24MB hard limit
    let result = monitor.check_available(64 * 1024);
    assert!(result.is_ok(), "Request should succeed when below hard limit");

    // Also verify that large requests near but below hard limit still succeed
    // Even if this triggers soft limit warning, the request should proceed
    let result = monitor.check_available(1024 * 1024);  // 1MB request
    assert!(result.is_ok(), "Large request below hard limit should succeed");
}

/// Test memory monitor configuration loading
#[tokio::test]
async fn test_memory_monitor_from_config() {
    // Create minimal config (values will be mocked)
    let config = ConfigRuntime::new(Default::default());
    let monitor = MemoryMonitor::from_config(&config);

    // Should successfully create with fallback defaults
    assert!(monitor.is_ok(), "Should create monitor from config");
    let monitor = monitor.unwrap();

    // Verify limits are set (either from config or defaults)
    assert!(monitor.limits.soft_limit > 0);
    assert!(monitor.limits.hard_limit > monitor.limits.soft_limit);
}

/// Test concurrent memory checks from multiple tasks
///
/// Simulates multiple concurrent requests checking memory availability,
/// ensuring the memory monitor is thread-safe and provides consistent results.
#[tokio::test]
async fn test_memory_monitor_concurrent_checks() {
    let monitor = Arc::new(MemoryMonitor::new());
    let mut handles = vec![];

    // Spawn 10 concurrent tasks checking memory availability
    for _ in 0..10 {
        let monitor_clone = Arc::clone(&monitor);
        let handle = task::spawn(async move {
            // All checks should succeed (within default 24MB limit)
            monitor_clone.check_available(1024).is_ok()
        });
        handles.push(handle);
    }

    // Wait for all tasks and verify they all succeeded
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task should complete successfully");
        assert!(result.unwrap(), "Memory check should succeed");
    }
}

/// Test memory monitor usage string formatting
///
/// Verifies that usage_string() produces valid output for logging and monitoring.
#[tokio::test]
async fn test_memory_usage_string_format() {
    let monitor = MemoryMonitor::new();
    let usage_str = monitor.usage_string();

    // Usage string should contain current and hard limit
    assert!(!usage_str.is_empty(), "Usage string should not be empty");
    assert!(usage_str.contains("MB") || usage_str.contains("KB"),
            "Usage string should include unit");
}

/// Test limits validation during monitor creation
#[tokio::test]
async fn test_memory_limits_validation() {
    // Valid limits (soft < hard) - validation happens in MemoryLimits::new()
    let valid = MemoryLimits::new(16 * 1024 * 1024, 24 * 1024 * 1024);
    assert!(valid.is_ok(), "Valid limits should be accepted");

    // Create monitor from valid limits
    let monitor = MemoryMonitor::with_limits(valid.unwrap());
    assert!(monitor.limits.soft_limit < monitor.limits.hard_limit);

    // Invalid limits (soft >= hard) - should fail validation
    let invalid = MemoryLimits::new(24 * 1024 * 1024, 16 * 1024 * 1024);
    assert!(invalid.is_err(), "Invalid limits should be rejected");
}

/// Test HTTP 503 response when memory hard limit exceeded
///
/// This tests the memory check middleware logic directly.
/// The middleware converts check_available() error into HTTP 503 response.
///
/// Example flow:
/// 1. Request arrives at memory_check_middleware
/// 2. middleware calls monitor.check_available(TYPICAL_REQUEST_SIZE)
/// 3. If Err (hard limit exceeded), return StatusCode::SERVICE_UNAVAILABLE
/// 4. Client receives HTTP 503
#[tokio::test]
async fn test_http_503_response_on_hard_limit() {
    // Test that check_available fails when request would exceed hard limit
    // This is the core logic that the middleware uses

    let limits = MemoryLimits::new(1024, 2048).expect("Valid limits");  // 1KB soft, 2KB hard
    let monitor = Arc::new(MemoryMonitor::with_limits(limits));

    // Middleware uses TYPICAL_REQUEST_SIZE = 64KB
    // With hard limit of 2KB, any request should fail
    const TYPICAL_REQUEST_SIZE: usize = 64 * 1024;

    let result = monitor.check_available(TYPICAL_REQUEST_SIZE);

    // Should fail - this is when middleware returns 503
    assert!(result.is_err(), "Should fail when request exceeds hard limit");

    // Verify error message indicates limit exceeded
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("hard limit") || err_msg.contains("exceeded"),
            "Error should mention hard limit: {}", err_msg);
}

/// Test connection persistence under memory pressure
///
/// Verifies that memory checks are stateless - each check is independent.
/// This ensures that existing connections can continue processing even
/// if the system was briefly under pressure.
#[tokio::test]
async fn test_connection_persistence_under_pressure() {
    let limits = MemoryLimits::new(16 * 1024 * 1024, 24 * 1024 * 1024).expect("Valid limits");  // 16MB soft, 24MB hard
    let monitor = Arc::new(MemoryMonitor::with_limits(limits));

    // Simulate multiple requests in sequence (like persistent connection)
    // Each should be checked independently
    let request_size = 64 * 1024;  // 64KB typical request

    // First request should succeed
    assert!(monitor.check_available(request_size).is_ok(),
            "First request should succeed");

    // Second request should also succeed (memory checks are stateless)
    assert!(monitor.check_available(request_size).is_ok(),
            "Second request should succeed");

    // Third request should also succeed
    assert!(monitor.check_available(request_size).is_ok(),
            "Third request should succeed");

    // Verify usage_string produces valid output (format check, not exact value)
    // Note: Exact values can vary due to live memory state changes
    let usage = monitor.usage_string();
    assert!(usage.contains("MB"), "Usage string should contain MB unit");
    assert!(usage.contains("/"), "Usage string should contain separator");
    assert!(usage.contains("%"), "Usage string should contain percentage");
}

/// Test that memory profiling features work when enabled
#[cfg(feature = "memory-profiling")]
#[tokio::test]
async fn test_memory_profiling_integration() {
    use onvif_rust::utils::AllocationTracker;

    let tracker = AllocationTracker::new();

    // Track some allocations
    tracker.track_allocation(0x1000, 1024, file!(), line!());
    tracker.track_allocation(0x2000, 2048, file!(), line!());

    assert_eq!(tracker.allocation_count(), 2);
    assert_eq!(tracker.current_total(), 3072);

    // Log stats (should not panic)
    tracker.log_stats();

    // Cleanup
    tracker.track_deallocation(0x1000);
    tracker.track_deallocation(0x2000);

    assert_eq!(tracker.allocation_count(), 0);
    // Peak should remain
    assert_eq!(tracker.get_peak_usage(), 3072);
}
