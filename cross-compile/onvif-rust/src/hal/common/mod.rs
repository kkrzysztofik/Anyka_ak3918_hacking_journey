//! Shared HAL abstractions: traits, RAII handles, utility functions, and SDK type mirrors.

pub mod sdk_types;
// NOSONAR: Wildcard re-export of sdk_types is standard pattern for facade modules.
pub use sdk_types::*;

pub mod audio;
pub mod imaging;
pub mod ptz;
pub mod video;

/// Anyka SDK success code as i32 for consistent comparisons.
#[allow(clippy::unnecessary_cast)]
pub const AK_SUCCESS_I32: i32 = AK_SUCCESS as i32;
/// Anyka SDK failure code as i32 for consistent comparisons.
#[allow(clippy::unnecessary_cast)]
pub const AK_FAILED_I32: i32 = AK_FAILED as i32;

/// Convert SDK return codes to `PlatformResult`.
///
/// Shared by all HAL submodules (audio, imaging, video, ptz).
///
/// # Arguments
///
/// * `ret` - SDK return code (0 = AK_SUCCESS, -1 = AK_FAILED, or other error codes)
/// * `context` - Context string for error messages
///
/// # Returns
///
/// * `Ok(())` if `ret == AK_SUCCESS`
/// * `Err(PlatformError::HardwareFailure(...))` otherwise
pub(crate) fn check_result(ret: i32, context: &str) -> crate::platform::PlatformResult<()> {
    match ret {
        AK_SUCCESS_I32 => Ok(()),
        AK_FAILED_I32 => Err(crate::platform::PlatformError::HardwareFailure(format!(
            "{} failed",
            context
        ))),
        _ => Err(crate::platform::PlatformError::HardwareFailure(format!(
            "{}: error code {}",
            context, ret
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformError;

    #[test]
    fn test_check_result_success() {
        let result = check_result(AK_SUCCESS_I32, "test_function");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_result_failed() {
        let result = check_result(AK_FAILED_I32, "test_function");
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("test_function"));
                assert!(msg.contains("failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_check_result_unknown_error() {
        let result = check_result(-42, "test_function");
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("test_function"));
                assert!(msg.contains("error code -42"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
