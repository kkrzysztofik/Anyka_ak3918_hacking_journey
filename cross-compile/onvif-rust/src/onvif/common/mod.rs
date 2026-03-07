//! Common ONVIF infrastructure modules.
//!
//! This module provides a single facade for the shared infrastructure that all
//! ONVIF services depend on. It re-exports the foundational modules (SOAP,
//! error handling, authentication, types, discovery, etc.) without moving any
//! code.
//!
//! # Usage
//!
//! Consumers can import via either the original path or the common facade:
//!
//! ```ignore
//! // Original (still works):
//! use crate::onvif::soap::SoapEnvelope;
//!
//! // Via common facade (also works):
//! use crate::onvif::common::soap::SoapEnvelope;
//! ```
//!
//! # New in Phase 0
//!
//! Additional common utilities were added in Phase 0 of the refactoring:
//! - `validation` — Shared token and range validators
//! - `faults` — SOAP fault builder helpers
//! - `dispatch` — Dispatch boilerplate helpers
//!
//! # What is NOT included
//!
//! Service-specific modules are NOT part of common:
//! - `device/` — Device Management Service
//! - `media/` — Media Service
//! - `ptz/` — PTZ Service
//! - `imaging/` — Imaging Service

pub use super::auth_requirements;
pub use super::discovery;
pub use super::dispatcher;
pub use super::error;
pub use super::server;
pub use super::soap;
pub use super::types;
pub use super::ws_security;

// New Phase 0 modules
mod dispatch;
mod faults;
mod service_capabilities;
mod validation;

// Re-export new modules for external use
pub use dispatch::{dispatch_async, dispatch_sync};
pub use faults::{action_not_supported, hardware_failure, invalid_token, no_entity, out_of_range};
pub use service_capabilities::{
    GetServiceCapabilities, GetServiceCapabilitiesResponse as SharedGetServiceCapabilitiesResponse,
};
pub use validation::{
    validate_max_length, validate_range_f32, validate_range_i32, validate_reference_token,
};

#[cfg(test)]
mod tests {
    use super::*;

    // Test that all 8 re-exported modules are accessible through the facade
    // Using compile-time type checks rather than runtime tests

    /// Verify auth_requirements module is accessible
    const _AUTH_MOD: () = assert!(
        std::mem::size_of::<auth_requirements::AuthLevel>() > 0,
        "auth_requirements::AuthLevel should be accessible"
    );

    /// Verify dispatcher module is accessible (has ServiceDispatcher)
    const _DISPATCHER_MOD: () = assert!(
        std::mem::size_of::<dispatcher::ServiceDispatcher>() > 0,
        "dispatcher::ServiceDispatcher should be accessible"
    );

    /// Verify discovery module is accessible
    const _DISCOVERY_MOD: () = assert!(
        std::mem::size_of::<discovery::WsDiscovery>() > 0,
        "discovery::WsDiscovery should be accessible"
    );

    /// Verify error module is accessible (has OnvifError)
    const _ERROR_MOD: () = assert!(
        std::mem::size_of::<error::OnvifError>() > 0,
        "error::OnvifError should be accessible"
    );

    /// Verify server module is accessible (has OnvifServer)
    const _SERVER_MOD: () = assert!(
        std::mem::size_of::<server::OnvifServer>() > 0,
        "server::OnvifServer should be accessible"
    );

    /// Verify soap module is accessible (has SoapEnvelope)
    const _SOAP_MOD: () = assert!(
        std::mem::size_of::<soap::SoapEnvelope<String>>() > 0,
        "soap::SoapEnvelope should be accessible"
    );

    /// Verify types module is accessible (has OnvifVersion)
    const _TYPES_MOD: () = assert!(
        std::mem::size_of::<types::OnvifVersion>() > 0,
        "types::OnvifVersion should be accessible"
    );

    /// Verify ws_security module is accessible (has WsSecurityConfig)
    const _WS_SEC_MOD: () = assert!(
        std::mem::size_of::<ws_security::WsSecurityConfig>() > 0,
        "ws_security::WsSecurityConfig should be accessible"
    );

    /// Test that re-exported types work correctly
    #[test]
    fn test_soap_envelope_construction() {
        // Verify SoapEnvelope can be constructed via re-export
        let envelope: soap::SoapEnvelope<String> = soap::SoapEnvelope {
            header: None,
            body: soap::SoapBody {
                content: "test".to_string(),
            },
        };

        // Verify body content
        assert_eq!(envelope.body.content, "test");
    }

    /// Test that re-exported error types work correctly
    #[test]
    fn test_onvif_error_construction() {
        // Verify OnvifError can be constructed via re-export
        let err = error::OnvifError::ActionNotSupported("test action".to_string());

        // Verify the error message
        assert!(err.to_string().contains("Action not supported"));
    }

    /// Test that re-exported WsSecurityConfig works correctly
    #[test]
    fn test_ws_security_config_default() {
        // Verify WsSecurityConfig can be constructed via re-export
        let config = ws_security::WsSecurityConfig::default();

        // Verify default values
        assert!(config.require_digest);
        assert_eq!(config.clock_skew_seconds, 300);
        assert_eq!(config.nonce_ttl_seconds, 300);
        assert_eq!(config.max_nonce_cache_size, 10000);
    }

    /// Test that re-exported AuthLevel works correctly
    #[test]
    fn test_auth_level_variants() {
        // Verify all AuthLevel variants are accessible
        assert_eq!(
            auth_requirements::AuthLevel::Anonymous,
            auth_requirements::AuthLevel::Anonymous
        );
        assert_eq!(
            auth_requirements::AuthLevel::User,
            auth_requirements::AuthLevel::User
        );
        assert_eq!(
            auth_requirements::AuthLevel::Operator,
            auth_requirements::AuthLevel::Operator
        );
        assert_eq!(
            auth_requirements::AuthLevel::Administrator,
            auth_requirements::AuthLevel::Administrator
        );
    }

    /// Test that types module re-exports work
    #[test]
    fn test_types_re_exports() {
        // Verify OnvifVersion can be constructed via re-export
        let version = types::OnvifVersion { major: 2, minor: 0 };
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 0);
    }

    // Tests for new Phase 0 modules

    /// Test validation module functions are accessible
    #[test]
    fn test_validation_module_exports() {
        // validate_reference_token
        assert!(validate_reference_token("valid_token", "test").is_ok());
        assert!(validate_reference_token("", "test").is_err());

        // validate_range_f32
        assert!(validate_range_f32(50.0, 0.0, 100.0, "test").is_ok());
        assert!(validate_range_f32(-1.0, 0.0, 100.0, "test").is_err());

        // validate_range_i32
        assert!(validate_range_i32(5, 1, 10, "test").is_ok());
        assert!(validate_range_i32(0, 1, 10, "test").is_err());

        // validate_max_length
        assert!(validate_max_length("short", 64, "test").is_ok());
        assert!(validate_max_length("a".repeat(100).as_str(), 64, "test").is_err());
    }

    /// Test faults module functions are accessible
    #[test]
    fn test_faults_module_exports() {
        // invalid_token
        let err = invalid_token("ProfileToken", "BadToken");
        assert!(err.to_string().contains("ProfileToken"));

        // no_entity
        let err = no_entity("ter:NotFound", "profile", "Token123");
        assert!(err.to_string().contains("profile"));

        // action_not_supported
        let err = action_not_supported("GetUnknown");
        assert!(err.to_string().contains("GetUnknown"));

        // out_of_range
        let err = out_of_range("brightness", 150.0, 0.0, 100.0);
        assert!(err.to_string().contains("brightness"));

        // hardware_failure
        let err = hardware_failure("PTZ", "Motor stuck");
        assert!(err.to_string().contains("PTZ"));
    }

    /// Test dispatch module functions are accessible
    #[test]
    fn test_dispatch_module_exports() {
        // dispatch_sync should be callable (function type exists)
        fn _test_fn(
            _req: crate::onvif::common::dispatch::TestRequestDispatch,
        ) -> crate::onvif::OnvifResult<crate::onvif::common::dispatch::TestResponseDispatch>
        {
            Ok(crate::onvif::common::dispatch::TestResponseDispatch {
                result: "ok".to_string(),
            })
        }
        // Function exists and has correct signature
        let _handler: fn(
            crate::onvif::common::dispatch::TestRequestDispatch,
        ) -> crate::onvif::OnvifResult<
            crate::onvif::common::dispatch::TestResponseDispatch,
        > = _test_fn;

        // Test that the dispatch functions exist and can be used
        // We test actual functionality in the dispatch module's own tests
        // Here we just verify the types work
        let test_request = crate::onvif::common::dispatch::TestRequestDispatch {
            value: Some("test".to_string()),
        };
        let _request = test_request;

        let test_response = crate::onvif::common::dispatch::TestResponseDispatch {
            result: "test".to_string(),
        };
        let _response = test_response;
    }
}
