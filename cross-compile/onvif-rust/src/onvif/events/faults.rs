//! Events Service specific fault types and mappings.
//!
//! This module defines faults specific to the Events Service operations
//! as defined in the ONVIF Events Service WSDL specification.

use crate::onvif::error::OnvifError;

// ============================================================================
// Subscription Faults
// ============================================================================

/// Create a SubscriptionNotSupported fault.
///
/// Returns `ActionNotSupported` because this scaffold implementation does not
/// provide a WS-BaseNotification subscription manager. ONVIF clients that
/// attempt `Subscribe` will receive this fault, directing them to use
/// `CreatePullPointSubscription` instead (which is also unsupported in the
/// current scaffold — see [`pull_point_not_supported`]).
pub fn subscription_not_supported() -> OnvifError {
    OnvifError::ActionNotSupported("PullPoint subscription not supported".to_string())
}

/// Create a PullPointNotSupported fault.
///
/// Returns `ActionNotSupported` because pull-point event delivery requires
/// server-side subscription state and message buffering that are not yet
/// implemented. This is distinct from [`subscription_not_supported`] which
/// covers the WS-Notification `Subscribe` path; this fault covers
/// `PullMessages`, `Seek`, and related pull-point operations.
pub fn pull_point_not_supported() -> OnvifError {
    OnvifError::ActionNotSupported("Pull point operations not supported".to_string())
}

/// Create an InvalidSubscription fault.
///
/// Used when the subscription token is invalid or expired.
pub fn invalid_subscription(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidSubscription".to_string(),
        reason: format!("Invalid subscription: {}", token),
    }
}

/// Create a TopicNotSupported fault.
///
/// Used when the specified topic is not recognized.
pub fn topic_not_supported(topic: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:TopicNotSupported".to_string(),
        reason: format!("Topic not supported: {}", topic),
    }
}

/// Create a FilterNotSupported fault.
///
/// Used when the specified filter expression is not supported.
pub fn filter_not_supported() -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:FilterNotSupported".to_string(),
        reason: "Filter expression not supported".to_string(),
    }
}

/// Create a InvalidFilter fault.
///
/// Used when the filter syntax is invalid.
pub fn invalid_filter(reason: &str) -> OnvifError {
    OnvifError::Internal(format!("Invalid filter: {}", reason))
}

/// Create a NotificationStreamFault fault.
///
/// Returns `Internal` because notification stream errors are server-side
/// failures (e.g. channel closed, buffer overflow) rather than client input
/// errors. This maps to the ONVIF `tev:NotificationStreamFault` fault code.
pub fn notification_stream_fault(reason: &str) -> OnvifError {
    OnvifError::Internal(format!("Notification stream error: {}", reason))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_not_supported() {
        let err = subscription_not_supported();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        assert!(err.to_string().contains("PullPoint"));
    }

    #[test]
    fn test_pull_point_not_supported() {
        let err = pull_point_not_supported();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        // Check that it mentions pull point
        assert!(err.to_string().to_lowercase().contains("pull"));
    }

    #[test]
    fn test_invalid_subscription() {
        let err = invalid_subscription("ExpiredToken123");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:InvalidSubscription")
        );
        assert!(err.to_string().contains("ExpiredToken123"));
    }

    #[test]
    fn test_topic_not_supported() {
        let err = topic_not_supported("tns1:Device/SomeEvent");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:TopicNotSupported")
        );
        assert!(err.to_string().contains("SomeEvent"));
    }

    #[test]
    fn test_filter_not_supported() {
        let err = filter_not_supported();
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:FilterNotSupported")
        );
    }

    #[test]
    fn test_invalid_filter() {
        let err = invalid_filter("mismatched parentheses");
        assert!(matches!(err, OnvifError::Internal(_)));
        assert!(err.to_string().contains("mismatched"));
    }

    #[test]
    fn test_notification_stream_fault() {
        let err = notification_stream_fault("connection lost");
        assert!(matches!(err, OnvifError::Internal(_)));
        assert!(err.to_string().contains("connection lost"));
    }
}
