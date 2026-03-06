//! Analytics Service specific fault types and mappings.
//!
//! This module defines faults specific to the Analytics Service operations
//! as defined in the ONVIF Analytics Service WSDL specification.

use crate::onvif::error::OnvifError;

// ============================================================================
// Analytics Module Faults
// ============================================================================

/// Create an AnalyticsModuleNotSupported fault.
///
/// Used when the specified analytics module type is not supported.
pub fn analytics_module_not_supported(module_type: &str) -> OnvifError {
    OnvifError::ActionNotSupported(format!(
        "Analytics module type not supported: {}",
        module_type
    ))
}

/// Create a RuleNotSupported fault.
///
/// Used when rule operations are requested but not supported.
pub fn rule_not_supported() -> OnvifError {
    OnvifError::ActionNotSupported("Rule operations are not supported".to_string())
}

/// Create an AnalyticsModuleNotFound fault.
///
/// Used when the specified analytics module cannot be found.
pub fn analytics_module_not_found(name: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:NoConfig".to_string(),
        reason: format!("Analytics module not found: {}", name),
    }
}

/// Create an InvalidConfigurationToken fault.
///
/// Used when the configuration token is invalid or not recognized.
pub fn invalid_configuration_token(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidToken".to_string(),
        reason: format!("Invalid configuration token: {}", token),
    }
}

/// Create a DuplicateName fault.
///
/// Used when attempting to create an analytics module with a name that already exists.
pub fn duplicate_name(name: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:Duplicate".to_string(),
        reason: format!("Analytics module name already exists: {}", name),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_module_not_supported() {
        let err = analytics_module_not_supported("SomeModuleType");
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        assert!(err.to_string().contains("SomeModuleType"));
    }

    #[test]
    fn test_rule_not_supported() {
        let err = rule_not_supported();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        assert!(err.to_string().contains("Rule"));
    }

    #[test]
    fn test_analytics_module_not_found() {
        let err = analytics_module_not_found("MyModule");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:NoConfig")
        );
        assert!(err.to_string().contains("MyModule"));
    }

    #[test]
    fn test_invalid_configuration_token() {
        let err = invalid_configuration_token("BadToken");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:InvalidToken")
        );
        assert!(err.to_string().contains("BadToken"));
    }

    #[test]
    fn test_duplicate_name() {
        let err = duplicate_name("ExistingModule");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:Duplicate")
        );
        assert!(err.to_string().contains("ExistingModule"));
    }
}
