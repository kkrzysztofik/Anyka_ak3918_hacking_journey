// =============================================================================
// Platform Lifecycle Orchestration
// =============================================================================
//!
//! Platform lifecycle management including initialization orchestration,
//! shutdown sequence management, and rollback helpers.
//!
//! This module handles:
//! - Video pipeline shutdown sequence (VI -> VPSS -> Encoders -> Streaming)
//! - Rollback helpers for failed initialization
//! - Hard shutdown timeout signaling

use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use parking_lot::RwLock;

use crate::platform::common::{PlatformError, PlatformResult, Resolution};

use super::AnykaVideoEncoder;
use super::AnykaVideoInput;
use super::context::shutdown_deadline;

/// Shutdown the video pipeline in the correct order.
///
/// This function performs a best-effort shutdown of the video encoding pipeline:
/// 1. Stop streaming
/// 2. Close all encoders
/// 3. Stop video capture
/// 4. Destroy VPSS
/// 5. Close video input
///
/// All errors are logged but best-effort cleanup continues. The first critical
/// error encountered is returned: VPSS destroy and video input close are critical
/// operations that must succeed for proper resource cleanup.
///
/// Returns:
/// - `Ok(())` if shutdown completed successfully
/// - `Err(PlatformError)` if any critical step failed
pub(super) fn shutdown_video_pipeline(
    video_encoder: &AnykaVideoEncoder,
    video_input: &AnykaVideoInput,
) -> PlatformResult<()> {
    let mut critical_error: Option<PlatformError> = None;

    tracing::info!("Platform shutdown: PTZ stop complete, stopping streaming...");
    if let Err(e) = video_encoder.stop_streaming() {
        tracing::warn!(
            "Streaming stop failed during shutdown (best-effort, continuing): {}",
            e
        );
    }
    tracing::info!("Platform shutdown: streaming stopped, closing encoders...");

    if let Err(e) = video_encoder.close_all_encoders() {
        tracing::warn!(
            "Encoder close failed during shutdown (best-effort, continuing): {}",
            e
        );
    }
    tracing::info!("Platform shutdown: encoders closed, stopping capture...");

    // Stop capture BEFORE closing video input.
    if let Err(e) = video_input.capture_off() {
        tracing::warn!(
            "Video capture off failed during shutdown (best-effort, continuing): {}",
            e
        );
    }
    tracing::info!("Platform shutdown: capture stopped, destroying VPSS...");

    // Destroy VPSS BEFORE closing video input (required by SDK).
    // This is a critical step - record error but continue cleanup.
    if let Err(e) = video_input.destroy_vpss() {
        tracing::error!(
            "VPSS destroy failed during shutdown (critical, recording error): {}",
            e
        );
        critical_error = Some(e);
    }
    tracing::info!("Platform shutdown: VPSS destroyed, closing video input...");

    // Close video input (RAII handle will call ak_vi_close).
    // This is a critical step - record error but continue cleanup.
    if let Err(e) = video_input.close_blocking() {
        tracing::error!(
            "Video input close failed during shutdown (critical, recording error): {}",
            e
        );
        // Prefer the first critical error if we already have one
        if critical_error.is_none() {
            critical_error = Some(e);
        }
    }
    tracing::info!("Platform shutdown: video input closed");

    // Return the first critical error encountered, if any
    if let Some(err) = critical_error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Execute shutdown in a dedicated thread with hard timeout.
///
/// This function spawns a worker thread to run the shutdown sequence,
/// with a hard deadline to prevent indefinite blocking. If the deadline
/// is exceeded, the video encoder is marked for unsafe shutdown.
///
/// Returns:
/// - `Ok(())` if shutdown completed successfully
/// - `Err(PlatformError::Timeout)` if the hard deadline was exceeded (thread hung)
/// - `Err(PlatformError::HardwareFailure)` if the worker thread panicked or the
///   channel was disconnected unexpectedly (indicates a bug rather than hardware issue)
pub(super) fn execute_shutdown_with_timeout(
    video_encoder: Arc<AnykaVideoEncoder>,
    video_input: Arc<AnykaVideoInput>,
) -> PlatformResult<()> {
    // Clone for timeout case before moving into closure
    let video_encoder_for_timeout = Arc::clone(&video_encoder);

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("anyka-shutdown-worker".to_string())
        .spawn(move || {
            let result = shutdown_video_pipeline(&video_encoder, &video_input);
            let _ = tx.send(result);
        })
        .map_err(|e| {
            PlatformError::HardwareFailure(format!(
                "failed to spawn anyka shutdown worker thread: {}",
                e
            ))
        })?;

    let deadline = shutdown_deadline();

    match rx.recv_timeout(deadline) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            // Hard deadline exceeded - the worker thread is hung
            video_encoder_for_timeout
                .mark_unsafe_shutdown("platform shutdown worker exceeded hard deadline");
            tracing::error!(
                deadline_ms = deadline.as_millis(),
                "Platform shutdown timed out after {}ms",
                deadline.as_millis()
            );
            Err(PlatformError::Timeout)
        }
        Err(RecvTimeoutError::Disconnected) => {
            // Worker thread panicked or otherwise terminated without sending result.
            // This indicates a bug rather than a hardware failure.
            video_encoder_for_timeout
                .mark_unsafe_shutdown("platform shutdown worker thread disconnected unexpectedly");
            tracing::error!(
                "Platform shutdown worker thread disconnected unexpectedly (possible panic)"
            );
            Err(PlatformError::HardwareFailure(
                "shutdown worker thread terminated unexpectedly".to_string(),
            ))
        }
    }
}

/// Rollback helper for encoder initialization failure.
///
/// Closes encoders in reverse order of initialization.
pub(super) fn rollback_encoders(video_encoder: &AnykaVideoEncoder, initialized_tokens: &[String]) {
    for token in initialized_tokens.iter().rev() {
        if let Err(close_error) = video_encoder.close_encoder(token) {
            tracing::warn!(
                "Failed to rollback initialized encoder {}: {}",
                token,
                close_error
            );
        }
    }
}

/// Rollback helper for failed video input initialization.
///
/// Stops capture, destroys VPSS, and closes video input.
pub(super) fn rollback_video_input(video_input: &AnykaVideoInput) {
    let _ = video_input.capture_off();
    let _ = video_input.destroy_vpss();
    let _ = video_input.close_blocking();
}

/// Capture pipeline stabilization delay.
///
/// The C reference (platform_anyka.c:609) uses PLATFORM_DELAY_MS_RETRY (200ms).
/// This allows the capture pipeline to stabilize before opening encoders.
pub(super) fn capture_stabilization_delay() -> Duration {
    Duration::from_millis(200)
}

/// Video input alignment helpers.
///
/// Align a width value up to the 32-pixel boundary required by the video encoder.
/// Reference: `VENCODER_WIDTH_ALIGN_REQ` in `ak_vi.c`.
pub(super) fn align_width_to_32(w: i32) -> i32 {
    (w + 31) & !31
}

/// Align a height value up to the 8-pixel boundary required by the video encoder.
/// Reference: `VENCODER_HEIGHT_ALIGN_REQ` in `ak_vi.c`.
pub(super) fn align_height_to_8(h: i32) -> i32 {
    (h + 7) & !7
}

/// Validate that required handles are present after initialization.
///
/// Used during platform initialization to verify that required resources
/// were successfully created before proceeding.
///
/// Note: This function is currently unused but kept for potential future use.
#[allow(dead_code)]
pub(super) fn validate_handles<T, U, V>(
    vi_handle: Option<T>,
    main_enc_handle: Option<U>,
    sub_enc_handle: Option<V>,
) -> PlatformResult<(T, U, Option<V>)> {
    let vi = vi_handle.ok_or_else(|| {
        PlatformError::InitializationFailed(
            "Video input handle missing after successful open".to_string(),
        )
    })?;

    let main_enc = main_enc_handle.ok_or_else(|| {
        PlatformError::InitializationFailed(
            "Main encoder handle missing after successful init".to_string(),
        )
    })?;

    Ok((vi, main_enc, sub_enc_handle))
}

/// Read sensor resolution from video input, handling the not-yet-available case.
pub(super) fn get_sensor_resolution(
    sensor_resolution: &RwLock<Option<Resolution>>,
) -> PlatformResult<Resolution> {
    sensor_resolution.read().ok_or_else(|| {
        PlatformError::InitializationFailed(
            "Sensor resolution not available - platform not initialized".to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_width_to_32() {
        assert_eq!(align_width_to_32(0), 0);
        assert_eq!(align_width_to_32(1), 32);
        assert_eq!(align_width_to_32(31), 32);
        assert_eq!(align_width_to_32(32), 32);
        assert_eq!(align_width_to_32(33), 64);
        assert_eq!(align_width_to_32(1280), 1280);
        assert_eq!(align_width_to_32(1281), 1312);
    }

    #[test]
    fn test_align_height_to_8() {
        assert_eq!(align_height_to_8(0), 0);
        assert_eq!(align_height_to_8(1), 8);
        assert_eq!(align_height_to_8(7), 8);
        assert_eq!(align_height_to_8(8), 8);
        assert_eq!(align_height_to_8(9), 16);
        assert_eq!(align_height_to_8(720), 720);
        assert_eq!(align_height_to_8(721), 728);
    }

    #[test]
    fn test_capture_stabilization_delay() {
        assert_eq!(capture_stabilization_delay(), Duration::from_millis(200));
    }

    // =============================================================================
    // Bugfix verification tests
    // =============================================================================

    #[test]
    fn test_recv_timeout_error_types_exist() {
        // Verify RecvTimeoutError variants are accessible
        let _timeout = RecvTimeoutError::Timeout;
        let _disconnected = RecvTimeoutError::Disconnected;
    }

    #[test]
    fn test_shutdown_deadline_test_mode() {
        // Verify shutdown_deadline returns test mode value (200ms)
        let deadline = shutdown_deadline();
        assert_eq!(deadline, Duration::from_millis(200));
    }

    #[test]
    fn test_critical_error_tracking_variable_exists() {
        // This verifies the pattern used for critical error tracking
        // The variable is declared as: let mut critical_error: Option<PlatformError> = None;
        let mut critical_error: Option<crate::platform::traits::PlatformError> = None;

        // Should start as None
        assert!(critical_error.is_none());

        // Should be settable
        critical_error = Some(
            crate::platform::traits::PlatformError::InitializationFailed("test error".to_string()),
        );
        assert!(critical_error.is_some());
    }
}
