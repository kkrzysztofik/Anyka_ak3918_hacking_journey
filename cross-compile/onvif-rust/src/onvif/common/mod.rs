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
}
