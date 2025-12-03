//! HTTP middleware for memory limit enforcement
//!
//! Implements per-request memory availability checking to ensure graceful
//! degradation under memory pressure (EC-016).

use axum::{
    extract::Extension,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::utils::MemoryMonitor;

/// Typical HTTP request body size (small to moderate)
pub const TYPICAL_REQUEST_SIZE: usize = 64 * 1024; // 64KB

/// Memory enforcement layer - checks hard limit before processing request
///
/// This middleware should be the FIRST in the stack to reject requests early
/// when under memory pressure, before any other processing occurs.
///
/// Uses Extension<Arc<MemoryMonitor>> for state injection.
pub async fn memory_check_middleware(
    Extension(memory): Extension<Arc<MemoryMonitor>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    // Check if request can be processed without exceeding hard limit
    if let Err(e) = memory.check_available(TYPICAL_REQUEST_SIZE) {
        warn!(
            memory_status = %memory.usage_string(),
            reason = %e,
            "Rejecting request due to memory pressure"
        );

        // Return HTTP 503 Service Unavailable per EC-016
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    // Log memory status for high utilization (>80%)
    let current = memory.current_usage();
    let hard_limit = memory.limits.hard_limit;
    let percent = (current * 100) / hard_limit;

    if percent > 80 {
        info!(
            memory_percent = percent,
            memory_status = %memory.usage_string(),
            "High memory utilization; continuing with caution"
        );
    }

    // Process request
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::MemoryLimits;

    #[test]
    fn test_memory_limits_validation() {
        let limits = MemoryLimits::default();
        assert_eq!(limits.soft_limit, 16 * 1024 * 1024);
        assert_eq!(limits.hard_limit, 24 * 1024 * 1024);
    }
}
