use std::sync::atomic::{AtomicBool, Ordering};

/// Controls high-frequency per-frame streaming debug logs.
///
/// Disabled by default because per-frame logging is expensive on embedded
/// targets and can materially reduce effective streaming performance.
static STREAM_FRAME_DEBUG_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable per-frame streaming debug logs.
pub fn set_stream_frame_debug_logging(enabled: bool) {
    STREAM_FRAME_DEBUG_LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Returns whether per-frame streaming debug logs are enabled.
pub fn stream_frame_debug_logging_enabled() -> bool {
    STREAM_FRAME_DEBUG_LOGGING_ENABLED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::{set_stream_frame_debug_logging, stream_frame_debug_logging_enabled};

    #[test]
    fn test_stream_frame_debug_logging_toggle_roundtrip() {
        set_stream_frame_debug_logging(false);
        assert!(!stream_frame_debug_logging_enabled());

        set_stream_frame_debug_logging(true);
        assert!(stream_frame_debug_logging_enabled());

        // Restore default to avoid surprising test interactions.
        set_stream_frame_debug_logging(false);
    }
}
