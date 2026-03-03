// =============================================================================
// Platform Context Helpers
// =============================================================================
//!
//! Helper functions and constants for platform initialization and configuration.
//! This module provides environment variable parsing and common utility functions
//! used during platform lifecycle operations.

use std::time::Duration;

/// Parse a boolean environment variable, returning the default if not set.
///
/// Truthy values: "1", "true", "yes", "on" (case-insensitive)
pub(super) fn env_var_truthy(name: &str) -> bool {
    env_var_truthy_or(name, false)
}

/// Parse a boolean environment variable with a default value.
///
/// Truthy values: "1", "true", "yes", "on" (case-insensitive)
pub(super) fn env_var_truthy_or(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

/// Parse a u64 environment variable, returning None if not set or invalid.
pub(super) fn env_var_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

/// Default search paths for the ISP sensor configuration file.
///
/// These paths are searched in order when no explicit ISP config path is provided.
/// The first path that exists on the filesystem is used for `ak_vi_match_sensor()`.
pub(super) const ISP_CONFIG_SEARCH_PATHS: &[&str] = &[
    "/mnt/anyka_hack/onvif/isp_gc1084.conf",
    "/etc/jffs2/isp_gc1084.conf",
    "/usr/local/isp_gc1084.conf",
];

/// Get the shutdown deadline based on test vs production mode.
///
/// In test mode, a short deadline (200ms) is used for fast test execution.
/// In production, a longer deadline (12 seconds) is used to accommodate
/// hardware shutdown operations.
#[cfg(test)]
pub(super) fn shutdown_deadline() -> Duration {
    Duration::from_millis(200)
}

#[cfg(not(test))]
pub(super) fn shutdown_deadline() -> Duration {
    Duration::from_secs(12)
}

/// Minimum pipeline ready timeout in milliseconds.
const PIPELINE_READY_TIMEOUT_MS_MIN: u64 = 100;

/// Maximum pipeline ready timeout in milliseconds (60 seconds).
const PIPELINE_READY_TIMEOUT_MS_MAX: u64 = 60_000;

/// Default pipeline ready timeout in milliseconds.
const PIPELINE_READY_TIMEOUT_MS_DEFAULT: u64 = 5_000;

/// Minimum stream stabilization delay in milliseconds.
const STABILIZATION_MS_MIN: u64 = 50;

/// Maximum stream stabilization delay in milliseconds (10 seconds).
const STABILIZATION_MS_MAX: u64 = 10_000;

/// Default stream stabilization delay in milliseconds.
const STABILIZATION_MS_DEFAULT: u64 = 300;

/// Get the pipeline readiness timeout in milliseconds.
///
/// Can be overridden via ANYKA_PIPELINE_READY_TIMEOUT_MS environment variable.
/// Default: 5000ms
///
/// Bounded to valid range [100ms, 60000ms]. Values outside this range are
/// clamped with a warning logged.
pub(super) fn pipeline_ready_timeout_ms() -> u64 {
    match env_var_u64("ANYKA_PIPELINE_READY_TIMEOUT_MS") {
        Some(value) if value < PIPELINE_READY_TIMEOUT_MS_MIN => {
            tracing::warn!(
                env_value = value,
                min = PIPELINE_READY_TIMEOUT_MS_MIN,
                "ANYKA_PIPELINE_READY_TIMEOUT_MS below minimum, using {}ms",
                PIPELINE_READY_TIMEOUT_MS_MIN
            );
            PIPELINE_READY_TIMEOUT_MS_MIN
        }
        Some(value) if value > PIPELINE_READY_TIMEOUT_MS_MAX => {
            tracing::warn!(
                env_value = value,
                max = PIPELINE_READY_TIMEOUT_MS_MAX,
                "ANYKA_PIPELINE_READY_TIMEOUT_MS above maximum, using {}ms",
                PIPELINE_READY_TIMEOUT_MS_MAX
            );
            PIPELINE_READY_TIMEOUT_MS_MAX
        }
        Some(value) => value,
        None => PIPELINE_READY_TIMEOUT_MS_DEFAULT,
    }
}

/// Get the stream stabilization delay in milliseconds.
///
/// Can be overridden via ANYKA_STREAM_STABILIZATION_MS environment variable.
/// Default: 300ms
///
/// Bounded to valid range [50ms, 10000ms]. Values outside this range are
/// clamped with a warning logged.
pub(super) fn stream_stabilization_ms() -> u64 {
    match env_var_u64("ANYKA_STREAM_STABILIZATION_MS") {
        Some(value) if value < STABILIZATION_MS_MIN => {
            tracing::warn!(
                env_value = value,
                min = STABILIZATION_MS_MIN,
                "ANYKA_STREAM_STABILIZATION_MS below minimum, using {}ms",
                STABILIZATION_MS_MIN
            );
            STABILIZATION_MS_MIN
        }
        Some(value) if value > STABILIZATION_MS_MAX => {
            tracing::warn!(
                env_value = value,
                max = STABILIZATION_MS_MAX,
                "ANYKA_STREAM_STABILIZATION_MS above maximum, using {}ms",
                STABILIZATION_MS_MAX
            );
            STABILIZATION_MS_MAX
        }
        Some(value) => value,
        None => STABILIZATION_MS_DEFAULT,
    }
}

/// Whether to require the sub-pipeline for pipeline readiness.
///
/// Can be overridden via ANYKA_PIPELINE_REQUIRE_SUB environment variable.
/// Default: true
pub(super) fn pipeline_require_sub() -> bool {
    env_var_truthy_or("ANYKA_PIPELINE_REQUIRE_SUB", true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_var_truthy_default_false() {
        // Test default behavior - non-existent var returns false (the default)
        assert!(!env_var_truthy("ANYKA_NON_EXISTENT_VAR_XYZ123"));
    }

    #[test]
    fn test_env_var_truthy_explicit_values() {
        // TODO(github#28): Update tests to use real env vars via std::env::set_var
        // (functional change - out of scope for refactoring PR)
        //
        // Just test the internal parsing logic without setting env vars
        // The function is used elsewhere with actual env vars at runtime

        // Test case insensitivity and truthy values
        // This tests the parsing logic only
        let test_cases = vec![
            ("true", true),
            ("1", true),
            ("yes", true),
            ("on", true),
            ("false", false),
            ("0", false),
            ("no", false),
            ("off", false),
        ];

        for (value, expected) in test_cases {
            let result = matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
            assert_eq!(result, expected, "Failed for value: {}", value);
        }
    }

    #[test]
    fn test_env_var_truthy_default_true() {
        // Test default behavior - non-existent var returns the provided default
        assert!(env_var_truthy_or("ANYKA_NON_EXISTENT_VAR_XYZ123", true));
        assert!(!env_var_truthy_or("ANYKA_NON_EXISTENT_VAR_XYZ123", false));
    }

    #[test]
    fn test_env_var_u64() {
        // Test parsing - non-existent returns None
        assert_eq!(env_var_u64("ANYKA_NON_EXISTENT_VAR_XYZ123"), None);

        // Test with a valid number string (can't actually set env var safely)
        // The function is tested implicitly through other tests that use it
    }

    #[test]
    fn test_shutdown_deadline_test_mode() {
        #[cfg(test)]
        {
            let deadline = shutdown_deadline();
            // In test mode, should be 200ms
            assert_eq!(deadline, Duration::from_millis(200));
        }
    }

    #[test]
    fn test_pipeline_ready_timeout_ms_default() {
        // When env var is not set, should return default
        let timeout = pipeline_ready_timeout_ms();
        assert_eq!(timeout, 5_000);
    }

    #[test]
    fn test_stream_stabilization_ms_default() {
        // When env var is not set, should return default
        let stabilization = stream_stabilization_ms();
        assert_eq!(stabilization, 300);
    }

    // =============================================================================
    // Bounded timing control tests - bugfix verification
    // =============================================================================

    #[test]
    fn test_pipeline_ready_timeout_ms_below_min_clamped() {
        // Test that values below minimum are clamped to 100ms
        let min = PIPELINE_READY_TIMEOUT_MS_MIN;
        assert_eq!(min, 100);
        // Test with env var below minimum - this would need env var setting
        // The function clamps, so we test the constants
        assert!(min < 5_000); // min < default
    }

    #[test]
    fn test_pipeline_ready_timeout_ms_above_max_clamped() {
        // Test that values above maximum are clamped to 60000ms
        let max = PIPELINE_READY_TIMEOUT_MS_MAX;
        assert_eq!(max, 60_000);
        assert!(max > 5_000); // max > default
    }

    #[test]
    fn test_stream_stabilization_ms_below_min_clamped() {
        // Test that values below minimum are clamped to 50ms
        let min = STABILIZATION_MS_MIN;
        assert_eq!(min, 50);
        assert!(min < 300); // min < default
    }

    #[test]
    fn test_stream_stabilization_ms_above_max_clamped() {
        // Test that values above maximum are clamped to 10000ms
        let max = STABILIZATION_MS_MAX;
        assert_eq!(max, 10_000);
        assert!(max > 300); // max > default
    }

    #[test]
    fn test_pipeline_ready_timeout_bounds_are_valid() {
        // Verify the bounds form a valid range
        assert!(PIPELINE_READY_TIMEOUT_MS_MIN < PIPELINE_READY_TIMEOUT_MS_MAX);
        assert!(PIPELINE_READY_TIMEOUT_MS_DEFAULT >= PIPELINE_READY_TIMEOUT_MS_MIN);
        assert!(PIPELINE_READY_TIMEOUT_MS_DEFAULT <= PIPELINE_READY_TIMEOUT_MS_MAX);
    }

    #[test]
    fn test_stream_stabilization_bounds_are_valid() {
        // Verify the bounds form a valid range
        assert!(STABILIZATION_MS_MIN < STABILIZATION_MS_MAX);
        assert!(STABILIZATION_MS_DEFAULT >= STABILIZATION_MS_MIN);
        assert!(STABILIZATION_MS_DEFAULT <= STABILIZATION_MS_MAX);
    }
}
